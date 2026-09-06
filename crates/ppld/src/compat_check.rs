use std::io::{self, Write};

use crossterm::{
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};
use icy_board_engine::executable::{Executable, ImplStatus, PPECommand, PPEExpr, PPEScript, function_status, statement_status};

struct UsageHit {
    span_start: usize,
    name: String,
    status: ImplStatus,
    is_function: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CompatibilitySummary {
    pub unimplemented: usize,
    pub unsupported: usize,
    pub partial: usize,
}

impl CompatibilitySummary {
    pub fn total(self) -> usize {
        self.unimplemented + self.unsupported + self.partial
    }

    /// Strict mode rejects every reported category, including partial support.
    pub fn has_findings(self) -> bool {
        self.total() != 0
    }
}

pub struct CompatibilityReport {
    pub summary: CompatibilitySummary,
    hits: Vec<UsageHit>,
}

// The compatibility tables live in `icy_board_engine::executable::compat` and are
// kept in sync with the VM by the `opcode_coverage` test in that crate.

fn classify_statement(name: &str) -> Option<ImplStatus> {
    match statement_status(name) {
        ImplStatus::Implemented | ImplStatus::Invalid => None,
        status => Some(status),
    }
}

fn classify_function(name: &str) -> Option<ImplStatus> {
    match function_status(name) {
        ImplStatus::Implemented | ImplStatus::Invalid => None,
        status => Some(status),
    }
}

/// Recursively walk expressions to find predefined function calls.
fn collect_expr_hits(expr: &PPEExpr, hits: &mut Vec<UsageHit>, span_start: usize) {
    match expr {
        PPEExpr::PredefinedFunctionCall(def, args) => {
            if let Some(status) = classify_function(def.name) {
                hits.push(UsageHit {
                    span_start,
                    name: def.name.to_string(),
                    status,
                    is_function: true,
                });
            }
            for a in args {
                collect_expr_hits(a, hits, span_start);
            }
        }
        PPEExpr::UnaryExpression(_, inner) => collect_expr_hits(inner, hits, span_start),
        PPEExpr::BinaryExpression(_, l, r) => {
            collect_expr_hits(l, hits, span_start);
            collect_expr_hits(r, hits, span_start);
        }
        PPEExpr::Member(inner, _) => collect_expr_hits(inner, hits, span_start),
        PPEExpr::MemberFunctionCall(obj, args, _) => {
            collect_expr_hits(obj, hits, span_start);
            for a in args {
                collect_expr_hits(a, hits, span_start);
            }
        }
        PPEExpr::Dim(_, dims) => {
            for d in dims {
                collect_expr_hits(d, hits, span_start);
            }
        }
        PPEExpr::FunctionCall(_, args) => {
            for a in args {
                collect_expr_hits(a, hits, span_start);
            }
        }
        PPEExpr::RecordLiteral(_, fields) => {
            for (_, value) in fields {
                collect_expr_hits(value, hits, span_start);
            }
        }
        PPEExpr::IndexedMember(base, _, dimensions) => {
            collect_expr_hits(base, hits, span_start);
            for dimension in dimensions {
                collect_expr_hits(dimension, hits, span_start);
            }
        }
        PPEExpr::Value(_) | PPEExpr::RoutineReference(_) | PPEExpr::Invalid => {}
    }
}

/// Walk a statement + its expressions.
fn collect_statement_hits(stmt: &icy_board_engine::executable::PPEStatement, hits: &mut Vec<UsageHit>) {
    let span_start = stmt.span.start;
    match &stmt.command {
        PPECommand::PredefinedCall(def, args) => {
            if let Some(status) = classify_statement(def.name) {
                hits.push(UsageHit {
                    span_start,
                    name: def.name.to_string(),
                    status,
                    is_function: false,
                });
            }
            for a in args {
                collect_expr_hits(a, hits, span_start);
            }
        }
        PPECommand::ProcedureCall(_, args) => {
            for a in args {
                collect_expr_hits(a, hits, span_start);
            }
        }
        PPECommand::IfNot(cond, _) => {
            collect_expr_hits(cond, hits, span_start);
        }
        PPECommand::Let(target, value) => {
            collect_expr_hits(target, hits, span_start);
            collect_expr_hits(value, hits, span_start);
        }
        PPECommand::MemberCall(expr) => {
            collect_expr_hits(expr, hits, span_start);
        }
        PPECommand::ForEach(_, collection, _) => {
            collect_expr_hits(collection, hits, span_start);
        }
        PPECommand::Return
        | PPECommand::End
        | PPECommand::Goto(_)
        | PPECommand::Gosub(_)
        | PPECommand::OnError(_)
        | PPECommand::EndFunc
        | PPECommand::EndProc
        | PPECommand::Stop
        | PPECommand::NextForEach(_) => {}
    }
}

/// Analyze without performing I/O. Findings are successful analysis results, not errors.
pub fn check_compatibility(executable: &Executable) -> Result<CompatibilityReport, Box<dyn std::error::Error>> {
    let script = PPEScript::from_ppe_file(executable).map_err(|e| format!("Failed to deserialize PPE: {e}"))?;

    let mut hits: Vec<UsageHit> = Vec::new();
    for stmt in &script.statements {
        collect_statement_hits(stmt, &mut hits);
    }

    let mut summary = CompatibilitySummary::default();
    for hit in &hits {
        match hit.status {
            ImplStatus::Unimplemented => summary.unimplemented += 1,
            ImplStatus::Unsupported => summary.unsupported += 1,
            ImplStatus::Partial => summary.partial += 1,
            ImplStatus::Implemented | ImplStatus::Invalid => {}
        }
    }

    // Preserve individual references (including repeats at the same location).
    // Stable offset ordering retains expression traversal order for ties.
    hits.sort_by_key(|hit| hit.span_start);
    Ok(CompatibilityReport { summary, hits })
}

impl CompatibilityReport {
    /// Render to the caller's stream. Only output failures are returned here.
    pub fn write_report(&self, mut output: impl Write) -> io::Result<()> {
        if !self.summary.has_findings() {
            execute!(
                output,
                SetForegroundColor(Color::Green),
                Print("✓ "),
                ResetColor,
                Print("No unsupported / unimplemented features detected.\n")
            )?;
            return Ok(());
        }

        execute!(
            output,
            SetAttribute(Attribute::Bold),
            SetForegroundColor(Color::Yellow),
            Print("Compatibility Report\n"),
            ResetColor,
            SetAttribute(Attribute::Reset),
            Print("--------------------------------------\n")
        )?;

        // Explicit category order, independent of hash iteration or locale.
        for status in [ImplStatus::Unimplemented, ImplStatus::Unsupported, ImplStatus::Partial] {
            let mut hits = self.hits.iter().filter(|hit| hit.status == status).peekable();
            if hits.peek().is_none() {
                continue;
            }
            let (title, color) = match status {
                ImplStatus::Unimplemented => ("Unimplemented", Color::Red),
                ImplStatus::Unsupported => ("Unsupported (stubbed)", Color::Magenta),
                ImplStatus::Partial => ("Partially Implemented", Color::Yellow),
                // `classify_*` never yields these, they are filtered out beforehand.
                ImplStatus::Implemented | ImplStatus::Invalid => continue,
            };
            execute!(
                output,
                SetAttribute(Attribute::Bold),
                Print(format!("{title}:\n")),
                SetAttribute(Attribute::Reset),
            )?;

            for h in hits {
                execute!(
                    output,
                    Print(format!("  [{:04X}] ", h.span_start)),
                    SetForegroundColor(color),
                    Print(if h.is_function {
                        format!("FUNCTION {}\n", h.name.to_ascii_uppercase())
                    } else {
                        format!("STATEMENT {}\n", h.name.to_ascii_uppercase())
                    }),
                    ResetColor
                )?;
            }
            writeln!(output)?;
        }

        let summary = self.summary;
        execute!(
            output,
            SetAttribute(Attribute::Bold),
            Print("Summary: ".to_string()),
            SetAttribute(Attribute::Reset),
            Print(format!("{} references -> ", summary.total())),
            SetForegroundColor(Color::Red),
            Print(format!("{} unimplemented ", summary.unimplemented)),
            ResetColor,
            SetForegroundColor(Color::Magenta),
            Print(format!("{} unsupported ", summary.unsupported)),
            ResetColor,
            SetForegroundColor(Color::Yellow),
            Print(format!("{} partial\n", summary.partial)),
            ResetColor
        )?;

        execute!(
            output,
            Print("\nRecommendation: Review or replace the above items for full runtime compatibility.\n")
        )?;

        Ok(())
    }
}

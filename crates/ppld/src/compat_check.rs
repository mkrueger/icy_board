use std::{collections::HashMap, io::stdout};

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
        PPECommand::Return
        | PPECommand::End
        | PPECommand::Goto(_)
        | PPECommand::Gosub(_)
        | PPECommand::OnError(_)
        | PPECommand::EndFunc
        | PPECommand::EndProc
        | PPECommand::Stop => {}
    }
}

pub fn check_compatibility(executable: &Executable) -> Result<(), Box<dyn std::error::Error>> {
    let script = PPEScript::from_ppe_file(executable).map_err(|e| format!("Failed to deserialize PPE: {e}"))?;

    let mut hits: Vec<UsageHit> = Vec::new();
    for stmt in &script.statements {
        collect_statement_hits(stmt, &mut hits);
    }

    // Deduplicate by (name, status, is_function, span_start) to keep location info separate.
    // Keep as-is; If you want to collapse locations per name, you can group later.
    if hits.is_empty() {
        execute!(
            stdout(),
            SetForegroundColor(Color::Green),
            Print("✓ "),
            ResetColor,
            Print("No unsupported / unimplemented features detected.\n")
        )?;
        return Ok(());
    }

    // Group by status for nicer output ordering.
    let mut grouped: HashMap<ImplStatus, Vec<&UsageHit>> = HashMap::new();
    for h in &hits {
        grouped.entry(h.status).or_default().push(h);
    }

    execute!(
        stdout(),
        SetAttribute(Attribute::Bold),
        SetForegroundColor(Color::Yellow),
        Print("Compatibility Report\n"),
        ResetColor,
        SetAttribute(Attribute::Reset),
        Print("--------------------------------------\n")
    )?;

    let order = [ImplStatus::Unimplemented, ImplStatus::Unsupported, ImplStatus::Partial];

    for status in order {
        if let Some(list) = grouped.get(&status) {
            if list.is_empty() {
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
                stdout(),
                SetAttribute(Attribute::Bold),
                Print(format!("{title}:\n")),
                SetAttribute(Attribute::Reset),
            )?;

            // Sort by offset for readability
            let mut sorted = list.clone();
            sorted.sort_by_key(|h| h.span_start);

            for h in sorted {
                execute!(
                    stdout(),
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
            println!();
        }
    }

    // Summary
    let total = hits.len();
    let unimpl = grouped.get(&ImplStatus::Unimplemented).map(|v| v.len()).unwrap_or(0);
    let unsup = grouped.get(&ImplStatus::Unsupported).map(|v| v.len()).unwrap_or(0);
    let partial = grouped.get(&ImplStatus::Partial).map(|v| v.len()).unwrap_or(0);

    execute!(
        stdout(),
        SetAttribute(Attribute::Bold),
        Print("Summary: ".to_string()),
        SetAttribute(Attribute::Reset),
        Print(format!("{total} references -> ")),
        SetForegroundColor(Color::Red),
        Print(format!("{unimpl} unimplemented ")),
        ResetColor,
        SetForegroundColor(Color::Magenta),
        Print(format!("{unsup} unsupported ")),
        ResetColor,
        SetForegroundColor(Color::Yellow),
        Print(format!("{partial} partial\n")),
        ResetColor
    )?;

    execute!(
        stdout(),
        Print("\nRecommendation: Review or replace the above items for full runtime compatibility.\n")
    )?;

    Ok(())
}

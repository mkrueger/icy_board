use std::{collections::HashMap, io::stdout};

use crossterm::{
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};
use icy_board_engine::executable::{Executable, PPECommand, PPEExpr, PPEScript};

/// Status categories for reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ImplStatus {
    Unsupported,   // Present, but intentionally not supported (logs warn) e.g. SOUNDDELAY
    Unimplemented, // Stubbed with unimplemented_stmt!/function!
    Partial,       // Partially implemented (works but missing features)
}

struct UsageHit {
    span_start: usize,
    name: String,
    status: ImplStatus,
    is_function: bool,
}

// Curated lists:
// NOTE: Keep names matching the canonical `StatementDefinition.name` or `FunctionDefinition.name` (case-insensitive compare).
// Statement names (from STATEMENT_DEFINITIONS) often use mixed case (e.g. "SoundDelay"); functions use their defined names.
const UNSUPPORTED_STATEMENTS: &[&str] = &[
    "SOUND",
    "SOUNDDELAY", // logs warn only
];

pub const UNIMPLEMENTED_STATEMENTS: &[&str] = &[
    "FPUTPAD",
    "DOINTR",
    "VARSEG",
    "VAROFF",
    "POKEB",
    "POKEW",
    "VARADDR",
    "WRUSYSDOOR",
    "WRUSYS",
    "RDUSYS",
    "OPENCAP",
    "CLOSECAP",
    "POKEDW",
    "FFLUSH",
    "FDPUTPAD",
    "TPAGET",
    "TPAPUT",
    "TPACGEA",
    "TPACPUT",
    "TPAREAD",
    "TPAWRITE",
    "TPACREAD",
    "TPACWRITE",
    "SETLMR",
    "STACKABORT",
    "DCREATE",
    "DOPEN",
    "DCLOSE",
    "DSETALIAS",
    "DPACK",
    "DCLOSEALL",
    "DLOCK",
    "DLOCKR",
    "DLOCKG",
    "DUNLOCK",
    "DNCREATE",
    "DNOPEN",
    "DNCLOSE",
    "DNCLOSEALL",
    "DNEW",
    "DADD",
    "DAPPEND",
    "DTOP",
    "DGO",
    "DBOTTOM",
    "DSKIP",
    "DBLANK",
    "DDELETE",
    "DRECALL",
    "DTAG",
    "DSEEK",
    "DFBLANK",
    "DGET",
    "DPUT",
    "DFCOPY",
    "KILLMSG",
    "FDOWRAKA",
    "FDOADDAKA",
    "FDOWRORG",
    "FDOADDOR",
    "FDOQMOD",
    "FDOQADD",
    "FDOQDEL",
    "MOVE_MSG",
];
const PARTIAL_STATEMENTS: &[&str] = &[
    // Add statements that are implemented but missing edge cases
];

const UNSUPPORTED_FUNCTIONS: &[&str] = &[
    // (If any functions only warn)
];

const UNIMPLEMENTED_FUNCTIONS: &[&str] = &[
    "REGAL",
    "REGAH",
    "REGBL",
    "REGBH",
    "REGCL",
    "REGCH",
    "REGDL",
    "REGDH",
    "REGAX",
    "REGBX",
    "REGCX",
    "REGDX",
    "REGSI",
    "REGDI",
    "REGF",
    "REGCF",
    "REGDS",
    "REGES",
    "PEEKB",
    "PEEKW",
    "MODEM",
    "CALLNUM",
    "MGETBYTE",
    "EVTTIMEADJ",
    "FMTREAL",
    "KBDBUFSIZE",
    "KBDFILUSUED",
    "DRIVESPACE",
    "DGETALIAS",
    "DBOF",
    "DCHANGED",
    "DDECIMALS",
    "DDELETED",
    "DEOF",
    "DERR",
    "DFIELDS",
    "DLENGTH",
    "DNAME",
    "DRECCOUNT",
    "DRECNO",
    "DTYPE",
    "DNEXT",
    "TODDATE",
    "DCLOSEALL",
    "DOPEN",
    "DCLOSE",
    "DSETALIAS",
    "DPACK",
    "DLOCKF",
    "DLOCK",
    "DLOCKR",
    "DUNLOCK",
    "DNOPEN",
    "DNCLOSE",
    "DNCLOSEALL",
    "DNEW",
    "DADD",
    "DAPPEND",
    "DTOP",
    "DGO",
    "DBOTTOM",
    "DSKIP",
    "DBLANK",
    "DDELETE",
    "DRECALL",
    "DTAG",
    "DSEEK",
    "DFBLANK",
    "DGET",
    "DPUT",
    "DFCOPY",
    "DSELECT",
    "DCHKSTAT",
    "DERRMSG",
    "SCANMSGHDR",
    "FDORDAKA",
    "FDORDORG",
    "FDORDAREA",
    "FDOQRD",
    "GETDRIVE",
    "SETDRIVE",
    "SETMSGHDR",
];

const PARTIAL_FUNCTIONS: &[&str] = &[
    // Add partially supported functions here
];

fn normalize(s: &str) -> String {
    s.to_ascii_uppercase()
}

fn classify_statement(name: &str) -> Option<ImplStatus> {
    let n = normalize(name);
    if UNSUPPORTED_STATEMENTS.iter().any(|s| normalize(s) == n) {
        return Some(ImplStatus::Unsupported);
    }
    if UNIMPLEMENTED_STATEMENTS.iter().any(|s| normalize(s) == n) {
        return Some(ImplStatus::Unimplemented);
    }
    if PARTIAL_STATEMENTS.iter().any(|s| normalize(s) == n) {
        return Some(ImplStatus::Partial);
    }
    None
}

fn classify_function(name: &str) -> Option<ImplStatus> {
    let n = normalize(name);
    if UNSUPPORTED_FUNCTIONS.iter().any(|s| normalize(s) == n) {
        return Some(ImplStatus::Unsupported);
    }
    if UNIMPLEMENTED_FUNCTIONS.iter().any(|s| normalize(s) == n) {
        return Some(ImplStatus::Unimplemented);
    }
    if PARTIAL_FUNCTIONS.iter().any(|s| normalize(s) == n) {
        return Some(ImplStatus::Partial);
    }
    None
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
        PPEExpr::Value(_) | PPEExpr::Invalid => {}
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
        PPECommand::Return | PPECommand::End | PPECommand::Goto(_) | PPECommand::Gosub(_) | PPECommand::EndFunc | PPECommand::EndProc | PPECommand::Stop => {}
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

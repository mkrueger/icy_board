//! Builds PPL sources for the fuzzer to compile.
//!
//! Random bytes almost never look like PPL, so the interesting parts of the front end are only
//! reached by a source that mostly parses. The program a fuzz target receives is described here
//! as a flat list of operations rather than a tree: rendering it needs no recursion, so a hostile
//! input can make the generated source deeply nested without the generator itself running out of
//! stack first.

use arbitrary::Arbitrary;
use icy_board_engine::{executable::SUPPORTED_PPL_LANGUAGE_VERSIONS, parser::ErrorReporter};
use std::path::Path;

/// Deep enough to cross the parser nesting limit, shallow enough to stay cheap.
const MAX_NESTING: usize = 96;
const MAX_SOURCE_LEN: usize = 48 * 1024;
const MAX_EXPRESSION_STACK: usize = 12;

const NAMES: &[&str] = &["i", "j", "s", "arr", "rec", "obj", "n", "t", "flag", "cur"];
const LABELS: &[&str] = &["top", "again", "done", "skip"];
const CONSTANTS: &[&str] = &[
    "TRUE",
    "FALSE",
    "AUTO",
    "NOCLEAR",
    "YESNO",
    "GRAPH",
    "SEC",
    "LANG",
    "HIGHASCII",
    "O_RW",
    "F_EXP",
    "START_BAL",
    "NEWLINE",
    "UPCASE",
    "STACKED",
];
const FUNCTIONS: &[&str] = &[
    "LEN",
    "UPPER",
    "LOWER",
    "MID",
    "LEFT",
    "RIGHT",
    "STRING",
    "ABS",
    "RANDOM",
    "U_NAME",
    "TIME",
    "DATE",
    "INSTR",
    "TRIM",
    "CHR",
    "ASC",
    "I2S",
    "S2I",
    "MASK_ASCII",
    "FILEINF",
];
const STATEMENTS: &[&str] = &[
    "PRINTLN",
    "PRINT",
    "NEWLINE",
    "CLS",
    "WAIT",
    "DELAY",
    "INPUT",
    "LOG",
    "SPRINT",
    "ANSIPOS",
    "COLOR",
    "FOPEN",
    "FCLOSE",
    "GETUSER",
    "PUTUSER",
    "REDIM",
    "INC",
    "DEC",
    "SORT",
    "TOKENIZE",
    "DISPFILE",
    "MESSAGE",
    "KBDSTUFF",
    // Names the code generator owns, which a source must not be able to reach.
    "PCALL",
    "STATIC",
    "PLACEHOLDER",
];
const TYPES: &[&str] = &[
    "INTEGER", "STRING", "BOOLEAN", "DATE", "MONEY", "BYTE", "WORD", "SBYTE", "SWORD", "REAL", "DREAL", "BIGSTR", "EDATE", "DDATE", "TIME", "UNSIGNED", "DWORD",
];
const MEMBERS: &[&str] = &["Length", "Left", "Right", "Name", "Field", "X", "Y", "Count"];
const UNARY: &[&str] = &["-", "+", "!", "~"];
const BINARY: &[&str] = &["+", "-", "*", "/", "%", "^", "=", "<>", "<", ">", "<=", ">=", "&", "|", "&&", "||"];
const STRINGS: &[&str] = &["", "a", "line", "@X07", "%s", "\\", "long string value"];

fn pick<'a>(table: &[&'a str], index: u8) -> &'a str {
    table[index as usize % table.len()]
}

#[derive(Debug, Arbitrary)]
pub struct Name(pub u8);

impl Name {
    fn as_str(&self) -> &'static str {
        pick(NAMES, self.0)
    }
}

/// One step of a stack machine that leaves a rendered expression behind.
#[derive(Debug, Arbitrary)]
pub enum ExprOp {
    Int(i16),
    Str(u8),
    Var(Name),
    Constant(u8),
    Unary(u8),
    Binary(u8),
    Paren,
    Call(u8, u8),
    Index(Name, u8),
    Member(u8),
}

fn render_expression(ops: &[ExprOp], budget: &mut usize) -> String {
    let mut stack: Vec<String> = Vec::new();
    for op in ops {
        if *budget == 0 {
            break;
        }
        let rendered = match op {
            ExprOp::Int(value) => value.to_string(),
            ExprOp::Str(index) => format!("\"{}\"", pick(STRINGS, *index)),
            ExprOp::Var(name) => name.as_str().to_string(),
            ExprOp::Constant(index) => pick(CONSTANTS, *index).to_string(),
            ExprOp::Unary(index) => {
                let operand = stack.pop().unwrap_or_else(|| "0".to_string());
                format!("{}{operand}", pick(UNARY, *index))
            }
            ExprOp::Binary(index) => {
                let right = stack.pop().unwrap_or_else(|| "0".to_string());
                let left = stack.pop().unwrap_or_else(|| "0".to_string());
                format!("{left} {} {right}", pick(BINARY, *index))
            }
            ExprOp::Paren => {
                let operand = stack.pop().unwrap_or_else(|| "0".to_string());
                format!("({operand})")
            }
            ExprOp::Call(function, arity) => {
                let arguments = pop_arguments(&mut stack, *arity % 4);
                format!("{}({})", pick(FUNCTIONS, *function), arguments.join(", "))
            }
            ExprOp::Index(name, arity) => {
                let arguments = pop_arguments(&mut stack, 1 + *arity % 3);
                format!("{}[{}]", name.as_str(), arguments.join(", "))
            }
            ExprOp::Member(index) => {
                let operand = stack.pop().unwrap_or_else(|| "obj".to_string());
                format!("{operand}.{}", pick(MEMBERS, *index))
            }
        };
        *budget = budget.saturating_sub(rendered.len());
        stack.push(rendered);
        if stack.len() > MAX_EXPRESSION_STACK {
            stack.remove(0);
        }
    }
    stack.pop().unwrap_or_else(|| "0".to_string())
}

fn pop_arguments(stack: &mut Vec<String>, count: u8) -> Vec<String> {
    let mut arguments = Vec::new();
    for _ in 0..count {
        arguments.push(stack.pop().unwrap_or_else(|| "0".to_string()));
    }
    arguments.reverse();
    arguments
}

#[derive(Debug, Arbitrary)]
pub enum StmtOp {
    Assign(Name, Vec<ExprOp>),
    AssignIndexed(Name, Vec<ExprOp>, Vec<ExprOp>),
    AssignMember(Name, u8, Vec<ExprOp>),
    Predefined(u8, Vec<Vec<ExprOp>>),
    Declare(u8, Name, u8),
    Label(u8),
    Goto(u8),
    Gosub(u8),
    Return,
    Break,
    Continue,
    End,
    Empty,
    Comment(u8),
    OpenIf(Vec<ExprOp>),
    ElseIf(Vec<ExprOp>),
    Else,
    OpenWhile(Vec<ExprOp>),
    OpenFor(Name, Vec<ExprOp>, Vec<ExprOp>),
    OpenSelect(Vec<ExprOp>),
    Case(Vec<ExprOp>),
    OpenRepeat,
    OpenLoop,
    OpenProcedure(u8, u8),
    OpenFunction(u8, u8, u8),
    Close,
    /// A closing keyword with nothing open, which the parser has to survive on its own.
    StrayClose(u8),
}

#[derive(Clone, Copy, PartialEq)]
enum Block {
    If,
    Select,
    While,
    For,
    Repeat,
    Loop,
    Procedure,
    Function,
}

impl Block {
    fn closer(self) -> &'static str {
        match self {
            Block::If => "ENDIF",
            Block::Select => "ENDSELECT",
            Block::While => "ENDWHILE",
            Block::For => "NEXT",
            Block::Repeat => "UNTIL FALSE",
            Block::Loop => "ENDLOOP",
            Block::Procedure => "ENDPROC",
            Block::Function => "ENDFUNC",
        }
    }
}

/// The nesting depths worth trying: the values around the parser limit matter far more than
/// uniformly random ones, and mutation on its own would hardly ever land on them.
const DEPTHS: &[usize] = &[0, 1, 2, 3, 8, 62, 63, 64, 65, 66, 96, 127, 128, 129, 255, 256, 512, 2048, 5000, 20000];

#[derive(Debug, Arbitrary)]
pub enum BoundaryKind {
    NestedIf,
    NestedParentheses,
    UnaryChain,
    MemberChain,
    IndexChain,
    NestedSelect,
    NestedWhile,
    NestedRoutine,
    BinaryChain,
    UnclosedIf,
    ElseIfChain,
    ArgumentList,
    DeclarationList,
}

#[derive(Debug, Arbitrary)]
pub enum Program {
    /// A whole source assembled from the operation list.
    Generated { language: u8, use_begin_end: bool, ops: Vec<StmtOp> },
    /// One construct repeated to a depth chosen from the table above.
    Boundary { language: u8, kind: BoundaryKind, depth: u8 },
}

impl Program {
    pub fn language_version(&self) -> u16 {
        let index = match self {
            Program::Generated { language, .. } | Program::Boundary { language, .. } => *language,
        };
        SUPPORTED_PPL_LANGUAGE_VERSIONS[index as usize % SUPPORTED_PPL_LANGUAGE_VERSIONS.len()]
    }

    pub fn render(&self) -> String {
        match self {
            Program::Generated { use_begin_end, ops, .. } => render_program(*use_begin_end, ops),
            Program::Boundary { kind, depth, .. } => render_boundary(kind, DEPTHS[*depth as usize % DEPTHS.len()]),
        }
    }
}

fn render_program(use_begin_end: bool, ops: &[StmtOp]) -> String {
    let mut source = String::new();
    let mut blocks: Vec<Block> = Vec::new();
    let mut budget = MAX_SOURCE_LEN;

    if use_begin_end {
        source.push_str("BEGIN\n");
    }

    for op in ops {
        if budget == 0 {
            break;
        }
        let before = source.len();
        emit_statement(&mut source, &mut blocks, &mut budget, op);
        budget = budget.saturating_sub(source.len() - before);
    }

    while let Some(block) = blocks.pop() {
        source.push_str(block.closer());
        source.push('\n');
    }
    if use_begin_end {
        source.push_str("END\n");
    }
    source
}

fn emit_statement(source: &mut String, blocks: &mut Vec<Block>, budget: &mut usize, op: &StmtOp) {
    match op {
        StmtOp::Assign(name, expr) => {
            source.push_str(&format!("{} = {}\n", name.as_str(), render_expression(expr, budget)));
        }
        StmtOp::AssignIndexed(name, index, value) => {
            source.push_str(&format!(
                "{}[{}] = {}\n",
                name.as_str(),
                render_expression(index, budget),
                render_expression(value, budget)
            ));
        }
        StmtOp::AssignMember(name, member, value) => {
            source.push_str(&format!(
                "{}.{} = {}\n",
                name.as_str(),
                pick(MEMBERS, *member),
                render_expression(value, budget)
            ));
        }
        StmtOp::Predefined(statement, arguments) => {
            let rendered: Vec<String> = arguments.iter().take(8).map(|expr| render_expression(expr, budget)).collect();
            source.push_str(&format!("{} {}\n", pick(STATEMENTS, *statement), rendered.join(", ")));
        }
        StmtOp::Declare(variable_type, name, dimensions) => {
            let dimensions = *dimensions % 5;
            let suffix = if dimensions == 0 {
                String::new()
            } else {
                format!("({})", vec!["4"; dimensions as usize].join(", "))
            };
            source.push_str(&format!("{} {}{suffix}\n", pick(TYPES, *variable_type), name.as_str()));
        }
        StmtOp::Label(index) => source.push_str(&format!(":{}\n", pick(LABELS, *index))),
        StmtOp::Goto(index) => source.push_str(&format!("GOTO {}\n", pick(LABELS, *index))),
        StmtOp::Gosub(index) => source.push_str(&format!("GOSUB {}\n", pick(LABELS, *index))),
        StmtOp::Return => source.push_str("RETURN\n"),
        StmtOp::Break => source.push_str("BREAK\n"),
        StmtOp::Continue => source.push_str("CONTINUE\n"),
        StmtOp::End => source.push_str("END\n"),
        StmtOp::Empty => source.push('\n'),
        StmtOp::Comment(index) => source.push_str(&format!("; {}\n", pick(STRINGS, *index))),
        StmtOp::OpenIf(condition) => {
            if push_block(blocks, Block::If) {
                source.push_str(&format!("IF ({}) THEN\n", render_expression(condition, budget)));
            }
        }
        StmtOp::ElseIf(condition) => {
            source.push_str(&format!("ELSEIF ({}) THEN\n", render_expression(condition, budget)));
        }
        StmtOp::Else => source.push_str("ELSE\n"),
        StmtOp::OpenWhile(condition) => {
            if push_block(blocks, Block::While) {
                source.push_str(&format!("WHILE ({}) DO\n", render_expression(condition, budget)));
            }
        }
        StmtOp::OpenFor(name, from, to) => {
            if push_block(blocks, Block::For) {
                source.push_str(&format!(
                    "FOR {} = {} TO {}\n",
                    name.as_str(),
                    render_expression(from, budget),
                    render_expression(to, budget)
                ));
            }
        }
        StmtOp::OpenSelect(value) => {
            if push_block(blocks, Block::Select) {
                source.push_str(&format!("SELECT CASE {}\n", render_expression(value, budget)));
            }
        }
        StmtOp::Case(value) => {
            source.push_str(&format!("CASE {}\n", render_expression(value, budget)));
        }
        StmtOp::OpenRepeat => {
            if push_block(blocks, Block::Repeat) {
                source.push_str("REPEAT\n");
            }
        }
        StmtOp::OpenLoop => {
            if push_block(blocks, Block::Loop) {
                source.push_str("LOOP\n");
            }
        }
        StmtOp::OpenProcedure(name, parameters) => {
            if push_block(blocks, Block::Procedure) {
                source.push_str(&format!("PROCEDURE p{}({})\n", name % 8, parameter_list(*parameters)));
            }
        }
        StmtOp::OpenFunction(name, parameters, return_type) => {
            if push_block(blocks, Block::Function) {
                source.push_str(&format!(
                    "FUNCTION f{}({}) {}\n",
                    name % 8,
                    parameter_list(*parameters),
                    pick(TYPES, *return_type)
                ));
            }
        }
        StmtOp::Close => {
            if let Some(block) = blocks.pop() {
                source.push_str(block.closer());
                source.push('\n');
            }
        }
        StmtOp::StrayClose(index) => {
            const CLOSERS: &[&str] = &["ENDIF", "ENDWHILE", "NEXT", "ENDSELECT", "ENDPROC", "ENDFUNC", "END", "ENDLOOP", "UNTIL TRUE"];
            source.push_str(pick(CLOSERS, *index));
            source.push('\n');
        }
    }
}

fn push_block(blocks: &mut Vec<Block>, block: Block) -> bool {
    if blocks.len() >= MAX_NESTING {
        return false;
    }
    blocks.push(block);
    true
}

fn parameter_list(parameters: u8) -> String {
    (0..parameters % 5).map(|index| format!("INTEGER a{index}")).collect::<Vec<_>>().join(", ")
}

fn render_boundary(kind: &BoundaryKind, depth: usize) -> String {
    match kind {
        BoundaryKind::NestedIf => format!("{}PRINTLN 1\n{}", "IF (TRUE) THEN\n".repeat(depth), "ENDIF\n".repeat(depth)),
        BoundaryKind::NestedParentheses => format!("PRINTLN {}1{}\n", "(".repeat(depth), ")".repeat(depth)),
        BoundaryKind::UnaryChain => format!("PRINTLN {}1\n", "!".repeat(depth)),
        BoundaryKind::MemberChain => format!("PRINTLN obj{}\n", ".Field".repeat(depth)),
        BoundaryKind::IndexChain => format!("PRINTLN arr{}\n", "[1]".repeat(depth)),
        BoundaryKind::NestedSelect => format!("{}PRINTLN 1\n{}", "SELECT CASE 1\nCASE 1\n".repeat(depth), "ENDSELECT\n".repeat(depth)),
        BoundaryKind::NestedWhile => format!("{}PRINTLN 1\n{}", "WHILE (TRUE) DO\n".repeat(depth), "ENDWHILE\n".repeat(depth)),
        BoundaryKind::NestedRoutine => format!("{}PRINTLN 1\n{}", "PROCEDURE p()\n".repeat(depth), "ENDPROC\n".repeat(depth)),
        BoundaryKind::BinaryChain => format!("PRINTLN 1{}\n", " + 1".repeat(depth)),
        BoundaryKind::UnclosedIf => "IF (TRUE) THEN\n".repeat(depth),
        BoundaryKind::ElseIfChain => format!("IF (TRUE) THEN\n{}ENDIF\n", "ELSEIF (TRUE) THEN\n".repeat(depth)),
        BoundaryKind::ArgumentList => format!("PRINTLN LEN({})\n", vec!["1"; depth.max(1)].join(", ")),
        BoundaryKind::DeclarationList => (0..depth).map(|index| format!("INTEGER v{index}\n")).collect(),
    }
}

/// A diagnostic an editor cannot point at is a defect of its own, and a span outside the source is
/// what makes the renderer in pplc panic rather than print.
pub fn check_diagnostic_spans(reporter: &ErrorReporter, source: &str, file_name: &Path) {
    for diagnostic in reporter.errors.iter().chain(reporter.warnings.iter()) {
        if diagnostic.file_name != file_name {
            continue;
        }
        assert!(
            diagnostic.span.start <= diagnostic.span.end,
            "reversed span {:?} for {}",
            diagnostic.span,
            diagnostic.error
        );
        assert!(
            diagnostic.span.end <= source.len(),
            "span {:?} past the end of a {} byte source for {}",
            diagnostic.span,
            source.len(),
            diagnostic.error
        );
    }
}

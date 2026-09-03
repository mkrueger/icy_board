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

/// Real sources, so a mutation starts from something the parser already accepts instead of from
/// bytes it rejects in the first token.
const SEEDS: &[&str] = &[
    include_str!("../../crates/icy_board_engine/tests/test_data/if_then.pps"),
    include_str!("../../crates/icy_board_engine/tests/test_data/function_test.pps"),
    include_str!("../../crates/icy_board_engine/tests/test_data/local_variables.pps"),
    include_str!("../../crates/icy_board_engine/tests/test_data/by_ref_parameter.pps"),
    include_str!("../../crates/icy_board_engine/tests/test_data/bitfunctions.pps"),
    include_str!("../../ppe/script2.pps"),
];

/// The pieces a mutation swaps in. Keeping them to things PPL already means lets a damaged source
/// stay parseable far enough to reach the compiler.
const REPLACEMENTS: &[&str] = &[
    "",
    "1",
    "0",
    "-1",
    "\"\"",
    "TRUE",
    "AUTO",
    "PCALL",
    "STATIC",
    "END",
    "ENDIF",
    "ENDPROC",
    "BEGIN",
    "(",
    ")",
    "[",
    "]",
    ",",
    ".",
    "=",
    "+",
    "PRINTLN",
    "INTEGER",
    "PROCEDURE",
    "GOTO",
    "obj.Field",
    "arr[1]",
    "999999999999999999999",
];

#[derive(Debug, Arbitrary)]
pub enum Edit {
    /// Swaps one line for another piece of PPL.
    ReplaceLine {
        line: u16,
        text: u8,
    },
    DeleteLine {
        line: u16,
    },
    /// Repeats a line, which is how an unbalanced block usually appears.
    DuplicateLine {
        line: u16,
    },
    SwapLines {
        line: u16,
        other: u16,
    },
    InsertLine {
        line: u16,
        text: u8,
    },
    /// Appends to a line so a statement gains a suffix it was not written with.
    AppendToLine {
        line: u16,
        text: u8,
    },
    /// Grows a chain in place, which random editing almost never manages.
    RepeatFragment {
        line: u16,
        fragment: u8,
        times: u8,
    },
    TruncateLine {
        line: u16,
        keep: u8,
    },
}

const FRAGMENTS: &[&str] = &["[1]", ".Field", "(1)", " + 1", "!", "(", ")"];

#[derive(Debug, Arbitrary)]
pub struct MutatedSource {
    pub seed: u8,
    pub language: u8,
    pub edits: Vec<Edit>,
}

impl MutatedSource {
    pub fn language_version(&self) -> u16 {
        SUPPORTED_PPL_LANGUAGE_VERSIONS[self.language as usize % SUPPORTED_PPL_LANGUAGE_VERSIONS.len()]
    }

    pub fn render(&self) -> String {
        let seed = SEEDS[self.seed as usize % SEEDS.len()];
        let mut lines: Vec<String> = seed.lines().map(ToString::to_string).collect();

        for edit in self.edits.iter().take(64) {
            if lines.is_empty() {
                lines.push(String::new());
            }
            let index = |line: &u16| *line as usize % lines.len();
            match edit {
                Edit::ReplaceLine { line, text } => {
                    let at = index(line);
                    lines[at] = pick(REPLACEMENTS, *text).to_string();
                }
                Edit::DeleteLine { line } => {
                    let at = index(line);
                    lines.remove(at);
                }
                Edit::DuplicateLine { line } => {
                    let at = index(line);
                    let copy = lines[at].clone();
                    lines.insert(at, copy);
                }
                Edit::SwapLines { line, other } => {
                    let (a, b) = (index(line), index(other));
                    lines.swap(a, b);
                }
                Edit::InsertLine { line, text } => {
                    let at = index(line);
                    lines.insert(at, pick(REPLACEMENTS, *text).to_string());
                }
                Edit::AppendToLine { line, text } => {
                    let at = index(line);
                    let addition = pick(REPLACEMENTS, *text).to_string();
                    lines[at].push_str(&addition);
                }
                Edit::RepeatFragment { line, fragment, times } => {
                    let at = index(line);
                    let repeat = DEPTHS[*times as usize % DEPTHS.len()].min(4096);
                    let addition = pick(FRAGMENTS, *fragment).repeat(repeat);
                    lines[at].push_str(&addition);
                }
                Edit::TruncateLine { line, keep } => {
                    let at = index(line);
                    let mut keep = (*keep as usize).min(lines[at].len());
                    while keep > 0 && !lines[at].is_char_boundary(keep) {
                        keep -= 1;
                    }
                    lines[at].truncate(keep);
                }
            }
            if lines.iter().map(String::len).sum::<usize>() > MAX_SOURCE_LEN {
                break;
            }
        }

        lines.join("\n")
    }
}

/// A directive the lexer acts on before the parser ever sees a token. They carry state of their
/// own in the conditional stack, and a `$DEFINE` value re-enters the parser, so they are worth
/// arranging in ways a source would not.
#[derive(Debug, Arbitrary)]
pub enum Directive {
    If(u8),
    ElseIf(u8),
    Else,
    EndIf,
    Define(u8, u8),
    DefineNoValue(u8),
    UseFuncs,
    LangVersion(u8),
    Include(u8),
    Statement,
    /// A directive spelled the way a source might get it slightly wrong.
    Malformed(u8),
}

const PREPROC_CONDITIONS: &[&str] = &[
    "TRUE",
    "FALSE",
    "1",
    "0",
    "-1",
    "",
    "NAME",
    "NAME = 1",
    "NAME > 0",
    "1 + 1",
    "(((1)))",
    "!TRUE",
    "\"text\"",
    "1 / 0",
    "$",
    "NAME NAME",
];

const DEFINE_VALUES: &[&str] = &[
    "1",
    "0",
    "TRUE",
    "FALSE",
    "-2147483648",
    "2147483647",
    "\"text\"",
    "1 + 1",
    "NAME",
    "",
    "1 1",
    "NAME + 1",
];

const DEFINE_NAMES: &[&str] = &["NAME", "OTHER", "_x", "1bad", "with space", "", "NAME2", "TRUE"];

const MALFORMED_DIRECTIVES: &[&str] = &[
    ";$",
    ";$IF",
    ";$ELSEIF",
    ";$ENDIF extra",
    ";$IFDEF NAME",
    ";$UNKNOWN",
    ";$LANGVERSION",
    ";$LANGVERSION 99999",
    ";$LANGVERSION -1",
    ";$INCLUDE:",
    ";$INCLUDE:../../../etc/passwd",
    ";$DEFINE",
];

#[derive(Debug, Arbitrary)]
pub enum Preprocessed {
    /// Directives in whatever order the mutator picked, balanced or not.
    Mixed { language: u8, directives: Vec<Directive> },
    /// One conditional nested to a depth from the table, with the closers left off when asked.
    NestedConditional { language: u8, depth: u8, closed: bool },
}

impl Preprocessed {
    pub fn language_version(&self) -> u16 {
        let index = match self {
            Preprocessed::Mixed { language, .. } | Preprocessed::NestedConditional { language, .. } => *language,
        };
        SUPPORTED_PPL_LANGUAGE_VERSIONS[index as usize % SUPPORTED_PPL_LANGUAGE_VERSIONS.len()]
    }

    pub fn render(&self) -> String {
        match self {
            Preprocessed::Mixed { directives, .. } => render_directives(directives),
            Preprocessed::NestedConditional { depth, closed, .. } => {
                let depth = DEPTHS[*depth as usize % DEPTHS.len()];
                let mut source = ";$DEFINE NAME = 1\n".to_string();
                source.push_str(&";$IF NAME\nPRINTLN 1\n".repeat(depth));
                if *closed {
                    source.push_str(&";$ENDIF\n".repeat(depth));
                }
                source
            }
        }
    }
}

fn render_directives(directives: &[Directive]) -> String {
    let mut source = String::new();
    for directive in directives {
        if source.len() >= MAX_SOURCE_LEN {
            break;
        }
        match directive {
            Directive::If(condition) => source.push_str(&format!(";$IF {}\n", pick(PREPROC_CONDITIONS, *condition))),
            Directive::ElseIf(condition) => source.push_str(&format!(";$ELSEIF {}\n", pick(PREPROC_CONDITIONS, *condition))),
            Directive::Else => source.push_str(";$ELSE\n"),
            Directive::EndIf => source.push_str(";$ENDIF\n"),
            Directive::Define(name, value) => source.push_str(&format!(";$DEFINE {} = {}\n", pick(DEFINE_NAMES, *name), pick(DEFINE_VALUES, *value))),
            Directive::DefineNoValue(name) => source.push_str(&format!(";$DEFINE {}\n", pick(DEFINE_NAMES, *name))),
            Directive::UseFuncs => source.push_str(";$USEFUNCS\n"),
            Directive::LangVersion(version) => source.push_str(&format!(";$LANGVERSION {}\n", u16::from(*version) * 10)),
            Directive::Include(path) => source.push_str(&format!(";$INCLUDE:{}\n", pick(&["a.pps", "../a.pps", "/etc/passwd", ""], *path))),
            Directive::Statement => source.push_str("PRINTLN NAME\n"),
            Directive::Malformed(index) => {
                source.push_str(pick(MALFORMED_DIRECTIVES, *index));
                source.push('\n');
            }
        }
    }
    source
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

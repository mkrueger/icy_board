//! PCBoard compatibility status of the predefined PPL statements and functions.
//!
//! This is the single source of truth consumed both at runtime (`ppld --compat-check`)
//! and by the `opcode_coverage` integration test. The test derives the real status by
//! scanning the VM dispatch tables and handler bodies, so these lists cannot drift out
//! of sync with the implementation.

/// How faithfully an opcode is implemented compared to the original PCBoard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImplStatus {
    /// Implemented and expected to behave like the original.
    Implemented,
    /// Deliberately not supported (handler logs a warning and continues).
    Unsupported,
    /// Stubbed out - the call is logged and a neutral result is substituted.
    Unimplemented,
    /// Works, but known to be missing edge case behaviour.
    Partial,
    /// Not callable as a statement/function; handled by its own PPECommand/PPEExpr.
    Invalid,
}

impl ImplStatus {
    /// True when a PPE using this opcode cannot run correctly.
    pub fn is_broken(self) -> bool {
        matches!(self, ImplStatus::Unimplemented)
    }
}

/// Statements that exist but intentionally do nothing meaningful.
pub const UNSUPPORTED_STATEMENTS: &[&str] = &["SOUND", "SOUNDDELAY"];

/// Statements stubbed with `unimplemented_stmt!`.
pub const UNIMPLEMENTED_STATEMENTS: &[&str] = &[
    "DOINTR",
    "VARSEG",
    "VAROFF",
    "POKE",
    "POKEB",
    "POKEW",
    "VARADDR",
    "POKEDW",
    "DCREATE",
    "DOPEN",
    "DCLOSE",
    "DSETALIAS",
    "DPACK",
    "DCLOSEALL",
    "DLOCK",
    "DLOCKF",
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
    "FDOWRAKA",
    "FDOADDAKA",
    "FDOWRORG",
    "FDOADDORG",
    "FDOQMOD",
    "FDOQADD",
    "FDOQDEL",
];

/// Statements implemented but known to miss edge cases.
pub const PARTIAL_STATEMENTS: &[&str] = &[];

/// Functions that exist but intentionally return a placeholder.
pub const UNSUPPORTED_FUNCTIONS: &[&str] = &["GETDRIVE", "SETDRIVE", "MODEM"];

/// Functions stubbed with `unimplemented_function!`.
pub const UNIMPLEMENTED_FUNCTIONS: &[&str] = &[
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
    "FDORDAKA",
    "FDORDORG",
    "FDORDAREA",
    "FDOQRD",
];

/// Functions implemented but known to miss edge cases.
pub const PARTIAL_FUNCTIONS: &[&str] = &[];

fn contains(list: &[&str], name: &str) -> bool {
    list.iter().any(|s| s.eq_ignore_ascii_case(name))
}

/// Compatibility status of a predefined statement, by its canonical name.
pub fn statement_status(name: &str) -> ImplStatus {
    if contains(UNSUPPORTED_STATEMENTS, name) {
        ImplStatus::Unsupported
    } else if contains(UNIMPLEMENTED_STATEMENTS, name) {
        ImplStatus::Unimplemented
    } else if contains(PARTIAL_STATEMENTS, name) {
        ImplStatus::Partial
    } else {
        ImplStatus::Implemented
    }
}

/// Compatibility status of a predefined function, by its canonical name.
pub fn function_status(name: &str) -> ImplStatus {
    if contains(UNSUPPORTED_FUNCTIONS, name) {
        ImplStatus::Unsupported
    } else if contains(UNIMPLEMENTED_FUNCTIONS, name) {
        ImplStatus::Unimplemented
    } else if contains(PARTIAL_FUNCTIONS, name) {
        ImplStatus::Partial
    } else {
        ImplStatus::Implemented
    }
}

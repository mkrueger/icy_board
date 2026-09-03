use crate::executable::{VariableType, VariableValue};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum VMError {
    #[error("Internal VM error")]
    InternalVMError,

    #[error("Label not found (0x{0:X})")]
    LabelNotFound(usize),

    #[error("Tried to pop from empty value stack.")]
    PushPopStackEmpty,

    #[error("Can't fread variable ({0}) with size {1} requested size:{2}")]
    FReadError(VariableType, usize, usize),

    #[error("File not found ({0})")]
    FileNotFound(String),

    #[error("Error in function call ({0}): {1}")]
    ErrorInFunctionCall(String, String),

    #[error("Invalid seek position ({0})")]
    InvalidSeekPosition(i32),

    #[error("File channel not open ({0})")]
    FileChannelNotOpen(i32),

    #[error("Pass value stack empty")]
    PassValueStackEmpty,

    #[error("Write back stack empty")]
    WriteBackStackEmpty,

    #[error("No user type base expression")]
    NoUserTypeBase,

    #[error("Type not found in registry")]
    TypeNotFoundInRegistry(u8),

    #[error("Object not found (internal VM error) ({0})")]
    NoObjectFound(u8),

    #[error("Member {1} not found for user type {0}")]
    InvalidMemberId(u8, usize),

    #[error("Member {1} of user type {0} is not a function")]
    InvalidMemberFunction(u8, usize),

    #[error("Member {1} of user type {0} expected {2} arguments, got {3}")]
    InvalidMemberArgumentCount(u8, usize, usize, usize),

    #[error("Invalid array dimension count: {0}")]
    InvalidArrayDimensionCount(usize),

    #[error("PPE call stack exhausted")]
    StackOverflow,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TerminalTarget {
    Both,
    User,
    Sysop,
}

pub struct StackFrame {
    pub values: HashMap<unicase::Ascii<String>, VariableValue>,
    pub cur_ptr: usize,
    pub label_table: HashMap<unicase::Ascii<String>, usize>,
}

/// Where `ON ERROR` sends a program when an operation fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ErrorHandler {
    #[default]
    Off,
    /// Jumps for good; the handler ends the program rather than coming back.
    Goto(usize),
    Gosub(usize),
    /// Calls the procedure with the id of its variable table entry.
    Procedure(usize),
}

pub struct ReturnAddress {
    ptr: usize,
    id: usize,
}

impl ReturnAddress {
    pub fn gosub(cur_ptr: usize) -> ReturnAddress {
        ReturnAddress { ptr: cur_ptr, id: 0 }
    }

    pub(super) fn func_call(cur_ptr: usize, proc_id: usize) -> ReturnAddress {
        ReturnAddress { ptr: cur_ptr, id: proc_id }
    }

    pub fn get_ptr(&self) -> usize {
        self.ptr
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn is_gosub(&self) -> bool {
        self.id == 0
    }
}

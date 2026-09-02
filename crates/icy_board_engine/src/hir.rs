//! Resolved PPL identities and expressions shared by semantic tools and code generation.
//!
//! Calls are already resolved to the opcode they dispatch to, so this is the backend's
//! view of a program rather than a source-shaped one.

use crate::ast::{BinOp, UnaryOp};

/// Stable index into semantic references for one compilation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

/// Parse-assigned identity preserved across AST transformations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CallId(pub u64);

/// Identity of a resolved control-flow target before byte offsets are assigned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LabelId(pub usize);

/// Byte offset in the final command stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CodeOffset(pub usize);

/// Variable-table entry holding a variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VariableId(pub usize);

/// Variable-table entry holding a constant value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConstantId(pub usize);

/// Variable-table entry holding a routine. Variables, constants and routines share
/// one numbering, so these three only say which kind of entry was resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RoutineId(pub usize);

/// Positional field or member identity within its resolved receiver type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MemberId(pub usize);

/// Registry type identity before executable type-table compaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UserTypeId(pub u8);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HirProgram {
    pub commands: Vec<HirCommand>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum HirExpr {
    #[default]
    Invalid,
    Variable(VariableId),
    Constant(ConstantId),
    RoutineReference(RoutineId),
    RecordLiteral(UserTypeId, Vec<(MemberId, HirExpr)>),
    Member(Box<HirExpr>, MemberId),
    IndexedMember(Box<HirExpr>, MemberId, Vec<HirExpr>),
    Unary(UnaryOp, Box<HirExpr>),
    Binary(BinOp, Box<HirExpr>, Box<HirExpr>),
    Dim(VariableId, Vec<HirExpr>),
    PredefinedCall(crate::executable::FuncOpCode, Vec<HirExpr>),
    FunctionCall(RoutineId, Vec<HirExpr>),
    MemberCall(Box<HirExpr>, Vec<HirExpr>, MemberId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirErrorTarget {
    Off,
    Goto(LabelId),
    Gosub(LabelId),
    Procedure(RoutineId),
}

#[derive(Clone, Debug, PartialEq)]
pub enum HirCommand {
    End,
    EndFunction,
    EndProcedure,
    Return,
    Goto(LabelId),
    Gosub(LabelId),
    OnError(HirErrorTarget),
    ConditionalGoto(HirExpr, LabelId),
    Let(HirExpr, HirExpr),
    MemberCall(HirExpr),
    PredefinedCall(crate::executable::OpCode, Vec<HirExpr>),
    ProcedureCall(RoutineId, Vec<HirExpr>),
    ForEach(VariableId, HirExpr, LabelId),
    NextForEach(CodeOffset),
}

impl HirExpr {
    pub fn variable(id: usize) -> Self {
        Self::Variable(VariableId(id))
    }

    pub fn constant(id: usize) -> Self {
        Self::Constant(ConstantId(id))
    }

    pub fn routine_reference(id: usize) -> Self {
        Self::RoutineReference(RoutineId(id))
    }

    pub fn member(base: Self, member: usize) -> Self {
        Self::Member(Box::new(base), MemberId(member))
    }

    pub fn indexed_member(base: Self, member: usize, dimensions: Vec<Self>) -> Self {
        Self::IndexedMember(Box::new(base), MemberId(member), dimensions)
    }

    pub fn dim(variable: usize, dimensions: Vec<Self>) -> Self {
        Self::Dim(VariableId(variable), dimensions)
    }

    pub fn predefined(opcode: crate::executable::FuncOpCode, arguments: Vec<Self>) -> Self {
        Self::PredefinedCall(opcode, arguments)
    }

    pub fn function(routine: usize, arguments: Vec<Self>) -> Self {
        Self::FunctionCall(RoutineId(routine), arguments)
    }

    pub fn member_call(receiver: Self, arguments: Vec<Self>, member: usize) -> Self {
        Self::MemberCall(Box::new(receiver), arguments, MemberId(member))
    }
}

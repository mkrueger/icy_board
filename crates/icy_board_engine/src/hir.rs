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

/// The first structural failure, with a zero-based HIR command index.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Invalid lowered program at command {command_index}: {reason}")]
pub struct HirValidationError {
    pub command_index: usize,
    pub reason: String,
}

impl HirProgram {
    /// Validate resolved structure, not operand types, arity or control-flow semantics.
    /// Storage IDs share the one-based variable table; labels remain zero-based HIR
    /// identities even after the compiler patches the separate PPE command stream.
    /// Code offsets and member/type IDs belong to other domains and are not checked here.
    pub fn validate(&self, declaration_count: usize, label_count: usize) -> Result<(), HirValidationError> {
        for (command_index, command) in self.commands.iter().enumerate() {
            command
                .validate(declaration_count, label_count)
                .map_err(|reason| HirValidationError { command_index, reason })?;
        }
        Ok(())
    }
}

fn validate_storage_id(kind: &str, id: usize, declaration_count: usize) -> Result<(), String> {
    if id == 0 || id > declaration_count {
        Err(format!("{kind}({id}) is outside the one-based variable table ({declaration_count} entries)"))
    } else {
        Ok(())
    }
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
    fn validate(&self, declaration_count: usize) -> Result<(), String> {
        match self {
            Self::Invalid => Err("unresolved HirExpr::Invalid".to_string()),
            Self::Variable(id) => validate_storage_id("VariableId", id.0, declaration_count),
            Self::Constant(id) => validate_storage_id("ConstantId", id.0, declaration_count),
            Self::RoutineReference(id) => validate_storage_id("RoutineId", id.0, declaration_count),
            Self::RecordLiteral(_, fields) => fields.iter().try_for_each(|(_, value)| value.validate(declaration_count)),
            Self::Member(base, _) | Self::Unary(_, base) => base.validate(declaration_count),
            Self::Binary(_, left, right) => {
                left.validate(declaration_count)?;
                right.validate(declaration_count)
            }
            Self::Dim(id, arguments) => {
                validate_storage_id("VariableId", id.0, declaration_count)?;
                arguments.iter().try_for_each(|argument| argument.validate(declaration_count))
            }
            Self::FunctionCall(id, arguments) => {
                validate_storage_id("RoutineId", id.0, declaration_count)?;
                arguments.iter().try_for_each(|argument| argument.validate(declaration_count))
            }
            Self::PredefinedCall(_, arguments) => arguments.iter().try_for_each(|argument| argument.validate(declaration_count)),
            Self::IndexedMember(base, _, arguments) | Self::MemberCall(base, arguments, _) => {
                base.validate(declaration_count)?;
                arguments.iter().try_for_each(|argument| argument.validate(declaration_count))
            }
        }
    }

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

impl HirCommand {
    fn validate(&self, declaration_count: usize, label_count: usize) -> Result<(), String> {
        let label = |id: LabelId| {
            if id.0 >= label_count {
                Err(format!("LabelId({}) is outside the zero-based label table ({label_count} entries)", id.0))
            } else {
                Ok(())
            }
        };
        match self {
            Self::End | Self::EndFunction | Self::EndProcedure | Self::Return | Self::NextForEach(_) => Ok(()),
            Self::Goto(id) | Self::Gosub(id) => label(*id),
            Self::OnError(target) => match target {
                HirErrorTarget::Off => Ok(()),
                HirErrorTarget::Goto(id) | HirErrorTarget::Gosub(id) => label(*id),
                HirErrorTarget::Procedure(id) => validate_storage_id("RoutineId", id.0, declaration_count),
            },
            Self::ConditionalGoto(condition, id) => {
                condition.validate(declaration_count)?;
                label(*id)
            }
            Self::Let(target, value) => {
                target.validate(declaration_count)?;
                value.validate(declaration_count)
            }
            Self::MemberCall(expression) => expression.validate(declaration_count),
            Self::PredefinedCall(_, arguments) => arguments.iter().try_for_each(|argument| argument.validate(declaration_count)),
            Self::ProcedureCall(id, arguments) => {
                validate_storage_id("RoutineId", id.0, declaration_count)?;
                arguments.iter().try_for_each(|argument| argument.validate(declaration_count))
            }
            Self::ForEach(id, collection, end) => {
                validate_storage_id("VariableId", id.0, declaration_count)?;
                collection.validate(declaration_count)?;
                label(*end)
            }
        }
    }
}

use std::collections::BTreeMap;

use crate::ast::{BinOp, UnaryOp};

use super::container::{ContainerError, Section};
use super::{FUNCTION_DEFINITIONS, OnErrorTarget, PPECommand, PPEExpr, PPEScript, PPEStatement, STATEMENT_DEFINITIONS};

pub(super) type Result<T> = std::result::Result<T, ContainerError>;
pub(super) const MAX_DEPTH: usize = 96;
pub(super) const MAX_ITEMS: usize = 1_000_000;

pub(super) struct Reader<'a> {
    data: &'a [u8],
    pub remaining_nodes: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            remaining_nodes: MAX_ITEMS,
        }
    }

    pub fn bytes(&mut self, length: usize) -> Result<&'a [u8]> {
        if length > self.data.len() {
            return Err(ContainerError::Invalid("truncated section record"));
        }
        let (value, rest) = self.data.split_at(length);
        self.data = rest;
        Ok(value)
    }

    pub fn word(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    pub fn count(&mut self, minimum_bytes: usize) -> Result<usize> {
        let count = self.word()? as usize;
        if count > MAX_ITEMS || count > self.data.len() / minimum_bytes.max(1) {
            return Err(ContainerError::Limit("section item count"));
        }
        Ok(count)
    }

    pub fn blob(&mut self) -> Result<&'a [u8]> {
        let length = self.word()? as usize;
        self.bytes(length)
    }

    pub fn text(&mut self) -> Result<String> {
        String::from_utf8(self.blob()?.to_vec()).map_err(|_| ContainerError::Invalid("invalid UTF-8"))
    }

    pub fn finish(self) -> Result<()> {
        if self.data.is_empty() {
            Ok(())
        } else {
            Err(ContainerError::Invalid("trailing section data"))
        }
    }
}

pub(super) fn word(output: &mut Vec<u8>, value: usize) -> Result<()> {
    output.extend_from_slice(&u32::try_from(value).map_err(|_| ContainerError::Limit("32-bit reference"))?.to_le_bytes());
    Ok(())
}

pub(super) fn blob(output: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    word(output, value.len())?;
    output.extend_from_slice(value);
    Ok(())
}

const BINARY: [BinOp; 16] = [
    BinOp::PoW,
    BinOp::Mul,
    BinOp::Div,
    BinOp::Mod,
    BinOp::Add,
    BinOp::Sub,
    BinOp::Eq,
    BinOp::NotEq,
    BinOp::Lower,
    BinOp::LowerEq,
    BinOp::Greater,
    BinOp::GreaterEq,
    BinOp::And,
    BinOp::Or,
    BinOp::ShortAnd,
    BinOp::ShortOr,
];
const UNARY: [UnaryOp; 3] = [UnaryOp::Plus, UnaryOp::Minus, UnaryOp::Not];

fn write_args(output: &mut Vec<u8>, args: &[PPEExpr], depth: usize) -> Result<()> {
    word(output, args.len())?;
    for arg in args {
        write_expr(output, arg, depth + 1)?;
    }
    Ok(())
}

fn write_expr(output: &mut Vec<u8>, expr: &PPEExpr, depth: usize) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(ContainerError::Limit("expression depth"));
    }
    match expr {
        PPEExpr::Invalid => return Err(ContainerError::Invalid("invalid expression")),
        PPEExpr::Value(id) | PPEExpr::RoutineReference(id) => {
            word(output, if matches!(expr, PPEExpr::Value(_)) { 1 } else { 2 })?;
            word(output, *id)?;
        }
        PPEExpr::RecordLiteral(id, fields) => {
            word(output, 3)?;
            word(output, *id as usize)?;
            word(output, fields.len())?;
            for (field, value) in fields {
                word(output, *field)?;
                write_expr(output, value, depth + 1)?;
            }
        }
        PPEExpr::Member(base, id) => {
            word(output, 4)?;
            word(output, *id)?;
            write_expr(output, base, depth + 1)?;
        }
        PPEExpr::IndexedMember(base, id, args) | PPEExpr::MemberFunctionCall(base, args, id) => {
            word(output, if matches!(expr, PPEExpr::IndexedMember(..)) { 5 } else { 11 })?;
            word(output, *id)?;
            write_expr(output, base, depth + 1)?;
            write_args(output, args, depth)?;
        }
        PPEExpr::UnaryExpression(op, value) => {
            word(output, 6)?;
            word(output, UNARY.iter().position(|candidate| candidate == op).unwrap())?;
            write_expr(output, value, depth + 1)?;
        }
        PPEExpr::BinaryExpression(op, left, right) => {
            word(output, 7)?;
            word(output, BINARY.iter().position(|candidate| candidate == op).unwrap())?;
            write_expr(output, left, depth + 1)?;
            write_expr(output, right, depth + 1)?;
        }
        PPEExpr::Dim(id, args) | PPEExpr::FunctionCall(id, args) => {
            word(output, if matches!(expr, PPEExpr::Dim(..)) { 8 } else { 10 })?;
            word(output, *id)?;
            write_args(output, args, depth)?;
        }
        PPEExpr::PredefinedFunctionCall(definition, args) => {
            word(output, 9)?;
            output.extend_from_slice(&(definition.opcode as i32).to_le_bytes());
            write_args(output, args, depth)?;
        }
    }
    Ok(())
}

fn read_args(input: &mut Reader<'_>, depth: usize) -> Result<Vec<PPEExpr>> {
    let count = input.count(8)?;
    (0..count).map(|_| read_expr(input, depth + 1)).collect()
}

fn read_expr(input: &mut Reader<'_>, depth: usize) -> Result<PPEExpr> {
    if depth > MAX_DEPTH || input.remaining_nodes == 0 {
        return Err(ContainerError::Limit("expression tree"));
    }
    input.remaining_nodes -= 1;
    let tag = input.word()?;
    let id = input.word()? as usize;
    Ok(match tag {
        1 => PPEExpr::Value(id),
        2 => PPEExpr::RoutineReference(id),
        3 => {
            let count = input.count(12)?;
            let fields = (0..count)
                .map(|_| Ok((input.word()? as usize, read_expr(input, depth + 1)?)))
                .collect::<Result<_>>()?;
            PPEExpr::RecordLiteral(id as u32, fields)
        }
        4 => PPEExpr::Member(Box::new(read_expr(input, depth + 1)?), id),
        5 | 11 => {
            let base = Box::new(read_expr(input, depth + 1)?);
            let args = read_args(input, depth)?;
            if tag == 5 {
                PPEExpr::IndexedMember(base, id, args)
            } else {
                PPEExpr::MemberFunctionCall(base, args, id)
            }
        }
        6 => PPEExpr::UnaryExpression(
            *UNARY.get(id).ok_or(ContainerError::Invalid("unary opcode"))?,
            Box::new(read_expr(input, depth + 1)?),
        ),
        7 => PPEExpr::BinaryExpression(
            *BINARY.get(id).ok_or(ContainerError::Invalid("binary opcode"))?,
            Box::new(read_expr(input, depth + 1)?),
            Box::new(read_expr(input, depth + 1)?),
        ),
        8 => PPEExpr::Dim(id, read_args(input, depth)?),
        9 => {
            let definition = FUNCTION_DEFINITIONS
                .iter()
                .find(|definition| definition.opcode as i32 == id as u32 as i32)
                .ok_or(ContainerError::Invalid("function opcode"))?;
            if definition.opcode.minimum_runtime() > 400 {
                return Err(ContainerError::Invalid("function runtime"));
            }
            let args = read_args(input, depth)?;
            if !matches!(definition.signature, super::FunctionSignature::FixedParameters(count) if count == args.len()) {
                return Err(ContainerError::Invalid("function argument count or signature"));
            }
            PPEExpr::PredefinedFunctionCall(definition, args)
        }
        10 => PPEExpr::FunctionCall(id, read_args(input, depth)?),
        _ => return Err(ContainerError::Invalid("expression opcode")),
    })
}

/// Byte size of the `CODE` section a program would occupy, so the compiler can
/// report an oversized program in the unit the container actually stores.
pub(crate) fn encoded_size(script: &PPEScript) -> Result<usize> {
    let mut normalized = script.clone();
    for statement in &mut normalized.statements {
        statement.command.normalize_control();
    }
    Ok(encode(&normalized)?.0.data.len())
}

pub(super) fn encode(script: &PPEScript) -> Result<(Section, BTreeMap<usize, u32>)> {
    if !script.bugged_offsets.is_empty() || script.statements.len() > MAX_ITEMS {
        return Err(ContainerError::Invalid("invalid script"));
    }
    let addresses: BTreeMap<_, _> = script
        .statements
        .iter()
        .enumerate()
        .map(|(index, statement)| (statement.span.start * 2, index as u32))
        .collect();
    if addresses.len() != script.statements.len() {
        return Err(ContainerError::Invalid("duplicate instruction address"));
    }
    let mut output = Vec::new();
    for statement in &script.statements {
        let mut record = Vec::new();
        let target = |address: usize| {
            addresses
                .get(&address)
                .copied()
                .map(|value| value as usize)
                .ok_or(ContainerError::Invalid("jump target"))
        };
        match &statement.command {
            PPECommand::End => word(&mut record, 0)?,
            PPECommand::Return => word(&mut record, 1)?,
            PPECommand::IfNot(expr, address) => {
                word(&mut record, 2)?;
                word(&mut record, target(*address)?)?;
                write_expr(&mut record, expr, 0)?;
            }
            PPECommand::ProcedureCall(id, args) => {
                word(&mut record, 3)?;
                word(&mut record, *id)?;
                write_args(&mut record, args, 0)?;
            }
            PPECommand::PredefinedCall(definition, args) => {
                word(&mut record, 4)?;
                word(&mut record, definition.opcode as usize)?;
                write_args(&mut record, args, 0)?;
            }
            PPECommand::Goto(address) | PPECommand::Gosub(address) | PPECommand::NextForEach(address) => {
                word(
                    &mut record,
                    match statement.command {
                        PPECommand::Goto(_) => 5,
                        PPECommand::Gosub(_) => 6,
                        _ => 14,
                    },
                )?;
                word(&mut record, target(*address)?)?;
            }
            PPECommand::EndFunc => word(&mut record, 7)?,
            PPECommand::EndProc => word(&mut record, 8)?,
            PPECommand::Stop => word(&mut record, 9)?,
            PPECommand::Let(left, right) => {
                word(&mut record, 10)?;
                write_expr(&mut record, left, 0)?;
                write_expr(&mut record, right, 0)?;
            }
            PPECommand::MemberCall(expr) => {
                word(&mut record, 11)?;
                write_expr(&mut record, expr, 0)?;
            }
            PPECommand::OnError(handler) => {
                word(&mut record, 12)?;
                let (mode, value) = match handler {
                    OnErrorTarget::Off => (0, 0),
                    OnErrorTarget::Goto(address) => (1, target(*address)?),
                    OnErrorTarget::Gosub(address) => (2, target(*address)?),
                    OnErrorTarget::Procedure(id) => (3, *id),
                };
                word(&mut record, mode)?;
                word(&mut record, value)?;
            }
            PPECommand::ForEach(id, expr, address) => {
                word(&mut record, 13)?;
                word(&mut record, *id)?;
                word(&mut record, target(*address)?)?;
                write_expr(&mut record, expr, 0)?;
            }
        }
        blob(&mut output, &record)?;
    }
    Ok((Section::new(*b"CODE", script.statements.len() as u32, output), addresses))
}

pub(super) fn decode(section: &Section) -> Result<PPEScript> {
    if section.entries as usize > MAX_ITEMS {
        return Err(ContainerError::Limit("instruction count"));
    }
    let mut input = Reader::new(&section.data);
    let mut script = PPEScript::default();
    let target = |id: u32| {
        if id < section.entries {
            Ok(id as usize * 2)
        } else {
            Err(ContainerError::Invalid("instruction reference"))
        }
    };
    for index in 0..section.entries as usize {
        let mut record = Reader::new(input.blob()?);
        record.remaining_nodes = input.remaining_nodes;
        let command = match record.word()? {
            0 => PPECommand::End,
            1 => PPECommand::Return,
            2 => {
                let address = target(record.word()?)?;
                PPECommand::IfNot(Box::new(read_expr(&mut record, 0)?), address)
            }
            3 => {
                let id = record.word()? as usize;
                PPECommand::ProcedureCall(id, read_args(&mut record, 0)?)
            }
            4 => {
                let opcode = record.word()?;
                let definition = STATEMENT_DEFINITIONS
                    .iter()
                    .find(|definition| definition.opcode as u32 == opcode)
                    .ok_or(ContainerError::Invalid("statement opcode"))?;
                let args = read_args(&mut record, 0)?;
                use super::StatementSignature;
                let valid = match definition.sig {
                    StatementSignature::Invalid => false,
                    StatementSignature::ArgumentsWithVariable(_, count) => args.len() == count,
                    StatementSignature::VariableArguments(_, min, max) => args.len() >= min && (max == 0 || args.len() <= max),
                    StatementSignature::SpecialCaseDlockg => args.len() == 3,
                    StatementSignature::SpecialCaseDcreate => args.len() == 4,
                    StatementSignature::SpecialCaseSort | StatementSignature::SpecialCaseVarSeg => args.len() == 2,
                    StatementSignature::SpecialCasePop => true,
                };
                if !valid || definition.opcode.minimum_runtime() > 400 {
                    return Err(ContainerError::Invalid("statement argument count or signature"));
                }
                let mut command = PPECommand::PredefinedCall(definition, args);
                command.normalize_control();
                command
            }
            5 => PPECommand::Goto(target(record.word()?)?),
            6 => PPECommand::Gosub(target(record.word()?)?),
            7 => PPECommand::EndFunc,
            8 => PPECommand::EndProc,
            9 => PPECommand::Stop,
            10 => PPECommand::Let(Box::new(read_expr(&mut record, 0)?), Box::new(read_expr(&mut record, 0)?)),
            11 => PPECommand::MemberCall(Box::new(read_expr(&mut record, 0)?)),
            12 => {
                let mode = record.word()?;
                let value = record.word()?;
                PPECommand::OnError(match mode {
                    0 if value == 0 => OnErrorTarget::Off,
                    1 => OnErrorTarget::Goto(target(value)?),
                    2 => OnErrorTarget::Gosub(target(value)?),
                    3 => OnErrorTarget::Procedure(value as usize),
                    _ => return Err(ContainerError::Invalid("error handler")),
                })
            }
            13 => {
                let id = record.word()? as usize;
                let address = target(record.word()?)?;
                PPECommand::ForEach(id, Box::new(read_expr(&mut record, 0)?), address)
            }
            14 => PPECommand::NextForEach(target(record.word()?)?),
            _ => return Err(ContainerError::Invalid("command opcode")),
        };
        input.remaining_nodes = record.remaining_nodes;
        record.finish()?;
        script.statements.push(PPEStatement {
            span: index..index + 1,
            command,
        });
    }
    input.finish()?;
    Ok(script)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_builtin_signatures_are_rejected() {
        for command in [
            PPECommand::PredefinedCall(super::super::OpCode::FCLOSE.get_definition(), vec![]),
            PPECommand::PredefinedCall(super::super::OpCode::PCALL.get_definition(), vec![]),
            PPECommand::MemberCall(Box::new(PPEExpr::PredefinedFunctionCall(
                super::super::FuncOpCode::LEN.get_definition(),
                vec![],
            ))),
        ] {
            let script = PPEScript {
                statements: vec![PPEStatement { span: 0..1, command }],
                ..Default::default()
            };
            assert!(decode(&encode(&script).unwrap().0).is_err());
        }
    }

    #[test]
    fn wide_references_and_short_circuit_roundtrip() {
        let command = PPECommand::Let(
            Box::new(PPEExpr::Value(80_000)),
            Box::new(PPEExpr::BinaryExpression(
                BinOp::ShortAnd,
                Box::new(PPEExpr::Value(90_000)),
                Box::new(PPEExpr::RecordLiteral(65_000, vec![(4_000, PPEExpr::Value(70_000))])),
            )),
        );
        let script = PPEScript {
            statements: vec![
                PPEStatement {
                    span: 0..50,
                    command: command.clone(),
                },
                PPEStatement {
                    span: 50..51,
                    command: PPECommand::Goto(0),
                },
            ],
            ..Default::default()
        };
        let (section, addresses) = encode(&script).unwrap();
        assert_eq!(Some(&1), addresses.get(&100));
        let restored = decode(&section).unwrap();
        assert_eq!(command, restored.statements[0].command);
        assert_eq!(PPECommand::Goto(0), restored.statements[1].command);
        for length in 0..section.data.len() {
            let mut truncated = section.clone();
            truncated.data.truncate(length);
            assert!(decode(&truncated).is_err(), "prefix {length}");
        }
        let mut unknown = section.clone();
        unknown.data[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode(&unknown).is_err());
    }
}

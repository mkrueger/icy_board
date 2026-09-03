use crate::{
    ast::{Constant, Expression},
    compiler::{CompilationErrorType, user_data::UserDataMemberRegistry},
    executable::{FIRST_STATIC_MEMBER_RUNTIME, FuncOpCode, OpCode, StatementSignature, VariableType},
    parser::{
        FIRST_BOARD_OBJECT_LANGUAGE_VERSION,
        lexer::{Spanned, Token},
    },
};

use super::SemanticVisitor;

pub(super) enum StaticReceiver {
    Instance(u8),
    StaticMember(u8),
    NotAType,
    Rejected,
}

/// A built-in array function that may also be written as a member of the array.
///
/// Every function that takes an array first has an entry here, so `a.Len(0)` and
/// `Len(a, 0)` are the same call and neither can drift from the other.
pub struct ArrayMember {
    pub name: &'static str,
    pub opcode: FuncOpCode,
    /// Arguments the member takes, on top of the array itself.
    pub arguments: std::ops::RangeInclusive<usize>,
    /// What a left out trailing argument stands for.
    pub defaults: &'static [i32],
    pub return_type: VariableType,
}

/// The members every array carries. `Redim` is a statement rather than a function
/// and is resolved next to the other member call statements.
pub const ARRAY_MEMBERS: &[ArrayMember] = &[ArrayMember {
    name: "Len",
    opcode: FuncOpCode::Len_Dim,
    arguments: 0..=1,
    defaults: &[-1],
    return_type: VariableType::Integer,
}];

pub fn array_member(name: &unicase::Ascii<String>) -> Option<&'static ArrayMember> {
    ARRAY_MEMBERS.iter().find(|member| *name == member.name)
}

pub struct ScalarMember {
    pub name: &'static str,
    pub arguments: std::ops::RangeInclusive<usize>,
    pub return_type: VariableType,
    pub is_static: bool,
}

pub const STRING_MEMBERS: &[ScalarMember] = &[
    ScalarMember {
        name: "Len",
        arguments: 0..=0,
        return_type: VariableType::Integer,
        is_static: false,
    },
    ScalarMember {
        name: "Find",
        arguments: 1..=3,
        return_type: VariableType::Integer,
        is_static: false,
    },
    ScalarMember {
        name: "FindLast",
        arguments: 1..=3,
        return_type: VariableType::Integer,
        is_static: false,
    },
    ScalarMember {
        name: "Contains",
        arguments: 1..=2,
        return_type: VariableType::Boolean,
        is_static: false,
    },
    ScalarMember {
        name: "StartsWith",
        arguments: 1..=2,
        return_type: VariableType::Boolean,
        is_static: false,
    },
    ScalarMember {
        name: "EndsWith",
        arguments: 1..=2,
        return_type: VariableType::Boolean,
        is_static: false,
    },
    ScalarMember {
        name: "Count",
        arguments: 1..=2,
        return_type: VariableType::Integer,
        is_static: false,
    },
    ScalarMember {
        name: "Equals",
        arguments: 1..=2,
        return_type: VariableType::Boolean,
        is_static: false,
    },
    ScalarMember {
        name: "Replace",
        arguments: 2..=2,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "Trim",
        arguments: 0..=1,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "TrimStart",
        arguments: 0..=1,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "TrimEnd",
        arguments: 0..=1,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "ToUpper",
        arguments: 0..=0,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "ToLower",
        arguments: 0..=0,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "Substring",
        arguments: 2..=2,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "Left",
        arguments: 1..=1,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "Right",
        arguments: 1..=1,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "Split",
        arguments: 1..=2,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "Join",
        arguments: 2..=2,
        return_type: VariableType::UnboundedString,
        is_static: true,
    },
    ScalarMember {
        name: "Repeat",
        arguments: 2..=2,
        return_type: VariableType::UnboundedString,
        is_static: true,
    },
    ScalarMember {
        name: "Split",
        arguments: 2..=3,
        return_type: VariableType::UnboundedString,
        is_static: true,
    },
    ScalarMember {
        name: "PadLeft",
        arguments: 1..=2,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "PadRight",
        arguments: 1..=2,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "Remove",
        arguments: 2..=2,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "Insert",
        arguments: 2..=2,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "Reverse",
        arguments: 0..=0,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "ToInt",
        arguments: 0..=1,
        return_type: VariableType::Integer,
        is_static: false,
    },
    ScalarMember {
        name: "ToMixedCase",
        arguments: 0..=0,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "StripATX",
        arguments: 0..=0,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
];

pub const BYTES_MEMBERS: &[ScalarMember] = &[
    ScalarMember {
        name: "Len",
        arguments: 0..=0,
        return_type: VariableType::Integer,
        is_static: false,
    },
    ScalarMember {
        name: "ToString",
        arguments: 0..=0,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "ToBase64",
        arguments: 0..=0,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "ToHex",
        arguments: 0..=0,
        return_type: VariableType::UnboundedString,
        is_static: false,
    },
    ScalarMember {
        name: "GetChecksum",
        arguments: 1..=1,
        return_type: VariableType::Bytes,
        is_static: false,
    },
    ScalarMember {
        name: "FromBase64",
        arguments: 1..=1,
        return_type: VariableType::Bytes,
        is_static: true,
    },
];

pub(super) fn string_member(name: &unicase::Ascii<String>, arguments: usize) -> Option<(FuncOpCode, VariableType, &'static [i32])> {
    let normalized = name.as_ref().to_ascii_lowercase();
    match (normalized.as_str(), arguments) {
        ("len", 0) => Some((FuncOpCode::LEN, VariableType::Integer, &[])),
        ("find", 1) => Some((FuncOpCode::StringFindFrom, VariableType::Integer, &[0])),
        ("find", 2) => Some((FuncOpCode::StringFindFrom, VariableType::Integer, &[])),
        ("find", 3) => Some((FuncOpCode::StringFindComparison, VariableType::Integer, &[])),
        ("findlast", 1) => Some((FuncOpCode::StringFindLastFrom, VariableType::Integer, &[i32::MAX])),
        ("findlast", 2) => Some((FuncOpCode::StringFindLastFrom, VariableType::Integer, &[])),
        ("findlast", 3) => Some((FuncOpCode::StringFindLastComparison, VariableType::Integer, &[])),
        ("contains", 1) => Some((FuncOpCode::StringContains, VariableType::Boolean, &[])),
        ("contains", 2) => Some((FuncOpCode::StringContainsComparison, VariableType::Boolean, &[])),
        ("startswith", 1) => Some((FuncOpCode::StringStartsWith, VariableType::Boolean, &[])),
        ("startswith", 2) => Some((FuncOpCode::StringStartsWithComparison, VariableType::Boolean, &[])),
        ("endswith", 1) => Some((FuncOpCode::StringEndsWith, VariableType::Boolean, &[])),
        ("endswith", 2) => Some((FuncOpCode::StringEndsWithComparison, VariableType::Boolean, &[])),
        ("count", 1) => Some((FuncOpCode::StringCount, VariableType::Integer, &[])),
        ("count", 2) => Some((FuncOpCode::StringCountComparison, VariableType::Integer, &[])),
        ("equals", 1) => Some((FuncOpCode::StringEquals, VariableType::Boolean, &[])),
        ("equals", 2) => Some((FuncOpCode::StringEqualsComparison, VariableType::Boolean, &[])),
        ("replace", 2) => Some((FuncOpCode::REPLACESTR, VariableType::UnboundedString, &[])),
        ("trim", 0) => Some((FuncOpCode::StringTrim, VariableType::UnboundedString, &[])),
        ("trim", 1) => Some((FuncOpCode::StringTrimChars, VariableType::UnboundedString, &[])),
        ("trimstart", 0) => Some((FuncOpCode::StringTrimStart, VariableType::UnboundedString, &[])),
        ("trimstart", 1) => Some((FuncOpCode::StringTrimStartChars, VariableType::UnboundedString, &[])),
        ("trimend", 0) => Some((FuncOpCode::StringTrimEnd, VariableType::UnboundedString, &[])),
        ("trimend", 1) => Some((FuncOpCode::StringTrimEndChars, VariableType::UnboundedString, &[])),
        ("toupper", 0) => Some((FuncOpCode::UPPER, VariableType::UnboundedString, &[])),
        ("tolower", 0) => Some((FuncOpCode::LOWER, VariableType::UnboundedString, &[])),
        ("split", 1) => Some((FuncOpCode::StringSplit, VariableType::UnboundedString, &[])),
        ("split", 2) => Some((FuncOpCode::StringSplitLimit, VariableType::UnboundedString, &[])),
        ("substring", 2) => Some((FuncOpCode::StringSubstring, VariableType::UnboundedString, &[])),
        ("left", 1) => Some((FuncOpCode::LEFT, VariableType::UnboundedString, &[])),
        ("right", 1) => Some((FuncOpCode::RIGHT, VariableType::UnboundedString, &[])),
        ("padleft", 1) => Some((FuncOpCode::StringPadLeft, VariableType::UnboundedString, &[])),
        ("padleft", 2) => Some((FuncOpCode::StringPadLeftChar, VariableType::UnboundedString, &[])),
        ("padright", 1) => Some((FuncOpCode::StringPadRight, VariableType::UnboundedString, &[])),
        ("padright", 2) => Some((FuncOpCode::StringPadRightChar, VariableType::UnboundedString, &[])),
        ("remove", 2) => Some((FuncOpCode::StringRemove, VariableType::UnboundedString, &[])),
        ("insert", 2) => Some((FuncOpCode::StringInsert, VariableType::UnboundedString, &[])),
        ("reverse", 0) => Some((FuncOpCode::StringReverse, VariableType::UnboundedString, &[])),
        ("toint", 0) => Some((FuncOpCode::StringToInt, VariableType::Integer, &[10])),
        ("toint", 1) => Some((FuncOpCode::StringToInt, VariableType::Integer, &[])),
        ("tomixedcase", 0) => Some((FuncOpCode::StringToMixedCase, VariableType::UnboundedString, &[])),
        ("stripatx", 0) => Some((FuncOpCode::StringStripAtx, VariableType::UnboundedString, &[])),
        _ => None,
    }
}

pub(super) fn string_member_type(name: &unicase::Ascii<String>) -> Option<VariableType> {
    STRING_MEMBERS
        .iter()
        .find(|member| !member.is_static && *name == member.name)
        .map(|member| member.return_type)
}

pub(super) fn bytes_member_type(name: &unicase::Ascii<String>) -> Option<VariableType> {
    BYTES_MEMBERS
        .iter()
        .find(|member| !member.is_static && *name == member.name)
        .map(|member| member.return_type)
}

pub(super) fn bytes_member(name: &unicase::Ascii<String>, arguments: usize) -> Option<(FuncOpCode, VariableType)> {
    let normalized = name.as_ref().to_ascii_lowercase();
    match (normalized.as_str(), arguments) {
        ("len", 0) => Some((FuncOpCode::LEN, VariableType::Integer)),
        ("tostring", 0) => Some((FuncOpCode::BytesToString, VariableType::UnboundedString)),
        ("tobase64", 0) => Some((FuncOpCode::BASE64ENC, VariableType::UnboundedString)),
        ("tohex", 0) => Some((FuncOpCode::BytesToHex, VariableType::UnboundedString)),
        ("getchecksum", 1) => Some((FuncOpCode::BytesGetChecksum, VariableType::Bytes)),
        _ => None,
    }
}

pub(super) fn string_type_name(expression: &Expression, lang_version: u16) -> bool {
    let Expression::Identifier(identifier) = expression else {
        return false;
    };
    matches!(
        crate::parser::built_in_type(identifier.get_identifier(), lang_version),
        Some(VariableType::String | VariableType::BigStr | VariableType::UnboundedString)
    )
}

/// The built-in array statements that may also be written as a member. `REDIM` is
/// the only one, and it takes one bound per dimension.
pub const ARRAY_PROCEDURES: &[(&str, OpCode, std::ops::RangeInclusive<usize>)] = &[("Redim", OpCode::REDIM, 1..=3)];

pub fn array_procedure(name: &unicase::Ascii<String>) -> Option<&'static (&'static str, OpCode, std::ops::RangeInclusive<usize>)> {
    ARRAY_PROCEDURES.iter().find(|(member, _, _)| *name == *member)
}

/// True where a statement wants the array itself rather than one of its elements,
/// the positions `PCBoard` compiled with `wrVID` instead of `wrVIDSUB`.
pub(super) fn takes_whole_array(opcode: OpCode, signature: StatementSignature, index: usize) -> bool {
    if opcode == OpCode::REDIM {
        return index == 0;
    }
    match signature {
        StatementSignature::SpecialCaseDlockg => index == 2,
        StatementSignature::SpecialCaseDcreate => index == 3,
        StatementSignature::SpecialCaseSort => index < 2,
        StatementSignature::Invalid
        | StatementSignature::ArgumentsWithVariable(_, _)
        | StatementSignature::VariableArguments(_, _, _)
        | StatementSignature::SpecialCaseVarSeg
        | StatementSignature::SpecialCasePop => false,
    }
}

impl SemanticVisitor {
    /// Looks a callable member up on a board object.
    pub(super) fn member_function_signature(
        &self,
        user_type: u8,
        name: &unicase::Ascii<String>,
    ) -> Option<(usize, usize, Vec<VariableType>, VariableType, u8)> {
        let registry = self.type_registry.get_type_from_id(user_type)?;
        let function = registry.functions.get(name)?;
        let member_id = registry.get_member_id(name)?;
        Some((
            member_id,
            function.required,
            function.parameters.clone(),
            function.return_type,
            function.return_rank,
        ))
    }

    pub(super) fn check_member_arg_types(&mut self, expected: &[VariableType], arguments: &[Expression]) {
        for (index, (argument, expected)) in arguments.iter().zip(expected).enumerate() {
            let actual = argument.visit(self);
            self.reject_bare_array_value(argument);
            if *expected != actual && (matches!(expected, VariableType::UserData(_)) || matches!(actual, VariableType::UserData(_))) {
                self.errors.lock().unwrap().report_error(
                    argument.get_span(),
                    CompilationErrorType::ArgumentTypeMismatch(index + 1, self.source_type_name(*expected), self.source_type_name(actual)),
                );
            }
        }
        for argument in arguments.iter().skip(expected.len()) {
            argument.visit(self);
        }
    }

    /// Walks a member receiver once and caches nested expressions.
    pub(super) fn visit_receiver(&mut self, receiver: &Expression, member_token: &Spanned<Token>) -> VariableType {
        if matches!(receiver, Expression::Identifier(_)) {
            return receiver.visit(self);
        }
        let key = member_token.span.start;
        if let Some(cached) = self.receiver_types.get(&key) {
            return *cached;
        }
        let receiver_type = receiver.visit(self);
        self.receiver_types.insert(key, receiver_type);
        receiver_type
    }

    pub(super) fn static_receiver(&mut self, expression: &Expression, member: &unicase::Ascii<String>) -> StaticReceiver {
        let Expression::Identifier(base) = expression else {
            return StaticReceiver::NotAType;
        };
        let identifier = base.get_identifier();
        if self.lookup_variable(identifier).is_some() {
            return StaticReceiver::NotAType;
        }
        let Some(VariableType::UserData(type_id)) = self.type_registry.get_board_object(identifier) else {
            return StaticReceiver::NotAType;
        };
        let span = base.get_identifier_token().span.clone();

        if self.lang_version < FIRST_BOARD_OBJECT_LANGUAGE_VERSION {
            self.errors
                .lock()
                .unwrap()
                .report_error(span, CompilationErrorType::TypeUsedAsValue(identifier.to_string()));
            return StaticReceiver::Rejected;
        }

        if self
            .type_registry
            .get_type_from_id(type_id)
            .is_some_and(|registry| registry.statics.contains(member))
        {
            if self.runtime < FIRST_STATIC_MEMBER_RUNTIME {
                self.errors.lock().unwrap().report_error(
                    span,
                    CompilationErrorType::BuiltinNeedsRuntime(format!("{identifier}.{member}"), FIRST_STATIC_MEMBER_RUNTIME),
                );
                return StaticReceiver::Rejected;
            }
            self.add_constant(&Constant::Integer(i32::from(type_id), crate::ast::constant::NumberFormat::Default));
            self.static_receiver_lookup.insert(span.start, type_id);
            return StaticReceiver::StaticMember(type_id);
        }

        let provider = self.type_registry.get_type_from_id(type_id).and_then(|registry| registry.instance_provider);
        let Some(provider) = provider else {
            self.errors
                .lock()
                .unwrap()
                .report_error(span, CompilationErrorType::TypeUsedAsValue(identifier.to_string()));
            return StaticReceiver::Rejected;
        };
        let minimum_runtime = provider.minimum_runtime();
        if self.runtime < minimum_runtime {
            self.errors.lock().unwrap().report_error(
                span.clone(),
                CompilationErrorType::BuiltinNeedsRuntime(provider.get_definition().name.to_string(), minimum_runtime),
            );
            return StaticReceiver::Rejected;
        }
        self.instance_provider_lookup.insert(span.start, provider);
        StaticReceiver::Instance(type_id)
    }
}

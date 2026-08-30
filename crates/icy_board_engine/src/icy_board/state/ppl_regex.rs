use async_trait::async_trait;
use regex::{Regex, RegexBuilder};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    icy_board::state::ppl_error::{ERR_INVALID, ERR_KIND_REGEX, PplError},
    parser::{REGEX_ID, REGEX_MATCH_ID, REGEX_OPTIONS_ENUM_ID},
};

const MAX_REGEX_RESULTS: usize = 100_000;
const MAX_REGEX_OUTPUT: usize = 16 * 1024 * 1024;

const OPTION_IGNORE_CASE: i32 = 1;
const OPTION_MULTI_LINE: i32 = 2;
const OPTION_DOT_MATCHES_NEW_LINE: i32 = 4;
const OPTION_IGNORE_WHITESPACE: i32 = 8;
const OPTION_SWAP_GREED: i32 = 16;
const OPTION_ASCII: i32 = 32;
const VALID_OPTIONS: i32 = OPTION_IGNORE_CASE | OPTION_MULTI_LINE | OPTION_DOT_MATCHES_NEW_LINE | OPTION_IGNORE_WHITESPACE | OPTION_SWAP_GREED | OPTION_ASCII;

pub static VALID: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Valid".to_string()));
pub static PATTERN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Pattern".to_string()));
pub static COMPILE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Compile".to_string()));
pub static ESCAPE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Escape".to_string()));
pub static IS_VALID: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("IsValid".to_string()));
pub static IS_MATCH: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("IsMatch".to_string()));
pub static FIND: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Find".to_string()));
pub static FIND_ALL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("FindAll".to_string()));
pub static REPLACE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Replace".to_string()));
pub static SPLIT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Split".to_string()));
pub(crate) static OPTIONS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("<options>".to_string()));
pub static SUCCESS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Success".to_string()));
pub static VALUE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Value".to_string()));
pub static START: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Start".to_string()));
pub static LENGTH: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Length".to_string()));
pub static GROUP_COUNT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("GroupCount".to_string()));
pub static GROUP: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Group".to_string()));
pub static NAMED_GROUP: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("NamedGroup".to_string()));
pub static GROUP_MATCHED: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("GroupMatched".to_string()));
pub static NAMED_GROUP_MATCHED: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("NamedGroupMatched".to_string()));
pub static GROUP_START: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("GroupStart".to_string()));
pub static NAMED_GROUP_START: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("NamedGroupStart".to_string()));
pub static GROUP_LENGTH: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("GroupLength".to_string()));
pub static NAMED_GROUP_LENGTH: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("NamedGroupLength".to_string()));

#[derive(Clone)]
struct PplRegexGroup {
    name: Option<String>,
    value: String,
    start: i32,
    length: i32,
    matched: bool,
}

impl Default for PplRegexGroup {
    fn default() -> Self {
        Self {
            name: None,
            value: String::new(),
            start: -1,
            length: 0,
            matched: false,
        }
    }
}

#[derive(Clone, Default)]
pub struct PplRegexMatch {
    groups: Vec<PplRegexGroup>,
}

impl PplRegexMatch {
    fn from_captures(regex: &Regex, text: &str, captures: &regex::Captures<'_>, base: usize) -> Self {
        let groups = regex
            .capture_names()
            .enumerate()
            .map(|(index, name)| {
                let Some(found) = captures.get(index) else {
                    return PplRegexGroup {
                        name: name.map(str::to_string),
                        ..PplRegexGroup::default()
                    };
                };
                let byte_start = base + found.start();
                PplRegexGroup {
                    name: name.map(str::to_string),
                    value: found.as_str().to_string(),
                    start: text[..byte_start].chars().count() as i32,
                    length: found.as_str().chars().count() as i32,
                    matched: true,
                }
            })
            .collect();
        Self { groups }
    }

    fn value(self) -> VariableValue {
        user_data_value(self, REGEX_MATCH_ID)
    }

    fn array_value(matches: Vec<Self>) -> VariableValue {
        VariableValue::new_vector(VariableType::UserData(REGEX_MATCH_ID as u8), matches.into_iter().map(Self::value).collect())
    }

    fn group(&self, index: i32) -> Option<&PplRegexGroup> {
        usize::try_from(index).ok().and_then(|index| self.groups.get(index))
    }

    fn named_group(&self, name: &str) -> Option<&PplRegexGroup> {
        self.groups.iter().find(|group| group.name.as_deref() == Some(name))
    }
}

#[derive(Clone, Default)]
pub struct PplRegex {
    pattern: String,
    options: i32,
    compiled: Option<Regex>,
    error: String,
}

impl PplRegex {
    pub fn value(self) -> VariableValue {
        user_data_value(self, REGEX_ID)
    }

    pub fn invalid() -> VariableValue {
        Self::default().value()
    }

    pub(crate) fn compile_pattern(pattern: String, options: i32) -> Result<Self, String> {
        if options & !VALID_OPTIONS != 0 {
            return Err(format!("REGEX options contain unknown flags: {}", options & !VALID_OPTIONS));
        }

        let mut builder = RegexBuilder::new(&pattern);
        builder
            .case_insensitive(options & OPTION_IGNORE_CASE != 0)
            .multi_line(options & OPTION_MULTI_LINE != 0)
            .dot_matches_new_line(options & OPTION_DOT_MATCHES_NEW_LINE != 0)
            .ignore_whitespace(options & OPTION_IGNORE_WHITESPACE != 0)
            .swap_greed(options & OPTION_SWAP_GREED != 0)
            .unicode(options & OPTION_ASCII == 0);

        builder
            .build()
            .map(|compiled| Self {
                pattern,
                options,
                compiled: Some(compiled),
                error: String::new(),
            })
            .map_err(|error| error.to_string())
    }

    fn start_byte(text: &str, start: i32) -> Option<usize> {
        if start < 0 {
            return None;
        }
        let start = start as usize;
        text.char_indices()
            .nth(start)
            .map(|(offset, _)| offset)
            .or_else(|| (start == text.chars().count()).then_some(text.len()))
    }
}

impl UserData for PplRegex {
    const TYPE_NAME: &'static str = "Regex";
    const STATIC_RECEIVER: Option<fn() -> VariableValue> = Some(PplRegex::invalid);

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(VALID.clone(), VariableType::Boolean, false);
        registry.add_property(PATTERN.clone(), VariableType::String, false);
        registry.add_named_static_function_with(
            COMPILE.clone(),
            vec![("pattern", VariableType::String), ("options", VariableType::UserData(REGEX_OPTIONS_ENUM_ID))],
            1,
            VariableType::UserData(REGEX_ID as u8),
        );
        registry.add_named_static_function(ESCAPE.clone(), vec![("text", VariableType::String)], VariableType::String);
        registry.add_named_static_function_with(
            IS_VALID.clone(),
            vec![("pattern", VariableType::String), ("options", VariableType::UserData(REGEX_OPTIONS_ENUM_ID))],
            1,
            VariableType::Boolean,
        );
        registry.add_named_function_with(
            IS_MATCH.clone(),
            vec![("text", VariableType::String), ("start", VariableType::Integer)],
            1,
            VariableType::Boolean,
        );
        registry.add_named_function_with(
            FIND.clone(),
            vec![("text", VariableType::String), ("start", VariableType::Integer)],
            1,
            VariableType::UserData(REGEX_MATCH_ID as u8),
        );
        registry.add_named_array_function_with(
            FIND_ALL.clone(),
            vec![("text", VariableType::String), ("start", VariableType::Integer), ("limit", VariableType::Integer)],
            1,
            VariableType::UserData(REGEX_MATCH_ID as u8),
            1,
        );
        registry.add_named_function_with(
            REPLACE.clone(),
            vec![("text", VariableType::String), ("replacement", VariableType::String), ("limit", VariableType::Integer)],
            2,
            VariableType::String,
        );
        registry.add_named_array_function_with(
            SPLIT.clone(),
            vec![("text", VariableType::String), ("limit", VariableType::Integer)],
            1,
            VariableType::String,
            1,
        );
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplRegex {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *VALID {
            return Ok(VariableValue::new_bool(self.compiled.is_some()));
        }
        if *name == *PATTERN {
            return Ok(VariableValue::new_string(self.pattern.clone()).convert_to(VariableType::BigStr));
        }
        if *name == *OPTIONS {
            return Ok(VariableValue::new_int(self.options));
        }
        Err(format!("Unknown REGEX property {name}").into())
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("REGEX property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *COMPILE {
            let pattern = arguments[0].as_string();
            let options = arguments.get(1).map_or(0, VariableValue::as_int);
            return match Self::compile_pattern(pattern, options) {
                Ok(regex) => {
                    vm.operation_succeeded();
                    Ok(regex.value())
                }
                Err(message) => {
                    vm.set_error(PplError::new(ERR_KIND_REGEX, ERR_INVALID, message.clone()));
                    Ok(Self {
                        pattern: arguments[0].as_string(),
                        options,
                        compiled: None,
                        error: message,
                    }
                    .value())
                }
            };
        }
        if *name == *ESCAPE {
            vm.operation_succeeded();
            return Ok(VariableValue::new_string(regex::escape(&arguments[0].as_string())).convert_to(VariableType::BigStr));
        }
        if *name == *IS_VALID {
            let options = arguments.get(1).map_or(0, VariableValue::as_int);
            vm.operation_succeeded();
            return Ok(VariableValue::new_bool(Self::compile_pattern(arguments[0].as_string(), options).is_ok()));
        }
        if *name == *IS_MATCH {
            let Some(compiled) = &self.compiled else {
                vm.set_error(PplError::new(ERR_KIND_REGEX, ERR_INVALID, self.error.clone()));
                return Ok(VariableValue::new_bool(false));
            };
            let text = arguments[0].as_string();
            let start = arguments.get(1).map_or(0, VariableValue::as_int);
            let matched = Self::start_byte(&text, start).is_some_and(|offset| compiled.find_at(&text, offset).is_some());
            vm.operation_succeeded();
            return Ok(VariableValue::new_bool(matched));
        }
        if *name == *FIND || *name == *FIND_ALL {
            let Some(compiled) = &self.compiled else {
                vm.set_error(PplError::new(ERR_KIND_REGEX, ERR_INVALID, self.error.clone()));
                return Ok(if *name == *FIND {
                    PplRegexMatch::default().value()
                } else {
                    PplRegexMatch::array_value(Vec::new())
                });
            };
            let text = arguments[0].as_string();
            let start = arguments.get(1).map_or(0, VariableValue::as_int);
            let Some(offset) = Self::start_byte(&text, start) else {
                vm.operation_succeeded();
                return Ok(if *name == *FIND {
                    PplRegexMatch::default().value()
                } else {
                    PplRegexMatch::array_value(Vec::new())
                });
            };
            if *name == *FIND {
                let found = compiled
                    .captures_at(&text, offset)
                    .map(|captures| PplRegexMatch::from_captures(compiled, &text, &captures, 0))
                    .unwrap_or_default();
                vm.operation_succeeded();
                return Ok(found.value());
            }
            let limit = arguments.get(2).map_or(0, VariableValue::as_int);
            if limit < 0 {
                vm.set_error(PplError::new(ERR_KIND_REGEX, ERR_INVALID, "REGEX.FindAll limit cannot be negative"));
                return Ok(PplRegexMatch::array_value(Vec::new()));
            }
            if limit as usize > MAX_REGEX_RESULTS {
                vm.set_error(PplError::new(
                    ERR_KIND_REGEX,
                    crate::icy_board::state::ppl_error::ERR_LIMIT,
                    "REGEX.FindAll limit exceeds 100000 matches",
                ));
                return Ok(PplRegexMatch::array_value(Vec::new()));
            }
            let maximum = if limit == 0 { MAX_REGEX_RESULTS } else { limit as usize };
            let take = if limit == 0 { maximum.saturating_add(1) } else { maximum };
            let matches: Vec<_> = compiled
                .captures_iter(&text)
                .skip_while(|captures| captures.get(0).is_some_and(|found| found.start() < offset))
                .take(take)
                .map(|captures| PplRegexMatch::from_captures(compiled, &text, &captures, 0))
                .collect();
            if limit == 0 && matches.len() > maximum {
                vm.set_error(PplError::new(
                    ERR_KIND_REGEX,
                    crate::icy_board::state::ppl_error::ERR_LIMIT,
                    "REGEX.FindAll result exceeds 100000 matches",
                ));
                return Ok(PplRegexMatch::array_value(Vec::new()));
            }
            vm.operation_succeeded();
            return Ok(PplRegexMatch::array_value(matches));
        }
        if *name == *REPLACE {
            let Some(compiled) = &self.compiled else {
                vm.set_error(PplError::new(ERR_KIND_REGEX, ERR_INVALID, self.error.clone()));
                return Ok(VariableValue::new_string(String::new()).convert_to(VariableType::BigStr));
            };
            let text = arguments[0].as_string();
            let replacement = arguments[1].as_string();
            let limit = arguments.get(2).map_or(0, VariableValue::as_int);
            if limit < 0 {
                vm.set_error(PplError::new(ERR_KIND_REGEX, ERR_INVALID, "REGEX.Replace limit cannot be negative"));
                return Ok(VariableValue::new_string(String::new()).convert_to(VariableType::BigStr));
            }
            let replaced = if limit == 0 {
                compiled.replace_all(&text, replacement.as_str())
            } else {
                compiled.replacen(&text, limit as usize, replacement.as_str())
            };
            if replaced.len() > MAX_REGEX_OUTPUT {
                vm.set_error(PplError::new(
                    ERR_KIND_REGEX,
                    crate::icy_board::state::ppl_error::ERR_LIMIT,
                    "REGEX.Replace result exceeds 16 MiB",
                ));
                return Ok(VariableValue::new_string(String::new()).convert_to(VariableType::BigStr));
            }
            vm.operation_succeeded();
            return Ok(VariableValue::new_string(replaced.into_owned()).convert_to(VariableType::BigStr));
        }
        if *name == *SPLIT {
            let Some(compiled) = &self.compiled else {
                vm.set_error(PplError::new(ERR_KIND_REGEX, ERR_INVALID, self.error.clone()));
                return Ok(VariableValue::new_vector(VariableType::BigStr, Vec::new()));
            };
            let text = arguments[0].as_string();
            let limit = arguments.get(1).map_or(0, VariableValue::as_int);
            if limit < 0 {
                vm.set_error(PplError::new(ERR_KIND_REGEX, ERR_INVALID, "REGEX.Split limit cannot be negative"));
                return Ok(VariableValue::new_vector(VariableType::BigStr, Vec::new()));
            }
            let parts: Vec<_> = if limit == 0 {
                compiled.split(&text).map(str::to_string).collect()
            } else {
                compiled.splitn(&text, limit as usize).map(str::to_string).collect()
            };
            vm.operation_succeeded();
            return Ok(VariableValue::new_vector(
                VariableType::BigStr,
                parts
                    .into_iter()
                    .map(|part| VariableValue::new_string(part).convert_to(VariableType::BigStr))
                    .collect(),
            ));
        }
        Err(format!("Unknown REGEX function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown REGEX method {name}").into())
    }
}

impl UserData for PplRegexMatch {
    const TYPE_NAME: &'static str = "RegexMatch";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(SUCCESS.clone(), VariableType::Boolean, false);
        registry.add_property(VALUE.clone(), VariableType::String, false);
        registry.add_property(START.clone(), VariableType::Integer, false);
        registry.add_property(LENGTH.clone(), VariableType::Integer, false);
        registry.add_property(GROUP_COUNT.clone(), VariableType::Integer, false);
        registry.add_named_function(GROUP.clone(), vec![("index", VariableType::Integer)], VariableType::String);
        registry.add_named_function(NAMED_GROUP.clone(), vec![("name", VariableType::String)], VariableType::String);
        registry.add_named_function(GROUP_MATCHED.clone(), vec![("index", VariableType::Integer)], VariableType::Boolean);
        registry.add_named_function(NAMED_GROUP_MATCHED.clone(), vec![("name", VariableType::String)], VariableType::Boolean);
        registry.add_named_function(GROUP_START.clone(), vec![("index", VariableType::Integer)], VariableType::Integer);
        registry.add_named_function(NAMED_GROUP_START.clone(), vec![("name", VariableType::String)], VariableType::Integer);
        registry.add_named_function(GROUP_LENGTH.clone(), vec![("index", VariableType::Integer)], VariableType::Integer);
        registry.add_named_function(NAMED_GROUP_LENGTH.clone(), vec![("name", VariableType::String)], VariableType::Integer);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplRegexMatch {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let whole = self.groups.first();
        if *name == *SUCCESS {
            return Ok(VariableValue::new_bool(whole.is_some_and(|group| group.matched)));
        }
        if *name == *VALUE {
            return Ok(VariableValue::new_string(whole.map(|group| group.value.clone()).unwrap_or_default()).convert_to(VariableType::BigStr));
        }
        if *name == *START {
            return Ok(VariableValue::new_int(whole.map_or(-1, |group| group.start)));
        }
        if *name == *LENGTH {
            return Ok(VariableValue::new_int(whole.map_or(0, |group| group.length)));
        }
        if *name == *GROUP_COUNT {
            return Ok(VariableValue::new_int(self.groups.len().saturating_sub(1) as i32));
        }
        Err(format!("Unknown REGEXMATCH property {name}").into())
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("REGEXMATCH property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        let group = if *name == *GROUP || *name == *GROUP_MATCHED || *name == *GROUP_START || *name == *GROUP_LENGTH {
            self.group(arguments[0].as_int())
        } else {
            self.named_group(&arguments[0].as_string())
        };
        let Some(group) = group else {
            vm.set_error(PplError::new(ERR_KIND_REGEX, ERR_INVALID, "REGEXMATCH group does not exist"));
            return Ok(if *name == *GROUP || *name == *NAMED_GROUP {
                VariableValue::new_string(String::new()).convert_to(VariableType::BigStr)
            } else if *name == *GROUP_MATCHED || *name == *NAMED_GROUP_MATCHED {
                VariableValue::new_bool(false)
            } else {
                VariableValue::new_int(0)
            });
        };
        vm.operation_succeeded();
        if *name == *GROUP || *name == *NAMED_GROUP {
            return Ok(VariableValue::new_string(group.value.clone()).convert_to(VariableType::BigStr));
        }
        if *name == *GROUP_MATCHED || *name == *NAMED_GROUP_MATCHED {
            return Ok(VariableValue::new_bool(group.matched));
        }
        if *name == *GROUP_START || *name == *NAMED_GROUP_START {
            return Ok(VariableValue::new_int(group.start));
        }
        if *name == *GROUP_LENGTH || *name == *NAMED_GROUP_LENGTH {
            return Ok(VariableValue::new_int(group.length));
        }
        Err(format!("Unknown REGEXMATCH function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown REGEXMATCH method {name}").into())
    }
}

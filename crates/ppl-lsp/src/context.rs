//! What the cursor sits in, read from the line it sits on.
//!
//! Completion has to work while the line is still incomplete - `conf.` does not
//! parse - so the context is taken from the text rather than from the AST.

/// Where the cursor is, as far as completion is concerned.
#[derive(Debug, PartialEq)]
pub enum CursorContext {
    /// After a `.`, with the expression left of it split into its parts:
    /// `member.Home.` gives `["member", "Home"]`.
    Member(Vec<String>),

    /// A field name inside `Type { ... }`, with the fields already named.
    RecordLiteralField {
        type_name: String,
        named_fields: Vec<String>,
    },

    /// Inside a string or a comment, where nothing should be offered.
    Nothing,

    Other,
}

/// The call the cursor is writing arguments for.
#[derive(Debug, PartialEq)]
pub struct CallContext {
    pub name: String,
    /// Zero based index of the argument the cursor is in.
    pub argument: usize,
    /// True when the call is written without parentheses, the way a built-in
    /// statement takes its arguments.
    pub bare: bool,
}

fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Stands for `[...]` in a member chain. It is the collection's own getter, which is
/// why it cannot collide with anything a source can write.
pub const INDEXED: &str = "<get>";

/// The part of the line that is code, cut off where a string or comment starts.
/// `None` when the cursor itself sits inside one.
fn code_before_cursor(line: &str) -> Option<&str> {
    let bytes: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            '"' => {
                i += 1;
                loop {
                    if i >= bytes.len() {
                        return None;
                    }
                    if bytes[i] == '"' {
                        // A doubled quote is a quote, not the end of the string.
                        if bytes.get(i + 1) == Some(&'"') {
                            i += 2;
                            continue;
                        }
                        break;
                    }
                    i += 1;
                }
            }
            ';' | '\'' => return Some(&line[..char_to_byte(line, i)]),
            _ => {}
        }
        i += 1;
    }
    Some(line)
}

fn char_to_byte(line: &str, char_index: usize) -> usize {
    line.char_indices().nth(char_index).map_or(line.len(), |(index, _)| index)
}

/// Reads the chain left of a `.`, from the dot backwards. Indexing and call
/// parentheses are stepped over, so `members[0].Home.` yields `members`, `Home`.
fn member_chain(chars: &[char], mut end: usize) -> Option<Vec<String>> {
    let mut path = Vec::new();
    loop {
        while end > 0 && chars[end - 1].is_whitespace() {
            end -= 1;
        }
        // Step over an index or an argument list.
        while end > 0 && (chars[end - 1] == ')' || chars[end - 1] == ']') {
            let (open, close) = if chars[end - 1] == ')' { ('(', ')') } else { ('[', ']') };
            // An index reads an element, so the chain continues in the element's type.
            if close == ']' {
                path.push(INDEXED.to_string());
            }
            let mut depth = 0;
            loop {
                if end == 0 {
                    return None;
                }
                end -= 1;
                if chars[end] == close {
                    depth += 1;
                } else if chars[end] == open {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            while end > 0 && chars[end - 1].is_whitespace() {
                end -= 1;
            }
        }

        let name_end = end;
        while end > 0 && is_identifier_char(chars[end - 1]) {
            end -= 1;
        }
        if end == name_end {
            return None;
        }
        path.push(chars[end..name_end].iter().collect::<String>());

        while end > 0 && chars[end - 1].is_whitespace() {
            end -= 1;
        }
        if end > 0 && chars[end - 1] == '.' {
            end -= 1;
            continue;
        }
        path.reverse();
        return Some(path);
    }
}

/// Looks for the `{` of a record literal the cursor is inside of.
fn record_literal(chars: &[char], from: usize) -> Option<CursorContext> {
    let mut depth = 0;
    let mut open = None;
    for i in (0..from).rev() {
        match chars[i] {
            '}' => depth += 1,
            '{' => {
                if depth == 0 {
                    open = Some(i);
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    let open = open?;

    let mut end = open;
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    let name_end = end;
    while end > 0 && is_identifier_char(chars[end - 1]) {
        end -= 1;
    }
    if end == name_end {
        return None;
    }
    let type_name: String = chars[end..name_end].iter().collect();

    // A field is only expected at the start of an entry; after the `=` a value is.
    let mut named_fields = Vec::new();
    let mut entry_start = open + 1;
    let mut depth = 0;
    let mut has_value = false;
    for i in open + 1..from {
        match chars[i] {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '=' if depth == 0 => {
                has_value = true;
                let name: String = chars[entry_start..i].iter().collect();
                let name = name.trim().to_string();
                if !name.is_empty() {
                    named_fields.push(name);
                }
            }
            ',' if depth == 0 => {
                entry_start = i + 1;
                has_value = false;
            }
            _ => {}
        }
    }
    if has_value {
        return Some(CursorContext::Other);
    }

    Some(CursorContext::RecordLiteralField { type_name, named_fields })
}

/// Reads the context out of the line up to the cursor.
pub fn cursor_context(line_before_cursor: &str) -> CursorContext {
    let Some(code) = code_before_cursor(line_before_cursor) else {
        return CursorContext::Nothing;
    };
    let chars: Vec<char> = code.chars().collect();

    let mut i = chars.len();
    while i > 0 && is_identifier_char(chars[i - 1]) {
        i -= 1;
    }
    let after_prefix = i;
    while i > 0 && chars[i - 1].is_whitespace() {
        i -= 1;
    }

    if i > 0 && chars[i - 1] == '.' {
        if let Some(path) = member_chain(&chars, i - 1) {
            return CursorContext::Member(path);
        }
        return CursorContext::Other;
    }

    record_literal(&chars, after_prefix).unwrap_or(CursorContext::Other)
}

/// The call whose arguments the cursor is writing, if any.
pub fn call_context(line_before_cursor: &str) -> Option<CallContext> {
    let code = code_before_cursor(line_before_cursor)?;
    let chars: Vec<char> = code.chars().collect();

    // Find the innermost parenthesis that is still open.
    let mut depth = 0;
    let mut open = None;
    for i in (0..chars.len()).rev() {
        match chars[i] {
            ')' | ']' => depth += 1,
            '(' | '[' => {
                if depth == 0 {
                    open = Some(i);
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
    }

    if let Some(open) = open {
        let mut end = open;
        while end > 0 && chars[end - 1].is_whitespace() {
            end -= 1;
        }
        let name_end = end;
        while end > 0 && is_identifier_char(chars[end - 1]) {
            end -= 1;
        }
        if end == name_end {
            return None;
        }
        return Some(CallContext {
            name: chars[end..name_end].iter().collect(),
            argument: count_arguments(&chars, open + 1),
            bare: false,
        });
    }

    // A built-in statement takes its arguments without parentheses.
    let mut start = 0;
    while start < chars.len() && chars[start].is_whitespace() {
        start += 1;
    }
    let mut name_end = start;
    while name_end < chars.len() && is_identifier_char(chars[name_end]) {
        name_end += 1;
    }
    if name_end == start || name_end >= chars.len() {
        return None;
    }
    if !chars[name_end].is_whitespace() {
        return None;
    }
    Some(CallContext {
        name: chars[start..name_end].iter().collect(),
        argument: count_arguments(&chars, name_end),
        bare: true,
    })
}

fn count_arguments(chars: &[char], from: usize) -> usize {
    let mut depth = 0;
    let mut count = 0;
    for c in &chars[from..] {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => count += 1,
            _ => {}
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_after_a_dot() {
        assert_eq!(cursor_context("    e."), CursorContext::Member(vec!["e".to_string()]));
        assert_eq!(
            cursor_context("PRINTLN m.Home."),
            CursorContext::Member(vec!["m".to_string(), "Home".to_string()])
        );
        assert_eq!(cursor_context("x = e.Na"), CursorContext::Member(vec!["e".to_string()]));
    }

    #[test]
    fn member_after_an_index_or_a_call() {
        // The index stays in the chain: on a collection it steps into the element type,
        // and on an array it is a step the type does not have and is skipped.
        assert_eq!(
            cursor_context("members[0].Home."),
            CursorContext::Member(vec!["members".to_string(), INDEXED.to_string(), "Home".to_string()])
        );
        assert_eq!(cursor_context("ConfInfo(CurConf())."), CursorContext::Member(vec!["ConfInfo".to_string()]));
    }

    #[test]
    fn record_literal_fields() {
        assert_eq!(
            cursor_context("Point origin = Point { "),
            CursorContext::RecordLiteralField {
                type_name: "Point".to_string(),
                named_fields: vec![]
            }
        );
        assert_eq!(
            cursor_context("p = Point { X = 1, "),
            CursorContext::RecordLiteralField {
                type_name: "Point".to_string(),
                named_fields: vec!["X".to_string()]
            }
        );
    }

    #[test]
    fn a_value_is_not_a_field_name() {
        assert_eq!(cursor_context("p = Point { X = "), CursorContext::Other);
    }

    #[test]
    fn nothing_inside_a_string_or_comment() {
        assert_eq!(cursor_context("PRINTLN \"a."), CursorContext::Nothing);
        assert_eq!(cursor_context("PRINTLN \"say \"\"hi\"\"\", b."), CursorContext::Member(vec!["b".to_string()]));
        assert_eq!(cursor_context("x = 1 ; e."), CursorContext::Other);
    }

    #[test]
    fn call_with_parentheses() {
        let ctx = call_context("    x = Mid(a, ").unwrap();
        assert_eq!(ctx.name, "Mid");
        assert_eq!(ctx.argument, 1);
        assert!(!ctx.bare);
    }

    #[test]
    fn nested_call() {
        let ctx = call_context("x = Left(Mid(a, 1, 2), ").unwrap();
        assert_eq!(ctx.name, "Left");
        assert_eq!(ctx.argument, 1);
    }

    #[test]
    fn statement_without_parentheses() {
        let ctx = call_context("    ANSIPOS 1, ").unwrap();
        assert_eq!(ctx.name, "ANSIPOS");
        assert_eq!(ctx.argument, 1);
        assert!(ctx.bare);
    }
}

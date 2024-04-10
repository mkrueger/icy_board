/// A pattern parsing error.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct PatternError {
    /// The approximate character index of where the error occurred.
    pub pos: usize,

    /// A message describing the error.
    pub msg: &'static str,
}

pub struct Pattern {
    tokens: Vec<PatternToken>,
}

enum PatternToken {
    Char(char),
    AnyChar,
    AnySequence,
    AnyWithin(Vec<CharSpecifier>),
    AnyExcept(Vec<CharSpecifier>),
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum CharSpecifier {
    SingleChar(char),
    CharRange(char, char),
}

#[derive(Copy, Clone, PartialEq)]
enum MatchResult {
    Match,
    SubPatternDoesntMatch,
    EntirePatternDoesntMatch,
}

const ERROR_INVALID_RANGE: &str = "invalid range pattern";

impl Pattern {
    /// This function compiles Unix shell style patterns.
    ///
    /// An invalid glob pattern will yield a `PatternError`.
    pub fn new(pattern: &str) -> Result<Self, PatternError> {
        let chars = pattern.chars().collect::<Vec<_>>();
        let mut tokens = Vec::new();
        let mut i = 0;

        while i < chars.len() {
            match chars[i] {
                '?' => {
                    tokens.push(PatternToken::AnyChar);
                    i += 1;
                }
                '*' => {
                    tokens.push(PatternToken::AnySequence);
                    i += 1;
                }
                '[' => {
                    if i + 4 <= chars.len() && chars[i + 1] == '!' {
                        match chars[i + 3..].iter().position(|x| *x == ']') {
                            None => (),
                            Some(j) => {
                                let chars = &chars[i + 2..i + 3 + j];
                                let cs = parse_char_specifiers(chars);
                                tokens.push(PatternToken::AnyExcept(cs));
                                i += j + 4;
                                continue;
                            }
                        }
                    } else if i + 3 <= chars.len() && chars[i + 1] != '!' {
                        match chars[i + 2..].iter().position(|x| *x == ']') {
                            None => (),
                            Some(j) => {
                                let cs = parse_char_specifiers(&chars[i + 1..i + 2 + j]);
                                tokens.push(PatternToken::AnyWithin(cs));
                                i += j + 3;
                                continue;
                            }
                        }
                    }

                    // if we get here then this is not a valid range pattern
                    return Err(PatternError {
                        pos: i,
                        msg: ERROR_INVALID_RANGE,
                    });
                }
                c => {
                    tokens.push(PatternToken::Char(c));
                    i += 1;
                }
            }
        }

        Ok(Self { tokens })
    }

    /// Escape metacharacters within the given string by surrounding them in
    /// brackets. The resulting string will, when compiled into a `Pattern`,
    /// match the input string and nothing else.
    pub fn escape(s: &str) -> String {
        let mut escaped = String::new();
        for c in s.chars() {
            match c {
                // note that ! does not need escaping because it is only special
                // inside brackets
                '?' | '*' | '[' | ']' => {
                    escaped.push('[');
                    escaped.push(c);
                    escaped.push(']');
                }
                c => {
                    escaped.push(c);
                }
            }
        }
        escaped
    }

    pub fn matches(&self, str: &str) -> bool {
        self.matches_with(str, MatchOptions::new())
    }

    /// Return if the given `str` matches this `Pattern` using the specified
    /// match options.
    pub fn matches_with(&self, str: &str, options: MatchOptions) -> bool {
        self.matches_from(true, str.chars(), 0, options) == MatchResult::Match
    }

    fn matches_from(
        &self,
        mut follows_separator: bool,
        mut file: std::str::Chars,
        i: usize,
        options: MatchOptions,
    ) -> MatchResult {
        for (ti, token) in self.tokens[i..].iter().enumerate() {
            match *token {
                PatternToken::AnySequence => {
                    // Empty match
                    match self.matches_from(follows_separator, file.clone(), i + ti + 1, options) {
                        MatchResult::SubPatternDoesntMatch => (), // keep trying
                        m => return m,
                    };

                    while let Some(c) = file.next() {
                        if follows_separator && options.require_literal_leading_dot && c == '.' {
                            return MatchResult::SubPatternDoesntMatch;
                        }
                        follows_separator = false;
                        match *token {
                            PatternToken::AnySequence
                                if options.require_literal_separator && follows_separator =>
                            {
                                return MatchResult::SubPatternDoesntMatch
                            }
                            _ => (),
                        }
                        match self.matches_from(
                            follows_separator,
                            file.clone(),
                            i + ti + 1,
                            options,
                        ) {
                            MatchResult::SubPatternDoesntMatch => (), // keep trying
                            m => return m,
                        }
                    }
                }
                _ => {
                    let c = match file.next() {
                        Some(c) => c,
                        None => return MatchResult::EntirePatternDoesntMatch,
                    };

                    let is_sep = false;

                    if !match *token {
                        PatternToken::AnyChar
                        | PatternToken::AnyWithin(..)
                        | PatternToken::AnyExcept(..)
                            if (options.require_literal_separator && is_sep)
                                || (follows_separator
                                    && options.require_literal_leading_dot
                                    && c == '.') =>
                        {
                            false
                        }
                        PatternToken::AnyChar => true,
                        PatternToken::AnyWithin(ref specifiers) => {
                            in_char_specifiers(specifiers, c, options)
                        }
                        PatternToken::AnyExcept(ref specifiers) => {
                            !in_char_specifiers(specifiers, c, options)
                        }
                        PatternToken::Char(c2) => chars_eq(c, c2, options.case_sensitive),
                        PatternToken::AnySequence => unreachable!(),
                    } {
                        return MatchResult::SubPatternDoesntMatch;
                    }
                    follows_separator = is_sep;
                }
            }
        }

        // Iter is fused.
        if file.next().is_none() {
            MatchResult::Match
        } else {
            MatchResult::SubPatternDoesntMatch
        }
    }
}

fn parse_char_specifiers(s: &[char]) -> Vec<CharSpecifier> {
    let mut cs = Vec::new();
    let mut i = 0;
    while i < s.len() {
        if i + 3 <= s.len() && s[i + 1] == '-' {
            cs.push(CharSpecifier::CharRange(s[i], s[i + 2]));
            i += 3;
        } else {
            cs.push(CharSpecifier::SingleChar(s[i]));
            i += 1;
        }
    }
    cs
}

fn in_char_specifiers(specifiers: &[CharSpecifier], c: char, options: MatchOptions) -> bool {
    for &specifier in specifiers.iter() {
        match specifier {
            CharSpecifier::SingleChar(sc) => {
                if chars_eq(c, sc, options.case_sensitive) {
                    return true;
                }
            }
            CharSpecifier::CharRange(start, end) => {
                // FIXME: work with non-ascii chars properly (issue #1347)
                if !options.case_sensitive && c.is_ascii() && start.is_ascii() && end.is_ascii() {
                    let start = start.to_ascii_lowercase();
                    let end = end.to_ascii_lowercase();

                    let start_up = start.to_uppercase().next().unwrap();
                    let end_up = end.to_uppercase().next().unwrap();

                    // only allow case insensitive matching when
                    // both start and end are within a-z or A-Z
                    if start != start_up && end != end_up {
                        let c = c.to_ascii_lowercase();
                        if c >= start && c <= end {
                            return true;
                        }
                    }
                }

                if c >= start && c <= end {
                    return true;
                }
            }
        }
    }

    false
}

/// A helper function to determine if two chars are (possibly case-insensitively) equal.
fn chars_eq(a: char, b: char, case_sensitive: bool) -> bool {
    if !case_sensitive && a.is_ascii() && b.is_ascii() {
        // FIXME: work with non-ascii chars properly (issue #9084)
        a.to_ascii_lowercase() == b.to_ascii_lowercase()
    } else {
        a == b
    }
}

/// Configuration options to modify the behaviour of `Pattern::matches_with(..)`.
#[allow(missing_copy_implementations)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MatchOptions {
    /// Whether or not patterns should be matched in a case-sensitive manner.
    /// This currently only considers upper/lower case relationships between
    /// ASCII characters, but in future this might be extended to work with
    /// Unicode.
    pub case_sensitive: bool,

    /// Whether or not path-component separator characters (e.g. `/` on
    /// Posix) must be matched by a literal `/`, rather than by `*` or `?` or
    /// `[...]`.
    pub require_literal_separator: bool,

    /// Whether or not paths that contain components that start with a `.`
    /// will require that `.` appears literally in the pattern; `*`, `?`, `**`,
    /// or `[...]` will not match. This is useful because such files are
    /// conventionally considered hidden on Unix systems and it might be
    /// desirable to skip them when listing files.
    pub require_literal_leading_dot: bool,
}

impl MatchOptions {
    /// Constructs a new `MatchOptions` with default field values. This is used
    /// when calling functions that do not take an explicit `MatchOptions`
    /// parameter.
    ///
    /// This function always returns this value:
    ///
    /// ```rust,ignore
    /// MatchOptions {
    ///     case_sensitive: true,
    ///     require_literal_separator: false,
    ///     require_literal_leading_dot: false
    /// }
    /// ```
    ///
    /// # Note
    /// The behavior of this method doesn't match `default()`'s. This returns
    /// `case_sensitive` as `true` while `default()` does it as `false`.
    // FIXME: Consider unity the behavior with `default()` in a next major release.
    pub fn new() -> Self {
        Self {
            case_sensitive: true,
            require_literal_separator: false,
            require_literal_leading_dot: false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::Pattern;

    #[test]
    fn test_pattern() {
        assert!(Pattern::new("foo").unwrap().matches("foo"));
        assert!(!Pattern::new("foo").unwrap().matches("bar"));

        assert!(!Pattern::new("fo").unwrap().matches("foo"));
    }

    #[test]
    fn test_any_char_pattern() {
        assert!(Pattern::new("f?o").unwrap().matches("foo"));
        assert!(Pattern::new("f?o").unwrap().matches("f0o"));
        assert!(!Pattern::new("f?o").unwrap().matches("f0_"));
    }

    #[test]
    fn test_sequence_pattern() {
        assert!(Pattern::new("f*o").unwrap().matches("foo"));
        assert!(Pattern::new("f*o").unwrap().matches("f00000o"));
        assert!(!Pattern::new("f*o").unwrap().matches("f00000b"));
    }

    #[test]
    fn test_char_range_pattern() {
        assert!(Pattern::new("f[o0]o").unwrap().matches("foo"));
        assert!(Pattern::new("f[o0]o").unwrap().matches("f0o"));
        assert!(!Pattern::new("f[o0]o").unwrap().matches("fAo"));
    }

    #[test]
    fn test_not_char_range_pattern() {
        assert!(!Pattern::new("f[!o0]o").unwrap().matches("foo"));
        assert!(!Pattern::new("f[!o0]o").unwrap().matches("f0o"));
        assert!(Pattern::new("f[!o0]o").unwrap().matches("fAo"));
    }

    #[test]
    fn test_extensions() {
        assert!(Pattern::new("*.lha").unwrap().matches("foo.lha"));
        assert!(!Pattern::new("*.lha").unwrap().matches("foo.exe"));
        assert!(Pattern::new("*.lha").unwrap().matches("foo.bar.lha"));
    }
}

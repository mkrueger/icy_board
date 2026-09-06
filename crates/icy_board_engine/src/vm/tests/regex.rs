use super::{compile_errors, compile_errors_with_runtime, run_ppl};

#[test]
fn api_review_find_all_starts_inside_a_match_and_keeps_full_text_context() {
    let output = run_ppl(
        r#"
REGEX pattern = REGEX.Compile("a+")
REGEXMATCH one = pattern.Find("aaa", 1)
REGEXMATCH all[] = pattern.FindAll("aaa", 1)
PRINTLN one.Start, "|", one.Value, "|", all.Len(), "|", all[0].Start, "|", all[0].Value
all = REGEX.Compile("ä+").FindAll("äää ää", 1)
PRINTLN all.Len(), "|", all[0].Start, "|", all[0].Value, "|", all[1].Start
all = REGEX.Compile("^a").FindAll("aaa", 1)
PRINTLN all.Len()
all = REGEX.Compile("\ba").FindAll("aaa a", 1)
PRINTLN all.Len(), "|", all[0].Start
all = pattern.FindAll("aaa aaa aaa", 1, 2)
PRINTLN all.Len(), "|", all[0].Value, "|", all[1].Start
"#,
    );
    assert_eq!(output, "1|aa|1|1|aa\n2|1|ää|4\n0\n1|4\n2|aa|4\n");
}

#[test]
fn api_review_find_all_empty_matches_advance_on_unicode_boundaries() {
    let output = run_ppl(
        r#"
REGEXMATCH all[] = REGEX.Compile("").FindAll("äβ", 1)
PRINTLN all.Len(), "|", all[0].Start, "|", all[1].Start
all = REGEX.Compile("a*").FindAll("aaä", 1)
PRINTLN all.Len(), "|", all[0].Start, "|", all[0].Value, "|", all[1].Start
all = REGEX.Compile("$").FindAll("äβ", 2)
PRINTLN all.Len(), "|", all[0].Start
all = REGEX.Compile("").FindAll("", 0)
PRINTLN all.Len(), "|", all[0].Start
all = REGEX.Compile("").FindAll("äβ", 3)
PRINTLN all.Len()
"#,
    );
    assert_eq!(output, "2|1|2\n2|1|a|3\n1|2\n1|0\n0\n");
}

#[test]
fn regex_compiles_tests_and_reports_errors() {
    let output = run_ppl(
        r#"
        REGEX pattern = REGEX.Compile("^grüße$", RegexOptions.IgnoreCase)
        PRINTLN pattern.Valid, " ", pattern.Pattern
        PRINTLN pattern.IsMatch("GRÜßE")
        PRINTLN REGEX.Compile("x", RegexOptions.IgnoreCaseAndMultiLine).Valid
        PRINTLN REGEX.Compile("ü").IsMatch("aü", 1)
        PRINTLN REGEX.Compile("$").IsMatch("", 0), " ", REGEX.Compile("$").Find("abc", 3).Start
        PRINTLN REGEX.Escape("a+b?")
        PRINTLN REGEX.IsValid("[")

        REGEX invalid = REGEX.Compile("[")
        PRINTLN invalid.Valid
        PRINTLN Error.Last().Kind = ErrKind.Regex, " ", Error.Last().Code = ErrCode.Invalid
        "#,
    );

    assert_eq!(output, "1 ^grüße$\n1\n1\n1\n1 3\na\\+b\\?\n0\n0\n1 1\n");
}

#[test]
fn regex_finds_captures_collections_and_replaces() {
    let output = run_ppl(
        r#"
        REGEX parser = REGEX.Compile("(?P<name>\w+):(?P<value>\d+)")
        REGEXMATCH found = parser.Find("ä score:120 end")
        PRINTLN found.Success, " ", found.Value, " ", found.Start, " ", found.Length, " ", found.GroupCount
        PRINTLN found.Group(0), " ", found.Group(1), " ", found.NamedGroup("value")
        PRINTLN found.GroupMatched(2), " ", found.NamedGroupMatched("missing")
        PRINTLN found.GroupStart(1), " ", found.NamedGroupStart("value"), " ", found.GroupLength(2)

        REGEXMATCH all[]
        all = REGEX.Compile("\w+").FindAll("ä one two", 2)
        PRINTLN all.Len(), " ", all[0].Value, " ", all[1].Value
        PRINTLN REGEX.Compile("^two").Find("one two", 4).Success
        REGEXMATCH missing = REGEX.Compile("z").Find("abc")
        PRINTLN missing.Start
        REGEXMATCH optional = REGEX.Compile("(a)?b").Find("b")
        PRINTLN optional.GroupMatched(1), " ", optional.GroupStart(1)
        REGEXMATCH limited[]
        limited = REGEX.Compile("\w+").FindAll("one two three", 0, 2)
        PRINTLN limited.Len(), " ", limited[1].Value
        PRINTLN parser.Replace("a:1 b:2 c:3", "$name=$value", 2)
        PRINTLN found.NamedGroup("missing")
        PRINTLN Error.Last().Kind = ErrKind.Regex, " ", Error.Last().Code = ErrCode.Invalid
        "#,
    );

    assert_eq!(
        output,
        "1 score:120 2 9 2\nscore:120 score 120\n1 0\n2 8 3\n2 one two\n0\n-1\n0 -1\n2 two\na=1 b=2 c:3\n\n1 1\n"
    );
}

#[test]
fn regex_api_requires_language_and_runtime_400() {
    let errors = compile_errors_with_runtime("REGEX pattern = REGEX.Compile(\"x\")", 340);
    assert!(errors.iter().any(|error| error.contains("REGEX") && error.contains("400")), "{errors:?}");
}

#[test]
fn regex_split_preserves_fields_limits_and_target_on_error() {
    let output = run_ppl(
        r#"
        REGEX separators = REGEX.Compile("[,;]\s*")
        BIGSTR parts[]
        parts = separators.Split("one, two;;four")
        PRINTLN parts.Len(), " ", STRING.Join(parts, "|")

        parts = separators.Split("one, two; three; four", 3)
        PRINTLN parts.Len(), " ", STRING.Join(parts, "|")
        PRINTLN separators.Split("a,b")[1]

        REGEX invalid = REGEX.Compile("[")
        parts = invalid.Split("changed")
        PRINTLN Error.Last().Kind = ErrKind.Regex
        PRINTLN parts.Len()
        "#,
    );

    assert_eq!(output, "4 one|two||four\n3 one|two|three; four\nb\n1\n0\n");

    let errors = compile_errors("REGEX regex = REGEX.Compile(\",\")\nBIGSTR parts[]\nregex.Split(\"a,b\", parts)");
    assert!(!errors.is_empty(), "the removed output-array REGEX.Split signature should not compile");
}

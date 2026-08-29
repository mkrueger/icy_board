use super::{compile_errors_with_runtime, run_ppl};

#[test]
fn regex_compiles_tests_and_reports_errors() {
    let output = run_ppl(
        r#"
        REGEX pattern = REGEX.Compile("^grüße$", RegexOptions.IgnoreCase)
        PRINTLN pattern.Valid, " ", pattern.Pattern
        PRINTLN pattern.IsMatch("GRÜßE")
        PRINTLN REGEX.Compile("x", RegexOptions.IgnoreCase | RegexOptions.MultiLine).Valid
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
        STRING parts(0)
        REGEX separators = REGEX.Compile("[,;]\s*")
        separators.Split("one, two;;four", parts)
        PRINTLN parts.Len(), " ", STRING.Join(parts, "|")

        separators.Split("one, two; three; four", parts, 3)
        PRINTLN parts.Len(), " ", STRING.Join(parts, "|")

        REGEX invalid = REGEX.Compile("[")
        invalid.Split("changed", parts)
        PRINTLN Error.Last().Kind = ErrKind.Regex
        PRINTLN parts.Len(), " ", STRING.Join(parts, "|")
        "#,
    );

    assert_eq!(output, "4 one|two||four\n3 one|two|three; four\n1\n3 one|two|three; four\n");
}

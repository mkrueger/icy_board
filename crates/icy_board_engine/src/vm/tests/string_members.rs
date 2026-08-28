use super::{compile_errors_with_runtime, run_ppl};

#[test]
fn string_member_syntax_requires_runtime_400() {
    let errors = compile_errors_with_runtime("STRING text\nPRINTLN text.Trim()", 340);
    assert!(errors.iter().any(|error| error.contains("STRING.Trim needs runtime 400")), "{errors:?}");
    assert!(compile_errors_with_runtime("STRING text\nPRINTLN Trim(text, \" \")", 340).is_empty());
}

#[test]
fn strings_offer_search_predicate_and_manipulation_members() {
    let output = run_ppl(
        r#"
        BIGSTR text = "  Grüße two two  "
        PRINTLN text.Len()
        PRINTLN text.Find("two"), " ", text.Find("two", 10)
        PRINTLN text.FindLast("two"), " ", text.FindLast("two", 11)
        PRINTLN text.Contains("Grüße"), text.StartsWith("  "), text.EndsWith("  "), text.Count("two")
        PRINTLN "[", text.Trim().ToLower().Replace("two", "three"), "]"
        PRINTLN "[", text.TrimStart(), "] [", text.TrimEnd(), "]"
        PRINTLN "xxhellox".Trim("x")
        PRINTLN "xyGrüßeyx".Trim("xy")
        PRINTLN "äxvalueyä".TrimStart("xä"), "|", "äxvalueyä".TrimEnd("yä")
        "#,
    );

    assert_eq!(
        output,
        "17\n9 13\n13 9\n1112\n[grüße three three]\n[Grüße two two  ] [  Grüße two two]\nhello\nGrüße\nvalueyä|äxvalue\n"
    );
}

#[test]
fn strings_split_and_join_through_instance_and_static_members() {
    let output = run_ppl(
        r#"
        STRING parts(0)
        "a,,b,".Split(",", parts)
        PRINTLN parts.Len(), " [", parts[0], "] [", parts[1], "] [", parts[2], "] [", parts[3], "]"
        PRINTLN STRING.Join(parts, "|")

        STRING.Split("one:two:three:four", ":", parts, 3)
        PRINTLN parts.Len(), " ", STRING.Join(parts, "|")
        PRINTLN STRING.Repeat("ab", 3)

        STRING string = " value "
        PRINTLN "[", string.trim(), "] ", string.contains("VALUE")
        "#,
    );

    assert_eq!(output, "3 [a] [] [b] []\na||b|\n2 one|two|three:four\nababab\n[value] 0\n");
}

#[test]
fn a_failed_split_leaves_the_target_unchanged() {
    let output = run_ppl(
        r#"
        STRING parts(1)
        STRING text = "text"
        parts[0] = "keep"
        parts[1] = "me"
        text.Split("", parts)
        PRINTLN parts.Len(), " ", parts[0], " ", parts[1]
        PRINTLN Error.Last().Kind = ErrKind.String, " ", Error.Last().Code = ErrCode.Invalid
        "#,
    );

    assert_eq!(output, "1 keep me\n1 1\n");
}

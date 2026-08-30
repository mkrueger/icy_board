use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::workspace::{CompilerData, Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
};
use ppl_lsp::{completion::get_completion, signature_help::get_signature_help};
use tower_lsp::lsp_types::{CompletionItem, CompletionItemTag, Documentation, ParameterLabel};

/// Parses a source the way the server does and hands back its semantic model.
fn analyze(source: &str) -> (icy_board_engine::ast::Ast, SemanticVisitor) {
    let workspace = Workspace::default();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut visitor = SemanticVisitor::new(&workspace, errors, registry);
    ast.visit(&mut visitor);
    visitor.finish();
    (ast, visitor)
}

/// Completes at the end of `source`, which is where the cursor is.
fn complete_items(source: &str) -> Vec<CompletionItem> {
    let (ast, visitor) = analyze(source);
    let line = source.lines().last().unwrap_or("");
    get_completion(&ast, &visitor, line, source.chars().count())
}

fn complete(source: &str) -> Vec<String> {
    complete_items(source).into_iter().map(|item| item.label).collect()
}

#[test]
fn bigstr_completion_is_deprecated_in_ppl_400() {
    let item = complete_items(";$LANGVERSION 400\nBIG")
        .into_iter()
        .find(|item| item.label.eq_ignore_ascii_case("BIGSTR"))
        .expect("BIGSTR completion");
    assert_eq!(item.tags, Some(vec![CompletionItemTag::DEPRECATED]));
}

fn completion_documentation(source: &str, label: &str) -> String {
    let item = complete_items(source)
        .into_iter()
        .find(|item| item.label.eq_ignore_ascii_case(label))
        .unwrap_or_else(|| panic!("{label} not offered for {source:?}"));
    match item.documentation.expect("completion has documentation") {
        Documentation::String(value) => value,
        Documentation::MarkupContent(value) => value.value,
    }
}

#[test]
fn keywords_include_localized_completion_documentation() {
    let if_doc = completion_documentation("I", "IF");
    assert!(if_doc.starts_with("```PPL\nIF\n```"), "{if_doc}");
    assert!(if_doc.len() > "```PPL\nIF\n```".len());

    let foreach_doc = completion_documentation(";$LANGVERSION 400\nF", "FOREACH");
    assert!(foreach_doc.starts_with("```PPL\nFOREACH\n```"), "{foreach_doc}");

    let exit_doc = completion_documentation(";$LANGVERSION 400\nE", "EXIT");
    assert!(exit_doc.starts_with("```PPL\nEXIT\n```"), "{exit_doc}");
}

fn help(source: &str) -> Vec<String> {
    let (_, visitor) = analyze(source);
    let line = source.lines().last().unwrap_or("");
    get_signature_help(line, &visitor)
        .map(|help| help.signatures.into_iter().map(|s| s.label).collect())
        .unwrap_or_default()
}

const RECORD: &str = r#"TYPE Address
    STRING Town
ENDTYPE
TYPE Member
    STRING Name
    INTEGER Age
    Address Home
ENDTYPE
Member m
Member people(10)
"#;

#[test]
fn a_record_offers_its_fields() {
    let items = complete(&format!("{RECORD}m."));
    assert_eq!(items, vec!["Name".to_string(), "Age".to_string(), "Home".to_string()]);
}

#[test]
fn a_field_of_a_record_offers_its_own_fields() {
    let items = complete(&format!("{RECORD}m.Home."));
    assert_eq!(items, vec!["Town".to_string()]);
}

#[test]
fn an_indexed_record_offers_its_fields() {
    let items = complete(&format!("{RECORD}people[0].Home."));
    assert_eq!(items, vec!["Town".to_string()]);
}

#[test]
fn a_prefix_does_not_hide_the_fields() {
    let items = complete(&format!("{RECORD}PRINTLN m.Na"));
    assert!(items.contains(&"Name".to_string()), "{items:?}");
}

#[test]
fn a_board_object_offers_fields_and_methods() {
    let items = complete("CONFERENCE conf = ConfInfo(CurConf())\nconf.");
    assert!(items.contains(&"Name".to_string()), "{items:?}");
    assert!(items.contains(&"HasAccess".to_string()), "{items:?}");
    assert!(items.contains(&"Doors".to_string()), "{items:?}");
}

#[test]
fn a_function_answering_an_object_offers_its_members() {
    let items = complete("Board.Conferences[CurConf()].");
    assert!(items.contains(&"Areas".to_string()), "{items:?}");
}

#[test]
fn an_indexed_board_user_offers_user_members() {
    let items = complete("Board.Users[0].");
    for member in ["Valid", "Name", "City", "Notes", "SetNote", "Contacts", "AddContact", "RemoveContact"] {
        assert!(items.contains(&member.to_string()), "{member} missing: {items:?}");
    }
    assert!(!items.contains(&"Count".to_string()), "{items:?}");
}

#[test]
fn new_board_user_api_completion_explains_its_types_and_members() {
    let board_users = completion_documentation("Board.", "Users");
    assert!(board_users.contains("`Board`") && board_users.contains("`USER[]`"), "{board_users}");

    let valid = completion_documentation("USER user\nuser.", "Valid");
    assert!(valid.contains("`Board.Users`") && valid.contains("`Valid`"), "{valid}");
}

#[test]
fn a_user_contact_offers_documented_record_fields() {
    let items = complete("USER user\nuser.Contacts[0].");
    assert_eq!(items, vec!["Service", "Account"]);

    let service = completion_documentation("USER user\nuser.Contacts[0].", "Service");
    assert!(service.contains("Kontaktdienst") || service.contains("contact service"), "{service}");
}

#[test]
fn session_user_and_http_workflows_have_completion_documentation() {
    let session_user = completion_documentation("Session.", "User");
    assert!(session_user.contains("currently selected") || session_user.contains("aktuell") && session_user.contains("ausgewählt"), "{session_user}");

    let contacts = completion_documentation("USER user\nuser.", "Contacts");
    assert!((contacts.contains("100 entries") || contacts.contains("100 Einträgen")) && contacts.contains("AddContact"), "{contacts}");

    let http_get = completion_documentation("HTTP.", "Get");
    assert!(http_get.contains("policy-controlled GET") || http_get.contains("richtliniengesteuerte GET"), "{http_get}");

    let set_text = completion_documentation("HTTPREQUEST request\nrequest.", "SetText");
    assert!(
        (set_text.contains("GET and HEAD") || set_text.contains("GET- und HEAD"))
            && (set_text.contains("UTF-8 body") || set_text.contains("UTF-8-Body")),
        "{set_text}"
    );

    let response_text = completion_documentation("HTTPRESPONSE response\nresponse.", "Text");
    assert!(
        (response_text.contains("strictly as UTF-8") || response_text.contains("strikt als UTF-8"))
            && response_text.contains("ErrCode.Format"),
        "{response_text}"
    );
}

/// Snapshot arrays expose array members rather than their internal legacy getter.
#[test]
fn a_collection_does_not_offer_its_internal_getter() {
    let items = complete("Board.Conferences.");
    assert!(items.contains(&"Len".to_string()), "{items:?}");
    assert!(!items.contains(&"Count".to_string()), "{items:?}");
    assert!(!items.iter().any(|item| item.starts_with('<')), "{items:?}");
}

#[test]
fn regex_completion_covers_static_instance_and_result_members() {
    let static_items = complete("REGEX.");
    for member in ["Compile", "Escape", "IsValid"] {
        assert!(static_items.contains(&member.to_string()), "{member} missing: {static_items:?}");
    }
    assert!(!static_items.contains(&"Find".to_string()), "{static_items:?}");

    let instance_items = complete("REGEX pattern\npattern.");
    for member in ["Valid", "Pattern", "IsMatch", "Find", "FindAll", "Replace", "Split"] {
        assert!(instance_items.contains(&member.to_string()), "{member} missing: {instance_items:?}");
    }
    assert!(!instance_items.contains(&"Compile".to_string()), "{instance_items:?}");

    let match_items = complete("REGEX pattern\npattern.Find(\"text\").");
    for member in ["Success", "Value", "Group", "NamedGroup"] {
        assert!(match_items.contains(&member.to_string()), "{member} missing: {match_items:?}");
    }

    let matches_items = complete("REGEX pattern\npattern.FindAll(\"text\").");
    assert!(matches_items.contains(&"Len".to_string()), "Len missing: {matches_items:?}");
    assert!(!matches_items.contains(&"Count".to_string()), "{matches_items:?}");
    assert!(!matches_items.contains(&"Get".to_string()), "{matches_items:?}");
}

#[test]
fn strings_offer_instance_static_and_chained_members() {
    let instance = complete("STRING text\ntext.");
    for member in [
        "Find",
        "FindLast",
        "Contains",
        "StartsWith",
        "EndsWith",
        "Count",
        "Equals",
        "Replace",
        "Trim",
        "Split",
    ] {
        assert!(instance.contains(&member.to_string()), "{member} missing: {instance:?}");
    }
    assert!(!instance.contains(&"Join".to_string()), "{instance:?}");

    let comparisons = complete("StringComparison.");
    for value in ["Ordinal", "OrdinalIgnoreCase"] {
        assert!(comparisons.contains(&value.to_string()), "{value} missing: {comparisons:?}");
    }

    let statik = complete("STRING.");
    assert_eq!(statik, vec!["Join", "Repeat", "Split"]);

    let chained = complete("STRING text\ntext.Trim().");
    assert!(chained.contains(&"ToLower".to_string()), "{chained:?}");
    assert!(chained.contains(&"Split".to_string()), "{chained:?}");

    assert!(complete(";$LANGVERSION 350\nSTRING text\ntext.").is_empty());
    assert!(complete(";$LANGVERSION 350\nSTRING.").is_empty());
}

#[test]
fn bytes_and_checksum_completion_match_the_engine_surface() {
    let instance = complete(";$LANGVERSION 400\nBYTES data\ndata.");
    assert_eq!(instance, vec!["Len", "ToString", "ToBase64", "ToHex", "GetChecksum"]);

    let statik = complete(";$LANGVERSION 400\nBYTES.");
    assert_eq!(statik, vec!["FromBase64"]);

    let algorithms = complete(";$LANGVERSION 400\nChecksum.");
    assert_eq!(algorithms, vec!["CRC32", "MD5", "SHA256"]);

    let to_hex = completion_documentation(";$LANGVERSION 400\nBYTES data\ndata.", "ToHex");
    assert!(to_hex.contains("leading zero bytes") || to_hex.contains("führende Nullbytes"), "{to_hex}");

    let checksum = completion_documentation(";$LANGVERSION 400\nBYTES data\ndata.", "GetChecksum");
    assert!(checksum.contains("CRC32") && checksum.contains("MD5") && checksum.contains("SHA256") && checksum.contains("32"), "{checksum}");
}

#[test]
fn runtime_400_objects_offer_their_registered_members() {
    for (source, expected) in [
        ("SURFACE value\nvalue.", &["Width", "SetPixel", "PresentRect"][..]),
        ("AUDIO value\nvalue.", &["Valid", "Volume", "Play"][..]),
        ("EVENT value\nvalue.", &["Kind", "Action", "LeftDown", "Ctrl"][..]),
        ("ERROR value\nvalue.", &["OK", "Message", "Channel"][..]),
        (
            "TERMINFO value\nvalue.",
            &["Program", "Columns", "InlineGraphics", "PixelMouse", "ClientBlit", "Audio"][..],
        ),
        ("TERMINPUT value\nvalue.", &["Poll", "Wait", "KeyboardOn", "Release"][..]),
        ("TERMINAL value\nvalue.", &["Info", "Gfx", "Input"][..]),
        ("GFX value\nvalue.", &["Init", "Backend", "Pacing"][..]),
        ("BOARD value\nvalue.", &["Name", "SysopName", "NodeCount", "Conferences", "Users"][..]),
        ("SESSION value\nvalue.", &["Conference", "Area", "Directory", "User", "Node", "MinutesLeft"][..]),
        (
            "USER value\nvalue.",
            &[
                "Valid",
                "Name",
                "Alias",
                "SecurityLevel",
                "Contacts",
                "Notes",
                "SetNote",
                "AddContact",
                "RemoveContact",
            ][..],
        ),
    ] {
        let items = complete(source);
        for member in expected {
            assert!(items.contains(&member.to_string()), "{member} missing for {source:?}: {items:?}");
        }
    }
}

/// The list is the type's own, so a member another type happens to have is not in it.
#[test]
fn an_object_offers_only_its_own_members() {
    let items = complete("TERMINPUT value\nvalue.");
    assert_eq!(items, vec!["KeyboardOff", "KeyboardOn", "MouseOff", "MouseOn", "Poll", "Release", "Wait"]);
}

#[test]
fn graphics_only_offers_session_control() {
    let items = complete("GFX value\nvalue.");
    assert_eq!(items, vec!["Backend", "Init", "Pacing", "Shutdown"]);
}

#[test]
fn graphics_completion_includes_member_documentation() {
    let init = completion_documentation(";$LANGVERSION 400\nTerminal.Gfx.", "Init");
    assert!(init.contains("Grafiksitzung") || init.contains("graphics session"), "{init}");
    assert!(init.contains("`backend`") && init.contains("`fullscreen`"), "{init}");
    assert!(init.contains("**Parameters**") || init.contains("**Parameter**"), "{init}");

    let present_at = completion_documentation(";$LANGVERSION 400\nSURFACE banner\nbanner.", "PresentAt");
    assert!(present_at.contains("Textspalte") || present_at.contains("text column"), "{present_at}");
}

#[test]
fn terminal_input_and_margins_completion_include_documentation() {
    let margins = completion_documentation(";$LANGVERSION 400\nTerminal.", "Margins");
    assert!(margins.contains("Scroll") || margins.contains("scrolling"), "{margins}");

    let set_horizontal = completion_documentation(";$LANGVERSION 400\nTerminal.Margins.", "SetHorizontal");
    assert!(set_horizontal.contains("1-basiert") || set_horizontal.contains("1-based"), "{set_horizontal}");
    assert!(set_horizontal.contains("CSI ? 69 h") && set_horizontal.contains("CSI left ; right s"), "{set_horizontal}");

    let mouse_on = completion_documentation(";$LANGVERSION 400\nTerminal.Input.", "MouseOn");
    assert!(mouse_on.contains("Tracking") || mouse_on.contains("tracking"), "{mouse_on}");
    assert!(mouse_on.contains("1006") && mouse_on.contains("1016"), "{mouse_on}");
}

#[test]
fn member_call_arguments_offer_qualified_enum_values() {
    let mouse_modes = complete(";$LANGVERSION 400\nTERMINPUT input\ninput.MouseOn(");
    assert_eq!(mouse_modes, vec!["MouseMode.Text", "MouseMode.Pixels"]);

    let tracking = complete(";$LANGVERSION 400\nTERMINPUT input\ninput.MouseOn(MouseMode.Text, ");
    assert_eq!(
        tracking,
        vec!["MouseTracking.Buttons", "MouseTracking.Drag", "MouseTracking.All"]
    );
}

#[test]
fn event_does_not_offer_raw_masks() {
    let items = complete("EVENT value\nvalue.");
    assert!(!items.contains(&"Buttons".to_string()), "{items:?}");
    assert!(!items.contains(&"Modifiers".to_string()), "{items:?}");
    for name in ["LeftDown", "MiddleDown", "RightDown", "Shift", "Alt", "Ctrl", "Meta"] {
        assert!(items.contains(&name.to_string()), "{name} missing: {items:?}");
    }
}

#[test]
fn a_record_literal_offers_the_fields_it_has_not_named() {
    let items = complete(&format!("{RECORD}m = Member {{ "));
    assert_eq!(items, vec!["Name".to_string(), "Age".to_string(), "Home".to_string()]);

    let items = complete(&format!("{RECORD}m = Member {{ Name = \"a\", "));
    assert_eq!(items, vec!["Age".to_string(), "Home".to_string()]);
}

#[test]
fn a_record_literal_value_is_not_a_field_name() {
    let items = complete(&format!("{RECORD}m = Member {{ Age = "));
    assert!(!items.is_empty());
    assert!(!items.contains(&"Age".to_string()), "{items:?}");
}

#[test]
fn declared_types_can_be_completed() {
    let items = complete(&format!("{RECORD}Mem"));
    assert!(items.contains(&"Member".to_string()), "{items:?}");
    assert!(items.contains(&"Conference".to_string()), "{items:?}");
}

#[test]
fn nothing_is_offered_inside_a_string() {
    assert!(complete("PRINTLN \"m.").is_empty());
}

#[test]
fn signature_help_for_a_user_function() {
    let source = "DECLARE FUNCTION Total(INTEGER value, STRING name) INTEGER\nx = Total(1, ";
    assert_eq!(help(source), vec!["FUNCTION Total(INTEGER value, STRING name) INTEGER".to_string()]);
}

#[test]
fn signature_help_shows_a_var_parameter() {
    let source = "DECLARE PROCEDURE Fill(VAR INTEGER values(10))\nFill(";
    assert_eq!(help(source), vec!["PROCEDURE Fill(VAR INTEGER values(10))".to_string()]);
}

#[test]
fn signature_help_for_a_user_procedure() {
    let source = "PROCEDURE Show(STRING text)\nENDPROC\nShow(";
    assert_eq!(help(source), vec!["PROCEDURE Show(STRING text)".to_string()]);
}

#[test]
fn signature_help_for_a_routine_parameter() {
    let source = "PROCEDURE Apply(FUNCTION f(INTEGER a) INTEGER)\nENDPROC\nApply(";
    assert_eq!(help(source), vec!["PROCEDURE Apply(FUNCTION f(INTEGER a) INTEGER)".to_string()]);
}

#[test]
fn signature_help_for_a_built_in_function() {
    let source = "x = Mid(a, 1, ";
    let signatures = help(source);
    assert!(signatures.iter().any(|s| s.starts_with("MID(")), "{signatures:?}");
}

#[test]
fn signature_help_for_a_built_in_statement() {
    let source = "ANSIPOS 1, ";
    let signatures = help(source);
    assert_eq!(signatures.len(), 1);
    assert!(signatures[0].starts_with("ANSIPOS "), "{signatures:?}");
}

#[test]
fn signature_help_for_member_calls_marks_optional_parameters() {
    let (_, visitor) = analyze(";$LANGVERSION 400\nTERMINPUT input\n");
    let help = get_signature_help("input.MouseOn(MouseMode.Text, ", &visitor).unwrap();
    assert_eq!(help.active_parameter, Some(1));
    assert_eq!(help.signatures[0].label, "TermInput.MouseOn(MouseMode mode, [MouseTracking tracking]) BOOLEAN");
    let tracking = &help.signatures[0].parameters.as_ref().unwrap()[1];
    assert!(tracking.documentation.is_some(), "optional tracking parameter has no documentation");

    let (_, visitor) = analyze(";$LANGVERSION 400\n");
    let help = get_signature_help("Terminal.Margins.SetHorizontal(", &visitor).unwrap();
    assert_eq!(help.active_parameter, Some(0));
    assert_eq!(help.signatures[0].label, "Margins.SetHorizontal(INTEGER left, INTEGER right) BOOLEAN");
}

#[test]
fn graphics_signature_help_names_and_documents_parameters() {
    let (_, visitor) = analyze(";$LANGVERSION 400\n");
    let help = get_signature_help("Terminal.Gfx.Init(GfxBackend.Auto, ", &visitor).unwrap();
    let signature = &help.signatures[0];
    assert_eq!(signature.label, "Gfx.Init([GfxBackend backend], [BOOLEAN fullscreen]) BOOLEAN");
    assert_eq!(help.active_parameter, Some(1));
    for parameter in signature.parameters.as_ref().unwrap() {
        assert!(parameter.documentation.is_some(), "parameter has no documentation: {parameter:?}");
    }
}

#[test]
fn the_argument_the_cursor_is_in_is_marked() {
    let (_, visitor) = analyze("DECLARE PROCEDURE Show(INTEGER a, INTEGER b)\n");
    let help = get_signature_help("Show(1, ", &visitor).unwrap();
    assert_eq!(help.active_parameter, Some(1));

    let signature = &help.signatures[help.active_signature.unwrap() as usize];
    let parameters = signature.parameters.as_ref().unwrap();
    let ParameterLabel::LabelOffsets([start, end]) = parameters[1].label else {
        panic!("expected offsets");
    };
    let label: Vec<char> = signature.label.chars().collect();
    let marked: String = label[start as usize..end as usize].iter().collect();
    assert_eq!(marked, "INTEGER b");
}

/// The words offered in an empty file written for that language version.
fn offered(version: u16) -> Vec<String> {
    offered_in(version, "")
}

/// The words offered in a file the workspace says is written for `version`.
fn offered_in(version: u16, source: &str) -> Vec<String> {
    let mut workspace = Workspace::default();
    workspace.compiler.get_or_insert_with(CompilerData::default).language_version = Some(version);

    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut visitor = SemanticVisitor::new(&workspace, errors, registry);
    ast.visit(&mut visitor);
    visitor.finish();

    let line = source.lines().last().unwrap_or("");
    get_completion(&ast, &visitor, line, source.chars().count())
        .into_iter()
        .map(|item| item.label)
        .collect()
}

#[test]
fn a_word_is_offered_from_the_version_that_gave_it_meaning() {
    let old = offered(340);
    let new = offered(400);

    for word in ["IF", "WHILE", "DECLARE"] {
        assert!(old.contains(&word.to_string()), "{word} should be offered in 340: {old:?}");
    }
    for word in ["CONST", "ENUM", "REPEAT", "TYPE", "EXIT", "ULONG"] {
        assert!(!old.contains(&word.to_string()), "{word} should not be offered in 340");
        assert!(new.contains(&word.to_string()), "{word} should be offered in 400");
    }

    let old_functions = offered_in(340, "PRINT To");
    let new_functions = offered_in(400, "PRINT To");
    assert!(!old_functions.contains(&"ToULong".to_string()), "{old_functions:?}");
    assert!(new_functions.contains(&"ToULong".to_string()), "{new_functions:?}");
}

#[test]
fn a_source_states_which_language_it_is_written_in() {
    // The workspace says 400, but the file itself says otherwise.
    let items = offered_in(400, ";$LANGVERSION 340\n");
    assert!(items.contains(&"WHILE".to_string()), "{items:?}");
    for word in ["CONST", "ENUM", "TYPE"] {
        assert!(
            !items.contains(&word.to_string()),
            "{word} should not be offered after $LANGVERSION 340: {items:?}"
        );
    }

    // And the other way round: an old workspace with a file written for 400.
    let items = offered_in(340, ";$LANGVERSION 400\n");
    for word in ["CONST", "ENUM", "TYPE"] {
        assert!(items.contains(&word.to_string()), "{word} should be offered after $LANGVERSION 400: {items:?}");
    }
}

#[test]
fn a_built_in_statement_is_offered_from_the_version_that_added_it() {
    let old = offered(330);
    assert!(old.contains(&"PrintLn".to_string()), "{old:?}");
    assert!(!old.contains(&"MoveMsg".to_string()), "{old:?}");

    let new = offered(400);
    assert!(new.contains(&"MoveMsg".to_string()), "{new:?}");
    for statement in ["FGetRec", "FPutRec", "FReadRec", "FWriteRec"] {
        assert!(!old.contains(&statement.to_string()), "{statement} should not be offered in 330");
        assert!(new.contains(&statement.to_string()), "{statement} should be offered in 400: {new:?}");
    }
}

#[test]
fn a_type_is_offered_from_the_version_that_named_it() {
    let first = offered(100);
    assert!(first.contains(&"INTEGER".to_string()), "{first:?}");
    for word in ["BIGSTR", "DDATE", "MSGAREAID"] {
        assert!(!first.contains(&word.to_string()), "{word} should not be offered in 100: {first:?}");
    }

    let second = offered(200);
    assert!(second.contains(&"BIGSTR".to_string()), "{second:?}");
    assert!(!second.contains(&"DDATE".to_string()), "{second:?}");

    assert!(offered(300).contains(&"DDATE".to_string()));
    assert!(offered(400).contains(&"MSGAREAID".to_string()));
}

#[test]
fn a_board_object_is_only_offered_from_400() {
    let last = offered(400);
    for word in ["Conference", "Area", "Directory", "Door"] {
        assert!(last.contains(&word.to_string()), "{word} should be offered in 400: {last:?}");
    }

    // An enum is a type from 350 on, so the objects are what 350 must not see.
    let older = offered(350);
    for word in ["Conference", "Area", "Directory", "Door"] {
        assert!(!older.contains(&word.to_string()), "{word} should not be offered in 350: {older:?}");
    }
}

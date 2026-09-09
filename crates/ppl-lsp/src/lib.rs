pub mod code_lens;
pub mod completion;
pub mod context;
pub mod document_symbol;
pub mod documentation;
pub mod formatting;
pub mod hover;
pub mod inlay_hints;
pub mod jump_definition;
pub mod reference;
pub mod semantic_tokens;
pub mod signature_help;
pub mod type_lookup;

use ropey::Rope;
use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "i18n"] // path to the compiled localization resources
struct Localizations;

use i18n_embed::{
    DesktopLanguageRequester,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};

use once_cell::sync::Lazy;
use tower_lsp::lsp_types::Position;
pub static LANGUAGE_LOADER: Lazy<FluentLanguageLoader> = Lazy::new(|| {
    let loader = fluent_language_loader!();
    let requested_languages: Vec<i18n_embed::unic_langid::LanguageIdentifier> = DesktopLanguageRequester::requested_languages();
    let _result = i18n_embed::select(&loader, &Localizations, &requested_languages);
    loader
});

pub fn diagnostic_message(error: &(dyn std::error::Error + Send + Sync + 'static), loader: &FluentLanguageLoader) -> String {
    match error.downcast_ref::<icy_board_ppl::compiler::CompilationErrorType>() {
        Some(icy_board_ppl::compiler::CompilationErrorType::TypeNotComparable(type_name)) => {
            i18n_embed_fl::fl!(loader, "diagnostic-type-not-comparable", type_name = type_name.as_str())
        }
        _ => error.to_string(),
    }
}

pub fn offset_to_position(offset: usize, rope: &Rope) -> Option<Position> {
    if offset > rope.len_chars() {
        return None;
    }
    let line = rope.try_char_to_line(offset).ok()?;
    let first_char_of_line = rope.try_line_to_char(line).ok()?;
    let column: usize = rope.slice(first_char_of_line..offset).chars().map(char::len_utf16).sum();
    Some(Position::new(line as u32, column as u32))
}

/// The character offset of an LSP position. A column in the middle of a UTF-16
/// surrogate pair, or past the end of the line, is not a position in the file.
pub fn position_to_offset(rope: &Rope, position: Position) -> Option<usize> {
    let line = rope.get_line(position.line as usize)?;
    let target = position.character as usize;
    let mut utf16 = 0;
    let mut chars = 0;
    for ch in line.chars() {
        if utf16 == target {
            return rope.try_line_to_char(position.line as usize).ok().map(|start| start + chars);
        }
        utf16 += ch.len_utf16();
        chars += 1;
        if utf16 > target {
            return None;
        }
    }
    if utf16 == target {
        rope.try_line_to_char(position.line as usize).ok().map(|start| start + chars)
    } else {
        None
    }
}

/// The text of the cursor's line up to the cursor.
pub fn line_before_cursor(rope: &Rope, position: Position) -> Option<String> {
    let start = rope.try_line_to_char(position.line as usize).ok()?;
    let end = position_to_offset(rope, position)?;
    Some(rope.slice(start..end).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use i18n_embed::LanguageLoader;
    use icy_board_ppl::{compiler::CompilationErrorType, executable::VariableType};

    #[test]
    fn s1_type_not_comparable_is_localized_with_independent_loaders() {
        for (locale, expected) in [
            (
                "en",
                "Type Envelope does not support equality because it is or contains a non-comparable host object",
            ),
            (
                "de",
                "Typ Envelope unterstützt keinen Gleichheitsvergleich, da er ein nicht vergleichbares Hostobjekt ist oder enthält",
            ),
        ] {
            let loader = fluent_language_loader!();
            loader.load_languages(&Localizations, &[locale.parse().unwrap()]).unwrap();
            loader.set_use_isolating(false);
            assert!(loader.has("diagnostic-type-not-comparable"), "{locale}");
            let error = CompilationErrorType::TypeNotComparable("Envelope".to_string());
            assert_eq!(diagnostic_message(&error, &loader), expected, "{locale}");
        }
    }

    #[test]
    fn s1_untranslated_diagnostics_keep_their_display_text() {
        for locale in ["en", "de"] {
            let loader = fluent_language_loader!();
            loader.load_languages(&Localizations, &[locale.parse().unwrap()]).unwrap();
            for error in [
                CompilationErrorType::RecordIoFieldNotSerializable("Child.Values".to_string(), VariableType::Integer),
                CompilationErrorType::VariableNotFound("missing".to_string()),
            ] {
                assert_eq!(diagnostic_message(&error, &loader), error.to_string(), "{locale}");
            }
            let error = std::io::Error::other("unchanged non-compiler error");
            assert_eq!(diagnostic_message(&error, &loader), error.to_string(), "{locale}");
        }
    }

    #[test]
    fn positions_use_utf16_columns() {
        let rope = Rope::from_str("a😀b\nnext");
        assert_eq!(Some(Position::new(0, 3)), offset_to_position(2, &rope));
        assert_eq!(Some(2), position_to_offset(&rope, Position::new(0, 3)));
        assert_eq!(None, position_to_offset(&rope, Position::new(0, 2)));
    }

    #[test]
    fn positions_outside_a_document_are_rejected() {
        let rope = Rope::from_str("short\n");
        assert_eq!(None, position_to_offset(&rope, Position::new(4, 0)));
        assert_eq!(None, position_to_offset(&rope, Position::new(0, 99)));
        assert_eq!(None, offset_to_position(rope.len_chars() + 1, &rope));
    }

    #[test]
    fn the_line_before_a_utf16_cursor_is_exact() {
        let rope = Rope::from_str("x = 😀 + value\n");
        assert_eq!(Some("x = 😀".to_string()), line_before_cursor(&rope, Position::new(0, 6)));
    }
}

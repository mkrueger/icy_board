pub mod completion;
pub mod context;
pub mod document_symbol;
pub mod documentation;
pub mod formatting;
pub mod hover;
pub mod jump_definition;
pub mod reference;
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

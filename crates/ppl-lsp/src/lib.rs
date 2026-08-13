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
    let line = rope.try_char_to_line(offset).ok()?;
    let first_char_of_line = rope.try_line_to_char(line).ok()?;
    let column = offset - first_char_of_line;
    Some(Position::new(line as u32, column as u32))
}

/// The text of the cursor's line up to the cursor.
pub fn line_before_cursor(rope: &Rope, position: Position) -> Option<String> {
    let line = rope.get_line(position.line as usize)?;
    let end = (position.character as usize).min(line.len_chars());
    Some(line.slice(..end).to_string())
}

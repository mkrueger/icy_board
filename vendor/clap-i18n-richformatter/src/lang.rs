use std::sync::LazyLock;

use i18n_embed::{
    LanguageLoader,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "./i18n/"]
pub struct ClapI18nLocalizations;

pub static CLAP_I18N_LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();

    loader
        .load_fallback_language(&ClapI18nLocalizations)
        .expect("Error while loading fallback language");

    loader
});

/// Get a translated string by message ID (runtime lookup, no compile-time validation)
#[doc(hidden)]
pub fn get_translation(message_id: &str) -> String {
    CLAP_I18N_LANGUAGE_LOADER.get(message_id)
}

macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!(crate::lang::CLAP_I18N_LANGUAGE_LOADER, $message_id)
    }};

    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!(crate::lang::CLAP_I18N_LANGUAGE_LOADER, $message_id, $($args), *)
    }};
}

pub(crate) use fl;

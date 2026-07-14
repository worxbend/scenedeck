//! Fluent-based localization loader shared by the UI layer.
//!
//! `.ftl` resources under `i18n/` are embedded into the binary at compile
//! time (see `i18n.toml` and [`Localizations`]), so SceneDeck ships as a
//! single self-contained executable with no runtime translation files.
//! Every `fl!()` call site reads from the shared [`LANGUAGE_LOADER`], which
//! [`init`] populates once at startup from the user's [`Language`]
//! preference, falling back to the desktop locale, then to `en`.

use std::sync::LazyLock;

use i18n_embed::{
    fluent::{fluent_language_loader, FluentLanguageLoader},
    DesktopLanguageRequester, LanguageLoader,
};
use rust_embed::RustEmbed;

use crate::domain::appearance::Language;

/// Fluent `.ftl` resources embedded into the binary at compile time.
#[derive(RustEmbed)]
#[folder = "i18n/"]
pub(crate) struct Localizations;

/// Shared Fluent loader used by every `fl!()` call site in the UI layer.
///
/// Eagerly loads the `en` fallback bundle as soon as it is first accessed, so
/// `fl!()` calls resolve correctly even before [`init`] runs (e.g. in unit
/// tests that never call `init`).
pub(crate) static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader = fluent_language_loader!();
    if let Err(err) = loader.load_languages(&Localizations, &[loader.fallback_language().clone()]) {
        tracing::error!(%err, "failed to load the en fallback locale");
    }
    disable_isolating_marks(&loader);
    loader
});

/// Fluent wraps interpolated arguments in bidi isolation marks by default;
/// disable that so GTK labels and tests see plain text. Every bundle
/// (re)load replaces the loader's internal Fluent bundles with fresh ones
/// that reset this to Fluent's default, so this must be re-applied after
/// every [`FluentLanguageLoader::load_languages`] / [`i18n_embed::select`]
/// call, not just once at construction.
fn disable_isolating_marks(loader: &FluentLanguageLoader) {
    loader.set_use_isolating(false);
}

/// Select the active locale for [`LANGUAGE_LOADER`] from the user's
/// [`Language`] preference. Called once at startup after config is read, and
/// again immediately when the user changes the language in Settings.
///
/// `Language::System` requests the desktop locale; any other variant pins
/// the exact shipped locale. [`i18n_embed::select`] always loads the
/// `en` fallback alongside the requested locale, so lookups for a message
/// missing from a translation still resolve to English rather than the
/// raw message id.
pub(crate) fn init(language: Language) {
    let requested = match language.locale_tag() {
        Some(tag) => tag.parse().map(|id| vec![id]).unwrap_or_else(|err| {
            tracing::warn!(%err, tag, "invalid locale tag, using system locale");
            DesktopLanguageRequester::requested_languages()
        }),
        None => DesktopLanguageRequester::requested_languages(),
    };

    if let Err(err) = i18n_embed::select(&*LANGUAGE_LOADER, &Localizations, &requested) {
        tracing::warn!(%err, "failed to select locale, falling back to en");
    }
    disable_isolating_marks(&LANGUAGE_LOADER);
}

#[cfg(test)]
mod tests {
    use super::*;
    use i18n_embed_fl::fl;

    #[test]
    fn resolves_english_fallback_message() {
        // Deliberately does not call `init()`: LANGUAGE_LOADER eagerly loads
        // `en` on first access, and `init()` briefly re-enables Fluent's
        // isolating marks while it reloads bundles before disabling them
        // again — calling it here would race with every other test in this
        // binary that resolves a message via the same shared static.
        assert_eq!(
            fl!(LANGUAGE_LOADER, "i18n-loader-smoke-test"),
            "Localization loaded."
        );
    }

    #[test]
    #[ignore = "mutates the shared global LANGUAGE_LOADER; run alone via \
                `cargo test -- --ignored --test-threads=1`, not part of the default suite"]
    fn every_shipped_locale_parses_and_loads() {
        for language in Language::ALL {
            let Some(tag) = language.locale_tag() else {
                continue;
            };
            init(language);
            let resolved = fl!(LANGUAGE_LOADER, "i18n-loader-smoke-test");
            assert_ne!(
                resolved, "i18n-loader-smoke-test",
                "locale {tag} failed to load (fl! fell back to the raw message id)"
            );
            assert!(
                !resolved.is_empty(),
                "locale {tag} resolved to an empty string"
            );
        }
    }
}

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

fn is_c_locale(value: &str) -> bool {
    matches!(
        value.split(['.', '@']).next().unwrap_or(value),
        "C" | "POSIX"
    )
}

fn desktop_requests_c_locale() -> bool {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
        .is_some_and(|value| is_c_locale(&value))
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
        // `C`, `C.UTF-8`, and `POSIX` deliberately request untranslated
        // messages, but `DesktopLanguageRequester` tries to parse them as BCP
        // 47 language tags and logs an error. Select the English fallback
        // directly for those standard process locales.
        None if desktop_requests_c_locale() => vec![LANGUAGE_LOADER.fallback_language().clone()],
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
    use std::collections::{BTreeMap, BTreeSet};

    fn message_ids(resource: &str) -> BTreeSet<&str> {
        resource
            .lines()
            .filter_map(|line| {
                let line = line.trim_end();
                if line.starts_with([' ', '\t', '#', '-']) {
                    return None;
                }

                line.split_once('=')
                    .map(|(id, _)| id.trim())
                    .filter(|id| !id.is_empty())
            })
            .collect()
    }

    fn message_variables(resource: &str) -> BTreeMap<String, BTreeSet<String>> {
        let mut variables = BTreeMap::<String, BTreeSet<String>>::new();
        let mut current_id = None;

        for line in resource.lines() {
            let trimmed = line.trim_end();
            if !trimmed.starts_with([' ', '\t', '#', '-']) {
                current_id = trimmed
                    .split_once('=')
                    .map(|(id, _)| id.trim().to_string())
                    .filter(|id| !id.is_empty());
            }

            let Some(id) = &current_id else {
                continue;
            };
            let entry = variables.entry(id.clone()).or_default();
            let mut remainder = trimmed;
            while let Some(dollar) = remainder.find('$') {
                remainder = &remainder[dollar + 1..];
                let length = remainder
                    .chars()
                    .take_while(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
                    })
                    .map(char::len_utf8)
                    .sum();
                if length == 0 {
                    continue;
                }
                entry.insert(remainder[..length].to_string());
                remainder = &remainder[length..];
            }
        }

        variables
    }

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
    fn c_and_posix_locales_are_not_treated_as_language_tags() {
        for locale in ["C", "C.UTF-8", "C.utf8", "POSIX", "POSIX@variant"] {
            assert!(is_c_locale(locale), "{locale}");
        }
        for locale in ["en_US.UTF-8", "uk_UA.UTF-8", "ca"] {
            assert!(!is_c_locale(locale), "{locale}");
        }
    }

    #[test]
    fn every_translation_has_the_same_messages_as_english() {
        let english_resource = include_str!("../../i18n/en/scenedeck.ftl");
        let english = message_ids(english_resource);
        let english_variables = message_variables(english_resource);
        let translations = [
            ("de-CH", include_str!("../../i18n/de-CH/scenedeck.ftl")),
            ("de", include_str!("../../i18n/de/scenedeck.ftl")),
            ("es", include_str!("../../i18n/es/scenedeck.ftl")),
            ("it", include_str!("../../i18n/it/scenedeck.ftl")),
            ("pl", include_str!("../../i18n/pl/scenedeck.ftl")),
            ("pt-PT", include_str!("../../i18n/pt-PT/scenedeck.ftl")),
            ("uk", include_str!("../../i18n/uk/scenedeck.ftl")),
        ];

        for (locale, resource) in translations {
            let translated = message_ids(resource);
            let translated_variables = message_variables(resource);
            let missing = english.difference(&translated).copied().collect::<Vec<_>>();
            let unexpected = translated.difference(&english).copied().collect::<Vec<_>>();
            assert!(
                missing.is_empty() && unexpected.is_empty(),
                "locale {locale} is out of sync: missing={missing:?}, unexpected={unexpected:?}"
            );
            assert_eq!(
                translated_variables, english_variables,
                "locale {locale} uses a different set of Fluent variables"
            );
        }
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

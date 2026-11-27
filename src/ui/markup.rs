//! The Pango markup boundary for text SceneDeck does not author.
//!
//! OBS scene names, OBS error messages and network errors can contain `<`
//! or `&`. `AdwToast:title` (libadwaita 1.4 and later) and
//! `AdwStatusPage:description` parse their text as Pango markup, so such
//! text either fails to parse and blanks the widget, or renders as markup
//! someone else wrote. Toasts are built here; `architecture_tests` keeps
//! every other module from building one directly.

/// `text` escaped so a Pango markup property shows it as typed.
pub(crate) fn escape(text: &str) -> glib::GString {
    glib::markup_escape_text(text)
}

/// A toast that shows `title` as typed, never as markup.
pub(crate) fn plain_toast(title: &str) -> adw::Toast {
    adw::Toast::builder().title(title).use_markup(false).build()
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBS_TEXT: &str = "<b>x</b> & y";

    #[test]
    fn escape_turns_markup_characters_into_entities() {
        assert_eq!(escape(OBS_TEXT), "&lt;b&gt;x&lt;/b&gt; &amp; y");
    }

    #[test]
    fn escape_round_trips_through_the_pango_parser() {
        let (_, parsed, _) =
            gtk4::pango::parse_markup(&escape(OBS_TEXT), '\0').expect("escaped text is markup");

        assert_eq!(parsed, OBS_TEXT);
    }

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_plain_toast_keeps_its_title_literal() {
        crate::ui::native_test!(|| {
            let toast = plain_toast(OBS_TEXT);

            assert!(!toast.uses_markup());
            assert_eq!(toast.title().as_deref(), Some(OBS_TEXT));
        });
    }
}

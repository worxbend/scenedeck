//! The one constructor for full-page empty, loading and error states.
//!
//! `AdwStatusPage:description` is Pango markup and descriptions often carry
//! an OBS scene name or error message, so this escapes every description.
//! `architecture_tests` keeps pages from building a status page directly.

use crate::ui::markup;
use adw::prelude::*;

/// A themed status page that shows `literal_description` as typed.
pub(crate) fn status_page(
    icon_name: &str,
    title: &str,
    literal_description: &str,
) -> adw::StatusPage {
    let page = adw::StatusPage::builder()
        .icon_name(icon_name)
        .title(title)
        .description(markup::escape(literal_description))
        .build();
    page.add_css_class("app-status-page");
    page
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_status_page_renders_its_description_literally() {
        crate::ui::native_test!(|| {
            const OBS_TEXT: &str = "<b>x</b> & y";
            let page = status_page("dialog-warning-symbolic", "Title", OBS_TEXT);

            let rendered =
                crate::ui::test_support::find_where(page.upcast_ref::<gtk4::Widget>(), |widget| {
                    widget
                        .downcast_ref::<gtk4::Label>()
                        .is_some_and(|label| label.text() == OBS_TEXT)
                });

            assert!(rendered.is_some(), "description must render as typed");
            assert!(page.has_css_class("app-status-page"));
        });
    }
}

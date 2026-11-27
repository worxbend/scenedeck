//! Load problems found at startup, shown once at the top of Settings.

use adw::{prelude::*, ActionRow, PreferencesGroup};

/// One warning row per notice, or `None` when startup found nothing to report.
pub(super) fn group(notices: &[String]) -> Option<PreferencesGroup> {
    if notices.is_empty() {
        return None;
    }
    let group = PreferencesGroup::new();
    for notice in notices {
        // GObject builder property order may set title before use-markup.
        let row = ActionRow::builder().use_markup(false).build();
        row.set_title(notice);
        super::with_icon(&row, "dialog-warning-symbolic");
        group.add(&row);
    }
    Some(group)
}

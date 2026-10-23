//! Copyable OBS-host output evidence, separate from command acknowledgement.

use crate::domain::output_result::{OutputResultKind, OutputResultStatus};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::navigation::NavigationContext;
use adw::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation};
use i18n_embed_fl::fl;

pub(crate) fn build(nav: &NavigationContext) -> gtk4::Widget {
    let root = GtkBox::new(Orientation::Vertical, 6);
    let entries: Vec<_> = nav
        .state
        .borrow()
        .workspace
        .output_results
        .entries()
        .rev()
        .cloned()
        .collect();
    populate(&root, entries.clone());
    let previous = std::cell::RefCell::new(entries);
    let weak = root.downgrade();
    let nav = nav.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
        let Some(root) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if root.is_mapped() {
            let entries: Vec<_> = nav
                .state
                .borrow()
                .workspace
                .output_results
                .entries()
                .rev()
                .cloned()
                .collect();
            if *previous.borrow() != entries {
                crate::ui::clear_children(&root);
                populate(&root, entries.clone());
                previous.replace(entries);
            }
        }
        glib::ControlFlow::Continue
    });
    let expander = gtk4::Expander::builder()
        .label(fl!(LANGUAGE_LOADER, "output-results-title"))
        .child(&root)
        .expanded(false)
        .build();
    expander.upcast()
}

fn populate(root: &GtkBox, entries: Vec<crate::domain::output_result::OutputResult>) {
    let note = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "output-results-detail"))
        .wrap(true)
        .max_width_chars(36)
        .xalign(0.0)
        .build();
    root.append(&note);
    for entry in entries {
        let kind = match entry.kind {
            OutputResultKind::ReplaySave => fl!(LANGUAGE_LOADER, "control-replay-save"),
            OutputResultKind::RecordingStop => fl!(LANGUAGE_LOADER, "output-results-record-stop"),
            OutputResultKind::RecordingSplit => fl!(LANGUAGE_LOADER, "control-split"),
            OutputResultKind::RecordingChapter => fl!(LANGUAGE_LOADER, "control-chapter"),
        };
        let status = match entry.status {
            OutputResultStatus::Requested => fl!(LANGUAGE_LOADER, "workflow-activity-requested"),
            OutputResultStatus::Acknowledged => fl!(LANGUAGE_LOADER, "output-results-acknowledged"),
            OutputResultStatus::Observed => fl!(LANGUAGE_LOADER, "workflow-activity-observed"),
            OutputResultStatus::Failed => fl!(LANGUAGE_LOADER, "workflow-activity-failed"),
            OutputResultStatus::Unconfirmed => fl!(LANGUAGE_LOADER, "output-results-unconfirmed"),
        };
        let time = glib::DateTime::from_unix_local(
            (entry.updated_unix_ms / 1000).min(i64::MAX as u64) as i64,
        )
        .ok()
        .and_then(|time| time.format("%H:%M:%S").ok())
        .map(|time| time.to_string())
        .unwrap_or_default();
        let title = Label::builder()
            .label(format!("{time} · {kind} · {status}"))
            .wrap(true)
            .selectable(true)
            .xalign(0.0)
            .build();
        root.append(&title);
        let scope = Label::builder()
            .label(format!(
                "{} · {}",
                entry.scope.endpoint, entry.scope.collection
            ))
            .wrap(true)
            .selectable(true)
            .xalign(0.0)
            .build();
        scope.add_css_class("caption");
        root.append(&scope);
        if let Some(detail) = entry.detail {
            root.append(
                &Label::builder()
                    .label(detail)
                    .wrap(true)
                    .selectable(true)
                    .xalign(0.0)
                    .build(),
            );
        }
        if let Some(path) = entry.path {
            let line = GtkBox::new(Orientation::Horizontal, 6);
            let label = Label::builder()
                .label(fl!(
                    LANGUAGE_LOADER,
                    "output-results-host-path",
                    path = path.clone()
                ))
                .wrap(true)
                .selectable(true)
                .hexpand(true)
                .xalign(0.0)
                .build();
            let copy = Button::builder()
                .icon_name("edit-copy-symbolic")
                .tooltip_text(fl!(LANGUAGE_LOADER, "output-results-copy-path"))
                .build();
            copy.update_property(&[gtk4::accessible::Property::Label(&fl!(
                LANGUAGE_LOADER,
                "output-results-copy-path"
            ))]);
            copy.connect_clicked(move |button| button.clipboard().set_text(&path));
            line.append(&label);
            line.append(&copy);
            root.append(&line);
        }
        root.append(&gtk4::Separator::new(Orientation::Horizontal));
    }
}

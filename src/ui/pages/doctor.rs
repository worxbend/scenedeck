//! Doctor page — structural diagnostics.
//!
//! Runs `DoctorService` over the current inventory, registry, and graph, then
//! lists the results grouped by severity (Errors, Warnings, Info).  Self-
//! refreshes whenever the page is shown, and offers a manual "Re-run" button.

use std::rc::Rc;

use adw::{prelude::*, ActionRow, PreferencesGroup, PreferencesPage, StatusPage};
use gtk4::{Box as GtkBox, Image, Orientation};
use i18n_embed_fl::fl;

use crate::domain::diagnostic::{Diagnostic, DiagnosticSeverity};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::services::doctor_service::DoctorService;
use crate::ui::navigation::NavigationContext;

fn no_diagnostics_summary() -> String {
    fl!(LANGUAGE_LOADER, "doctor-all-clear-title")
}

fn no_diagnostics_detail() -> String {
    fl!(LANGUAGE_LOADER, "doctor-all-clear-detail")
}

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    let container = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .build();
    container.add_css_class("app-page");
    container.add_css_class("doctor-page");

    populate(&container, &nav);

    let refresh_fn: Rc<dyn Fn()> = Rc::new({
        let nav = nav.clone();
        let container = container.clone();
        move || rebuild(&container, &nav)
    });

    container.connect_map({
        let refresh = refresh_fn.clone();
        move |_| refresh()
    });

    (container.upcast(), refresh_fn)
}

fn rebuild(container: &GtkBox, nav: &NavigationContext) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    populate(container, nav);
}

fn populate(container: &GtkBox, nav: &NavigationContext) {
    let (inventory, graph) = {
        let state = nav.state.borrow();
        (state.scene_inventory.clone(), state.scene_graph.clone())
    };

    if inventory.scenes.is_empty() {
        let empty = StatusPage::builder()
            .icon_name("emblem-default-symbolic")
            .title(fl!(LANGUAGE_LOADER, "doctor-empty-state-title"))
            .description(fl!(LANGUAGE_LOADER, "doctor-empty-state-description"))
            .build();
        container.append(&empty);
        return;
    }

    let registry = nav.state.borrow().registry.clone();
    let registry_snapshot = registry.snapshot();
    let diagnostics = DoctorService::run(&inventory, &registry_snapshot, &graph);

    let page = PreferencesPage::builder()
        .title(fl!(LANGUAGE_LOADER, "doctor-page-title"))
        .vexpand(true)
        .build();
    page.add_css_class("app-preferences-page");

    // ── Summary / re-run ──────────────────────────────────────────────────────
    let summary_group = PreferencesGroup::new();
    let summary_row = ActionRow::builder()
        .title(fl!(LANGUAGE_LOADER, "doctor-summary-row-title"))
        .subtitle(diagnostic_summary(&diagnostics))
        .build();

    let rerun_btn = gtk4::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text(fl!(LANGUAGE_LOADER, "doctor-rerun-tooltip"))
        .valign(gtk4::Align::Center)
        .build();
    rerun_btn.add_css_class("flat");
    rerun_btn.connect_clicked({
        let nav = nav.clone();
        let container = container.clone();
        move |_| rebuild(&container, &nav)
    });
    summary_row.add_suffix(&rerun_btn);
    summary_group.add(&summary_row);
    page.add(&summary_group);

    // ── All clear ─────────────────────────────────────────────────────────────
    if diagnostics.is_empty() {
        let ok_group = PreferencesGroup::new();
        let ok_row = ActionRow::builder()
            .title(no_diagnostics_summary())
            .subtitle(no_diagnostics_detail())
            .build();
        let icon = Image::from_icon_name("object-select-symbolic");
        icon.add_css_class("diag-ok");
        ok_row.add_prefix(&icon);
        ok_group.add(&ok_row);
        page.add(&ok_group);
        container.append(&page);
        return;
    }

    // ── One group per severity (Errors first) ─────────────────────────────────
    for severity in DiagnosticSeverity::DISPLAY_ORDER {
        let group_diags: Vec<&Diagnostic> = diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .collect();
        if group_diags.is_empty() {
            continue;
        }

        let group = PreferencesGroup::builder().title(severity.label()).build();
        for diag in group_diags {
            let title = diag.title();
            let row = ActionRow::builder().title(&title).build();
            if let Some(suggestion) = &diag.suggestion {
                row.set_subtitle(suggestion);
            }
            let icon = Image::from_icon_name(severity.icon_name());
            icon.add_css_class(severity.css_class());
            row.add_prefix(&icon);
            group.add(&row);
        }
        page.add(&group);
    }

    container.append(&page);
}

fn diagnostic_summary(diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return no_diagnostics_summary();
    }

    DiagnosticSeverity::DISPLAY_ORDER
        .iter()
        .map(|severity| severity.format_count(severity.count_in(diagnostics)))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_summary_uses_all_clear_text_when_empty() {
        assert_eq!(diagnostic_summary(&[]), "No problems found");
    }

    #[test]
    fn diagnostic_summary_counts_findings_by_display_order() {
        let diagnostics = vec![
            diagnostic(DiagnosticSeverity::Warning),
            diagnostic(DiagnosticSeverity::Error),
            diagnostic(DiagnosticSeverity::Info),
            diagnostic(DiagnosticSeverity::Warning),
        ];

        assert_eq!(
            diagnostic_summary(&diagnostics),
            "1 error, 2 warnings, 1 info item"
        );
    }

    fn diagnostic(severity: DiagnosticSeverity) -> Diagnostic {
        Diagnostic {
            severity,
            scene: None,
            message: String::new(),
            suggestion: None,
        }
    }
}

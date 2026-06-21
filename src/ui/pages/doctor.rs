//! Doctor page — structural diagnostics.
//!
//! Runs `DoctorService` over the current inventory, registry, and graph, then
//! lists the results grouped by severity (Errors, Warnings, Info).  Self-
//! refreshes whenever the page is shown, and offers a manual "Re-run" button.

use std::rc::Rc;

use adw::{prelude::*, ActionRow, PreferencesGroup, PreferencesPage, StatusPage};
use gtk4::{Box as GtkBox, Image, Orientation};

use crate::domain::diagnostic::{Diagnostic, DiagnosticSeverity};
use crate::services::doctor_service::DoctorService;
use crate::storage::registry::read_registry;
use crate::ui::navigation::NavigationContext;

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
            .title("Nothing to Check")
            .description("Connect to OBS to run architecture diagnostics.")
            .build();
        container.append(&empty);
        return;
    }

    let registry = read_registry();
    let registry_snapshot = registry.snapshot();
    let diagnostics = DoctorService::run(&inventory, &registry_snapshot, &graph);

    let page = PreferencesPage::builder()
        .title("Doctor")
        .vexpand(true)
        .build();
    page.add_css_class("app-preferences-page");

    // ── Summary / re-run ──────────────────────────────────────────────────────
    let errors = count(&diagnostics, DiagnosticSeverity::Error);
    let warnings = count(&diagnostics, DiagnosticSeverity::Warning);
    let infos = count(&diagnostics, DiagnosticSeverity::Info);

    let summary_group = PreferencesGroup::new();
    let summary_row = ActionRow::builder()
        .title("Diagnostics")
        .subtitle(format!(
            "{errors} error(s), {warnings} warning(s), {infos} info"
        ))
        .build();

    let rerun_btn = gtk4::Button::builder()
        .label("Re-run")
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
            .title("No problems found")
            .subtitle("The scene architecture satisfies all checks.")
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
    for severity in [
        DiagnosticSeverity::Error,
        DiagnosticSeverity::Warning,
        DiagnosticSeverity::Info,
    ] {
        let group_diags: Vec<&Diagnostic> = diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .collect();
        if group_diags.is_empty() {
            continue;
        }

        let group = PreferencesGroup::builder().title(severity.label()).build();
        for diag in group_diags {
            let title = diag
                .scene
                .as_deref()
                .map(|s| format!("{s}: {}", diag.message))
                .unwrap_or_else(|| diag.message.clone());

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

fn count(diags: &[Diagnostic], severity: DiagnosticSeverity) -> usize {
    diags.iter().filter(|d| d.severity == severity).count()
}

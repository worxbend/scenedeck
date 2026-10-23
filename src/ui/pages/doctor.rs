//! Doctor page — structural diagnostics.
//!
//! Runs `DoctorService` over a complete inventory, registry, and graph, then
//! lists the results grouped by severity (Errors, Warnings, Info).
//! Re-analyzes cached observations when shown; manual refresh reads OBS again.

use std::rc::Rc;

use adw::{prelude::*, ActionRow, PreferencesGroup, PreferencesPage};
use gtk4::{Box as GtkBox, Image};
use i18n_embed_fl::fl;

use crate::controller::command::AppCommand;
use crate::controller::state::{AppState, ObsStatus, TopologyStatus};
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
    let read_nav = nav.clone();
    crate::ui::stateful_page(
        "doctor-page",
        false,
        move || {
            let state = read_nav.state.borrow();
            (
                state.workspace.epoch,
                state.config.language,
                matches!(state.obs_status, ObsStatus::Connected { .. }),
                state
                    .scene_inventory
                    .scenes
                    .iter()
                    .map(|s| s.id.clone())
                    .collect::<Vec<_>>(),
                state.scene_graph.edges.clone(),
                state.registry.snapshot(),
                state.topology_status.clone(),
            )
        },
        move |container| {
            populate(container, &nav);
        },
    )
}

/// Refill the page in place.
///
/// Kept as its own function, unlike the Graph page, because `populate` wires
/// it to the "run checks again" button and so needs to call it from inside
/// itself — where the refresh callback that `stateful_page` returns is not
/// reachable.
fn rebuild(container: &GtkBox, nav: &NavigationContext) {
    crate::ui::rebuild_page(container, |container| populate(container, nav));
}

fn populate(container: &GtkBox, nav: &NavigationContext) {
    let (inventory, graph, topology_status) = {
        let state = nav.state.borrow();
        (
            state.scene_inventory.clone(),
            state.scene_graph.clone(),
            state.topology_status.clone(),
        )
    };

    let page = PreferencesPage::builder()
        .title(fl!(LANGUAGE_LOADER, "doctor-page-title"))
        .vexpand(true)
        .build();
    page.add_css_class("app-preferences-page");

    // ── Summary / re-run ──────────────────────────────────────────────────────
    let summary_group = PreferencesGroup::builder()
        .title(fl!(LANGUAGE_LOADER, "doctor-structural-title"))
        .build();
    let summary_row = ActionRow::builder()
        .use_markup(false)
        .title(fl!(LANGUAGE_LOADER, "doctor-summary-row-title"))
        .build();

    let rerun_btn = gtk4::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text(fl!(LANGUAGE_LOADER, "doctor-refresh-topology-tooltip"))
        .valign(gtk4::Align::Center)
        .build();
    rerun_btn.add_css_class("flat");
    rerun_btn.set_sensitive(
        matches!(nav.state.borrow().obs_status, ObsStatus::Connected { .. })
            && topology_status != TopologyStatus::Loading,
    );
    rerun_btn.connect_clicked({
        let nav = nav.clone();
        let container = container.downgrade();
        move |_| {
            if let Some(container) = container.upgrade() {
                let command = begin_topology_refresh(&mut nav.state.borrow_mut());
                if let Some(command) = command {
                    rebuild(&container, &nav);
                    nav.dispatch(command);
                }
            }
        }
    });
    summary_row.add_suffix(&rerun_btn);
    summary_group.add(&summary_row);
    page.add(&summary_group);

    let notice = topology_status.notice().or_else(|| {
        inventory.scenes.is_empty().then(|| {
            (
                fl!(LANGUAGE_LOADER, "topology-empty-title"),
                fl!(LANGUAGE_LOADER, "topology-empty-description"),
            )
        })
    });
    if let Some((title, description)) = notice {
        summary_row.set_title(&title);
        summary_row.set_subtitle(&description);
        container.append(&page);
        return;
    }

    let registry_snapshot = nav.state.borrow().registry.snapshot();
    let diagnostics = DoctorService::run(&inventory, &registry_snapshot, &graph);
    summary_row.set_subtitle(&diagnostic_summary(&diagnostics));

    // ── All clear ─────────────────────────────────────────────────────────────
    if diagnostics.is_empty() {
        let ok_group = PreferencesGroup::new();
        let ok_row = ActionRow::builder()
            .use_markup(false)
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
            let row = ActionRow::builder().use_markup(false).build();
            row.set_title(&title);
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

fn begin_topology_refresh(state: &mut AppState) -> Option<AppCommand> {
    if !matches!(state.obs_status, ObsStatus::Connected { .. })
        || state.topology_status == TopologyStatus::Loading
    {
        return None;
    }
    state.set_topology_loading();
    Some(AppCommand::RefreshData)
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
    use crate::domain::{graph::SceneGraph, role::SceneRole, scene::Scene};

    #[test]
    fn doctor_rerun_requests_fresh_obs_data_and_immediately_retires_ready_verdict() {
        let mut state = AppState::new(Default::default(), Default::default(), None, None);
        state.obs_status = ObsStatus::Connected {
            obs_version: "fixture".into(),
        };
        for status in [
            TopologyStatus::Missing,
            TopologyStatus::Ready,
            TopologyStatus::Failed("old".into()),
        ] {
            state.topology_status = status;
            assert!(matches!(
                begin_topology_refresh(&mut state),
                Some(AppCommand::RefreshData)
            ));
            assert_eq!(state.topology_status, TopologyStatus::Loading);
            assert!(begin_topology_refresh(&mut state).is_none());
        }
        state.set_topology_ready(SceneGraph::default());
        state.obs_status = ObsStatus::Disconnected;
        assert!(begin_topology_refresh(&mut state).is_none());
        assert_eq!(state.topology_status, TopologyStatus::Ready);
    }

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_doctor_requires_ready_topology_and_rerun_shows_loading_immediately() {
        crate::ui::test_support::native_test(
            "ui::pages::doctor::tests::native_doctor_requires_ready_topology_and_rerun_shows_loading_immediately",
            || {
                let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
                let container = GtkBox::new(gtk4::Orientation::Vertical, 0);
                {
                    let mut state = nav.state.borrow_mut();
                    state.scene_inventory.scenes = vec![Scene { id: "Main".into() }];
                    state.registry.set_scene_role("Main", Some(SceneRole::Primary));
                }
                populate(&container, &nav);
                assert!(page_text(container.upcast_ref()).contains("Scene topology not observed"));
                assert!(!page_text(container.upcast_ref()).contains("No problems found"));
                nav.state.borrow_mut().scene_inventory.scenes.clear();
                nav.state.borrow_mut().set_topology_ready(SceneGraph::default());
                rebuild(&container, &nav);
                assert!(page_text(container.upcast_ref()).contains("No scenes observed"));
                {
                    let mut state = nav.state.borrow_mut();
                    state.obs_status = ObsStatus::Connected { obs_version: "fixture".into() };
                    state.scene_inventory.scenes = vec![Scene { id: "Main".into() }];
                    state.registry.set_scene_role("Main", Some(SceneRole::Primary));
                    state.set_topology_ready(SceneGraph::default());
                }
                rebuild(&container, &nav);
                assert!(page_text(container.upcast_ref()).contains("No problems found"));
                let button = refresh_button(container.upcast_ref()).expect("refresh button");
                assert!(button.is_sensitive());
                button.emit_clicked();
                assert_eq!(nav.state.borrow().topology_status, TopologyStatus::Loading);
                let text = page_text(container.upcast_ref());
                assert!(text.contains("Reading scene topology"));
                assert!(!text.contains("No problems found"));
                assert!(!refresh_button(container.upcast_ref()).unwrap().is_sensitive());
                nav.state.borrow_mut().set_topology_failed("Group <b>& read failed".into());
                rebuild(&container, &nav);
                let text = page_text(container.upcast_ref());
                assert!(text.contains("Scene topology could not be read"));
                assert!(text.contains("Group <b>& read failed"));
                assert!(!text.contains("No problems found"));
                refresh_button(container.upcast_ref()).unwrap().emit_clicked();
                assert_eq!(nav.state.borrow().topology_status, TopologyStatus::Loading);
                nav.state.borrow_mut().set_topology_ready(SceneGraph::default());
                rebuild(&container, &nav);
                assert!(page_text(container.upcast_ref()).contains("No problems found"));
                nav.state.borrow_mut().invalidate_collection_runtime();
                rebuild(&container, &nav);
                assert!(page_text(container.upcast_ref()).contains("Scene topology not observed"));
                nav.state.borrow_mut().reset_obs_session(ObsStatus::Disconnected);
                rebuild(&container, &nav);
                assert!(page_text(container.upcast_ref()).contains("Scene topology not observed"));
            },
        );
    }

    fn page_text(widget: &gtk4::Widget) -> String {
        let mut text = widget
            .downcast_ref::<gtk4::Label>()
            .map(|label| label.text().to_string())
            .unwrap_or_default();
        let mut child = widget.first_child();
        while let Some(widget) = child {
            text.push_str(&page_text(&widget));
            child = widget.next_sibling();
        }
        text
    }

    fn refresh_button(widget: &gtk4::Widget) -> Option<gtk4::Button> {
        if let Some(button) = widget.downcast_ref::<gtk4::Button>() {
            if button.icon_name().as_deref() == Some("view-refresh-symbolic") {
                return Some(button.clone());
            }
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            if let Some(button) = refresh_button(&widget) {
                return Some(button);
            }
            child = widget.next_sibling();
        }
        None
    }

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

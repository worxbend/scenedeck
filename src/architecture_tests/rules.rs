use super::scanner::{source_violations, violations, Forbidden};

const GTK: &[Forbidden] = &[
    Forbidden::PathPrefix("gtk4"),
    Forbidden::PathPrefix("adw::"),
    Forbidden::PathPrefix("glib::"),
    Forbidden::PathPrefix("gio::"),
];

#[test]
fn ui_never_reaches_the_obs_adapter_directly() {
    let offenders = violations("ui", &[Forbidden::PathPrefix("crate::obs")], &[]);
    assert!(
        offenders.is_empty(),
        "UI must dispatch an AppCommand through NavigationContext, not call src/obs/ directly:\n{}",
        offenders.join("\n")
    );
}

/// The OBS WebSocket library stays inside the OBS adapter.
///
/// Keeping `obws` in one module is what makes the protocol replaceable and
/// keeps protocol details out of code that decides what the app should do.
/// This had already been broken once: the controller's event loop matched
/// raw `obws` event variants and carried its own copy of the mapping that
/// `obs/mapper.rs` already owned.
#[test]
fn only_the_obs_adapter_imports_obws() {
    let offenders = violations("", &[Forbidden::PathPrefix("obws")], &["obs"]);

    assert!(
        offenders.is_empty(),
        "`obws` may only be used inside src/obs/. Wrap it in `ObsClient`, \
         `ObsEventStream`, or a `mapper` function and hand a domain type \
         across the boundary instead:\n{}",
        offenders.join("\n")
    );
}

/// Domain types stay free of GTK and of the OBS protocol.
///
/// `src/domain/` holds what the app is about — scenes, roles, audio levels,
/// output state. If a widget type or a protocol type reaches in there, the
/// concepts stop being testable without a display or a running OBS, and
/// the app's own vocabulary starts being shaped by its dependencies.
#[test]
fn domain_types_depend_on_neither_gtk_nor_obs() {
    let mut forbidden = GTK.to_vec();
    forbidden.push(Forbidden::PathPrefix("obws"));
    let offenders = violations("domain", &forbidden, &[]);

    assert!(
        offenders.is_empty(),
        "src/domain/ must not depend on GTK or on `obws`; convert at the \
         adapter boundary instead:\n{}",
        offenders.join("\n")
    );
}

/// Services stay free of GTK.
///
/// `src/services/` is pure logic over domain snapshots — dB conversion,
/// graph edge classification, the Doctor's checks. Keeping GTK out is what
/// lets those rules be tested by calling them, with no display, no main
/// loop, and no widget tree to assemble first.
#[test]
fn services_hold_no_gtk() {
    let offenders = violations("services", GTK, &[]);

    assert!(
        offenders.is_empty(),
        "src/services/ must not depend on GTK. A service decides what is \
         true; a widget decides how to show it:\n{}",
        offenders.join("\n")
    );
}

/// Neither the domain nor the services know how anything is stored.
///
/// `src/storage/` owns the serde representations, the file layout, and the
/// keyring. Its types convert to domain snapshots at the boundary. If the
/// dependency ran the other way, changing the shape of `config.json` would
/// mean editing the rules of the application.
#[test]
fn domain_and_services_do_not_reach_into_storage() {
    let storage = [Forbidden::PathPrefix("crate::storage")];
    let mut offenders = violations("domain", &storage, &[]);
    offenders.extend(violations("services", &storage, &[]));

    assert!(
        offenders.is_empty(),
        "src/domain/ and src/services/ must not depend on src/storage/. \
         Convert a storage type into a domain snapshot and pass that \
         instead:\n{}",
        offenders.join("\n")
    );
}

/// Pages and widgets reach the controller only through `NavigationContext`.
///
/// `NavigationContext` exposes exactly two things: switch to a page, and
/// dispatch a command. That narrow surface is what keeps the pages from
/// growing their own opinions about connection lifecycles or OBS calls.
///
/// `window.rs` and `navigation.rs` are exempt because they are where the
/// application is assembled: one builds the controller, the other wraps it.
#[test]
fn pages_reach_the_controller_only_through_navigation() {
    let offenders = violations(
        "ui",
        &[Forbidden::AnySegment("AppController")],
        &["ui/window.rs", "ui/navigation.rs"],
    );

    assert!(
        offenders.is_empty(),
        "UI pages and widgets must go through `NavigationContext` \
         (`switch_to_page` / `dispatch`) rather than naming \
         `AppController` directly:\n{}",
        offenders.join("\n")
    );
}

/// Background layers stay off GTK.
///
/// Controller, storage and OBS code run on Tokio or the blocking pool. GTK
/// objects are main-thread only, so a widget or `glib` handle here is a
/// threading bug waiting for a caller. Results cross back as `AppEvent`.
#[test]
fn background_layers_hold_no_gtk_or_ui() {
    let mut forbidden = GTK.to_vec();
    forbidden.push(Forbidden::PathPrefix("crate::ui"));
    let mut offenders = violations("controller", &forbidden, &[]);
    offenders.extend(violations("storage", &forbidden, &[]));
    offenders.extend(violations("obs", &forbidden, &[]));

    assert!(
        offenders.is_empty(),
        "src/controller/, src/storage/ and src/obs/ must not depend on GTK \
         or src/ui/. Send an `AppEvent` and let the GTK thread apply it:\n{}",
        offenders.join("\n")
    );
}

const INVENTORY_LIST_BAN: &[Forbidden] = &[
    Forbidden::AnySegment("ColumnView"),
    Forbidden::PathPrefix("build_column_controls"),
];

/// Inventory rows stay a reflowing list.
///
/// `include_str!("inventory.rs")` named one file, so a move to
/// `inventory/mod.rs` compiled against the wrong text or failed outright.
/// `Path::starts_with` matches whole components, so `inventory.rs` and
/// `inventory/` need both scopes. `AnySegment` sees `ColumnView` in a
/// qualified path. Method and field names are not paths;
/// `assert_native_list` checks the built widget for those.
#[test]
fn inventory_does_not_use_a_column_view() {
    let mut offenders = violations("ui/pages/inventory", INVENTORY_LIST_BAN, &[]);
    offenders.extend(violations("ui/pages/inventory.rs", INVENTORY_LIST_BAN, &[]));
    assert!(
        offenders.is_empty(),
        "Inventory must reflow list rows, not a column view:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn column_view_matches_any_segment_and_a_called_name() {
    for source in [
        "fn build() { let _view = ColumnView::new(); }",
        "fn build() { let _view = gtk4::ColumnView::new(); }",
        "fn build() { let _view = foo::ColumnView::new(); }",
        "use gtk4::ColumnView;",
        "fn build() { build_column_controls(); }",
    ] {
        assert!(
            !source_violations(source, INVENTORY_LIST_BAN).is_empty(),
            "scanner missed {source}"
        );
    }
}

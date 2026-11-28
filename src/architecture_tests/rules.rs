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

const TOAST: &[Forbidden] = &[Forbidden::AnySegment("Toast")];
const STATUS_PAGE: &[Forbidden] = &[Forbidden::AnySegment("StatusPage")];

/// Toasts and status pages show OBS and network text as typed.
///
/// `AdwToast:title` and `AdwStatusPage:description` parse Pango markup, and
/// both often carry a scene name or an error message. A scene named `Q&A`
/// blanked the Mixer error page and could spoof toast text. The two helpers
/// are the only places that build either widget.
#[test]
fn toasts_and_status_pages_go_through_the_literal_helpers() {
    let mut offenders = violations("", TOAST, &["ui/markup.rs"]);
    offenders.extend(violations("", STATUS_PAGE, &["ui/widgets/status_page.rs"]));

    assert!(
        offenders.is_empty(),
        "Build toasts with `ui::markup::plain_toast` and status pages with \
         `ui::widgets::status_page::status_page`:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn literal_helper_ban_matches_qualified_and_imported_names() {
    for source in [
        "fn build() { let _toast = adw::Toast::new(\"x\"); }",
        "fn build() { let _toast = Toast::builder().build(); }",
        "use adw::{prelude::*, Toast};",
    ] {
        assert!(
            !source_violations(source, TOAST).is_empty(),
            "scanner missed {source}"
        );
    }
    for source in [
        "fn build() { let _page = StatusPage::builder().build(); }",
        "use adw::{prelude::*, StatusPage};",
    ] {
        assert!(
            !source_violations(source, STATUS_PAGE).is_empty(),
            "scanner missed {source}"
        );
    }
}

fn crate_modules(names: &[&'static str]) -> Vec<Forbidden<'static>> {
    names.iter().copied().map(Forbidden::PathPrefix).collect()
}

/// The OBS adapter sits under the application, not beside it.
///
/// `src/obs/` translates the protocol into domain types. If it reaches up
/// into the controller, the UI, storage or a service, the protocol layer
/// starts deciding application policy, and no fake OBS can stand in for it
/// without dragging that policy along.
#[test]
fn obs_adapter_depends_only_on_domain_and_infra() {
    let offenders = violations(
        "obs",
        &crate_modules(&[
            "crate::controller",
            "crate::ui",
            "crate::storage",
            "crate::services",
        ]),
        &[],
    );

    assert!(
        offenders.is_empty(),
        "src/obs/ may depend on src/domain/ and src/infra/ only. Map to a \
         domain type and let the controller decide what to do with it:\n{}",
        offenders.join("\n")
    );
}

/// OBS passwords go through one port.
///
/// `CredentialStore` is the only way to read or write a password, and
/// `storage::secret::KeyringCredentialStore` is its only implementation,
/// built once by the controller's dependencies. The controller and the
/// Settings editor once had a keyring trait each, so a fix to one path
/// missed the other.
#[test]
fn the_keyring_is_reached_only_through_the_credential_store() {
    let mut offenders = violations(
        "",
        &[Forbidden::PathPrefix("keyring")],
        &["storage/secret.rs"],
    );
    offenders.extend(violations(
        "",
        &[Forbidden::PathPrefix("crate::storage::secret")],
        &["controller/dependencies.rs"],
    ));

    assert!(
        offenders.is_empty(),
        "Read and write OBS passwords through `CredentialStore`; only \
         src/storage/secret.rs uses `keyring`, and only \
         src/controller/dependencies.rs builds its store:\n{}",
        offenders.join("\n")
    );
}

/// Storage persists domain snapshots and knows nothing above them.
///
/// `src/storage/` is called by the controller and by GTK-side persistence.
/// An import back into either, or into the OBS adapter or a service, makes
/// the file format depend on runtime orchestration.
#[test]
fn storage_depends_only_on_domain_and_infra() {
    let offenders = violations(
        "storage",
        &crate_modules(&[
            "crate::controller",
            "crate::obs",
            "crate::services",
            "crate::ui",
        ]),
        &[],
    );

    assert!(
        offenders.is_empty(),
        "src/storage/ may depend on src/domain/ and src/infra/ only. Take \
         the value as a parameter instead of reaching for its owner:\n{}",
        offenders.join("\n")
    );
}

/// The domain is the innermost layer.
///
/// Every other module may use domain types; the domain uses none of them.
/// That is what lets a rule about scenes or audio be tested by calling it.
#[test]
fn domain_is_innermost() {
    let offenders = violations(
        "domain",
        &crate_modules(&[
            "crate::controller",
            "crate::ui",
            "crate::obs",
            "crate::storage",
            "crate::services",
        ]),
        &[],
    );

    assert!(
        offenders.is_empty(),
        "src/domain/ must not depend on any outer module. Move the shared \
         concept into the domain, or pass the value in:\n{}",
        offenders.join("\n")
    );
}

/// Services compute over domain snapshots and nothing else.
///
/// A service that reads storage, calls OBS or touches the controller or
/// UI is no longer a pure decision; it becomes a second orchestrator.
#[test]
fn services_depend_only_on_domain() {
    let offenders = violations(
        "services",
        &crate_modules(&[
            "crate::controller",
            "crate::ui",
            "crate::obs",
            "crate::storage",
        ]),
        &[],
    );

    assert!(
        offenders.is_empty(),
        "src/services/ may depend on src/domain/ (and src/infra/) only. Have \
         the caller fetch the data and pass the snapshot in:\n{}",
        offenders.join("\n")
    );
}

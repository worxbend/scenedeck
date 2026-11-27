//! Choosing which show workspace holds the curation, once OBS reports its
//! collection, and announcing workspaces evicted to make room.

use std::rc::Rc;

use super::{
    after_registry_import, begin_registry_import, reconcile_registry, registry_edit_allowed,
};
use crate::storage::show_workspaces::{RegistryTarget, WorkspaceKey, MAX_WORKSPACES};
use crate::ui::background_io::SerialLane;
use crate::ui::navigation::NavigationContext;

/// Resolve curation only after OBS reports its actual collection. The registry
/// lane and import gate keep migration, imports, and accepted edits in order.
/// `toast` announces workspaces evicted to make room for a new one.
pub(crate) fn select_registry_scope(
    nav: &NavigationContext,
    toast: Option<adw::ToastOverlay>,
    refresh: impl Fn() + 'static,
) {
    let Some(target) = nav.state.borrow().desired_registry_target() else {
        return;
    };
    let ticket = nav.state.borrow_mut().select_registry(target.clone());
    if ticket.is_none() && registry_edit_allowed() {
        reconcile_registry(nav);
    }
    refresh();
    let Some(generation) = ticket else {
        return;
    };
    let nav = nav.scoped();
    let failed_nav = nav.clone();
    let failed_target = target.clone();
    let refresh: Rc<dyn Fn()> = Rc::new(refresh);
    let failed_refresh = refresh.clone();
    if !after_registry_import(move || {
        if !nav.is_current()
            || !nav
                .state
                .borrow()
                .accepts_registry_load(generation, &target)
        {
            return;
        }
        let guard = begin_registry_import().expect("deferred registry jobs run outside the gate");
        let legacy = {
            let state = nav.state.borrow();
            let original_endpoint = state
                .config
                .legacy_registry_endpoint
                .clone()
                .unwrap_or_else(|| format!("{}:{}", state.config.obs.host, state.config.obs.port));
            matches!(&target, RegistryTarget::Workspace { key, .. }
                if key.connection == "default" && key.endpoint == original_endpoint)
            .then(crate::storage::xdg::registry_path)
        };
        let destination = target.clone();
        crate::ui::background_io::run_serialized(
            SerialLane::Registry,
            move || destination.load_or_create(legacy.as_deref()),
            move |result| {
                let result = result.map(|loaded| {
                    announce_eviction(toast.as_ref(), &loaded.evicted);
                    loaded.registry
                });
                if nav.is_current() {
                    nav.state
                        .borrow_mut()
                        .finish_registry_load(generation, &target, result);
                }
                if nav.is_current() {
                    reconcile_registry(&nav);
                }
                drop(guard);
                refresh();
            },
        );
    }) {
        failed_nav.state.borrow_mut().finish_registry_load(
            generation,
            &failed_target,
            Err(std::io::Error::other("too many pending workspace changes")),
        );
        failed_refresh();
    }
}

/// The eviction is already on disk, so announce it even when this selection
/// is stale. Name what went: its curation is gone.
fn announce_eviction(toast: Option<&adw::ToastOverlay>, evicted: &[WorkspaceKey]) {
    let Some(message) = eviction_notice(evicted) else {
        return;
    };
    tracing::info!(
        count = evicted.len(),
        "evicted least recently used show workspaces"
    );
    if let Some(toast) = toast {
        toast.add_toast(crate::ui::markup::plain_toast(&message));
    }
}

fn eviction_notice(evicted: &[WorkspaceKey]) -> Option<String> {
    use crate::infra::i18n::LANGUAGE_LOADER;
    use i18n_embed_fl::fl;
    (!evicted.is_empty()).then(|| {
        let collections: Vec<&str> = evicted.iter().map(|key| key.collection.as_str()).collect();
        fl!(
            LANGUAGE_LOADER,
            "workspace-evicted",
            limit = MAX_WORKSPACES.to_string(),
            collections = collections.join(", ")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eviction_notice_names_every_evicted_collection_and_stays_silent_otherwise() {
        let key = |collection: &str| WorkspaceKey {
            connection: "default".into(),
            endpoint: "obs:4455".into(),
            collection: collection.into(),
        };

        let notice = eviction_notice(&[key("Podcast"), key("Gaming")]).unwrap();

        assert!(notice.contains("Podcast"), "{notice}");
        assert!(notice.contains("Gaming"), "{notice}");
        assert!(notice.contains(&MAX_WORKSPACES.to_string()), "{notice}");
        assert_eq!(eviction_notice(&[]), None);
    }
}

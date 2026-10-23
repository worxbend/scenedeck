//! Visible-only scene still scheduler and bounded GTK texture cache.
use crate::{
    controller::{command::AppCommand, state::ObsStatus, thumbnails::ThumbnailRequest},
    domain::workflow::ScopeBinding,
    ui::{navigation::NavigationContext, pages::live::LivePageHandle},
};
use gtk4::{gdk, prelude::*};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    rc::Rc,
    time::{Duration, Instant},
};
const MAX_IMAGES: usize = 24;
const INTERVAL: Duration = Duration::from_millis(500);
const REFRESH: Duration = Duration::from_secs(3);
#[derive(Default)]
pub(crate) struct PreviewState {
    scope: Option<(u64, ScopeBinding)>,
    generation: u64,
    pending: Option<(ThumbnailRequest, Instant)>,
    last_request: Option<Instant>,
    images: HashMap<String, (gdk::Texture, Instant)>,
    attempts: HashMap<String, Instant>,
    failures: HashSet<String>,
}

pub(crate) fn install(handle: Rc<LivePageHandle>, nav: NavigationContext) {
    let weak = Rc::downgrade(&handle);
    let last_setting = Cell::new(None::<bool>);
    glib::timeout_add_local(Duration::from_millis(100), move || {
        let Some(handle) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let images = nav.state.borrow().config.live.scene_previews;
        if last_setting.replace(Some(images)) != Some(images) {
            let inventory = nav.state.borrow().scene_inventory.clone();
            crate::ui::pages::live::rebuild_scene_cards(&handle, &inventory, &nav);
        }
        for card in handle.scene_cards.borrow().iter() {
            card.fit_image_height(handle.scene_scroll.height());
        }
        tick(&handle, &nav);
        glib::ControlFlow::Continue
    });
}
pub(crate) fn clear(handle: &LivePageHandle, nav: &NavigationContext) {
    let mut cache = handle.previews.borrow_mut();
    if cache.scope.is_some() || cache.pending.is_some() || !cache.images.is_empty() {
        cache.generation = cache.generation.wrapping_add(1);
        cache.scope = None;
        cache.pending = None;
        cache.images.clear();
        cache.attempts.clear();
        cache.failures.clear();
        cache.last_request = None;
        nav.dispatch(AppCommand::CancelThumbnails);
        for card in handle.scene_cards.borrow().iter() {
            card.picture.set_paintable(gdk::Paintable::NONE);
        }
    }
}
fn visible_window(root: &gtk4::Stack) -> bool {
    root.is_mapped()
        && root
            .root()
            .and_then(|root| root.downcast::<gtk4::Window>().ok())
            .and_then(|window| window.surface())
            .and_then(|surface| surface.downcast::<gdk::Toplevel>().ok())
            .is_none_or(|surface| !surface.state().contains(gdk::ToplevelState::MINIMIZED))
}

fn tick(handle: &LivePageHandle, nav: &NavigationContext) {
    let (enabled, scope, epoch, supported) = {
        let state = nav.state.borrow();
        (
            state.config.live.scene_previews
                && visible_window(&handle.root)
                && handle.root.visible_child_name().as_deref() == Some("live")
                && matches!(state.obs_status, ObsStatus::Connected { .. }),
            state.workflow_scope(),
            state.workspace.epoch,
            state
                .available_requests
                .iter()
                .any(|r| r == "GetSourceScreenshot"),
        )
    };
    if !enabled || !supported || scope.collection.is_empty() || scope.endpoint.is_empty() {
        clear(handle, nav);
        return;
    }
    if handle.previews.borrow().scope.as_ref() != Some(&(epoch, scope.clone())) {
        clear(handle, nav);
        handle.previews.borrow_mut().scope = Some((epoch, scope.clone()));
    }
    let mut cache = handle.previews.borrow_mut();
    if cache
        .pending
        .as_ref()
        .is_some_and(|(_, at)| at.elapsed() < Duration::from_secs(3))
    {
        return;
    }
    if let Some((request, _)) = cache.pending.take() {
        cache.failures.insert(request.scene);
        nav.dispatch(AppCommand::CancelThumbnails);
    }
    if cache.last_request.is_some_and(|at| at.elapsed() < INTERVAL) {
        return;
    }
    let cards = handle.scene_cards.borrow();
    cache
        .attempts
        .retain(|scene, _| cards.iter().any(|card| &card.scene_id == scene));
    cache
        .images
        .retain(|scene, _| cards.iter().any(|card| &card.scene_id == scene));
    cache
        .failures
        .retain(|scene| cards.iter().any(|card| &card.scene_id == scene));
    let visible: Vec<_> = cards
        .iter()
        .filter(|card| {
            card.root
                .compute_bounds(&handle.scene_scroll)
                .is_some_and(|r| {
                    r.y() + r.height() > 0.0 && r.y() < (handle.scene_scroll.height() as f32)
                })
        })
        .collect();
    // Oldest first is fair; Program/Preview win equal-age initial requests.
    let next = visible
        .into_iter()
        .filter(|card| {
            cache.attempts.get(&card.scene_id).is_none_or(|at| {
                at.elapsed() >= retry_interval(cache.failures.contains(&card.scene_id))
            })
        })
        .min_by_key(|card| {
            (
                cache.attempts.get(&card.scene_id).copied(),
                !(card.root.has_css_class("scene-card-active")
                    || card.root.has_css_class("scene-card-preview")),
            )
        });
    if let Some(card) = next {
        let request = ThumbnailRequest {
            epoch,
            generation: cache.generation,
            scope,
            scene: card.scene_id.clone(),
        };
        let now = Instant::now();
        cache.attempts.insert(card.scene_id.clone(), now);
        cache.last_request = Some(now);
        cache.pending = Some((request.clone(), now));
        nav.dispatch(AppCommand::CaptureThumbnail(request));
    }
}

fn retry_interval(failed: bool) -> Duration {
    if failed {
        Duration::from_secs(15)
    } else {
        REFRESH
    }
}

fn bounded_png(data: &str) -> Option<Vec<u8>> {
    let encoded = data.strip_prefix("data:image/png;base64,")?;
    if encoded.len() > 512 * 1024 {
        return None;
    }
    let bytes = glib::base64_decode(encoded);
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    (width > 0 && height > 0 && width <= 384 && height <= 216).then_some(bytes)
}

pub(crate) fn apply(
    handle: &LivePageHandle,
    nav: &NavigationContext,
    request: ThumbnailRequest,
    result: Result<String, String>,
) {
    let state = nav.state.borrow();
    let mut cache = handle.previews.borrow_mut();
    if !state.config.live.scene_previews
        || !visible_window(&handle.root)
        || !matches!(state.obs_status, ObsStatus::Connected { .. })
        || request.epoch != state.workspace.epoch
        || request.scope != state.workflow_scope()
        || cache.pending.as_ref().map(|(pending, _)| pending) != Some(&request)
    {
        return;
    }
    cache.pending = None;
    let Some(card_index) = handle
        .scene_cards
        .borrow()
        .iter()
        .position(|c| c.scene_id == request.scene)
    else {
        return;
    };
    let texture = result
        .ok()
        .and_then(|data| bounded_png(&data))
        .and_then(|bytes| gdk::Texture::from_bytes(&glib::Bytes::from_owned(bytes)).ok());
    if let Some(texture) = texture {
        cache.failures.remove(&request.scene);
        if !cache.images.contains_key(&request.scene) && cache.images.len() >= MAX_IMAGES {
            if let Some(oldest) = cache
                .images
                .iter()
                .min_by_key(|(_, (_, at))| *at)
                .map(|(scene, _)| scene.clone())
            {
                cache.images.remove(&oldest);
                for card in handle
                    .scene_cards
                    .borrow()
                    .iter()
                    .filter(|c| c.scene_id == oldest)
                {
                    card.picture.set_paintable(gdk::Paintable::NONE);
                }
            }
        }
        handle.scene_cards.borrow()[card_index]
            .picture
            .set_paintable(Some(&texture));
        cache
            .images
            .insert(request.scene, (texture, Instant::now()));
    } else {
        // A failed still does not invalidate OBS command freshness.
        cache.failures.insert(request.scene.clone());
        cache.images.remove(&request.scene);
        handle.scene_cards.borrow()[card_index]
            .picture
            .set_paintable(gdk::Paintable::NONE);
    }
}

/// Reuse current-session images when a deliberate registry/preference rebuild occurs.
pub(crate) fn restore(handle: &LivePageHandle, nav: &NavigationContext) {
    let state = nav.state.borrow();
    let cache = handle.previews.borrow();
    if !state.config.live.scene_previews
        || cache.scope.as_ref() != Some(&(state.workspace.epoch, state.workflow_scope()))
    {
        return;
    }
    for card in handle.scene_cards.borrow().iter() {
        if let Some((texture, _)) = cache.images.get(&card.scene_id) {
            card.picture.set_paintable(Some(texture));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_errors_back_off_without_slowing_healthy_cards() {
        assert_eq!(retry_interval(false), Duration::from_secs(3));
        assert_eq!(retry_interval(true), Duration::from_secs(15));
    }
    #[test]
    fn png_header_is_checked_before_allocating_a_texture() {
        assert!(bounded_png("data:image/png;base64,aGVsbG8=").is_none());
        assert!(bounded_png("data:image/jpeg;base64,aGVsbG8=").is_none());
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        bytes.extend(384u32.to_be_bytes());
        bytes.extend(216u32.to_be_bytes());
        let valid = format!("data:image/png;base64,{}", glib::base64_encode(&bytes));
        assert!(bounded_png(&valid).is_some());
        bytes[16..20].copy_from_slice(&385u32.to_be_bytes());
        assert!(bounded_png(&format!(
            "data:image/png;base64,{}",
            glib::base64_encode(&bytes)
        ))
        .is_none());
    }
}

#[cfg(test)]
#[path = "preview_scheduler_tests.rs"]
mod scheduler_tests;

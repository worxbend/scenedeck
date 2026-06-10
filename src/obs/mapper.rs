//! Maps obws response types to our domain types.
//!
//! All OBS-specific types stop here.  Nothing above this module imports from `obws`.

use obws::responses::general::Version;
use obws::responses::scenes::Scenes;

use crate::controller::event::ConnectionInfo;
use crate::domain::scene::{Scene, SceneInventory};

pub fn map_version(v: &Version) -> ConnectionInfo {
    ConnectionInfo {
        obs_version: v.obs_studio_version.to_string(),
        websocket_version: v.obs_web_socket_version.to_string(),
    }
}

/// Convert the `GetSceneList` response into a domain `SceneInventory`.
///
/// OBS returns scenes in reverse order (highest index = first in the UI switcher),
/// so we reverse the list here to match the user's scene switcher order.
pub fn map_scenes(resp: Scenes) -> SceneInventory {
    let current_id = resp.current_program_scene.map(|s| s.name.clone());

    let scenes = resp
        .scenes
        .iter()
        .rev()
        .map(|s| Scene {
            id: s.id.name.clone(),
            name: s.id.name.clone(),
            role: None, // role comes from the local registry, never from OBS
        })
        .collect();

    SceneInventory { scenes, current_id }
}

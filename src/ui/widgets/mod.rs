//! Reusable GTK widgets shared by multiple SceneDeck pages.
//!
//! Widgets keep their own presentation helpers and tests close to the GTK
//! construction code so pages can compose them without duplicating styling
//! state.

pub(crate) mod audio_card;
pub(crate) mod scene_card;

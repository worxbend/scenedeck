//! Icons a user can pin to a scene or an audio source.
//!
//! Nerd Fonts ships more than ten thousand glyphs, which is far too many to
//! choose from in a popover. This module is the curated shortlist: a stable key
//! that goes in `registry.json`, the icon name GTK resolves, and a label for
//! the picker. Keys are ours rather than Nerd Fonts', so the underlying glyph
//! can be swapped without rewriting anyone's registry.

use i18n_embed_fl::fl;

use crate::infra::i18n::LANGUAGE_LOADER;

/// One entry in the icon picker.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct IconChoice {
    /// Stable key persisted in `registry.json`.
    pub key: &'static str,
    /// GTK icon name, resolved from the Nerd Fonts resource bundle.
    pub icon_name: &'static str,
}

impl IconChoice {
    /// User-facing name shown in the picker.
    pub fn label(self) -> String {
        icon_label(self.key)
    }
}

/// Icons offered for scenes and audio sources, in picker order.
///
/// One shared catalogue rather than two: a scene called "Podcast" and a source
/// called "Podcast" want the same glyph, and a single list is one list to
/// translate.
pub const ICON_CATALOGUE: [IconChoice; 30] = [
    IconChoice {
        key: "camera",
        icon_name: "nf-fa-video-camera-symbolic",
    },
    IconChoice {
        key: "desktop",
        icon_name: "nf-fa-desktop-symbolic",
    },
    IconChoice {
        key: "game",
        icon_name: "nf-fa-gamepad-symbolic",
    },
    IconChoice {
        key: "film",
        icon_name: "nf-fa-film-symbolic",
    },
    IconChoice {
        key: "images",
        icon_name: "nf-fa-images-symbolic",
    },
    IconChoice {
        key: "television",
        icon_name: "nf-md-television-classic-symbolic",
    },
    IconChoice {
        key: "browser",
        icon_name: "nf-fa-globe-symbolic",
    },
    IconChoice {
        key: "terminal",
        icon_name: "nf-cod-terminal-symbolic",
    },
    IconChoice {
        key: "code",
        icon_name: "nf-cod-code-symbolic",
    },
    IconChoice {
        key: "chat",
        icon_name: "nf-fa-message-symbolic",
    },
    IconChoice {
        key: "guests",
        icon_name: "nf-fa-users-symbolic",
    },
    IconChoice {
        key: "star",
        icon_name: "nf-fa-star-symbolic",
    },
    IconChoice {
        key: "alert",
        icon_name: "nf-cod-bell-symbolic",
    },
    IconChoice {
        key: "break",
        icon_name: "nf-cod-coffee-symbolic",
    },
    IconChoice {
        key: "countdown",
        icon_name: "nf-fa-clock-o-symbolic",
    },
    IconChoice {
        key: "start",
        icon_name: "nf-cod-play-symbolic",
    },
    IconChoice {
        key: "pause",
        icon_name: "nf-fa-pause-symbolic",
    },
    IconChoice {
        key: "stop",
        icon_name: "nf-cod-stop-circle-symbolic",
    },
    IconChoice {
        key: "settings",
        icon_name: "nf-cod-gear-symbolic",
    },
    IconChoice {
        key: "layers",
        icon_name: "nf-cod-layers-symbolic",
    },
    IconChoice {
        key: "microphone",
        icon_name: "nf-fa-microphone-symbolic",
    },
    IconChoice {
        key: "headset",
        icon_name: "nf-md-microphone-variant-symbolic",
    },
    IconChoice {
        key: "headphones",
        icon_name: "nf-fa-headphones-symbolic",
    },
    IconChoice {
        key: "speaker",
        icon_name: "nf-md-speaker-symbolic",
    },
    IconChoice {
        key: "volume",
        icon_name: "nf-md-volume-high-symbolic",
    },
    IconChoice {
        key: "music",
        icon_name: "nf-fa-music-symbolic",
    },
    IconChoice {
        key: "instrument",
        icon_name: "nf-md-guitar-acoustic-symbolic",
    },
    IconChoice {
        key: "radio",
        icon_name: "nf-md-radio-symbolic",
    },
    IconChoice {
        key: "call",
        icon_name: "nf-fa-phone-symbolic",
    },
    IconChoice {
        key: "waveform",
        icon_name: "nf-md-waveform-symbolic",
    },
];

/// Look up a persisted icon key.
///
/// Unknown keys return `None` rather than a placeholder: a registry written by
/// a newer build, or hand-edited, should lose the icon and keep everything else.
pub fn icon_choice(key: &str) -> Option<IconChoice> {
    ICON_CATALOGUE.iter().copied().find(|icon| icon.key == key)
}

/// GTK icon name for a persisted key, if the key is known.
pub fn icon_name(key: &str) -> Option<&'static str> {
    icon_choice(key).map(|icon| icon.icon_name)
}

/// User-facing name for one catalogue key.
fn icon_label(key: &str) -> String {
    match key {
        "camera" => fl!(LANGUAGE_LOADER, "icon-camera"),
        "desktop" => fl!(LANGUAGE_LOADER, "icon-desktop"),
        "game" => fl!(LANGUAGE_LOADER, "icon-game"),
        "film" => fl!(LANGUAGE_LOADER, "icon-film"),
        "images" => fl!(LANGUAGE_LOADER, "icon-images"),
        "television" => fl!(LANGUAGE_LOADER, "icon-television"),
        "browser" => fl!(LANGUAGE_LOADER, "icon-browser"),
        "terminal" => fl!(LANGUAGE_LOADER, "icon-terminal"),
        "code" => fl!(LANGUAGE_LOADER, "icon-code"),
        "chat" => fl!(LANGUAGE_LOADER, "icon-chat"),
        "guests" => fl!(LANGUAGE_LOADER, "icon-guests"),
        "star" => fl!(LANGUAGE_LOADER, "icon-star"),
        "alert" => fl!(LANGUAGE_LOADER, "icon-alert"),
        "break" => fl!(LANGUAGE_LOADER, "icon-break"),
        "countdown" => fl!(LANGUAGE_LOADER, "icon-countdown"),
        "start" => fl!(LANGUAGE_LOADER, "icon-start"),
        "pause" => fl!(LANGUAGE_LOADER, "icon-pause"),
        "stop" => fl!(LANGUAGE_LOADER, "icon-stop"),
        "settings" => fl!(LANGUAGE_LOADER, "icon-settings"),
        "layers" => fl!(LANGUAGE_LOADER, "icon-layers"),
        "microphone" => fl!(LANGUAGE_LOADER, "icon-microphone"),
        "headset" => fl!(LANGUAGE_LOADER, "icon-headset"),
        "headphones" => fl!(LANGUAGE_LOADER, "icon-headphones"),
        "speaker" => fl!(LANGUAGE_LOADER, "icon-speaker"),
        "volume" => fl!(LANGUAGE_LOADER, "icon-volume"),
        "music" => fl!(LANGUAGE_LOADER, "icon-music"),
        "instrument" => fl!(LANGUAGE_LOADER, "icon-instrument"),
        "radio" => fl!(LANGUAGE_LOADER, "icon-radio"),
        "call" => fl!(LANGUAGE_LOADER, "icon-call"),
        "waveform" => fl!(LANGUAGE_LOADER, "icon-waveform"),
        _ => fl!(LANGUAGE_LOADER, "icon-none"),
    }
}

/// Label for the picker entry that clears the icon.
pub fn no_icon_label() -> String {
    fl!(LANGUAGE_LOADER, "icon-none")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalogue_keys_are_unique() {
        let keys: HashSet<&str> = ICON_CATALOGUE.iter().map(|icon| icon.key).collect();

        assert_eq!(keys.len(), ICON_CATALOGUE.len());
    }

    #[test]
    fn catalogue_icon_names_are_unique_and_symbolic() {
        let names: HashSet<&str> = ICON_CATALOGUE.iter().map(|icon| icon.icon_name).collect();

        assert_eq!(names.len(), ICON_CATALOGUE.len());
        assert!(ICON_CATALOGUE.iter().all(|icon| {
            icon.icon_name.starts_with("nf-") && icon.icon_name.ends_with("-symbolic")
        }));
    }

    #[test]
    fn every_key_has_its_own_label() {
        let labels: HashSet<String> = ICON_CATALOGUE.iter().map(|icon| icon.label()).collect();

        assert_eq!(
            labels.len(),
            ICON_CATALOGUE.len(),
            "two icons share a label, so the picker would be ambiguous"
        );
        assert!(
            !labels.contains(&no_icon_label()),
            "an icon must not be labelled as no icon"
        );
    }

    #[test]
    fn known_keys_resolve_and_unknown_ones_do_not() {
        assert_eq!(icon_name("camera"), Some("nf-fa-video-camera-symbolic"));
        assert_eq!(
            icon_choice("microphone").map(|icon| icon.key),
            Some("microphone")
        );
        assert_eq!(icon_name("not-an-icon"), None);
        assert_eq!(icon_choice(""), None);
    }
}

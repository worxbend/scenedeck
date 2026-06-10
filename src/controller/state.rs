//! Runtime application state — the single source of truth held inside an
//! `Rc<RefCell<AppState>>` on the GTK main thread.

use crate::domain::audio::AudioInput;
use crate::domain::diagnostic::Diagnostic;
use crate::domain::graph::SceneGraph;
use crate::domain::obs::ObsNamedList;
use crate::domain::output::OutputStatus;
use crate::domain::scene::SceneInventory;

// ── Navigation ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Live,
    Graph,
    Inventory,
    Doctor,
    Settings,
}

impl Page {
    /// Stable `gtk4::Stack` child name.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Graph => "graph",
            Self::Inventory => "inventory",
            Self::Doctor => "doctor",
            Self::Settings => "settings",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Live => "Live",
            Self::Graph => "Graph",
            Self::Inventory => "Inventory",
            Self::Doctor => "Doctor",
            Self::Settings => "Settings",
        }
    }

    /// Symbolic icon name for the sidebar row.
    pub const fn icon_name(self) -> &'static str {
        match self {
            Self::Live => "media-record-symbolic",
            Self::Graph => "view-grid-symbolic",
            Self::Inventory => "view-list-symbolic",
            Self::Doctor => "emblem-default-symbolic",
            Self::Settings => "preferences-system-symbolic",
        }
    }
}

// ── Theme ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

impl std::str::FromStr for ThemeMode {
    /// Parsing never fails: unknown values fall back to `System`.
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        })
    }
}

// ── OBS connection ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ObsStatus {
    Disconnected,
    Connecting,
    Connected { obs_version: String },
    Error(String),
}

impl ObsStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Disconnected => "Disconnected",
            Self::Connecting => "Connecting…",
            Self::Connected { .. } => "Connected",
            Self::Error(_) => "Error",
        }
    }

    pub fn css_class(&self) -> &str {
        match self {
            Self::Disconnected => "obs-disconnected",
            Self::Connecting => "obs-connecting",
            Self::Connected { .. } => "obs-connected",
            Self::Error(_) => "obs-error",
        }
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AppState {
    pub current_page: Page,
    pub theme_mode: ThemeMode,
    pub obs_status: ObsStatus,
    pub scene_inventory: SceneInventory,
    pub scene_graph: SceneGraph,
    pub profiles: ObsNamedList,
    pub scene_collections: ObsNamedList,
    pub stream_status: OutputStatus,
    pub record_status: OutputStatus,
    pub audio_inputs: Vec<AudioInput>,
    pub diagnostics: Vec<Diagnostic>,
    /// Human-readable config-load notice shown once on the Settings page.
    pub startup_notice: Option<String>,
}

impl AppState {
    pub fn new(theme_mode: ThemeMode, startup_notice: Option<String>) -> Self {
        Self {
            current_page: Page::Live,
            theme_mode,
            obs_status: ObsStatus::Disconnected,
            scene_inventory: SceneInventory::default(),
            scene_graph: SceneGraph::default(),
            profiles: ObsNamedList::default(),
            scene_collections: ObsNamedList::default(),
            stream_status: OutputStatus::default(),
            record_status: OutputStatus::default(),
            audio_inputs: Vec::new(),
            diagnostics: Vec::new(),
            startup_notice,
        }
    }

    pub fn set_page(&mut self, page: Page) {
        self.current_page = page;
    }
    pub fn set_theme_mode(&mut self, mode: ThemeMode) {
        self.theme_mode = mode;
    }
    pub fn set_obs_status(&mut self, status: ObsStatus) {
        self.obs_status = status;
    }
}

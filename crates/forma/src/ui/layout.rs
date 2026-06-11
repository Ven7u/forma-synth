use serde::{Deserialize, Serialize};

// ── Mode / tab enums ─────────────────────────────────────────────────────────

/// Top-level application mode — same patch data, three UI surfaces.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AppMode {
    #[default]
    Studio,
    DrumMachine,
    Live,
}

/// Active tab in the Studio central editing area.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StudioTab {
    #[default]
    Voice,
    Shape,
    Fx,
    Sequencer,
    Settings,
}

impl StudioTab {
    #[allow(dead_code)]
    pub const ALL: &'static [StudioTab] = &[
        StudioTab::Voice,
        StudioTab::Shape,
        StudioTab::Fx,
        StudioTab::Sequencer,
        StudioTab::Settings,
    ];

    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            StudioTab::Voice => "VOICE",
            StudioTab::Shape => "SHAPE",
            StudioTab::Fx => "FX",
            StudioTab::Sequencer => "SEQ",
            StudioTab::Settings => "SETTINGS",
        }
    }
}

// ── Persisted layout state ───────────────────────────────────────────────────

/// Persisted layout state — saved/loaded from disk.
#[derive(Clone, Serialize, Deserialize)]
pub struct LayoutState {
    pub theme_name: String,
    pub panels: PanelVisibilityState,
    /// Active mode (Studio/Live). Defaults to Studio for existing saved files.
    #[serde(default)]
    pub app_mode: AppMode,
    /// Active studio tab. Defaults to Voice for existing saved files.
    #[serde(default)]
    pub studio_tab: StudioTab,
    /// Starred patch names.
    #[serde(default)]
    pub patch_favorites: Vec<String>,
    /// Recently loaded patch names, newest first (capped at 12).
    #[serde(default)]
    pub patch_recent: Vec<String>,
    /// Last connected MIDI port name — used to auto-reconnect on startup.
    #[serde(default)]
    pub midi_port_name: Option<String>,
    /// Last window inner size, in logical px. None on first launch.
    #[serde(default)]
    pub window_size: Option<[f32; 2]>,
    /// Last window outer position. None on first launch.
    #[serde(default)]
    pub window_pos: Option<[f32; 2]>,
    /// Global UI zoom factor. 0.7..=1.4. Default 0.9.
    #[serde(default = "default_zoom")]
    pub zoom_factor: f32,
}

fn default_zoom() -> f32 {
    0.9
}

/// Serializable mirror of PanelVisibility.
#[derive(Clone, Serialize, Deserialize)]
pub struct PanelVisibilityState {
    pub oscillators: bool,
    pub modulation: bool,
    pub keyboard: bool,
    pub sequencer: bool,
    pub arp_walker: bool,
    pub fx_chain: bool,
    pub scope: bool,
    pub midi: bool,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            theme_name: "Midnight".into(),
            app_mode: AppMode::Studio,
            studio_tab: StudioTab::Voice,
            patch_favorites: Vec::new(),
            patch_recent: Vec::new(),
            midi_port_name: None,
            window_size: None,
            window_pos: None,
            zoom_factor: default_zoom(),
            panels: PanelVisibilityState {
                oscillators: true,
                modulation: true,
                keyboard: true,
                sequencer: true,
                arp_walker: true,
                fx_chain: true,
                scope: true,
                midi: true,
            },
        }
    }
}

// ── Layout presets (legacy, kept for compatibility) ──────────────────────────

pub struct LayoutPreset {
    pub name: &'static str,
    pub description: &'static str,
    pub panels: PanelVisibilityState,
}

pub fn preset_sound_design() -> LayoutPreset {
    LayoutPreset {
        name: "Sound Design",
        description: "All panels visible for patch creation.",
        panels: PanelVisibilityState {
            oscillators: true,
            modulation: true,
            keyboard: true,
            sequencer: true,
            arp_walker: true,
            fx_chain: true,
            scope: true,
            midi: true,
        },
    }
}

pub fn preset_performance() -> LayoutPreset {
    LayoutPreset {
        name: "Performance",
        description: "Keyboard + FX + Scope for live playing.",
        panels: PanelVisibilityState {
            oscillators: false,
            modulation: false,
            keyboard: true,
            sequencer: false,
            arp_walker: false,
            fx_chain: true,
            scope: true,
            midi: false,
        },
    }
}

pub fn preset_sequencer() -> LayoutPreset {
    LayoutPreset {
        name: "Sequencer",
        description: "Sequencer + Arp/Walker + Scope for pattern work.",
        panels: PanelVisibilityState {
            oscillators: false,
            modulation: false,
            keyboard: true,
            sequencer: true,
            arp_walker: true,
            fx_chain: false,
            scope: true,
            midi: false,
        },
    }
}

pub fn builtin_presets() -> Vec<LayoutPreset> {
    vec![
        preset_sound_design(),
        preset_performance(),
        preset_sequencer(),
    ]
}

// ── Persistence ──────────────────────────────────────────────────────────────

pub fn save_layout(state: &LayoutState) {
    crate::layout_store::save_current(state);
}

pub fn load_layout() -> LayoutState {
    crate::layout_store::load_current()
}

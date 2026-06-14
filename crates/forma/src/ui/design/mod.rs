//! Forma design system — Layer 1 (tokens) skeleton.
//!
//! See `docs/design-system/` for the full spec. This module will grow to host
//! the SynthUi trait, layout helpers, and the migrated knob/button/fader
//! components. For now it exposes the two enums that the rest of the system
//! is parameterized over.

#![allow(dead_code)] // Phase 0/2 establish the API; callers land in Phase 5+.

pub mod chip;
pub mod fader;
pub mod gallery;
pub mod knob;
pub mod layout;
pub mod level_meter;
pub mod mini_bar;
pub mod section;
pub mod slider;
pub mod step_pad;
pub mod toggle;

#[allow(unused_imports)] // Re-exported for Phase 5+ panel migrations.
pub use layout::SynthUi;

use egui::Vec2;

/// Musical-importance tier. Drives knob arc color, stroke width, and the
/// arc-color token selection on every interactive component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Performance controls — touched while music plays.
    Primary,
    /// Sound-design controls — adjusted between notes.
    Secondary,
    /// Configuration — set once per session/patch.
    Tertiary,
}

/// Knob size variant. Maps to allocated rect, knob radius, and arc stroke
/// width per `docs/design-system/02-tokens.md` §5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnobSize {
    /// 64 × 88 px allocated rect, 24 px radius. Tier 1.
    Large,
    /// 44 × 64 px allocated rect, 16 px radius. Tier 2.
    Standard,
    /// 32 × 48 px allocated rect, 11 px radius. Tier 3.
    Small,
}

impl KnobSize {
    /// Total rect the knob+label occupies in layout.
    pub fn rect(self) -> Vec2 {
        match self {
            KnobSize::Large => Vec2::new(64.0, 88.0),
            KnobSize::Standard => Vec2::new(44.0, 64.0),
            KnobSize::Small => Vec2::new(32.0, 48.0),
        }
    }

    /// Radius of the knob circle itself (not including label).
    pub fn radius(self) -> f32 {
        match self {
            KnobSize::Large => 24.0,
            KnobSize::Standard => 16.0,
            KnobSize::Small => 11.0,
        }
    }

    /// Arc stroke width. Tier 1 knobs get more visual weight.
    pub fn arc_stroke(self) -> f32 {
        match self {
            KnobSize::Large => 2.5,
            KnobSize::Standard => 2.0,
            KnobSize::Small => 1.5,
        }
    }
}

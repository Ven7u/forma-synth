use egui::Color32;
use serde::{Deserialize, Serialize};

/// A complete visual theme for Forma.
///
/// Organised in three layers:
///   1. Semantic backgrounds / borders / text  — used by SynthFrame and apply_to_egui
///   2. Domain-specific colors                 — scope glow, per-FX chips, sequencer cells
///   3. Geometry tokens                        — spacing scale, rounding, stroke widths
#[derive(Clone, Serialize, Deserialize)]
pub struct SynthTheme {
    pub name: String,

    // ── Semantic backgrounds (darkest → lightest) ──────────────────────────
    /// App window fill — the darkest background layer.
    pub bg_app: [u8; 3],
    /// Panel / card surface — one step above the app background.
    pub bg_surface: [u8; 3],
    /// Inset / input background — sits inside a surface (equal or darker).
    pub bg_sunken: [u8; 3],
    /// Global bar and transport strips — slightly distinct from the main app bg.
    pub bg_bar: [u8; 3],

    // ── Borders ────────────────────────────────────────────────────────────
    pub border: [u8; 3],
    pub border_focus: [u8; 3],

    // ── Text (semantic) ────────────────────────────────────────────────────
    pub text_primary: [u8; 3],
    pub text_secondary: [u8; 3],
    pub text_disabled: [u8; 3],
    /// Text color for use on top of an accent fill — must contrast clearly
    /// against the bright accent. Typically a very dark color.
    #[serde(default)]
    pub text_on_accent: [u8; 3],

    // ── Accent ─────────────────────────────────────────────────────────────
    pub accent: [u8; 3],
    pub accent_dim: [u8; 3],

    // ── Knob arc colors by tier ────────────────────────────────────────────
    /// Tier 1 (performance) — full accent.
    #[serde(default)]
    pub knob_tier1_arc: [u8; 3],
    /// Tier 2 (sound design) — dimmed accent.
    #[serde(default)]
    pub knob_tier2_arc: [u8; 3],
    /// Tier 3 (config) — nearly neutral.
    #[serde(default)]
    pub knob_tier3_arc: [u8; 3],

    // ── Special accents ─────────────────────────────────────────────────────
    pub accent_hard_sync: [u8; 3],
    pub accent_fm: [u8; 3],
    pub accent_ring: [u8; 3],
    pub accent_hold: [u8; 3],
    pub accent_walker: [u8; 3],
    pub accent_limiter: [u8; 3],

    // ── FX per-effect ───────────────────────────────────────────────────────
    pub fx_overdrive: [u8; 3],
    pub fx_distortion: [u8; 3],
    pub fx_chorus: [u8; 3],
    pub fx_delay: [u8; 3],
    pub fx_reverb: [u8; 3],
    pub fx_shimmer: [u8; 3],
    pub fx_crystallizer: [u8; 3],

    // ── Sequencer ───────────────────────────────────────────────────────────
    pub seq_step_on: [u8; 3],
    pub seq_step_off: [u8; 3],
    pub seq_current: [u8; 3],
    pub seq_note_bar_on: [u8; 3],
    pub seq_note_bar_off: [u8; 3],
    pub seq_chord_major: [u8; 3],
    pub seq_chord_minor: [u8; 3],
    pub seq_chord_dim: [u8; 3],
    pub seq_kb_major: [u8; 3],
    pub seq_kb_minor: [u8; 3],
    pub seq_kb_dim: [u8; 3],
    /// Velocity-bar fill color in the note sequencer step grid.
    #[serde(default)]
    pub seq_velocity_bar: [u8; 3],
    /// Probability-bar low zone (< 50%).
    #[serde(default)]
    pub seq_prob_low: [u8; 3],
    /// Probability-bar mid zone (50% – 99%).
    #[serde(default)]
    pub seq_prob_mid: [u8; 3],
    /// Probability-bar high zone (100%).
    #[serde(default)]
    pub seq_prob_high: [u8; 3],
    /// Step-entry record cursor border.
    #[serde(default)]
    pub seq_rec_cursor: [u8; 3],
    /// Chord-sequencer octave-offset bar fill.
    #[serde(default)]
    pub seq_octave_bar: [u8; 3],

    // ── Keyboard ────────────────────────────────────────────────────────────
    pub key_white_pressed: [u8; 3],
    pub key_black_pressed: [u8; 3],
    /// Default (unlit) white key fill — natural ivory.
    #[serde(default = "default_key_white_default")]
    pub key_white_default: [u8; 3],
    /// Default (unlit) black key fill — natural ebony.
    #[serde(default = "default_key_black_default")]
    pub key_black_default: [u8; 3],
    /// White key within the computer-keyboard octave range (subtle tint).
    #[serde(default = "default_key_white_range")]
    pub key_white_range: [u8; 3],
    /// Black key within the computer-keyboard octave range (subtle tint).
    #[serde(default = "default_key_black_range")]
    pub key_black_range: [u8; 3],
    /// Root note highlight on a white key.
    #[serde(default = "default_key_scale_root")]
    pub key_scale_root: [u8; 3],
    /// Root note highlight on a black key.
    #[serde(default = "default_key_scale_root_dark")]
    pub key_scale_root_dark: [u8; 3],
    /// In-scale highlight on a white key.
    #[serde(default = "default_key_scale_in")]
    pub key_scale_in: [u8; 3],
    /// In-scale highlight on a black key.
    #[serde(default = "default_key_scale_in_dark")]
    pub key_scale_in_dark: [u8; 3],
    /// White key separator / outline stroke color.
    #[serde(default = "default_key_stroke")]
    pub key_stroke: [u8; 3],
    /// Octave label text (C3, C4, …) painted on white C keys.
    #[serde(default = "default_key_label")]
    pub key_label: [u8; 3],

    // ── Scope / visualizer ──────────────────────────────────────────────────
    pub scope_bg: [u8; 3],
    pub scope_zero: [u8; 3],
    pub scope_glow_outer: [u8; 4],
    pub scope_glow_mid: [u8; 4],
    pub scope_glow_core: [u8; 4],
    pub scope_label: [u8; 3],

    // ── Peak meter ──────────────────────────────────────────────────────────
    pub meter_bg: [u8; 3],
    pub meter_green: [u8; 3],
    pub meter_clip: [u8; 3],

    // ── ADSR visualizer ─────────────────────────────────────────────────────
    pub adsr_fill: [u8; 4],
    pub adsr_outline: [u8; 3],
    pub adsr_label: [u8; 4],
    pub adsr_cursor: [u8; 3],

    // ── Latency indicator ───────────────────────────────────────────────────
    pub latency_ok: [u8; 3],
    pub latency_warn: [u8; 3],
    pub latency_bad: [u8; 3],

    // ── Patch browser ───────────────────────────────────────────────────────
    pub patch_browser_model: [u8; 3],
    pub patch_load_fx_on: [u8; 3],

    // ── MIDI ────────────────────────────────────────────────────────────────
    pub midi_connected: [u8; 3],

    // ── Legacy panel backgrounds (kept for custom Painter calls) ────────────
    pub bg_panel: [u8; 3],
    pub bg_seq_bar: [u8; 3],
    pub bg_adsr: [u8; 3],

    // ── Geometry tokens (spacing, rounding, stroke) ─────────────────────────
    /// 2 px — internal widget micro-gaps (step-pad gaps, knob arc padding).
    #[serde(default = "default_sp_xxs")]
    pub sp_xxs: f32,
    /// 4 px — tightest gap; used between related controls.
    pub sp_xs: f32,
    /// 8 px — standard item spacing.
    pub sp_sm: f32,
    /// 12 px — section inner margin.
    pub sp_md: f32,
    /// 16 px — comfortable breathing room.
    pub sp_lg: f32,
    /// 24 px — section-to-section gap.
    pub sp_xl: f32,
    /// 40 px — major panel separation.
    #[serde(default = "default_sp_xxl")]
    pub sp_xxl: f32,
    /// 2 px — tiny corner radius (step buttons, micro-chips).
    #[serde(default = "default_rounding_xs")]
    pub rounding_xs: f32,
    /// Small corner radius — step buttons, chips.
    pub rounding_sm: f32,
    /// Medium corner radius — section cards.
    pub rounding_md: f32,
    /// Large corner radius — windows, popovers.
    pub rounding_lg: f32,
    /// Pill / badge — fully rounded.
    #[serde(default = "default_rounding_full")]
    pub rounding_full: f32,
    /// Default border stroke width.
    pub stroke_ui: f32,
    /// Focused / hovered border stroke width.
    pub stroke_focus: f32,
    /// Active / pressed border stroke width.
    pub stroke_active: f32,
}

fn default_sp_xxs() -> f32 {
    2.0
}
fn default_sp_xxl() -> f32 {
    40.0
}
fn default_rounding_xs() -> f32 {
    2.0
}
fn default_rounding_full() -> f32 {
    999.0
}
fn default_key_white_default() -> [u8; 3] { [245, 245, 245] }
fn default_key_black_default() -> [u8; 3] { [25, 25, 25] }
fn default_key_white_range() -> [u8; 3] { [230, 240, 245] }
fn default_key_black_range() -> [u8; 3] { [40, 40, 50] }
fn default_key_scale_root() -> [u8; 3] { [255, 210, 80] }
fn default_key_scale_root_dark() -> [u8; 3] { [120, 80, 10] }
fn default_key_scale_in() -> [u8; 3] { [200, 240, 210] }
fn default_key_scale_in_dark() -> [u8; 3] { [30, 70, 40] }
fn default_key_stroke() -> [u8; 3] { [180, 180, 180] }
fn default_key_label() -> [u8; 3] { [140, 140, 140] }

impl SynthTheme {
    // ── Color helpers ────────────────────────────────────────────────────────

    pub fn c(&self, rgb: &[u8; 3]) -> Color32 {
        Color32::from_rgb(rgb[0], rgb[1], rgb[2])
    }

    pub fn ca(&self, rgba: &[u8; 4]) -> Color32 {
        Color32::from_rgba_premultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
    }

    #[allow(dead_code)]
    pub fn active(&self, on: bool) -> Color32 {
        if on {
            self.c(&self.accent)
        } else {
            Color32::GRAY
        }
    }

    // ── egui integration ─────────────────────────────────────────────────────

    /// Apply this theme to egui's global `Visuals` and `Style`.
    ///
    /// Call once at the start of every `update()` frame so that all egui
    /// built-in widgets (buttons, sliders, labels, separators) automatically
    /// match the active theme without per-widget overrides.
    pub fn apply_to_egui(&self, ctx: &egui::Context) {
        use egui::style::WidgetVisuals;
        use egui::{Color32, CornerRadius, Margin, Shadow, Stroke, Vec2, Visuals};

        let bg_surface = self.c(&self.bg_surface);
        let bg_app = self.c(&self.bg_app);
        let bg_sunken = self.c(&self.bg_sunken);
        let border = self.c(&self.border);
        let border_focus = self.c(&self.border_focus);
        let text_primary = self.c(&self.text_primary);
        let text_secondary = self.c(&self.text_secondary);
        let accent = self.c(&self.accent);

        let round_md = CornerRadius::same(self.rounding_md as u8);

        // Slightly brighten a color for hover feedback.
        let lighten = |c: Color32, by: u8| {
            Color32::from_rgb(
                c.r().saturating_add(by),
                c.g().saturating_add(by),
                c.b().saturating_add(by),
            )
        };

        // Dim accent to use as active widget fill.
        let accent_fill =
            Color32::from_rgba_premultiplied(accent.r() / 5, accent.g() / 5, accent.b() / 5, 200);

        let wv = |bg: Color32, text: Color32, stroke_c: Color32, sw: f32| WidgetVisuals {
            bg_fill: bg,
            weak_bg_fill: bg,
            bg_stroke: Stroke::new(sw, stroke_c),
            corner_radius: round_md,
            fg_stroke: Stroke::new(1.0, text),
            expansion: 0.0,
        };

        let mut vis = Visuals::dark();

        // Background layers
        vis.panel_fill = bg_app;
        vis.window_fill = bg_surface;
        // Slider rails use extreme_bg_color — must be visibly distinct from bg_surface.
        // bg_sunken is often darker than bg_surface, making rails invisible; use a lightened surface instead.
        vis.extreme_bg_color = lighten(bg_surface, 22);
        vis.code_bg_color = bg_sunken;
        vis.faint_bg_color = bg_sunken;

        // Window chrome
        vis.window_corner_radius = CornerRadius::same(self.rounding_lg as u8);
        vis.window_stroke = Stroke::new(self.stroke_ui, border);
        vis.window_shadow = Shadow::NONE;
        vis.popup_shadow = Shadow::NONE;
        vis.menu_corner_radius = round_md;

        // Selection
        vis.selection.bg_fill =
            Color32::from_rgba_premultiplied(accent.r() / 5, accent.g() / 5, accent.b() / 5, 90);
        vis.selection.stroke = Stroke::new(self.stroke_focus, accent);
        vis.hyperlink_color = accent;

        // Widget states
        // inactive.bg_fill is used by Slider as the rail color — must be distinct from bg_surface.
        vis.widgets.noninteractive = wv(bg_surface, text_secondary, border, self.stroke_ui);
        vis.widgets.inactive = wv(
            lighten(bg_surface, 28),
            text_primary,
            border,
            self.stroke_ui,
        );
        vis.widgets.hovered = wv(
            lighten(bg_surface, 40),
            text_primary,
            border_focus,
            self.stroke_focus,
        );
        vis.widgets.active = wv(accent_fill, accent, accent, self.stroke_focus);
        vis.widgets.open = wv(
            lighten(bg_surface, 22),
            text_primary,
            border_focus,
            self.stroke_ui,
        );

        ctx.set_visuals(vis);

        // Layout / spacing
        let mut style = (*ctx.global_style()).clone();
        style.spacing.item_spacing = Vec2::new(self.sp_sm, self.sp_xs);
        style.spacing.window_margin = Margin::same(self.sp_md as i8);
        style.spacing.button_padding = Vec2::new(self.sp_sm, 3.0);
        style.spacing.menu_margin = Margin::same(self.sp_sm as i8);
        style.spacing.indent = self.sp_lg;
        style.spacing.interact_size = Vec2::new(40.0, 20.0);

        // Bind every egui TextStyle to a token from this theme. This is
        // what makes `ui.label()`, `RichText::small()`, `.heading()`,
        // `.monospace()`, button labels, and menu text align to the design
        // system without per-site `.font(...)` overrides.
        use egui::TextStyle;
        let text_styles = std::collections::BTreeMap::from([
            (TextStyle::Heading, self.font_heading()),
            (TextStyle::Body, self.font_body()),
            (TextStyle::Button, self.font_body()),
            (TextStyle::Small, self.font_small()),
            (TextStyle::Monospace, self.font_value()),
        ]);
        style.text_styles = text_styles;

        ctx.set_global_style(style);
    }
}

// ── Font tokens ──────────────────────────────────────────────────────────────
// Base sizes; the global pixels_per_point factor scales them at render time.
// Theme-independent. Allowed dead_code while panel files still use hardcoded
// `.size(N)` calls; Phase 3 migrates them.

#[allow(dead_code)]
impl SynthTheme {
    /// 14 pt — panel / section title.
    pub fn font_heading(&self) -> egui::FontId {
        egui::FontId::proportional(14.0)
    }

    /// 12 pt — parameter labels, button text.
    pub fn font_body(&self) -> egui::FontId {
        egui::FontId::proportional(12.0)
    }

    /// 11 pt monospace — knob value readouts, numeric displays.
    pub fn font_value(&self) -> egui::FontId {
        egui::FontId::monospace(11.0)
    }

    /// 10 pt — secondary labels, unit suffixes.
    pub fn font_small(&self) -> egui::FontId {
        egui::FontId::proportional(10.0)
    }

    /// 9 pt — sequencer step indices, keyboard note names (absolute floor).
    pub fn font_micro(&self) -> egui::FontId {
        egui::FontId::proportional(9.0)
    }
}

// ── Geometry defaults shared by all themes ───────────────────────────────────

/// All themes share the same geometry scale. Stored as a struct so additions
/// don't ripple into every theme constructor.
struct Geometry {
    sp_xxs: f32,
    sp_xs: f32,
    sp_sm: f32,
    sp_md: f32,
    sp_lg: f32,
    sp_xl: f32,
    sp_xxl: f32,
    rounding_xs: f32,
    rounding_sm: f32,
    rounding_md: f32,
    rounding_lg: f32,
    rounding_full: f32,
    stroke_ui: f32,
    stroke_focus: f32,
    stroke_active: f32,
}

fn geometry() -> Geometry {
    Geometry {
        sp_xxs: 2.0,
        sp_xs: 4.0,
        sp_sm: 8.0,
        sp_md: 12.0,
        sp_lg: 16.0,
        sp_xl: 24.0,
        sp_xxl: 40.0,
        rounding_xs: 2.0,
        rounding_sm: 4.0,
        rounding_md: 8.0,
        rounding_lg: 12.0,
        rounding_full: 999.0,
        stroke_ui: 1.0,
        stroke_focus: 1.5,
        stroke_active: 2.0,
    }
}

// ── Built-in themes ──────────────────────────────────────────────────────────

/// Midnight — dark navy-blue with teal accent.
pub fn midnight() -> SynthTheme {
    let g = geometry();
    SynthTheme {
        name: "Midnight".into(),

        bg_app: [6, 8, 12],
        bg_surface: [14, 18, 26],
        bg_sunken: [8, 11, 17],
        bg_bar: [10, 13, 19],

        border: [28, 35, 50],
        border_focus: [0, 160, 120],

        text_primary: [210, 218, 230],
        text_secondary: [110, 125, 145],
        text_disabled: [50, 60, 78],
        text_on_accent: [6, 8, 12],

        accent: [0, 220, 160],
        accent_dim: [0, 180, 130],

        knob_tier1_arc: [0, 220, 160],
        knob_tier2_arc: [0, 140, 105],
        knob_tier3_arc: [80, 95, 100],

        accent_hard_sync: [255, 180, 0],
        accent_fm: [120, 180, 255],
        accent_ring: [255, 130, 200],
        accent_hold: [255, 200, 0],
        accent_walker: [100, 180, 255],
        accent_limiter: [40, 220, 130],

        fx_overdrive: [255, 140, 60],
        fx_distortion: [220, 60, 60],
        fx_chorus: [80, 200, 140],
        fx_delay: [80, 160, 255],
        fx_reverb: [170, 90, 240],
        fx_shimmer: [120, 200, 255],
        fx_crystallizer: [255, 170, 90],

        seq_step_on: [0, 180, 120],
        seq_step_off: [40, 40, 55],
        seq_current: [255, 200, 50],
        seq_note_bar_on: [0, 120, 80],
        seq_note_bar_off: [40, 50, 55],
        seq_chord_major: [0, 100, 70],
        seq_chord_minor: [60, 80, 140],
        seq_chord_dim: [120, 50, 50],
        seq_kb_major: [30, 80, 55],
        seq_kb_minor: [40, 55, 100],
        seq_kb_dim: [80, 35, 35],
        seq_velocity_bar: [80, 140, 200],
        seq_prob_low: [180, 70, 50],
        seq_prob_mid: [180, 140, 40],
        seq_prob_high: [60, 160, 80],
        seq_rec_cursor: [220, 50, 50],
        seq_octave_bar: [120, 80, 180],

        key_white_pressed: [100, 180, 255],
        key_black_pressed: [60, 120, 200],
        key_white_default: [245, 245, 245],
        key_black_default: [25, 25, 25],
        key_white_range: [230, 240, 245],
        key_black_range: [40, 40, 50],
        key_scale_root: [255, 210, 80],
        key_scale_root_dark: [120, 80, 10],
        key_scale_in: [200, 240, 210],
        key_scale_in_dark: [30, 70, 40],
        key_stroke: [180, 180, 180],
        key_label: [140, 140, 140],

        scope_bg: [4, 10, 7],
        scope_zero: [12, 28, 18],
        scope_glow_outer: [0, 160, 90, 14],
        scope_glow_mid: [0, 210, 130, 45],
        scope_glow_core: [55, 255, 165, 230],
        scope_label: [60, 100, 80],

        meter_bg: [10, 15, 20],
        meter_green: [0, 200, 80],
        meter_clip: [255, 50, 30],

        adsr_fill: [0, 160, 100, 30],
        adsr_outline: [0, 200, 130],
        adsr_label: [80, 160, 110, 180],
        adsr_cursor: [0, 255, 160],

        latency_ok: [0, 180, 120],
        latency_warn: [200, 180, 0],
        latency_bad: [200, 70, 50],

        patch_browser_model: [100, 180, 255],
        patch_load_fx_on: [255, 180, 60],

        midi_connected: [0, 220, 120],

        bg_panel: [10, 15, 20],
        bg_seq_bar: [25, 25, 35],
        bg_adsr: [8, 14, 10],

        sp_xxs: g.sp_xxs,
        sp_xs: g.sp_xs,
        sp_sm: g.sp_sm,
        sp_md: g.sp_md,
        sp_lg: g.sp_lg,
        sp_xl: g.sp_xl,
        sp_xxl: g.sp_xxl,
        rounding_xs: g.rounding_xs,
        rounding_sm: g.rounding_sm,
        rounding_md: g.rounding_md,
        rounding_lg: g.rounding_lg,
        rounding_full: g.rounding_full,
        stroke_ui: g.stroke_ui,
        stroke_focus: g.stroke_focus,
        stroke_active: g.stroke_active,
    }
}

/// Winamp Classic — dark grey with vivid green.
pub fn winamp_classic() -> SynthTheme {
    let g = geometry();
    SynthTheme {
        name: "Winamp Classic".into(),

        bg_app: [10, 10, 10],
        bg_surface: [22, 22, 22],
        bg_sunken: [14, 14, 14],
        bg_bar: [18, 18, 18],

        border: [42, 42, 42],
        border_focus: [0, 200, 0],

        text_primary: [215, 215, 215],
        text_secondary: [130, 130, 130],
        text_disabled: [65, 65, 65],
        text_on_accent: [10, 10, 10],

        accent: [0, 230, 0],
        accent_dim: [0, 180, 0],

        knob_tier1_arc: [0, 230, 0],
        knob_tier2_arc: [0, 140, 0],
        knob_tier3_arc: [80, 100, 80],

        accent_hard_sync: [255, 200, 0],
        accent_fm: [150, 200, 60],
        accent_ring: [255, 150, 60],
        accent_hold: [255, 220, 0],
        accent_walker: [150, 200, 60],
        accent_limiter: [0, 230, 0],

        fx_overdrive: [255, 170, 0],
        fx_distortion: [255, 80, 40],
        fx_chorus: [0, 200, 100],
        fx_delay: [80, 180, 255],
        fx_reverb: [200, 120, 255],
        fx_shimmer: [100, 220, 255],
        fx_crystallizer: [255, 200, 60],

        seq_step_on: [0, 200, 0],
        seq_step_off: [40, 40, 40],
        seq_current: [255, 220, 0],
        seq_note_bar_on: [0, 140, 0],
        seq_note_bar_off: [45, 45, 45],
        seq_chord_major: [0, 120, 0],
        seq_chord_minor: [60, 90, 120],
        seq_chord_dim: [140, 60, 40],
        seq_kb_major: [20, 70, 20],
        seq_kb_minor: [40, 50, 80],
        seq_kb_dim: [80, 40, 30],
        seq_velocity_bar: [60, 200, 100],
        seq_prob_low: [220, 80, 40],
        seq_prob_mid: [220, 180, 30],
        seq_prob_high: [40, 220, 60],
        seq_rec_cursor: [255, 80, 60],
        seq_octave_bar: [180, 140, 60],

        key_white_pressed: [0, 220, 0],
        key_black_pressed: [0, 160, 0],
        key_white_default: [245, 245, 245],
        key_black_default: [25, 25, 25],
        key_white_range: [220, 240, 225],
        key_black_range: [35, 55, 40],
        key_scale_root: [255, 210, 80],
        key_scale_root_dark: [100, 70, 5],
        key_scale_in: [200, 240, 210],
        key_scale_in_dark: [25, 65, 35],
        key_stroke: [180, 180, 180],
        key_label: [120, 140, 120],

        scope_bg: [6, 6, 6],
        scope_zero: [20, 30, 20],
        scope_glow_outer: [0, 160, 0, 14],
        scope_glow_mid: [0, 210, 0, 45],
        scope_glow_core: [55, 255, 55, 230],
        scope_label: [80, 120, 80],

        meter_bg: [14, 14, 14],
        meter_green: [0, 220, 0],
        meter_clip: [255, 40, 20],

        adsr_fill: [0, 160, 0, 30],
        adsr_outline: [0, 200, 0],
        adsr_label: [80, 160, 80, 180],
        adsr_cursor: [0, 255, 0],

        latency_ok: [0, 200, 0],
        latency_warn: [220, 200, 0],
        latency_bad: [220, 60, 40],

        patch_browser_model: [150, 200, 60],
        patch_load_fx_on: [255, 200, 0],

        midi_connected: [0, 230, 0],

        bg_panel: [18, 18, 18],
        bg_seq_bar: [30, 30, 30],
        bg_adsr: [12, 12, 12],

        sp_xxs: g.sp_xxs,
        sp_xs: g.sp_xs,
        sp_sm: g.sp_sm,
        sp_md: g.sp_md,
        sp_lg: g.sp_lg,
        sp_xl: g.sp_xl,
        sp_xxl: g.sp_xxl,
        rounding_xs: g.rounding_xs,
        rounding_sm: g.rounding_sm,
        rounding_md: g.rounding_md,
        rounding_lg: g.rounding_lg,
        rounding_full: g.rounding_full,
        stroke_ui: g.stroke_ui,
        stroke_focus: g.stroke_focus,
        stroke_active: g.stroke_active,
    }
}

/// Phosphor — CRT green-on-black.
pub fn phosphor() -> SynthTheme {
    let g = geometry();
    SynthTheme {
        // Vintage Bakelite hardware aesthetic. Warm amber for all controls
        // and indicators; phosphor green reserved for actual screen surfaces
        // (oscilloscope glow tokens, ADSR display, MIDI signal dot).
        name: "Phosphor".into(),

        // ── Surfaces — dark warm Bakelite/wood casing ───────────────────────
        bg_app:     [20, 15, 10],
        bg_surface: [30, 22, 16],
        bg_sunken:  [13, 10,  7],
        bg_bar:     [24, 18, 13],

        border:       [ 50, 36, 24],
        border_focus: [ 88, 64, 42],

        // ── Text — warm cream / parchment labels ────────────────────────────
        text_primary:   [225, 210, 185],
        text_secondary: [148, 128,  98],
        text_disabled:  [ 78,  62,  46],
        text_on_accent: [ 18,  13,   8],

        // ── Main accent — warm amber (vintage LED / backlit indicator) ───────
        accent:     [195, 155, 65],
        accent_dim: [ 80,  58, 20],

        knob_tier1_arc: [210, 168, 72],
        knob_tier2_arc: [175, 135, 52],
        knob_tier3_arc: [128,  98, 44],

        // ── Secondary accents — all desaturated / pastel ────────────────────
        accent_hard_sync: [155, 105, 175], // dusty violet
        accent_fm:        [105, 148, 170], // powder blue
        accent_ring:      [175, 145,  85], // warm tan-gold
        accent_hold:      [185, 118,  88], // pastel terracotta
        accent_walker:    [138, 168, 105], // dusty sage
        accent_limiter:   [190, 110,  80], // pastel burnt orange

        // ── FX — all desaturated, vintage panel-label feel ──────────────────
        fx_overdrive:   [182, 118,  62],
        fx_distortion:  [182,  88,  72],
        fx_chorus:      [152, 118,  80],
        fx_delay:       [128, 152,  88],
        fx_reverb:      [ 88, 128, 152],
        fx_shimmer:     [105, 152, 168],
        fx_crystallizer:[168, 135,  85],

        // ── Sequencer — warm amber pastels ──────────────────────────────────
        seq_step_on:      [165, 128,  52],
        seq_step_off:     [ 32,  24,  16],
        seq_current:      [215, 175,  75],
        seq_note_bar_on:  [105,  82,  32],
        seq_note_bar_off: [ 28,  22,  15],
        seq_chord_major:  [ 82,  65,  25],
        seq_chord_minor:  [ 48,  62,  82],
        seq_chord_dim:    [ 82,  45,  35],
        seq_kb_major:     [ 42,  32,  14],
        seq_kb_minor:     [ 28,  38,  55],
        seq_kb_dim:       [ 55,  28,  22],
        seq_velocity_bar: [145, 115,  55],
        seq_prob_low:     [ 95,  75,  48],
        seq_prob_mid:     [145, 115,  55],
        seq_prob_high:    [195, 155,  65],
        seq_rec_cursor:   [185,  98,  82], // pastel dusty red
        seq_octave_bar:   [155, 118,  52],

        // ── Piano keys — aged ivory and ebony ───────────────────────────────
        key_white_pressed:    [195, 155,  65], // amber glow (= accent)
        key_black_pressed:    [130,  98,  32], // dark amber
        key_white_default:    [208, 192, 172], // aged ivory
        key_black_default:    [ 28,  20,  14], // dark ebony
        key_white_range:      [188, 175, 155], // slightly dimmed ivory
        key_black_range:      [ 45,  34,  22],
        key_scale_root:       [195, 155,  65], // amber root (= accent)
        key_scale_root_dark:  [100,  78,  28],
        key_scale_in:         [172, 158, 132], // warm tan
        key_scale_in_dark:    [ 55,  42,  28],
        key_stroke:           [148, 128, 100],
        key_label:            [115,  95,  70],

        // ── Scope — phosphor green glow to support hardcoded waveform ───────
        scope_bg:         [  4,   8,   5],
        scope_zero:       [ 10,  22,  12],
        scope_glow_outer: [  0, 140,  60, 14],
        scope_glow_mid:   [ 10, 200,  90, 45],
        scope_glow_core:  [ 60, 255, 140, 230],
        scope_label:      [ 40,  90,  55],

        // ── Meters — vintage VU feel (sage green, not harsh phosphor) ───────
        meter_bg:    [ 16,  12,   8],
        meter_green: [128, 168,  80], // sage green pastel
        meter_clip:  [188,  82,  62], // pastel coral red

        // ── ADSR display — phosphor green (screen visualizer) ────────────────
        adsr_fill:    [  0, 160,  80,  25],
        adsr_outline: [ 20, 200,  90],
        adsr_label:   [ 60, 155,  85, 180],
        adsr_cursor:  [ 40, 220, 120],

        // ── Status ───────────────────────────────────────────────────────────
        latency_ok:   [128, 175,  80], // sage green
        latency_warn: [195, 155,  65], // amber
        latency_bad:  [185,  82,  62], // pastel red

        patch_browser_model: [105, 148, 170], // dusty blue
        patch_load_fx_on:    [195, 155,  65], // amber

        midi_connected: [100, 210, 140], // phosphor green (signal indicator)

        // ── Legacy panel bg tokens ───────────────────────────────────────────
        bg_panel:   [ 18,  13,   9],
        bg_seq_bar: [ 26,  20,  14],
        bg_adsr:    [ 13,  10,   7],

        sp_xxs: g.sp_xxs,
        sp_xs: g.sp_xs,
        sp_sm: g.sp_sm,
        sp_md: g.sp_md,
        sp_lg: g.sp_lg,
        sp_xl: g.sp_xl,
        sp_xxl: g.sp_xxl,
        rounding_xs: g.rounding_xs,
        rounding_sm: g.rounding_sm,
        rounding_md: g.rounding_md,
        rounding_lg: g.rounding_lg,
        rounding_full: g.rounding_full,
        stroke_ui: g.stroke_ui,
        stroke_focus: g.stroke_focus,
        stroke_active: g.stroke_active,
    }
}

pub fn builtin_themes() -> Vec<SynthTheme> {
    vec![midnight(), winamp_classic(), phosphor()]
}

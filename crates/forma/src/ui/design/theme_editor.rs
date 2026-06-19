//! Theme Editor — live color and geometry editor for `SynthTheme`.
//!
//! A floating window that exposes every token in the active theme as an
//! inline color swatch or slider. Edits take effect on the same frame so
//! the entire UI reflects changes immediately. Themes can be exported to
//! and imported from JSON files via the native file dialog.
//!
//! Toggle with Ctrl/Cmd + Shift + T.

use egui::{Color32, Context, RichText, Ui, Vec2, Window};

use crate::ui::theme::{builtin_themes, SynthTheme};

// ─── Category sidebar ────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum Group {
    Surfaces,
    Text,
    Accent,
    FxColors,
    Sequencer,
    PianoKeys,
    Scope,
    Meters,
    Adsr,
    Status,
    Geometry,
}

impl Group {
    const ALL: &'static [Group] = &[
        Group::Surfaces,
        Group::Text,
        Group::Accent,
        Group::FxColors,
        Group::Sequencer,
        Group::PianoKeys,
        Group::Scope,
        Group::Meters,
        Group::Adsr,
        Group::Status,
        Group::Geometry,
    ];

    fn label(self) -> &'static str {
        match self {
            Group::Surfaces  => "Surfaces",
            Group::Text      => "Text",
            Group::Accent    => "Accent",
            Group::FxColors  => "FX Colors",
            Group::Sequencer => "Sequencer",
            Group::PianoKeys => "Piano Keys",
            Group::Scope     => "Scope",
            Group::Meters    => "Meters",
            Group::Adsr      => "ADSR",
            Group::Status    => "Status",
            Group::Geometry  => "Geometry",
        }
    }
}

// ─── Persistent editor state ─────────────────────────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct EditorState {
    group: Group,
    /// Index into `builtin_themes()` for the "Copy from" dropdown.
    copy_from_idx: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self { group: Group::Surfaces, copy_from_idx: 0 }
    }
}

const STATE_ID: &str = "forma_theme_editor_state";

fn load_state(ctx: &Context) -> EditorState {
    ctx.data(|d| d.get_temp::<EditorState>(egui::Id::new(STATE_ID)))
        .unwrap_or_default()
}

fn save_state(ctx: &Context, s: EditorState) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(STATE_ID), s));
}

// ─── Helpers — token row widgets ─────────────────────────────────────────────

/// Render a labeled color-swatch button for a `[u8; 3]` token.
fn color_row(ui: &mut Ui, label: &str, token: &mut [u8; 3]) {
    ui.horizontal(|ui| {
        ui.color_edit_button_srgb(token);
        ui.label(RichText::new(label).size(11.0).color(Color32::from_gray(200)));
    });
}

/// Render a labeled color-swatch button for a `[u8; 4]` (RGBA) token.
fn color_row_rgba(ui: &mut Ui, label: &str, token: &mut [u8; 4]) {
    ui.horizontal(|ui| {
        ui.color_edit_button_srgba_unmultiplied(token);
        ui.label(RichText::new(label).size(11.0).color(Color32::from_gray(200)));
    });
}

/// Render a sub-section label styled as a dim separator.
fn sub_label(ui: &mut Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(text).size(10.0).color(Color32::from_gray(100)));
    ui.add_space(2.0);
}

// ─── Token group panels ───────────────────────────────────────────────────────

fn group_surfaces(ui: &mut Ui, t: &mut SynthTheme) {
    sub_label(ui, "Backgrounds");
    color_row(ui, "bg_app", &mut t.bg_app);
    color_row(ui, "bg_surface", &mut t.bg_surface);
    color_row(ui, "bg_sunken", &mut t.bg_sunken);
    color_row(ui, "bg_bar", &mut t.bg_bar);
    color_row(ui, "bg_panel", &mut t.bg_panel);
    color_row(ui, "bg_seq_bar", &mut t.bg_seq_bar);
    color_row(ui, "bg_adsr", &mut t.bg_adsr);
    sub_label(ui, "Borders");
    color_row(ui, "border", &mut t.border);
    color_row(ui, "border_focus", &mut t.border_focus);
}

fn group_text(ui: &mut Ui, t: &mut SynthTheme) {
    color_row(ui, "text_primary", &mut t.text_primary);
    color_row(ui, "text_secondary", &mut t.text_secondary);
    color_row(ui, "text_disabled", &mut t.text_disabled);
    color_row(ui, "text_on_accent", &mut t.text_on_accent);
}

fn group_accent(ui: &mut Ui, t: &mut SynthTheme) {
    sub_label(ui, "Main accent");
    color_row(ui, "accent", &mut t.accent);
    color_row(ui, "accent_dim", &mut t.accent_dim);
    sub_label(ui, "Knob arc tiers");
    color_row(ui, "knob_tier1_arc", &mut t.knob_tier1_arc);
    color_row(ui, "knob_tier2_arc", &mut t.knob_tier2_arc);
    color_row(ui, "knob_tier3_arc", &mut t.knob_tier3_arc);
    sub_label(ui, "Special accents");
    color_row(ui, "accent_hard_sync", &mut t.accent_hard_sync);
    color_row(ui, "accent_fm", &mut t.accent_fm);
    color_row(ui, "accent_ring", &mut t.accent_ring);
    color_row(ui, "accent_hold", &mut t.accent_hold);
    color_row(ui, "accent_walker", &mut t.accent_walker);
    color_row(ui, "accent_limiter", &mut t.accent_limiter);
}

fn group_fx(ui: &mut Ui, t: &mut SynthTheme) {
    color_row(ui, "fx_overdrive", &mut t.fx_overdrive);
    color_row(ui, "fx_distortion", &mut t.fx_distortion);
    color_row(ui, "fx_chorus", &mut t.fx_chorus);
    color_row(ui, "fx_delay", &mut t.fx_delay);
    color_row(ui, "fx_reverb", &mut t.fx_reverb);
    color_row(ui, "fx_shimmer", &mut t.fx_shimmer);
    color_row(ui, "fx_crystallizer", &mut t.fx_crystallizer);
}

fn group_sequencer(ui: &mut Ui, t: &mut SynthTheme) {
    sub_label(ui, "Step pads");
    color_row(ui, "seq_step_on", &mut t.seq_step_on);
    color_row(ui, "seq_step_off", &mut t.seq_step_off);
    color_row(ui, "seq_current", &mut t.seq_current);
    color_row(ui, "seq_rec_cursor", &mut t.seq_rec_cursor);
    sub_label(ui, "Note bars");
    color_row(ui, "seq_note_bar_on", &mut t.seq_note_bar_on);
    color_row(ui, "seq_note_bar_off", &mut t.seq_note_bar_off);
    color_row(ui, "seq_octave_bar", &mut t.seq_octave_bar);
    sub_label(ui, "Chord pads");
    color_row(ui, "seq_chord_major", &mut t.seq_chord_major);
    color_row(ui, "seq_chord_minor", &mut t.seq_chord_minor);
    color_row(ui, "seq_chord_dim", &mut t.seq_chord_dim);
    color_row(ui, "seq_kb_major", &mut t.seq_kb_major);
    color_row(ui, "seq_kb_minor", &mut t.seq_kb_minor);
    color_row(ui, "seq_kb_dim", &mut t.seq_kb_dim);
    sub_label(ui, "Velocity / probability");
    color_row(ui, "seq_velocity_bar", &mut t.seq_velocity_bar);
    color_row(ui, "seq_prob_low", &mut t.seq_prob_low);
    color_row(ui, "seq_prob_mid", &mut t.seq_prob_mid);
    color_row(ui, "seq_prob_high", &mut t.seq_prob_high);
}

fn group_piano(ui: &mut Ui, t: &mut SynthTheme) {
    sub_label(ui, "White keys");
    color_row(ui, "key_white_default", &mut t.key_white_default);
    color_row(ui, "key_white_pressed", &mut t.key_white_pressed);
    color_row(ui, "key_white_range", &mut t.key_white_range);
    sub_label(ui, "Black keys");
    color_row(ui, "key_black_default", &mut t.key_black_default);
    color_row(ui, "key_black_pressed", &mut t.key_black_pressed);
    color_row(ui, "key_black_range", &mut t.key_black_range);
    sub_label(ui, "Scale highlights");
    color_row(ui, "key_scale_root", &mut t.key_scale_root);
    color_row(ui, "key_scale_root_dark", &mut t.key_scale_root_dark);
    color_row(ui, "key_scale_in", &mut t.key_scale_in);
    color_row(ui, "key_scale_in_dark", &mut t.key_scale_in_dark);
    sub_label(ui, "Chrome");
    color_row(ui, "key_stroke", &mut t.key_stroke);
    color_row(ui, "key_label", &mut t.key_label);
}

fn group_scope(ui: &mut Ui, t: &mut SynthTheme) {
    color_row(ui, "scope_bg", &mut t.scope_bg);
    color_row(ui, "scope_zero", &mut t.scope_zero);
    color_row(ui, "scope_label", &mut t.scope_label);
    sub_label(ui, "Glow layers (RGBA)");
    color_row_rgba(ui, "scope_glow_outer", &mut t.scope_glow_outer);
    color_row_rgba(ui, "scope_glow_mid", &mut t.scope_glow_mid);
    color_row_rgba(ui, "scope_glow_core", &mut t.scope_glow_core);
}

fn group_meters(ui: &mut Ui, t: &mut SynthTheme) {
    color_row(ui, "meter_bg", &mut t.meter_bg);
    color_row(ui, "meter_green", &mut t.meter_green);
    color_row(ui, "meter_clip", &mut t.meter_clip);
}

fn group_adsr(ui: &mut Ui, t: &mut SynthTheme) {
    color_row(ui, "adsr_outline", &mut t.adsr_outline);
    color_row(ui, "adsr_cursor", &mut t.adsr_cursor);
    sub_label(ui, "Fill / label (RGBA)");
    color_row_rgba(ui, "adsr_fill", &mut t.adsr_fill);
    color_row_rgba(ui, "adsr_label", &mut t.adsr_label);
}

fn group_status(ui: &mut Ui, t: &mut SynthTheme) {
    sub_label(ui, "Latency indicator");
    color_row(ui, "latency_ok", &mut t.latency_ok);
    color_row(ui, "latency_warn", &mut t.latency_warn);
    color_row(ui, "latency_bad", &mut t.latency_bad);
    sub_label(ui, "Misc");
    color_row(ui, "midi_connected", &mut t.midi_connected);
    color_row(ui, "patch_browser_model", &mut t.patch_browser_model);
    color_row(ui, "patch_load_fx_on", &mut t.patch_load_fx_on);
}

fn group_geometry(ui: &mut Ui, t: &mut SynthTheme) {
    sub_label(ui, "Spacing");
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.sp_xxs, 0.0..=8.0).text("sp_xxs").step_by(0.5)); });
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.sp_xs,  0.0..=12.0).text("sp_xs").step_by(0.5)); });
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.sp_sm,  0.0..=16.0).text("sp_sm").step_by(0.5)); });
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.sp_md,  0.0..=24.0).text("sp_md").step_by(0.5)); });
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.sp_lg,  0.0..=32.0).text("sp_lg").step_by(0.5)); });
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.sp_xl,  0.0..=48.0).text("sp_xl").step_by(1.0)); });
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.sp_xxl, 0.0..=80.0).text("sp_xxl").step_by(1.0)); });
    sub_label(ui, "Rounding");
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.rounding_xs,   0.0..=8.0).text("rounding_xs").step_by(0.5)); });
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.rounding_sm,   0.0..=12.0).text("rounding_sm").step_by(0.5)); });
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.rounding_md,   0.0..=16.0).text("rounding_md").step_by(0.5)); });
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.rounding_lg,   0.0..=24.0).text("rounding_lg").step_by(0.5)); });
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.rounding_full, 0.0..=64.0).text("rounding_full").step_by(1.0)); });
    sub_label(ui, "Strokes");
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.stroke_ui,     0.0..=4.0).text("stroke_ui").step_by(0.25)); });
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.stroke_focus,  0.0..=4.0).text("stroke_focus").step_by(0.25)); });
    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut t.stroke_active, 0.0..=4.0).text("stroke_active").step_by(0.25)); });
}

// ─── Top bar ─────────────────────────────────────────────────────────────────

fn top_bar(ui: &mut Ui, theme: &mut SynthTheme, state: &mut EditorState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Name").size(11.0).color(Color32::from_gray(160)));
        ui.add(
            egui::TextEdit::singleline(&mut theme.name)
                .desired_width(120.0)
                .font(egui::FontId::proportional(12.0)),
        );

        ui.add_space(8.0);

        // Copy from built-in
        let themes = builtin_themes();
        ui.label(RichText::new("Copy from").size(11.0).color(Color32::from_gray(160)));
        egui::ComboBox::from_id_salt("theme_editor_copy_from")
            .selected_text(&themes[state.copy_from_idx].name)
            .width(100.0)
            .show_ui(ui, |ui| {
                for (i, t) in themes.iter().enumerate() {
                    ui.selectable_value(&mut state.copy_from_idx, i, &t.name);
                }
            });
        if ui.small_button("Copy").clicked() {
            let name = theme.name.clone();
            *theme = themes[state.copy_from_idx].clone();
            theme.name = name;
        }

        ui.add_space(8.0);

        // Save JSON
        if ui.small_button("Save JSON").clicked() {
            let json = serde_json::to_string_pretty(theme).unwrap_or_default();
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON theme", &["json"])
                .set_file_name(format!("{}.json", theme.name.to_lowercase().replace(' ', "_")))
                .save_file()
            {
                let _ = std::fs::write(path, json);
            }
        }

        // Load JSON
        if ui.small_button("Load JSON").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON theme", &["json"])
                .pick_file()
            {
                if let Ok(raw) = std::fs::read_to_string(path) {
                    if let Ok(loaded) = serde_json::from_str::<SynthTheme>(&raw) {
                        *theme = loaded;
                    }
                }
            }
        }
    });
}

// ─── Sidebar ─────────────────────────────────────────────────────────────────

fn sidebar(ui: &mut Ui, state: &mut EditorState) {
    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
        for &g in Group::ALL {
            let selected = state.group == g;
            let label = RichText::new(g.label()).size(12.0);
            if ui.selectable_label(selected, label).clicked() {
                state.group = g;
            }
        }
    });
}

// ─── Public entry point ───────────────────────────────────────────────────────

pub fn show(ctx: &Context, open: &mut bool, theme: &mut SynthTheme) {
    let mut state = load_state(ctx);

    let win = Window::new("Theme Editor")
        .open(open)
        .resizable(true)
        .default_size(Vec2::new(520.0, 560.0))
        .min_width(400.0)
        .min_height(300.0);

    win.show(ctx, |ui| {
        top_bar(ui, theme, &mut state);
        ui.add_space(4.0);
        ui.separator();

        egui::SidePanel::left("theme_editor_sidebar")
            .resizable(false)
            .exact_width(100.0)
            .show_inside(ui, |ui| {
                sidebar(ui, &mut state);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(4.0);
                match state.group {
                    Group::Surfaces  => group_surfaces(ui, theme),
                    Group::Text      => group_text(ui, theme),
                    Group::Accent    => group_accent(ui, theme),
                    Group::FxColors  => group_fx(ui, theme),
                    Group::Sequencer => group_sequencer(ui, theme),
                    Group::PianoKeys => group_piano(ui, theme),
                    Group::Scope     => group_scope(ui, theme),
                    Group::Meters    => group_meters(ui, theme),
                    Group::Adsr      => group_adsr(ui, theme),
                    Group::Status    => group_status(ui, theme),
                    Group::Geometry  => group_geometry(ui, theme),
                }
                ui.add_space(8.0);
            });
        });
    });

    save_state(ctx, state);
}

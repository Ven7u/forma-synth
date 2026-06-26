use crate::SynthApp;
use egui::WidgetText;
use egui_dock::{DockState, NodeIndex, TabViewer};

/// Each dockable panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Oscillators,
    Mixer,
    Modulation,
    Filter,
    Sequencer,
    ArpWalker,
    FxChain,
    Equalizer,
    Scope,
    Midi,
}

impl Tab {
    pub fn title(self) -> &'static str {
        match self {
            Tab::Oscillators => "Oscillators",
            Tab::Mixer => "Mixer",
            Tab::Modulation => "Modulation",
            Tab::Filter => "Filter & Envelopes",
            Tab::Sequencer => "Sequencer",
            Tab::ArpWalker => "Arp & Walker",
            Tab::FxChain => "FX Chain",
            Tab::Equalizer => "Equalizer",
            Tab::Scope => "Oscilloscope",
            Tab::Midi => "MIDI & Latency",
        }
    }

    pub const ALL: &[Tab] = &[
        Tab::Oscillators,
        Tab::Mixer,
        Tab::Modulation,
        Tab::Filter,
        Tab::Sequencer,
        Tab::ArpWalker,
        Tab::FxChain,
        Tab::Equalizer,
        Tab::Scope,
        Tab::Midi,
    ];
}

/// Height of the egui_dock tab bar — used when computing split fractions
/// so the fraction accounts for the chrome consumed by the tab strip.
const TAB_BAR_H: f32 = 28.0;

/// Build the default dock layout.
///
/// ```text
/// ┌─────────────────────────┬───────────────────────┐
/// │  Oscillators            │  Oscilloscope         │
/// ├────────────┬────────────┼───────────────────────┤
/// │ Modulation │Arp & Walker│  FX Chain             │
/// │ & Filter   │   (tabbed) │                       │
/// ├────────────┴────────────┴───────────────────────┤
/// │  Keyboard  │  Sequencer    (tabbed)              │
/// └─────────────────────────────────────────────────┘
/// ```
/// Compute the OSC/Modulation split fraction for a given total dock height
/// and measured OSC content height.  Falls back to 0.55 before the first
/// measurement is available (osc_content_h == 0.0).
pub fn osc_split_fraction(dock_available_h: f32, osc_content_h: f32) -> f32 {
    if osc_content_h <= 0.0 || dock_available_h <= 0.0 {
        return 0.55;
    }
    // The top area is 75% of the total dock height (bottom 25% = sequencer).
    // Inside that, the OSC pane needs: measured content + tab bar chrome.
    let top_area_h = dock_available_h * 0.75;
    let osc_pane_h = osc_content_h + TAB_BAR_H;
    (osc_pane_h / top_area_h).clamp(0.35, 0.80)
}

pub fn default_dock_state() -> DockState<Tab> {
    // Oscillators + Mixer share the root node as sibling tabs. Click between
    // them; both get the same full width of the upper-left dock column.
    let mut state = DockState::new(vec![Tab::Oscillators, Tab::Mixer]);
    let surface = state.main_surface_mut();

    // 1. Split bottom from root: Sequencer + ArpWalker tabbed — bottom 25%.
    let [top, _bottom] = surface.split_below(
        NodeIndex::root(),
        0.75,
        vec![Tab::Sequencer, Tab::ArpWalker],
    );

    // 2. In top area, split right: Oscilloscope — right takes 40%.
    let [top_left, top_right] = surface.split_right(top, 0.60, vec![Tab::Scope]);

    // 3. Split top-left vertically: Oscillators/Mixer top, Modulation/Filter bottom.
    // Fraction is overridden by ui_synth_dock() using the measured OSC content height;
    // 0.55 is the fallback used only on the very first launch (before measurement).
    let [_osc_mixer, _mod] = surface.split_below(top_left, 0.55, vec![Tab::Modulation, Tab::Filter]);

    // 4. Split top-right vertically: FX Chain + Equalizer tabbed below Oscilloscope.
    let [_scope, _fx] = surface.split_below(top_right, 0.50, vec![Tab::Equalizer, Tab::FxChain]);

    state
}

/// Render the full egui-dock area — used by Studio mode and the LIVE per-track view.
impl crate::SynthApp {
    pub fn ui_synth_dock(&mut self, ui: &mut egui::Ui) {
        if self.reset_layout_pending {
            self.dock_state = default_dock_state();
            self.osc_split_calibrated = false; // allow re-calibration on next frame
            self.reset_layout_pending = false;
        }

        // One-shot calibration: frame 0 renders with the 0.55 fallback and
        // measures actual OSC content height; frame 1 applies the precise
        // fraction before the dock renders again.
        // NodeIndex(3) is the Vertical split OSC/Mixer ↔ Modulation/Filter.
        if !self.osc_split_calibrated && self.osc_tab_content_h > 0.0 {
            let frac = osc_split_fraction(ui.available_height(), self.osc_tab_content_h);
            if let Some(tree) = self.dock_state
                .get_surface_mut(egui_dock::SurfaceIndex::main())
                .and_then(|s| s.node_tree_mut())
            {
                if let egui_dock::Node::Vertical(split) = &mut tree[egui_dock::NodeIndex(3)] {
                    split.fraction = frac;
                }
            }
            self.osc_split_calibrated = true;
        }
        let mut dock_state =
            std::mem::replace(&mut self.dock_state, egui_dock::DockState::new(vec![]));
        let mut s = egui_dock::Style::from_egui(ui.style());
        s.tab_bar.show_scroll_bar_on_overflow = false;
        s.separator.width = 6.0;
        s.separator.color_idle = egui::Color32::TRANSPARENT;
        s.separator.color_hovered = egui::Color32::from_black_alpha(60);
        s.separator.color_dragged = egui::Color32::from_black_alpha(100);
        s.dock_area_padding = Some(egui::Margin::same(6i8));
        let rm = self.theme.rounding_md as u8;
        let r_top = egui::CornerRadius {
            nw: rm,
            ne: rm,
            sw: 0,
            se: 0,
        };
        let r_body = egui::CornerRadius {
            nw: 0,
            ne: rm,
            sw: rm,
            se: rm,
        };
        let bg_surface = self.theme.c(&self.theme.bg_surface);
        let border = self.theme.c(&self.theme.border);
        let text_pri = self.theme.c(&self.theme.text_primary);
        let text_sec = self.theme.c(&self.theme.text_secondary);
        let accent = self.theme.c(&self.theme.accent);
        s.tab.tab_body.corner_radius = r_body;
        s.tab.tab_body.stroke = egui::Stroke::new(self.theme.stroke_ui, border);
        s.tab_bar.bg_fill = egui::Color32::TRANSPARENT;
        s.tab_bar.hline_color = egui::Color32::TRANSPARENT;
        s.tab_bar.corner_radius = r_body;
        s.tab_bar.height = 28.0;
        s.tab.active = egui_dock::TabInteractionStyle {
            corner_radius: r_top,
            bg_fill: bg_surface,
            text_color: text_pri,
            outline_color: accent,
        };
        s.tab.focused = egui_dock::TabInteractionStyle {
            corner_radius: r_top,
            bg_fill: bg_surface,
            text_color: accent,
            outline_color: accent,
        };
        s.tab.inactive = egui_dock::TabInteractionStyle {
            corner_radius: r_top,
            bg_fill: egui::Color32::TRANSPARENT,
            text_color: text_sec,
            outline_color: egui::Color32::TRANSPARENT,
        };
        s.tab.hovered = egui_dock::TabInteractionStyle {
            corner_radius: r_top,
            bg_fill: bg_surface,
            text_color: text_pri,
            outline_color: border,
        };
        egui_dock::DockArea::new(&mut dock_state)
            .style(s)
            .show_inside(ui, &mut SynthTabViewer { app: self });
        self.dock_state = dock_state;
    }
}

/// Tab viewer that delegates rendering to SynthApp methods.
pub struct SynthTabViewer<'a> {
    pub app: &'a mut SynthApp,
}

impl<'a> TabViewer for SynthTabViewer<'a> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Tab) -> WidgetText {
        tab.title().into()
    }

    // Disable horizontal scrolling — all tab content is designed to fit the
    // panel width. The spurious horizontal scrollbar was appearing because
    // egui_dock enables both axes by default.
    fn scroll_bars(&self, _tab: &Tab) -> [bool; 2] {
        [false, true]
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        match tab {
            Tab::Oscillators => {
                let top = ui.min_rect().top();
                ui.columns(3, |cols| {
                    self.app.ui_osc_panel(&mut cols[0], 0);
                    self.app.ui_osc_panel(&mut cols[1], 1);
                    self.app.ui_osc_panel(&mut cols[2], 2);
                });
                // Record actual content height so layout reset can set the
                // split fraction to exactly fit the OSC panel.
                self.app.osc_tab_content_h = ui.min_rect().bottom() - top;
            }
            Tab::Mixer => {
                self.app.ui_mixer_panel(ui);
            }
            Tab::Modulation => {
                ui.vertical(|ui| {
                    ui.columns(2, |cols| {
                        self.app.ui_lfo_panel(&mut cols[0]);
                        self.app.ui_lfo2_panel(&mut cols[1]);
                    });
                    self.app.ui_pulse_panel(ui);
                    ui.columns(3, |cols| {
                        self.app.ui_mod_wheel_panel(&mut cols[0]);
                        self.app.ui_aftertouch_panel(&mut cols[1]);
                        self.app.ui_mod_matrix_panel(&mut cols[2]);
                    });
                });
            }
            Tab::Filter => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    self.app.ui_filter_panel(ui);
                    ui.columns(2, |cols| {
                        self.app.ui_adsr_panel(
                            &mut cols[0],
                            "Filter Env",
                            &mut [0usize, 1, 2, 3],
                            true,
                        );
                        self.app.ui_adsr_panel(
                            &mut cols[1],
                            "Amp Env",
                            &mut [0usize, 1, 2, 3],
                            false,
                        );
                    });
                });
            }
            Tab::Sequencer => {
                self.app.ui_sequencer_panel(ui);
            }
            Tab::ArpWalker => {
                ui.columns(2, |cols| {
                    self.app.ui_arp_panel(&mut cols[0]);
                    self.app.ui_walker_panel(&mut cols[1]);
                });
            }
            Tab::FxChain => {
                self.app.ui_fx_chain(ui);
            }
            Tab::Equalizer => {
                self.app.ui_eq_panel(ui);
            }
            Tab::Scope => {
                self.app.ui_oscilloscope(ui);
            }
            Tab::Midi => {
                self.app.ui_midi_panel(ui);
                ui.separator();
                super::scope::draw_latency_bar(
                    ui,
                    &self.app.engine,
                    self.app.engine.amp_attack(),
                    &self.app.theme,
                );
            }
        }
    }
}

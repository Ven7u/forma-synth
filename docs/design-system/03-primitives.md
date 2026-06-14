# Layer 2 — Primitives

Primitives are the low-level building blocks that components (Layer 3) are
assembled from. They provide two things:

1. **SynthFrame** — named container styles (already exists, needs minor additions)
2. **SynthUi** — an extension trait on `egui::Ui` that adds synth-specific
   layout helpers and the top-level component entry points

No panel or pattern should call `egui::Frame::new()` directly — only `SynthFrame`.
No panel should manually compute column widths — only layout helpers.

---

## SynthFrame — container styles

Each variant encodes: fill color, rounding, stroke, and inner/outer margin.
All values come from `SynthTheme` tokens.

| Variant | Surface token | Use |
|---------|--------------|-----|
| `SynthFrame::app_bg()` | `bg_app` | The app-level background (outermost) |
| `SynthFrame::bar()` | `bg_bar` | Transport strip, top/bottom toolbars |
| `SynthFrame::section()` | `bg_surface` | Primary card/panel container |
| `SynthFrame::inset()` | `bg_sunken` | Sub-group inside a section (darker) |
| `SynthFrame::screen()` | `bg_sunken` | Visualizer background (scope, EQ canvas) |
| `SynthFrame::overlay()` | `bg_overlay` | Floating tooltip / popover background |
| `SynthFrame::tier1()` | `bg_surface` + `accent` border | Highlights a Tier 1 control group |

**Additions needed:**
- `SynthFrame::tier1()` — a section frame with the accent-colored border to
  visually elevate the performance control region on each panel
- `SynthFrame::overlay()` — for tooltip/popover contexts

---

## SynthUi — extension trait

`SynthUi` adds domain-specific methods to `egui::Ui`. This is the idiomatic
egui pattern for a component library: instead of importing functions, you call
methods on the `ui` handle you already have.

```rust
// Instead of this (raw egui):
let knob = Knob::new(&mut state.cutoff).label("CUTOFF").size(KnobSize::Large);
ui.add(knob);

// You write this (SynthUi):
ui.synth_knob(&mut state.cutoff, "CUTOFF", KnobSize::Large, Tier::Primary);
```

### Layout helpers on SynthUi

These replace all manual column-width math currently scattered across panels.

```rust
// Lay out N knobs in a row, spacing them evenly within available width.
// Uses egui_flex internally. Handles resize automatically.
ui.knob_row(&mut [
    KnobSpec { value: &mut state.cutoff,    label: "CUT",  size: Large,    tier: Primary },
    KnobSpec { value: &mut state.resonance, label: "RES",  size: Large,    tier: Primary },
    KnobSpec { value: &mut state.drive,     label: "DRV",  size: Standard, tier: Secondary },
    KnobSpec { value: &mut state.env_amt,   label: "ENV",  size: Standard, tier: Secondary },
]);

// Lay out N items in equal-width columns with a label header per column.
ui.labeled_columns(&["OSC 1", "OSC 2", "OSC 3"], |cols| {
    cols[0].synth_knob(...);
    cols[1].synth_knob(...);
    cols[2].synth_knob(...);
});

// A chip selector row — mutually exclusive options, button-style.
ui.chip_selector(&mut state.waveform, &[
    (Waveform::Sin, "SIN"),
    (Waveform::Saw, "SAW"),
    (Waveform::Sqr, "SQR"),
    (Waveform::Tri, "TRI"),
]);

// A labeled horizontal fader (the 8px track, vertically oriented).
ui.synth_fader(&mut state.volume, "VOL", FaderSize::Medium, Tier::Secondary);

// A section header (bold label + optional right-aligned slot for a toggle).
ui.section_header("FILTER", Some(|ui: &mut Ui| {
    ui.synth_toggle(&mut state.filter_enabled, "");
}));
```

### Direct component entry points

These are convenience wrappers that call the component with defaults.

```rust
ui.synth_knob(&mut value, "LABEL", KnobSize::Standard, Tier::Secondary);
ui.synth_knob_primary(&mut value, "LABEL");   // Large + Tier 1
ui.synth_toggle(&mut value, "SYNC");
ui.synth_chip(&mut value, options);
ui.value_display(value, "Hz");               // read-only numeric readout
ui.status_dot(is_active, color);             // small colored indicator dot
```

---

## Layout philosophy for panels

### Fixed-zone layout

Every panel should be structured as a fixed set of zones. The panel's total
height is `CARD_H` (a token, not a magic number). Within that height:

```
┌─────────────────────────────────┐  ← section frame top
│  section_header("FILTER")       │  font_heading, sp_md padding   ~28 px
├─────────────────────────────────┤
│  Primary controls (Tier 1)      │  knob_row with Large knobs      ~96 px
├─────────────────────────────────┤
│  Secondary controls (Tier 2)    │  knob_row with Standard knobs   ~72 px
├─────────────────────────────────┤
│  Tertiary / config (Tier 3)     │  compact chips / DragValues     ~40 px
└─────────────────────────────────┘  ← section frame bottom
```

Tier 1 controls always occupy the top (most visible) zone. Tier 3 controls
always occupy the bottom (least prominent, first to scroll out of view if
space is tight).

### Width strategy

**Never hardcode a width.** Use one of:

1. `ui.available_width()` — fill whatever remains
2. `ui.available_width() / n` — divide evenly (only for equal-weight items)
3. `egui_flex` via `knob_row()` / `labeled_columns()` — for responsive equal-
   width allocation with proper gap handling
4. Token-defined fixed sizes (`btn_size_lg`, etc.) — for individual controls
   that should never shrink

### The scroll boundary rule

A `ScrollArea` must wrap every section that can be taller than its allocated
zone. The rule: **content that is bounded by design (known N of knobs)
should not scroll; content that is data-driven (patch list, step sequence)
must scroll.**

Scrollable regions in Forma:
- Patch browser list
- Scene browser list
- Patch history list
- Sequencer (when step count > visible area)
- FX chain (horizontal scroll when > 5 effects)
- MIDI preset list

Non-scrollable (bounded by design — just add more layout space):
- Oscillator cards
- Filter section
- LFO section
- Mixer channel strip

---

## Module structure under `ui/design/`

```
ui/design/
  mod.rs          re-exports everything; one-line imports for panels
  tokens.rs       SynthTheme additions (font tokens, zoom, tier tokens)
  frame.rs        SynthFrame (refactored from existing frame.rs)
  layout.rs       SynthUi trait definition + layout helper impls
  knob.rs         Knob component (refactored from widgets/knob.rs)
  button.rs       ToggleButton, ChipSelector, MomentaryButton
  fader.rs        Vertical + horizontal fader
  display.rs      ValueDisplay, StatusDot, LevelMeter
  step_pad.rs     Step button for sequencer / drum machine
  card.rs         Section card pattern (header + zones)
```

Existing `ui/widgets/knob.rs` migrates into `ui/design/knob.rs`. 
Existing `ui/frame.rs` migrates into `ui/design/frame.rs`.
Existing `ui/theme.rs` stays but gains new token fields.

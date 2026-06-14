# Layer 4 — Patterns

Patterns are named, reusable compositions of components. They encode the
"how panels are structured" knowledge. No panel should assemble its own
layout from scratch — it should call a pattern and fill in the slots.

Every pattern is a function in `ui/design/layout.rs` (or card.rs) that takes
a `&mut Ui`, data bindings, and optional configuration.

---

## Pattern index

| Pattern | Function signature sketch | Use |
|---------|--------------------------|-----|
| [KnobRow](#knobrow) | `ui.knob_row(&mut specs)` | Any horizontal row of knobs |
| [SectionCard](#sectioncard) | `ui.section_card("TITLE", tier, content_fn)` | Every panel section |
| [TieredCard](#tieredcard) | `ui.tiered_card(t1_fn, t2_fn, t3_fn)` | Full oscillator/filter card |
| [FxModule](#fxmodule) | `ui.fx_module(name, color, enabled, params_fn)` | Single effect in the FX chain |
| [ChipRow](#chiprow) | `ui.chip_row(label, &mut value, options)` | A labeled chip selector |
| [FaderColumn](#fadercolumn) | `ui.fader_column(label, &mut vol, meter)` | A mixer channel strip |
| [TransportBar](#transportbar) | `ui.transport_bar(state)` | The global transport strip |
| [EnvelopeEditor](#envelopeeditor) | `ui.envelope_editor(adsr, size)` | ADSR visual + four knobs |

---

## KnobRow

The most-used pattern. Lays out N knobs horizontally, evenly spaced within
available width, using `egui_flex` so it reflows correctly on resize.

### Usage

```rust
ui.knob_row(&mut [
    KnobSpec::new(&mut params.cutoff,    "CUT",  KnobSize::Large,    Tier::Primary),
    KnobSpec::new(&mut params.resonance, "RES",  KnobSize::Large,    Tier::Primary),
    KnobSpec::new(&mut params.env_amt,   "ENV",  KnobSize::Standard, Tier::Secondary),
    KnobSpec::new(&mut params.key_track, "KEY",  KnobSize::Small,    Tier::Tertiary),
]);
```

### Behaviour

- `egui_flex` allocates equal column width per knob
- Each knob is centered in its column
- Gaps use `sp_md` between knobs
- If available width < sum of minimum knob widths, the row becomes horizontally
  scrollable (not clipped)
- Different size knobs in the same row are bottom-aligned (labels line up)

### Mixed-tier rows

Allowed and common — e.g. two Large Tier 1 knobs + two Standard Tier 2 knobs.
The visual weight difference communicates importance naturally. Keep Tier 1
knobs at the left / center of the row (more natural hand position).

---

## SectionCard

The standard container for any panel section. Wraps content in a `SynthFrame`
with a `SectionHeader`.

### Usage

```rust
ui.section_card("FILTER", Tier::Primary, |ui| {
    // content goes here — knob rows, chip selectors, etc.
});
```

### Behaviour

- `Tier::Primary` → uses `SynthFrame::tier1()` (accent border) + larger header
- `Tier::Secondary` → uses `SynthFrame::section()` (standard border)
- `Tier::Tertiary` → uses `SynthFrame::inset()` (sunken, no border)
- Height is not fixed by this pattern — the content determines height
- The card adds `sp_md` inner padding and `sp_xs` outer margin

---

## TieredCard

A full panel card with three pre-defined zones: Tier 1 at top, Tier 2 in
middle, Tier 3 at bottom. This is the template for oscillator, filter, and
LFO cards.

### Usage

```rust
ui.tiered_card(
    // Tier 1 zone — performance controls
    |ui| {
        ui.knob_row(&mut [
            KnobSpec::new(&mut params.cutoff, "CUT", Large, Primary),
            KnobSpec::new(&mut params.res,    "RES", Large, Primary),
        ]);
    },
    // Tier 2 zone — sound design controls
    |ui| {
        ui.knob_row(&mut [
            KnobSpec::new(&mut params.env_amt,  "ENV", Standard, Secondary),
            KnobSpec::new(&mut params.key_track,"KEY", Standard, Secondary),
            KnobSpec::new(&mut params.drive,    "DRV", Standard, Secondary),
        ]);
    },
    // Tier 3 zone — configuration
    |ui| {
        ui.chip_row("TYPE", &mut params.filter_type, &[
            (FilterType::Lp12, "LP12"),
            (FilterType::Lp24, "LP24"),
            (FilterType::Hp,   "HP"),
            (FilterType::Bp,   "BP"),
        ]);
    },
);
```

### Zone heights

Zone heights adjust to content. The minimum guaranteed height for each zone:
- Tier 1 zone: Large knob height (`knob_size_lg.height`) + `sp_md` × 2
- Tier 2 zone: Standard knob height + `sp_md` × 2
- Tier 3 zone: `chip_height` + `sp_sm` × 2

A thin separator line (`stroke_ui` color) divides zones visually.

---

## FxModule

A single effect unit in the FX chain. Has a fixed-width card with: title bar
(name + enable toggle + color indicator), and a vertical stack of sliders.

### Usage

```rust
ui.fx_module("DELAY", theme.fx_delay, &mut params.delay_enabled, |ui| {
    ui.synth_toggle(&mut params.delay_sync, "SYNC");
    ui.chip_row("DIV", &mut params.delay_div, DELAY_DIVISIONS);
    ui.knob_row(&mut [
        KnobSpec::new(&mut params.delay_time,     "TIME", Standard, Secondary),
        KnobSpec::new(&mut params.delay_feedback,  "FB",  Standard, Secondary),
        KnobSpec::new(&mut params.delay_mix,      "MIX",  Standard, Secondary),
    ]);
});
```

### Behaviour

- Module width: fixed `120 px` minimum, expands to fill equal share of FX chain
- When disabled (`enabled = false`): entire content dimmed to 40% opacity,
  title color changes to `text_muted`
- The colored left border (4px wide) is the FX domain color token
- Reorder handle: a drag grip on the title bar (uses `egui_dnd`)

---

## ChipRow

A common pattern: a short label + a chip selector inline.

### Usage

```rust
ui.chip_row("WAVE", &mut osc.waveform, &[
    (Waveform::Sin, "SIN"),
    (Waveform::Saw, "SAW"),
    (Waveform::Sqr, "SQR"),
    (Waveform::Tri, "TRI"),
]);
```

### Layout

```
WAVE  [SIN][SAW][SQR][TRI]
```

Label width: fixed to the longest label in the panel (align labels in a
column for multiple ChipRows stacked vertically).

---

## FaderColumn

A mixer channel strip. Vertical fader + level meter + label.

### Usage

```rust
ui.fader_column("T1", &mut track.volume, track.peak_level, track.is_clipping);
```

### Layout

```
  T1             ← label (font_small, centered, text_secondary)
┃█     ┃         ← fader + VU meter side by side (fader_h_md height)
  Lead           ← patch name (font_micro, centered, text_muted)
 [M][S]          ← mute/solo toggle buttons (btn_size_sm)
```

Total column width: `CHANNEL_WIDTH` token. Columns are placed in a horizontal
strip with `sp_xs` gap.

---

## TransportBar

The global transport strip — always visible at the top of the app.

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│ [◀◀] [▶] [■] [●]   BPM: [128.0]  [4/4]   SCENE  PATCH  ... │
└──────────────────────────────────────────────────────────────┘
```

- Height: `btn_size_lg.height` + `sp_sm` × 2 = fixed ~52 px
- Play/Stop/Record buttons: `btn_size_lg`, `Tier::Primary`
- BPM input: `ValueDisplay::Primary` + interactive (click to type)
- All transport controls are Tier 1 — largest hit targets in the app

---

## EnvelopeEditor

An ADSR visualizer + four labeled knobs below it. Used wherever a full
envelope is exposed (filter, amp, modulation).

### Layout

```
┌─────────────────────────────────┐
│        ╱─────╲                  │  ← ADSR shape preview (SynthFrame::screen)
│       ╱       ─────╲            │     height: 48px (Tier 2 context)
│      ╱                ╲         │     or 64px (Tier 1 context)
└─────────────────────────────────┘
 [A]       [D]       [S]     [R]     ← four Standard knobs in a KnobRow
 0.01s     0.3s     0.7     1.2s    ← value readouts (in the knob)
```

The preview shape redraws every frame from current ADSR values — no separate
state needed. Animate the playhead cursor across the shape when a note is
held (shows current envelope position live).

---

## Pattern composition rules

1. Patterns call components. Patterns do not call raw egui widgets directly.
2. Panels call patterns. Panels do not call components directly (except trivially
   simple cases like a single `ui.synth_toggle()`).
3. A new panel = assemble from existing patterns. If something doesn't fit,
   add a new pattern rather than breaking the rule.
4. Patterns accept content closures (`FnOnce(&mut Ui)`) for their variable
   parts. Fixed structure is enforced by the pattern; variable content is
   provided by the caller.

---

## Section-level layout — general guidelines

These guidelines govern how multiple cards or sections relate to each other
within a panel. Detailed specifications for each panel are deferred until
Phase 5+ of implementation, when real pixels can inform the decisions. This
section captures the decision rules so they don't have to be re-derived
mid-implementation.

### The three layout shapes

Every panel body is one of these three shapes. Identify which one before
writing any layout code:

| Shape | Description | Examples |
|-------|-------------|---------|
| **Strip** | N homogeneous cards in a row (all same type, same controls) | Oscillators, Mixer channels |
| **Grid** | Rows × columns, both dimensions meaningful | Drum machine, Sequencer steps |
| **Stack** | Vertical sequence of heterogeneous sections | Arpeggiator, Filter, LFO, MIDI settings |

Stacks are the simplest and require no special layout logic beyond the default
egui vertical flow. Strips and grids require explicit slot math.

### The sizing contract

Before placing any card or section, decide which direction the sizing
authority flows:

> **Fixed item count at compile time → container defines size, content adapts.**
> **Item count driven by user data → content defines size, container scrolls.**

| Fixed count (container authority) | Data-driven (content authority) |
|-----------------------------------|----------------------------------|
| 3 oscillator cards | Patch browser list |
| 8 drum channels | Sequencer steps (variable length) |
| N FX modules in the chain | Scene list |
| 4 mixer tracks | Patch history entries |

When in doubt: if you know the number while writing code, it's fixed. If you
only know it at runtime, it's data-driven.

### Strip math

For N equal cards in a horizontal strip:

```
card_width = (available_width - gap * (N - 1)) / N
```

Where `gap` is `sp_md` for related cards (oscillators) or `sp_lg` for more
visually distinct sections. Card height is always a fixed token (e.g. `CARD_H`),
never derived from content.

**Minimum card width:** Define a minimum below which the content becomes
unusable (typically the width of the widest KnobRow it contains). If
`card_width < minimum`, the strip switches to a horizontal `ScrollArea` rather
than crushing content.

### Grid math

For a rows × columns grid:

```
cell_width  = (available_width  - row_header_width  - gap * (cols - 1)) / cols
cell_height = (available_height - col_header_height - gap * (rows - 1)) / rows
```

In practice, one dimension is usually fixed (cell height for drum/sequencer
grids) and the other fills available space. Overflow in the fixed dimension
direction triggers a ScrollArea on the grid body only — control headers stay
pinned above/beside.

### The escape valve rule

Every layout must have a named policy for what happens when content overflows.
Decide this before implementing, not after:

| Layout | Overflow direction | Escape valve |
|--------|-------------------|--------------|
| Horizontal strip (oscillators) | Too many cards or window too narrow | Horizontal scroll |
| FX chain strip | Too many effects | Horizontal scroll |
| Mixer strip | Too many channels | Horizontal scroll |
| Sequencer grid | Too many steps (horizontal) | Horizontal scroll on grid body |
| Drum machine grid | Too many channels (vertical) | Vertical scroll on grid body |
| Stack panel (arp, LFO) | Too many controls | Vertical scroll |
| Fixed card contents | Window too small | Minimum window size enforced (never scroll a fixed-count card) |

The last row is important: if a card has a fixed number of knobs, the answer
to overflow is **not** to scroll the card — it is to enforce the minimum window
size so the card always has enough room.

### Pinned vs. scrollable regions

Within a panel that has a ScrollArea, not everything scrolls:

- **Always pinned:** column/row headers, control bars above a grid, section titles
- **Always scrollable:** the data grid body, long lists

```
┌──────────────────────────────────┐
│  Controls bar (pinned)           │  ← play, mode, BPM — never scrolls
├──────┬───────────────────────────┤
│ Row  │                           │
│ hdrs │   Grid body (scrollable)  │  ← steps/cells scroll; headers stay
│(pin) │                           │
└──────┴───────────────────────────┘
```

### Deferred details

The following section-level layouts are documented here only at the
decision-rule level. Full specs (exact widths, minimum sizes, scroll thresholds)
are written during Phase 5–6 of the implementation plan, against real
implemented content:

- **Oscillator panel:** EqualStrip of 3 TieredCards
- **FX Chain panel:** ScrollableStrip of FxModules
- **Mixer panel:** EqualStrip of FaderColumns
- **Sequencer panel:** Pinned control bar + scrollable step grid
- **Drum Machine panel:** Pinned channel strip + scrollable step grid
- **Arpeggiator panel:** Stack (no special layout needed)
- **Keyboard panel:** Custom draw, width-driven key sizing

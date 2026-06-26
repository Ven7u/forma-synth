# Layer 3 — Component Catalog

Components are self-contained, reusable widgets. Each component:
- Reads sizes and colors from `SynthTheme` tokens only (no magic numbers)
- Has a defined set of variants/states
- Is documented with: purpose, variants, interactive states, usage rules

---

## Component index

| Component | File | Tier applicability |
|-----------|------|-------------------|
| [Knob](#knob) | `design/knob.rs` | All tiers (via KnobSize) |
| [ToggleButton](#togglebutton) | `design/button.rs` | All tiers |
| [MomentaryButton](#momentarybutton) | `design/button.rs` | Tier 1, 2 |
| [ChipSelector](#chipselector) | `design/button.rs` | Tier 2, 3 |
| [Fader](#fader) | `design/fader.rs` | All tiers |
| [StepPad](#steppad) | `design/step_pad.rs` | Sequencer / drum only |
| [ValueDisplay](#valuedisplay) | `design/display.rs` | All (read-only) |
| [StatusDot](#statusdot) | `design/display.rs` | Status indicators |
| [LevelMeter](#levelmeter) | `design/display.rs` | VU / peak meters |
| [SectionHeader](#sectionheader) | `design/card.rs` | Panel structure |

---

## Knob

The primary continuous-value control. The most visible component in the synth.

### Variants by tier

| Variant | Token | Rect | Radius | Arc stroke | When to use |
|---------|-------|------|--------|------------|-------------|
| `KnobSize::Large` | `knob_size_lg` | 64 × 88 px | 24 px | `stroke_tier1` | Tier 1 — performance controls |
| `KnobSize::Standard` | `knob_size_md` | 44 × 64 px | 16 px | `stroke_tier2` | Tier 2 — sound design |
| `KnobSize::Small` | `knob_size_sm` | 32 × 48 px | 11 px | `stroke_tier3` | Tier 3 — config |

### Anatomy

```
  ┌──────────────────┐
  │    ╭──────╮      │  ← arc track (bg_sunken colored ring, 270°)
  │   ╱  ●    ╲     │  ← center dot (grey at rest, accent when hovered)
  │   ╲  │    ╱     │  ← indicator line (points to current value)
  │    ╰──────╯      │
  │     9.5 kHz      │  ← value text (font_value, text_secondary)
  │      FREQ        │  ← label text (font_body, text_primary for Tier 1,
  └──────────────────┘               text_secondary for Tier 2+)
```

Value text sits between knob bottom and label. Label sits at the very bottom
of the allocated rect.

### Arc color by tier

| Tier | Arc color token |
|------|----------------|
| Tier 1 | `knob_tier1_arc` (accent) |
| Tier 2 | `knob_tier2_arc` (accent dimmed ~60%) |
| Tier 3 | `knob_tier3_arc` (accent dimmed ~30%, nearly neutral) |

This means at a glance, the brightly-colored knobs are the important ones.
Tier 3 knobs are visually quiet.

### Interactive states

| State | Visual change |
|-------|---------------|
| At rest | Center dot is `text_muted` |
| Hovered | Center dot brightens to `accent_dim`; arc stroke slightly wider |
| Dragging | Center dot is `accent`; arc fills at live value |
| Double-clicked | Resets to default value (visual snap) |
| Shift+drag | Fine mode — movement 10× slower; add visual indicator (small "F" badge or cursor change) |

### Interaction spec

- **Drag axis:** vertical (up = increase). Consistent across all knob sizes.
- **Sensitivity:** `(max - min) / 300px` for Standard; `/ 500px` for Large
  (larger knob = finer control per pixel, more satisfying feel)
- **Fine mode:** Shift held → sensitivity ÷ 10
- **Reset:** Double-click → jumps to `default_value`
- **Tooltip:** On hover, show full parameter name + current value + unit in
  egui tooltip (not visible at rest — keeps UI clean)
- **Range display:** Optional — on hover, show min/max at arc endpoints in
  `font_micro`

### Usage rules

- Every knob must have a label. Never a bare knob.
- Value text is always shown (not just on hover). Musicians need to read
  values without interacting.
- Do not use a Large knob for Tier 2 or Tier 3 controls — it misrepresents
  importance.
- Knobs in a row should all be the same size tier unless there is a deliberate
  visual hierarchy being communicated.

---

## ToggleButton

A button that switches between two states: on/off, active/inactive.

### Sizes

Follows `btn_size_lg/md/sm` tokens. Default for Tier 2 is `btn_size_md`.

### Visual states

| State | Fill | Text color | Border |
|-------|------|------------|--------|
| Off / inactive | `state_idle` | `text_secondary` | `stroke_ui` |
| Off / hovered | `state_hovered` | `text_primary` | `stroke_focus` |
| On / active | `state_enabled` (accent fill) | `text_on_accent` | none |
| On / hovered | `accent` slightly lighter | `text_on_accent` | `stroke_focus` |
| Disabled | `state_idle` dimmed | `state_disabled_fg` | `stroke_ui` dimmed |

### Usage rules

- Use for binary settings: SYNC, HOLD, MUTE, SOLO, UNISON, REC
- Label is always short (≤ 5 characters) — it's a chip, not a sentence
- Do not use for momentary actions (use MomentaryButton)
- Group related toggles in a horizontal strip with `sp_xs` gap

---

## MomentaryButton

A button that fires an action on click (not a toggle). Think: PLAY, STOP,
RESET, STEP BACK.

### Visual states

| State | Fill | Border |
|-------|------|--------|
| At rest | `state_idle` | `stroke_ui` |
| Hovered | `state_hovered` | `stroke_focus` |
| Pressed | `state_active` | `stroke_active` |

### Usage rules

- Tier 1 actions (play, stop) use `btn_size_lg`
- Use an icon OR a short text label — not both (too noisy)
- For transport controls, group play/stop/rec together with consistent sizing
- Never use accent fill for a momentary button at rest — accent means
  "currently on", which is meaningless for a one-shot action

---

## ChipSelector

A row of mutually exclusive options. Like a radio group but presented as
connected chips. Used for waveform selection, mode selection, divisions.

### Layout

```
┌─────┬─────┬─────┬─────┐
│ SIN │ SAW │ SQR │ TRI │   all same height (chip_height token)
└─────┴─────┴─────┴─────┘
```

Chips share borders (no gap between them). The selected chip uses
`state_enabled` fill. Unselected use `state_idle`.

### Sizing

Width is content-driven (text + `sp_sm` horizontal padding each side).
When chips must fit a fixed width, divide equally: `available_width / n`.

### Usage rules

- Maximum 6 options before a ComboBox is preferable
- All chips in a group are the same width when options are similar length
- Works for: waveforms (4 options), octave divisions, Arp modes, EQ band types
- Does NOT work well for: long strings, icons with different visual weights

---

## Fader

A continuous-value control presented as a linear slider. Used where visual
position along a line communicates value better than a rotary angle (volumes,
panning, sends).

### Variants

| Variant | Orientation | Default track width | Use |
|---------|-------------|---------------------|-----|
| `FaderOrientation::Vertical` | Bottom = min, top = max | `fader_track_w` (8 px) | Channel volume, expression |
| `FaderOrientation::Horizontal` | Left = min, right = max | fills available width | Mix sends, pan, FM depth |

### Visual anatomy

```
Vertical:
    ─── ← thumb (rect, rounding_sm, stroke_active color when dragged)
    │
    │   ← track (fill = bg_sunken, stroke = stroke_ui)
    │
    ─── (bottom)
```

### Sizes by tier

| Tier | Fader height (vertical) | Thumb height |
|------|------------------------|--------------|
| Tier 1 | `fader_h_lg` (120 px) | 16 px |
| Tier 2 | `fader_h_md` (80 px) | 12 px |
| Tier 3 | `fader_h_sm` (48 px) | 8 px |

### Usage rules

- For volume controls, always pair with a LevelMeter immediately adjacent
- Label goes below (vertical) or to the left (horizontal)
- Show current value on hover in a tooltip (not permanently — takes too much space)
- Fine mode: Shift+drag, same as Knob

---

## StepPad

A button for sequencer/drum grid steps. Has three states: inactive, active,
and current (the step currently playing).

### States

| State | Fill | Border |
|-------|------|--------|
| Inactive | `seq_step_inactive` | none |
| Active (programmed) | `seq_step_active` | none |
| Current (playhead) | `seq_step_active` + `accent_glow` | `stroke_active` |
| Hovered | lightened fill | `stroke_focus` |
| Pressed | `state_active` | `stroke_active` |

### Sizing

Fixed: 26 × 24 px (drum machine) / 20 × 20 px (note sequencer). These are
intentionally small — you have many of them. Gap between steps: `sp_xxs` (2 px).

### Velocity encoding

When velocity is exposed, encode it visually via the step fill opacity or
height (a shorter fill within the pad = lower velocity). This avoids needing
a second control per step for the common case.

---

## ValueDisplay

A read-only numeric display. Shows the current value of a parameter without
any interactive affordance. Used in: BPM display, scope axis labels, MIDI
note number, latency readout.

### Variants

| Variant | Font | Color | Use |
|---------|------|-------|-----|
| `ValueDisplay::Primary` | `font_body`, monospace | `text_primary` | BPM, main readouts |
| `ValueDisplay::Secondary` | `font_value`, monospace | `text_secondary` | Knob values (embedded) |
| `ValueDisplay::Micro` | `font_micro`, monospace | `text_muted` | Axis labels, indices |

### Usage rules

- Use monospace font for any numerically changing value (prevents layout jump)
- Do not use for labels that never change — that's just `ui.label()`
- Always include a unit suffix as a separate, dimmer span: `"440"` + `" Hz"`

---

## StatusDot

A small colored circle indicating binary state. Used for: MIDI connected,
recording active, track playing, voice active.

### Sizes

- Large: 10 px diameter — for important status (MIDI connection, REC indicator)
- Small: 6 px diameter — for secondary status (track active, voice active)

### Colors

Use the semantic state color directly: `accent` for active/playing,
`vu_warn` for warning, `vu_clip` for error, `text_muted` for inactive.

---

## LevelMeter

A vertical or horizontal bar representing audio level. Already implemented —
this entry documents the design spec it should conform to.

### Color zones

| Range | Color token |
|-------|-------------|
| 0 – 60% | `vu_normal` |
| 60 – 90% | `vu_warn` |
| 90 – 100% | `vu_clip` |

### Sizing

Track width: 6 px (narrow — these appear in multiples). Height: matches the
fader it is paired with.

Peak hold: a 1px line at the recent peak position, held for ~1.5s then
falling slowly. Implemented in audio engine, displayed here.

---

## SectionHeader

The standard header row for a panel section. Composes: title text + optional
right-aligned slot.

### Anatomy

```
┌─────────────────────────────────────────┐
│  OSC 1                          [FLIP ▶] │  ← title (font_heading) + optional right slot
└─────────────────────────────────────────┘
  ↑ sp_md padding left                sp_md padding right ↑
```

The right slot can hold: a toggle, a chip selector, a close button, or nothing.

### Usage rules

- Title uses `font_heading`, `text_primary`
- Never put more than one control in the right slot (use inline controls below
  if more are needed)
- The header row height is fixed: heading font height + `sp_md` top + `sp_sm` bottom
- Use `SynthFrame::tier1()` border on the section frame when the section
  contains Tier 1 controls

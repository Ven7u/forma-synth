# Layer 1 — Tokens

Tokens are named constants that encode every design decision. No component,
pattern, or panel should contain a hardcoded color hex, pixel size, or font
point value. If a value appears more than once, it belongs here.

---

## How tokens are exposed

All tokens live in `SynthTheme`, accessed via `ctx.data(|d| d.get_temp::<SynthTheme>(...))` 
or passed explicitly. The geometry tokens are theme-independent (same values
across Midnight, Winamp, Phosphor); color tokens are per-theme.

Future: when extracted to `forma-ui`, `SynthTheme` becomes a public API type.

---

## 1. Spacing scale

A fixed 7-step scale. **No value outside this set is permitted** in component
code. If a control needs breathing room, choose the next step up; don't invent
an intermediate value.

| Token | Value | Use |
|-------|-------|-----|
| `sp_xxs` | 2 px | Internal widget micro-gaps (knob arc padding) |
| `sp_xs` | 4 px | Tightest gap between tightly related controls |
| `sp_sm` | 8 px | Standard item spacing (egui default zone) |
| `sp_md` | 12 px | Section inner margin |
| `sp_lg` | 16 px | Breathing room between control groups |
| `sp_xl` | 24 px | Section-to-section gap |
| `sp_xxl` | 40 px | Major panel separation |

---

## 2. Typography scale

Previously font sizes were scattered as magic numbers (7–13 pt). This replaces
all of them with six named roles.

| Token | Base size (pt) | Use | Minimum at 0.7× zoom |
|-------|---------------|-----|----------------------|
| `font_display` | 18 | Mode name, large status readout | 12.6 pt |
| `font_heading` | 14 | Panel/section title | 9.8 pt |
| `font_body` | 12 | Parameter labels, button text | 8.4 pt |
| `font_value` | 11 | Current value readouts on knobs | 7.7 pt |
| `font_small` | 10 | Secondary labels, unit suffixes | 7.0 pt |
| `font_micro` | 9 | Keyboard note names, sequencer step numbers | 6.3 pt |

**Rules:**
- `font_micro` is the absolute floor. Never go below 9 pt base.
- Use `font_display` only for the 1–2 most prominent labels per screen.
- Value readouts (what the knob is currently set to) use `font_value` — one size
  *smaller* than the label, and dimmer. The label answers "what", the value
  answers "how much" — they should not compete.
- Monospace font applies to: BPM readouts, step indices, scope overlays,
  frequency/Hz values.

### How implicit text styles bind to tokens

`SynthTheme::apply_to_egui` writes the entire `style.text_styles` map every
frame, binding egui's built-in `TextStyle` enum to our font tokens:

| `egui::TextStyle` | Token | Affects |
|-------------------|-------|---------|
| `Body` | `font_body` | `ui.label("…")`, plain text without `.font(...)` |
| `Button` | `font_body` | Button labels, menu items, dropdown text |
| `Heading` | `font_heading` | `RichText::heading()`, `ui.heading()` |
| `Small` | `font_small` | `RichText::small()`, `.weak().small()` chains |
| `Monospace` | `font_value` | `RichText::monospace()`, `.code()` |

This means there are **two valid ways** for a panel to render text on-system:
1. Explicit: `RichText::new("CUT").font(theme.font_body())` — required when
   the visual role (label vs. value vs. heading) doesn't match the default
   `TextStyle::Body`.
2. Implicit: `ui.label("foo")` — picks up `TextStyle::Body` → `font_body`
   automatically via the global style binding.

A panel that uses only `ui.label()` / `.small()` / `.heading()` / `.monospace()`
without ever writing `.size(N)` or constructing a `FontId` is already
token-compliant. Phase 3 audited and migrated every explicit `.size(N)` site;
the implicit ones are covered by the `TextStyle` map.

---

## 3. Color tokens

Color tokens are semantic (named by role, not by color value). Each theme
provides values for every token.

### Surface hierarchy

The surface tokens form a luminance ladder. Each step must be perceptibly
distinct — minimum 6% relative luminance difference between adjacent levels.

| Token | Role |
|-------|------|
| `bg_app` | App background — lowest layer |
| `bg_surface` | Panel/card surface — one step above app bg |
| `bg_sunken` | Inset controls, readout backgrounds — one step below surface |
| `bg_bar` | Transport/toolbar strip — distinct from surface |
| `bg_overlay` | Floating windows, tooltips — elevated |

### Interactive states

| Token | Role |
|-------|------|
| `state_idle` | Widget at rest (fill) |
| `state_hovered` | Pointer over widget (fill) |
| `state_active` | Pressed/engaged (fill) |
| `state_focus_ring` | Keyboard-focus ring color |
| `state_enabled` | Toggle/button in ON state (typically accent) |
| `state_disabled_fg` | Dimmed text for inactive controls |

### Text

| Token | Role |
|-------|------|
| `text_primary` | Main labels, headings |
| `text_secondary` | Value readouts, helper text |
| `text_muted` | Tier 3 control labels, status text |
| `text_on_accent` | Text sitting on top of an accent-colored background |

### Accent and highlight

| Token | Role |
|-------|------|
| `accent` | Primary brand color — active states, playing indicator |
| `accent_dim` | Softer version for fills behind text |
| `accent_glow` | Animated glow for live/playing state (used sparingly) |

### Tier-specific accent colors

Tier 1 controls use the main `accent`. Tier 2 and 3 controls use toned-down
variants so they don't compete visually:

| Token | Tier | Usage |
|-------|------|-------|
| `knob_tier1_arc` | Tier 1 | Arc fill color on performance knobs |
| `knob_tier2_arc` | Tier 2 | Arc fill on standard sound-design knobs |
| `knob_tier3_arc` | Tier 3 | Arc fill on config knobs (muted) |

### Domain-specific tokens (already in theme.rs — keep these)

These encode musical domain knowledge and should not be replaced with generic
accent colors:

- `fx_overdrive`, `fx_distortion`, `fx_chorus`, `fx_delay`, `fx_reverb`,
  `fx_shimmer`, `fx_crystallizer` — FX module accent colors
- `seq_step_active`, `seq_step_inactive`, `seq_step_current` — sequencer grid
- `key_white`, `key_black`, `key_pressed` — piano keyboard
- `scope_waveform`, `scope_grid` — oscilloscope
- `adsr_attack`, `adsr_decay`, `adsr_sustain`, `adsr_release` — envelope viz
- `vu_normal`, `vu_warn`, `vu_clip` — level meters

---

## 4. Shape tokens

| Token | Value | Use |
|-------|-------|-----|
| `rounding_xs` | 2 px | Step buttons, tiny chips |
| `rounding_sm` | 4 px | Chips, small buttons |
| `rounding_md` | 8 px | Cards, section frames |
| `rounding_lg` | 12 px | Windows, large panels |
| `rounding_full` | 999 px | Pill/badge shapes |

| Token | Value | Use |
|-------|-------|-----|
| `stroke_ui` | 1.0 px | Default border |
| `stroke_focus` | 1.5 px | Focused/hovered border |
| `stroke_active` | 2.0 px | Active/pressed border |
| `stroke_tier1` | 2.5 px | Tier 1 knob arc fill (more visual weight) |
| `stroke_tier2` | 2.0 px | Tier 2 knob arc fill |
| `stroke_tier3` | 1.5 px | Tier 3 knob arc fill |

---

## 5. Sizing tokens — control dimensions by tier

These define the hit-target and visual size for interactive controls per tier.
They are used by the component layer; panels never set sizes directly.

### Knob sizes

| Token | Allocated rect | Knob radius | Arc stroke | Tier |
|-------|---------------|-------------|------------|------|
| `knob_size_lg` | 64 × 88 px | 24 px | `stroke_tier1` | Tier 1 |
| `knob_size_md` | 44 × 64 px | 16 px | `stroke_tier2` | Tier 2 |
| `knob_size_sm` | 32 × 48 px | 11 px | `stroke_tier3` | Tier 3 |

### Button / chip sizes

| Token | Size | Tier |
|-------|------|------|
| `btn_size_lg` | min 56 × 36 px | Tier 1 |
| `btn_size_md` | min 40 × 24 px | Tier 2 |
| `btn_size_sm` | min 28 × 18 px | Tier 3 |
| `chip_height` | 22 px | All tiers (width is content-driven) |

### Fader sizes

| Token | Size | Tier |
|-------|------|------|
| `fader_track_w` | 8 px | — |
| `fader_h_lg` | 120 px | Tier 1 |
| `fader_h_md` | 80 px | Tier 2 |
| `fader_h_sm` | 48 px | Tier 3 |

---

## 6. Zoom factor

| Token | Default | Range | Description |
|-------|---------|-------|-------------|
| `zoom_factor` | 0.9 | 0.7 – 1.4 | Global egui `pixels_per_point` multiplier |

This is the **only** mechanism for adapting the UI to screen density or user
preference. It is persisted in user settings. See `06-window-scaling.md` for
full details.

---

## Token naming conventions

- All tokens are `snake_case`
- Color tokens never include a color name (use `accent`, not `teal_accent`)
- Size tokens always include a tier or scale suffix (`_lg`, `_md`, `_sm`)
- Surface tokens follow `bg_` prefix; text tokens follow `text_` prefix
- Domain tokens follow `domain_role` pattern (`fx_delay`, `seq_step_active`)

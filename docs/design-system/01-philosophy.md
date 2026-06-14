# Philosophy — Designing for a Musical Instrument

Most UI design advice targets productivity apps: dashboards, forms, settings
panels. A synthesizer is different. It is a **real-time performance instrument**
as much as a design tool. This changes the rules.

---

## The core tension

A synth UI has two distinct operating modes and they have conflicting needs:

| Mode | User goal | UI need |
|------|-----------|---------|
| **Patch editing** | Shape the sound deliberately | Dense information, fine control, discoverability |
| **Live performance** | React to music in real time | Fast targets, no cognitive load, zero accidental triggers |

Most existing soft-synth UIs optimise only for patch editing. The result: small
knobs with adjacent controls that are easy to mis-click under pressure, panels
that require hunting for the right parameter, and no visual hierarchy that
communicates "this matters right now."

The Forma design system addresses this by introducing a **control importance
hierarchy** that governs size, placement, and visual weight.

---

## The control importance hierarchy

### Tier 1 — Performance controls
**Definition:** Controls you touch *while music is playing.* Speed and accuracy
matter more than density. A 100ms window to hit the target.

**Examples in Forma:**
- Filter cutoff / resonance
- Master volume / expression
- Play / Stop / Record
- Main envelope decay / release
- Scene and patch recall

**Design rules:**
- Minimum hit target: **56 × 56 px** (finger-sized, not cursor-sized)
- Visual separation from adjacent controls: at least `sp_lg` (16px) gap
- Always visible regardless of panel layout — never hidden behind a tab
- Color and contrast: highest luminance/saturation, stands out on background
- Knob size: **Large** (see component catalog)

### Tier 2 — Sound design controls
**Definition:** Controls you adjust when programming a patch, between notes,
or during a session. Precision matters; speed less so.

**Examples in Forma:**
- Oscillator waveform, detune, unison
- LFO rate, depth, waveform
- FX parameters (drive, time, mix)
- Envelope attack, sustain
- Sequencer step values

**Design rules:**
- Standard hit target: **44 × 44 px** minimum
- Normal spacing (`sp_sm` / `sp_md`)
- Grouped by function, clearly labeled
- Knob size: **Standard**

### Tier 3 — Configuration controls
**Definition:** Set once per session, per patch, or rarely. Incorrect adjustment
during performance is not catastrophic.

**Examples in Forma:**
- MIDI channel, device selection
- Polyphony / voice mode
- Scale / key settings
- Technical calibration values

**Design rules:**
- Can be smaller: **32 × 32 px** minimum hit target acceptable
- Can live behind a secondary tab or collapsible section
- Lower visual weight (muted label color, smaller font)
- Knob size: **Small**, or replaced by a compact DragValue / ComboBox

---

## Applying the hierarchy in practice

When placing a new control, ask these three questions in order:

1. **When is this touched?** During play, during patch editing, or rarely?
   → Determines the tier.

2. **What breaks if it's mis-triggered?**
   A volume spike during a live set is catastrophic. Changing a MIDI channel is
   not. Higher consequence → more spacing from neighbors, larger target.

3. **Does it change the sound continuously or discretely?**
   Continuous (pitch, filter) → knob or fader, with visible current value.
   Discrete (waveform, mode, on/off) → chip selector or toggle, no knob.

---

## Visual weight and the "one glance" rule

A musician glancing at the screen for half a second should immediately see:
- What is currently playing (transport state)
- The most important sound-shaping control for the active patch
- Whether anything unexpected is active (red indicators, clipping, etc.)

Everything else is secondary. The hierarchy is encoded by:
- **Size** — larger = more important (Tier 1 > Tier 2 > Tier 3)
- **Contrast** — Tier 1 labels are full-brightness; Tier 3 are dimmed
- **Position** — Tier 1 controls cluster near center or lower-center (natural
  hand position); Tier 3 cluster at edges or behind tabs
- **Color accent** — active/playing state uses the theme accent color; Tier 3
  uses only neutral tones

---

## Consistency as a performance feature

In a live context, muscle memory matters. Controls should be in the same place
across patches, panels, and app modes. This means:

- **Never move a Tier 1 control** based on parameter state or patch content
- Filter section always shows cutoff + resonance in the same positions
- Transport always occupies the same strip
- Tab order is deterministic and learned

This is the reason the design system uses components (one definition of
"knob position in a row") rather than per-panel ad-hoc layouts.

---

## The accessibility floor

These minimums apply regardless of tier or aesthetic preference:

- **Text:** never below 9pt at 1.0 zoom factor (real pixel size after scaling)
- **Hit targets:** never below 32 × 32 px for any interactive control
- **Contrast:** label text on background must pass WCAG AA (4.5:1 ratio)
- **Zoom range:** the UI must remain fully functional between 0.7× and 1.4× zoom

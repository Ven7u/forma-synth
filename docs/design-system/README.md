# Forma Design System

A four-layer UI design system for Forma Synth, built on egui. Designed to be
extracted into a standalone crate (`forma-ui`) once shared across multiple
projects.

---

## Why this exists

The UI was accumulating magic numbers, duplicated layout code, and inconsistent
visual weight across panels. This design system gives every dimension, color,
and interaction pattern a single home so that consistency comes from the system
rather than from manually keeping things in sync.

It is also explicitly designed for a **musical instrument** — where controls
have different importance levels and the UI must remain usable during live
performance, not just during patch editing.

---

## Document map

| File | Contents |
|------|----------|
| [01-philosophy.md](01-philosophy.md) | Musical instrument UX principles; the control-importance hierarchy |
| [02-tokens.md](02-tokens.md) | **Layer 1** — All named constants: colors, spacing, typography, scaling |
| [03-primitives.md](03-primitives.md) | **Layer 2** — SynthFrame containers; layout helpers; `SynthUi` trait |
| [04-components.md](04-components.md) | **Layer 3** — Full component catalog with sizes, states, and usage rules |
| [05-patterns.md](05-patterns.md) | **Layer 4** — Composed patterns: knob rows, FX modules, cards |
| [06-window-scaling.md](06-window-scaling.md) | Window sizing strategy; global zoom factor; DPI handling |
| [07-implementation-plan.md](07-implementation-plan.md) | Phased migration roadmap with acceptance criteria |
| [08-ux-conventions.md](08-ux-conventions.md) | Field-tested rules for card layout, sizing, interaction, color, spacing |

---

## The four layers at a glance

```
Layer 4 — PATTERNS
  Composed, named UI regions: "oscillator card", "fx module", "sequencer row"
  Built entirely from Layer 3. No raw egui calls.

Layer 3 — COMPONENTS
  Reusable widgets: Knob, ToggleButton, ChipSelector, Fader, StepPad, ...
  Built from Layer 2 layout helpers and Layer 1 tokens.

Layer 2 — PRIMITIVES
  SynthFrame variants (section, bar, inset, screen)
  SynthUi extension trait (adds .synth_knob(), .synth_toggle(), ... to egui::Ui)
  Layout helpers (knob_row(), labeled_columns(), ...)

Layer 1 — TOKENS
  Colors, spacing scale, font sizes, rounding, stroke widths, zoom factor
  All in SynthTheme. No magic numbers anywhere else.
```

---

## Design for reuse

Everything in `ui/design/` is written against egui with no dependency on
Forma's audio engine types. When a second project needs the same components,
the migration path is:

1. Move `ui/design/` → `crates/forma-ui/src/`
2. Add `forma-ui` as a dependency in `forma` and the new project
3. Replace `crate::ui::design::` imports with `forma_ui::`

No logic changes required.

---

## Golden rule

> **Tokens are the only place where values live.**
> Components read from tokens. Patterns read from components.
> No panel hardcodes a color, font size, spacing value, or widget dimension.

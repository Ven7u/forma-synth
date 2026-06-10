# Implementation Plan

A phased migration roadmap. Each phase is independently shippable — the UI
remains functional throughout. No big-bang rewrite.

The phases are ordered to maximize early visible improvement while building
the foundation that later phases depend on.

---

## Phase 0 — Foundation (no visible change)

**Goal:** Set up the module structure and token extensions without changing
any existing behavior. Everything compiles, nothing looks different.

**Tasks:**
- [ ] Create `ui/design/` directory
- [ ] Create `ui/design/mod.rs` re-exporting everything
- [ ] Add `font_heading`, `font_body`, `font_value`, `font_small`, `font_micro`
      methods to `SynthTheme` (returns `FontId` with the base sizes from
      `06-window-scaling.md`)
- [ ] Add `knob_size_lg`, `knob_size_sm` to geometry tokens (Standard already
      exists implicitly as the current knob)
- [ ] Add `KnobSize` enum and `Tier` enum to `ui/design/mod.rs`
- [ ] Add `zoom_factor: f32` to the user settings struct (default 0.9),
      persisted but not yet applied
- [ ] Add `font_tier1_arc`, `font_tier2_arc`, `font_tier3_arc` color tokens
      to all three themes

**Acceptance:** `cargo check` passes. No visual change.

---

## Phase 1 — Window and scaling (immediate quality-of-life fix)

**Goal:** Fix the "can't see everything" problem. This is the highest-priority
user-facing fix and does not require the component system to be complete.

**Tasks:**
- [ ] Set `min_inner_size([720.0, 480.0])` in `NativeOptions`
- [ ] Query monitor available rect at startup; clamp initial window size to 90%
- [ ] Open maximized if clamped < minimum viable
- [ ] Apply `ctx.set_pixels_per_point(zoom_factor * native_ppp)` in the main
      render loop
- [ ] Handle `Ctrl +` / `Ctrl -` / `Ctrl 0` in the top-level `update()` for
      zoom adjustment
- [ ] Show current zoom level as a brief overlay on zoom change (fades after
      1.5s — store fade timer in app state)
- [ ] Persist window size, position, and zoom_factor to settings

**Acceptance:**
- App opens without overflowing any common laptop screen
- Ctrl + and Ctrl - visibly scale the entire UI
- Zoom preference is remembered across restarts

---

## Phase 2 — SynthUi trait skeleton + knob refactor

**Goal:** Establish the vocabulary that all future panel work uses.

**Tasks:**
- [ ] Create `ui/design/layout.rs` with `SynthUi` trait (empty method stubs
      returning `()` initially)
- [ ] Refactor `ui/widgets/knob.rs` into `ui/design/knob.rs` with:
      - `KnobSize` parameter (Large/Standard/Small with correct token dimensions)
      - `Tier` parameter affecting arc color
      - Interaction sensitivity scaled by KnobSize (larger = finer per-pixel)
      - All sizes from tokens (no hardcoded px values inside knob.rs)
- [ ] Implement `SynthUi::synth_knob()` calling the new Knob
- [ ] Implement `SynthUi::knob_row()` using `egui_flex`
      (add `egui_flex` to Cargo.toml)
- [ ] Add `egui_flex` dependency
- [ ] Keep old knob path working via re-export (so panel files don't break yet)

**Acceptance:**
- All existing knobs render identically (Standard size = old knob)
- `ui.knob_row()` available and lays out knobs correctly on resize

---

## Phase 3 — Font token migration

**Goal:** Remove all hardcoded font sizes. No visual change (zoom factor
compensates).

This is mechanical work but important — it's what enables consistent rescaling.

**Tasks (per file):**
- [ ] `ui/widgets/knob.rs` → `ui/design/knob.rs` (already done in Phase 2)
- [ ] `ui/oscillators.rs` — replace all `.size(N)` with `.font(theme.font_X())`
- [ ] `ui/modulation.rs`
- [ ] `ui/fx_chain.rs`
- [ ] `ui/sequencer_ui.rs`
- [ ] `ui/keyboard.rs`
- [ ] `ui/arp_walker.rs`
- [ ] `ui/eq_ui.rs`
- [ ] `ui/live_view.rs`
- [ ] `ui/mixer.rs`
- [ ] `ui/drum_machine_ui.rs`
- [ ] All remaining ui/*.rs files

**Acceptance:**
- `grep -r "\.size([0-9]" crates/forma/src/ui/` returns zero matches
- Visual appearance at zoom 1.0 is slightly larger text (base sizes are larger);
  visual appearance at zoom 0.9 is approximately same as before

---

## Phase 4 — Scroll zones and overflow fixes

**Goal:** No content clips or overflows regardless of window size.

**Tasks:**
- [ ] Audit every panel for content that can exceed its allocated height;
      wrap with `ScrollArea` where data-driven (lists); add more space where
      bounded by design
- [ ] FX chain: add horizontal `ScrollArea` when chain exceeds panel width
- [ ] Patch browser, Scene browser, History: verify scroll areas work
- [ ] Sequencer: verify horizontal scroll for long sequences
- [ ] Document the "scrollable vs bounded" rule in code comments on each scroll area

**Acceptance:**
- At minimum window size (720 × 480), no panel clips or overlaps another
- At very small window size, panels become scrollable rather than broken

---

## Phase 5 — Oscillators panel migration (reference implementation)

**Goal:** Migrate one panel entirely to the new design system as the reference
others will follow. Oscillators chosen because it has the most controls.

**Tasks:**
- [ ] Implement remaining `SynthUi` methods: `synth_toggle`, `chip_selector`,
      `section_header`, `chip_row`
- [ ] Implement `SynthFrame::tier1()` variant
- [ ] Implement `TieredCard` pattern
- [ ] Rewrite `ui/oscillators.rs` using only design system vocabulary:
      - SectionCard with tier1 frame for each oscillator
      - TieredCard layout: waveform chips (T3) → detune/pw knobs (T2) →
        (no Tier 1 on oscillators — filter/volume are T1 in other panels)
      - knob_row for all knob groups
      - chip_row for waveform selector
      - synth_toggle for unison, FM, ring, sync
- [ ] Verify layout on window resize (no overflow, no clipping)
- [ ] Take a screenshot for design documentation

**Acceptance:**
- Oscillator panel passes visual review
- All knob/button sizes match the tier specification
- Panel is fully functional

---

## Phase 6 — Remaining panel migrations

Migrate each panel, one at a time. Order by visual impact:

- [ ] Filter / Modulation (`ui/modulation.rs`) — has Tier 1 controls (cutoff)
- [ ] FX Chain (`ui/fx_chain.rs`) — FxModule pattern; add egui_dnd reordering
- [ ] Mixer (`ui/mixer.rs`) — FaderColumn pattern
- [ ] Sequencer (`ui/sequencer_ui.rs`) — StepPad component
- [ ] Arp & Walker (`ui/arp_walker.rs`)
- [ ] EQ (`ui/eq_ui.rs`) — canvas is custom; only the control strip uses DS
- [ ] Drum Machine (`ui/drum_machine_ui.rs`) — StepPad + FaderColumn
- [ ] Live View (`ui/live_view.rs`)
- [ ] MIDI panel (`ui/midi.rs`) — mostly Tier 3 controls
- [ ] Keyboard (`ui/keyboard.rs`) — custom draw; only surrounding controls use DS

---

## Phase 7 — Polish

Only after all panels are migrated.

**Tasks:**
- [ ] Add `egui_animation` for mode transitions (Studio ↔ Live ↔ Drum Machine)
- [ ] Animate panel show/hide in dock
- [ ] Animate LFO rate indicator (pulsing at LFO rate)
- [ ] Animate envelope playhead on active voice
- [ ] Review contrast ratios for all three themes (WCAG AA check)
- [ ] Review all Tier 1 control sizes against the 56×56 minimum hit target rule
- [ ] User test: can a musician change filter cutoff within 1 second of opening
      any patch?

---

## Future: extracting forma-ui crate

When a second project needs the design system:

1. `mv crates/forma/src/ui/design crates/forma-ui/src/`
2. Add `[package]` to new crate, `egui` and `egui_flex` as public deps
3. In `forma`: replace `crate::ui::design::` with `forma_ui::`
4. Publish or path-dep as needed

No changes to the design system code itself — it was written without Forma
internals from the start.

---

## Effort estimate (rough)

| Phase | Estimated effort | Risk |
|-------|-----------------|------|
| 0 — Foundation | 1–2h | Low |
| 1 — Window/scaling | 2–3h | Low |
| 2 — SynthUi + knob | 4–6h | Medium (knob refactor) |
| 3 — Font migration | 3–5h | Low (mechanical) |
| 4 — Scroll zones | 2–3h | Low |
| 5 — Oscillators | 4–6h | Medium |
| 6 — Remaining panels | 12–20h | Low per panel |
| 7 — Polish | 4–8h | Low |

**Total: ~35–50h of focused implementation**, fully interruptible at any phase
boundary.

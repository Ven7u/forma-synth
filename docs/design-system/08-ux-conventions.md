# UX Conventions

Field-tested rules learned during Phase 5/6 panel migrations. Each rule is a
default to apply — bend it only when there's a written reason in the commit
message.

These conventions are not a separate aesthetic layer; they encode the lessons
we collected by iterating on real panels (Oscillator rewrite, Mixer redesign,
sequencer toolbar fixes). When a new panel migration violates one of these,
the result almost always reads as broken. When the design system itself
enforces them — via pattern functions, component defaults, or token choices —
the panel author can't accidentally get it wrong.

---

## 1. Card layout

### 1.1 Cards in a row share height; cards stacked vertically share width.

A horizontal strip of cards reads as a row only if heights match. A vertical
stack reads as one column only if widths match. Mismatch breaks the "row" or
"column" gestalt and the eye registers each card as an isolated rectangle
instead of part of a larger composition.

### 1.2 Equal heights come from content, not `set_min_height`.

Forcing a min-height creates dead space inside short cards. Redesign content
(use Large component variants, add a sub-section, vary spacing) until natural
heights match. Reserve `set_min_height` for truly variable content (data-driven
lists where height genuinely varies with state).

### 1.3 Card height comes from one "anchor" component.

Most cards have one component that defines vertical extent (a fader, a knob,
a meter). Pick it deliberately. Smaller controls cluster around the anchor.
In the Mixer:

- CHANNELS card: Large fader is the anchor.
- MASTER card: Large fader is the anchor.
- VOICE & SAFETY card: Large GLIDE knob is the anchor (placed center band).

All three cards end up with similar heights because their anchors are similar
extents.

### 1.4 Distinguish "important" cards by decoration, not size.

The master strip uses the same fader length as channel strips; what marks it
as master is `SynthFrame::tier1`'s accent border and a paired LevelMeter.
Bigger ≠ more important — *different* is more important. A larger fader on the
master strip would also work, but it breaks rule 2.1 (same role → same size)
and looks heavy.

### 1.5 No dead space inside a card.

If a card has visible empty space below or beside its content, the design is
wrong. Either: a) use larger component variants, b) add a sub-section that
uses the space, c) reconsider whether the card belongs here at all (split it
out into its own tab, or merge it into a more loaded sibling card).

### 1.6 Card content stacks vertically by default.

Heading at top, content groups stack downward, each group has its own small
caption when more than one lives in the card. The Mixer's VOICE & SAFETY
card has three sub-groups (VOICE / GLIDE / LIMITER), each with its own caption
and spacing between them.

### 1.7 Three indent levels is a maximum.

A card has heading → group caption → controls. More nesting than that means
rethink the card — either split it, or the wrap is the wrong pattern.

---

## 2. Component sizing

### 2.1 Same role → same size.

Channel fader and master fader = same length. Cutoff knob and resonance knob
= same size. Visual uniformity across same-role components beats tier-strict
sizing. The eye reads a row of identical sizes as "these are the same kind of
thing"; mixed sizes read as a hierarchy where there shouldn't be one.

### 2.2 Tier shows through color and grouping, not always size.

Tier 1 controls *can* be Large, but they don't have to be. The accent arc
color (`knob_tier1_arc`), accent-bordered card frame, paired meters, and
central placement carry the "tier 1" signal independently. Reserve Large
for when the available vertical space actually wants to be filled.

### 2.3 Pick component size by visual weight needed, not strict tier rules.

A Tier 2 GLIDE knob can be Large if it's the only knob in its card and needs
to fill the card's vertical band. Document the choice in a comment if it
deviates from the tier default — future readers should see the reason.

### 2.4 Pair related components at compatible sizes.

A Standard fader (80 px) pairs with a Standard LevelMeter (80 px). A Large
fader (120 px) wants Large meters too — or two stacked Standard meters that
visually add up. Mixing sizes in a paired strip looks unintentional.

---

## 3. Tab and pane structure

### 3.1 Sibling tabs share a pane; separate concerns go in separate panes.

Oscillators + Mixer = sibling tabs (related, click between them). Oscillators
+ Modulation = separate panes (different concerns, both visible at once).
The egui-dock split structure encodes this — `vec![Tab::A, Tab::B]` for
siblings, `split_below` / `split_right` for separate panes.

### 3.2 Don't pack unrelated concerns into one card.

"VOICE & SAFETY" is on the line because voice mode and limiter both don't
fit elsewhere, but they're not really one logical group. When a panel has
unrelated content, separate cards (or separate tabs) read better than one
card with mixed concerns. If we accumulate more "settings" controls, they
should split off into their own card or tab.

### 3.3 Adjust dock split ratios to fit content.

If a panel needs more vertical space, the dock proportions are the right
place to fix it — not by shrinking the content. `CARD_H` (the per-panel card
height) and the dock split must agree: dock ratio × dock height ≥ `CARD_H` +
margins, or the bottom of the card clips.

---

## 4. Interaction

### 4.1 `ScrollArea::drag_to_scroll(false)` whenever scrollable content is interactive.

Step grids, knob columns, fader strips — if the user drags to edit, they
shouldn't accidentally scroll. Wheel and scrollbar thumb still scroll. This
is the default for any scroll wrapper around a grid of editable cells.

### 4.2 Conditional scroll wrap.

`ScrollArea` only mounts when natural content width exceeds the panel. At
wider panels the wrap is invisible. Pre-compute the natural-width; branch
on `needs_scroll`. The sequencer's step grids use this — at 32 steps on a
720 px window the scroll wrap mounts; at 32 steps on a 1400 px window it
doesn't.

### 4.3 No scroll-wheel-edits-value handlers inside scrollable areas.

A `bar_resp.hovered()` + `inp.smooth_scroll_delta.y` pattern that edits a
value conflicts with any parent ScrollArea. Use drag for value edits;
reserve the wheel for scrolling.

### 4.4 `horizontal_wrapped` for control bars that might overflow.

Toolbars, button strips, anything content-driven and length-uncertain:
`horizontal_wrapped` so they degrade gracefully on narrow windows.
`horizontal` is for fixed-N rows you've sized to fit.

### 4.5 Hit targets respect the tier minimums.

Tier 1: 56 × 56 px. Tier 2: 44 × 44 px. Tier 3: 32 × 32 px. The component
catalog encodes this — `KnobSize::Large` allocates 64 × 88 px, etc. If a
panel ever shrinks a Tier 1 control below 56 × 56, something is wrong at
the layout level, not at the component level.

---

## 5. Color and feedback

### 5.1 Active fill needs `text_on_accent`.

Light text on a saturated accent fill is illegible — we discovered this with
the Winamp theme's bright green active toggle. The `text_on_accent` token
(very dark, theme-specific) is the correct color for text inside an active
toggle/chip. Components in the design system already do this; never override
it back to `text_primary`.

### 5.2 Card borders use `accent_dim`, not `accent`.

A full-saturation perimeter reads as "this widget is pressed." Cards are
passive surfaces, not interactive widgets. `SynthFrame::tier1` already uses
`accent_dim` per this rule.

### 5.3 Stroke widths: 1.0 baseline, 1.5 emphasis, 2.0 active widget.

- `stroke_ui` (1.0) — regular borders.
- `stroke_focus` (1.5) — cards with emphasis (Tier 1 frame), knob arcs on
  hover, peak-hold lines.
- `stroke_active` (2.0) — reserved for "this widget is being interacted
  with right now" — knob being dragged, button being pressed.

Using `stroke_active` for a card perimeter reads as "card is being clicked,"
which it isn't.

### 5.4 Domain accents > generic accent for domain-specific toggles.

SYNC uses `accent_hard_sync`, FM uses `accent_fm`, RING uses `accent_ring`,
LIM uses `accent_limiter`. Pass them via the toggle's `accent_color: Option<Color32>`
override. This is how a single active toggle row stays color-coded by
function instead of monotonously green.

---

## 6. Spacing

### 6.1 Always use the spacing scale, never literal pixels.

`theme.sp_xxs` through `theme.sp_xxl`. Anywhere `ui.add_space(4.0)` or
`Margin::same(8)` appears in panel code, replace it with a token. The
Phase 5/6 compliance gate enforces this.

### 6.2 Inside a card: `sp_sm` between groups, `sp_md` between sub-sections.

A card has heading → `sp_sm` → first group → `sp_md` → second group →
`sp_md` → third group. The mixer's VOICE & SAFETY card follows this exactly.

### 6.3 Step pads use `sp_xxs` (2 px) gap, not the global item_spacing.

Per `04-components.md` §StepPad. Override
`ui.spacing_mut().item_spacing.x = theme.sp_xxs` inside the step grid
horizontal. The global default (`sp_sm` = 8 px) is too generous for
densely-packed grids.

### 6.4 Outer card gaps use `sp_sm`.

Cards in a horizontal row are separated by `sp_sm`. Card outer margins
(set by `SynthFrame::section`) are `sp_xs`. Combined, you get a clean
gutter between cards.

---

## 7. Token compliance gate

### 7.1 Each migrated panel must drive five greps to zero.

Listed in `07-implementation-plan.md` §"Token compliance":

```bash
grep -E '\.size\([0-9]|FontId::(proportional|monospace)\([0-9]' <file>
grep -E 'Color32::(from_(rgb|gray|rgba)|WHITE|BLACK|GRAY|RED|GREEN|BLUE)' <file>
grep -E '\.add_space\([0-9]' <file>
grep -E 'Stroke::new\([0-9]+\.[0-9]' <file>
grep -E '(CornerRadius|Rounding|Margin)::same\([0-9]' <file>
```

No exceptions without a one-line comment explaining why a token doesn't apply.

### 7.2 Token-derived expressions are allowed.

`Color32::from_rgba_premultiplied(accent.r() / 5, …)` is on-system because
it derives from a token. Annotate with a comment so it's clear:

```rust
// Token-derived: interpolation between theme.meter_green and theme.meter_clip.
```

The compliance grep doesn't know it's derived; the comment is the reader's
hint.

---

## How the design system embraces these rules

The design system itself is the first line of defense — if it bakes a rule
into a pattern or component default, panel authors can't accidentally get it
wrong.

| Rule | Where it's enforced |
|------|--------------------|
| 1.1, 1.6, 1.7 | `SectionCard`, `TieredCard` patterns bake the heading + body structure |
| 1.4, 5.2 | `SynthFrame::tier1` uses `accent_dim` + `stroke_focus` so a tier-1 card automatically inherits the soft accent border |
| 2.1, 2.2 | `KnobSize::Standard` is the default — deliberate `KnobSize::Large` requires the author to think about why |
| 2.4 | `FaderColumn` pattern composes Fader + LevelMeter so they automatically pair at compatible sizes |
| 3.1 | `default_dock_state` codifies sibling-vs-separate-pane choices for every tab |
| 4.1 | When we build a `ScrollableGrid` Layer 4 pattern, it'll set `drag_to_scroll(false)` automatically |
| 4.5 | Component min_rect tokens (`knob_size_*`, `btn_size_*`) encode the tier hit-target floor |
| 5.1 | `toggle.rs` and `chip.rs` hardcode `text_on_accent` for active state |
| 5.3 | `SynthFrame::*` variants each pick the correct stroke width — panel can't override |
| 5.4 | `synth_toggle`'s `accent_color: Option<Color32>` parameter exists exactly so domain accents can be passed in |
| 6.1, 6.3 | `theme.sp_*` tokens are the only spacing surface; the compliance grep catches literals |
| 7.* | `07-implementation-plan.md` Phase 5/6 acceptance criteria |

---

## Bending the rules

Each rule above is a default, not a hard prohibition. Real cases will arise
where the right answer breaks one of them — that's fine. The convention is:

- **Comment the deviation in code.** Future readers should see why the rule
  was bent here.
- **Reference the rule by number.** `// Deviates from §2.1 because …`
- **Mention it in the commit message.** Reviewers can challenge the choice
  before it lands.

If you find yourself bending the same rule across three panels, the rule
probably needs updating — open a PR against this doc.

---

## Appendix: changelog of lessons

Updates to this doc track Phase 6 migration lessons. Each entry: a one-line
description of what was learned and which rule it added or changed.

- **2026-06**: Initial draft from Mixer redesign. Rules 1.1–1.7, 2.1–2.4,
  5.1–5.4 distilled from the Oscillator/Mixer iteration sessions.

# Composer Layer — Design Document

**Status:** Design / pre-implementation
**Scope:** `forma` (app), `forma-engine` (`generative/`), `forma-common`
**Related:** PR #36 (audio loop station), `doc/ambient-architecture-proposal.md` and
`doc/markov-music-system.md` in the `the-synth` repo (earlier self-composing ambient experiment)

---

## 1. Motivation

`the-synth` has a self-composing ambient module: Markov harmonic / rhythmic / melodic chains,
voice roles, moods, a phrase counter and a timeline. It runs its own 4-track engine
(`forma-ambient`) and is separate from the instrument.

Since then `forma` has gained the building blocks of a real rig: a chord sequencer, a note
sequencer, an arpeggiator with scale walker, an 8-lane drum machine, a 4-track engine and (PR #36)
an Ableton-style audio loop grid. None of it is connected to a composer.

Goal: **a composer layer that sits on top of these modules and drives them**, autonomous
enough to play a piece by itself, but always playable — the user can tune the mood and take
over any part at any time.

### Non-goals
- Not a DAW: no arrangement timeline, no piano roll.
- Not a replacement for manual use. With the composer off, every module behaves exactly as today.
- Not a trained-model system. Rules + small Markov tables, deterministic given a seed.

### Current state (what exists)

| Piece | Where | State |
|---|---|---|
| Chord sequencer (24 steps: degree, type, voicing, octave, velocity, probability, voice-lead) | `forma/src/sequencer.rs` `ChordSeqState` | Used by app |
| Note sequencer, Chord KB mode | `sequencer.rs` | Used by app |
| Arpeggiator + scale walker | `forma-engine/src/arp.rs` | Used by app |
| Drum machine (8 lanes × 16, patterns A–D, Euclidean) | `forma-engine/src/drum.rs` | Used by app |
| 4-track engine, send buses | `forma-engine/src/multi.rs` | Engine only |
| Beat clock | `forma-common/src/beat_clock.rs` | Used by app |
| Markov / mood / role / timeline / macros | `forma-engine/src/generative/` | **Compiled but not wired into the `forma` app** |
| Audio loop grid (12 pads, bar-quantized, audio-thread) | PR #36 `audio_loops.rs` | **Open, unmerged** |

The sequencer state is shared with the UI through `Arc<Mutex<…>>` plus atomics
(`SequencerHandle`). The composer can use that same surface, so it needs no new audio-thread machinery.

---

## 2. Design Principles

1. **The composer writes into existing modules; it does not replace them.** Anything it
   produces is visible and editable in the normal UI.
2. **One shared musical context.** Tracks do not listen to each other. They all read one
   `HarmonyContext`, which is what keeps them from clashing.
3. **Compose ahead, never at the deadline.** The composer runs on a control thread and fills
   the next bar or phrase in advance. The audio thread is untouched.
4. **Everything is a lock-or-follow decision.** Each track has an autonomy level and each step
   or lane can be locked. The composer works around what the user has touched.
5. **Small modules behind traits.** Every stage (harmony, rhythm, melody, density, form) is
   swappable and testable in isolation, because the right formula is not known up front.
6. **Deterministic by seed.** A seed plus mood settings reproduces a piece, which makes tuning
   and bug reports possible.
7. **No allocation or locking on the audio thread**, consistent with `dsp-guidelines`.

---

## 3. Architecture

```
            ┌────────────────────────── Control surface ──────────────────────────┐
            │ Mood axes · Macros · Autonomy per track · Locks · Seed · Section     │
            └───────────────────────────────┬──────────────────────────────────────┘
                                            ▼
                                  ┌───────────────────┐
   BeatClock ───────────────────► │     Composer      │ ◄── Form / Timeline
 (bar, beat, phrase)              │  (control thread) │
                                  └─────────┬─────────┘
                       writes HarmonyContext │  once per bar / phrase
        ┌───────────────┬───────────────┬───┴────────────┬───────────────────┐
        ▼               ▼               ▼                ▼                   ▼
  Harmony Planner  Pulse generator  Melody generator  Drum generator    Texture/Pad
  (chord seq)      (arp config)     (note seq)        (pattern slots)   (chord KB/voicing)
        │               │               │                │                   │
        ▼               ▼               ▼                ▼                   ▼
   ChordSeqState    ArpShared       NoteSeqState     DrumTrackState     per-track patch
        └───────────────┴───────┬───────┴────────────────┴───────────────────┘
                                ▼
                  existing engine (tracks, buses, limiter)
                                ▼
                  Capture ──► Loop grid pads (PR #36)
```

### 3.1 HarmonyContext

The single source of musical truth, rebuilt by the Harmony Planner and read by every generator.

```rust
pub struct HarmonyContext {
    pub root: u8,                 // 0..11
    pub scale: ScaleType,
    pub chord_now: ChordInfo,     // degree, type, tones as pitch classes
    pub chord_next: ChordInfo,    // lookahead so generators can anticipate
    pub bar: u32,
    pub beat_in_bar: u8,
    pub phrase: u32,
    pub section: SectionId,
    pub density_budget: f32,      // 0..1, shared across tracks (see 3.5)
    pub mood: MoodAxes,
    pub seed: u64,
}
```

### 3.2 Harmony Planner

- Owns key, scale and the progression.
- Plans N bars ahead (N = phrase length, default 4 or 8) and writes the result into
  `ChordSeqState`: `degrees`, `chord_types`, `voicings`, `probabilities`, with `voice_lead` on.
- Candidate source: the existing `HarmonicChain` (function-level Markov), constrained by mood
  (tension favors secondary dominants and suspended chords, calm favors I–vi–IV–V style loops).
- At phrase boundaries it may take a wider move (relative major/minor, a fifth up), as in
  the `PhraseCounter` design.
- Key changes are rare and quantized to phrase boundaries; the loop grid needs this (see §6).

### 3.3 Role generators

Each generator implements:

```rust
pub trait RoleGenerator: Send {
    fn role(&self) -> VoiceRole;
    /// Called by the Composer ahead of the bar; writes into the module state it owns.
    fn compose_bar(&mut self, ctx: &HarmonyContext, locks: &LockMask, rng: &mut Rng);
}
```

| Role | Drives | Behavior |
|---|---|---|
| **Foundation** | Note seq / mono track | Root and fifth, long gates, few notes, low register |
| **Pad** | Chord sequencer voicing / chord KB | Chord tones, slow, wide voicings |
| **Pulse** | Arp (`ArpShared`, scale walker) | Picks mode, octave range, clock division and gate from `chord_now` + density |
| **Melody** | Note sequencer | Motif engine (§3.4) |
| **Drums** | Drum patterns A–D | Euclidean + density rules (§3.6) |
| **Texture** | Spare track / FX macros | Sparse events, shimmer/crystallizer macros |

A generator only sees the context and its own lock mask, so adding or removing a role never
touches the others.

### 3.4 Melody maker

Hardest part; start conservative, improve behind the same trait.

- **Motif first.** Generate a 2–4 note rhythmic-melodic cell, store it, and *reuse it with
  variation* (transpose by degree, invert, skip a note, shift rhythm). Repetition with variation
  is what separates "composed" from "random walk".
- **Constraints.** Strong beats prefer chord tones from `chord_now`; weak beats prefer scale
  tones; a leap is followed by a step in the opposite direction; range is bounded by role.
- **Lookahead.** The last note of a bar targets a tone of `chord_next`.
- **Markov as flavor, not backbone.** The existing `MelodicChain` supplies interval
  preferences; the motif engine decides structure.
- **Rests are first-class.** Mood density sets rest probability; ambient wants silence.
- Output goes into `NoteSeqState` (it has per-step probabilities already) so the melody
  is visible and editable.

### 3.5 Density budget

Without coordination, six tracks all get busy at once. A shared `density_budget` (0..1)
set by mood and section is **spent** by generators in priority order:

1. Foundation, 2. Pad, 3. Pulse, 4. Drums, 5. Melody, 6. Texture.

Each reserves a share per bar. If the budget is spent, lower-priority roles thin out
(more rests, longer divisions). A section peak raises the budget; a release lowers it.

### 3.6 Drum generator

- Per lane: Euclidean `hits/steps/offset` driven by density, with fixed role rules
  (kick anchors the downbeat, hats fill the grid sparsely, snare/clap on backbeats when
  energy is high).
- Writes into pattern slots A–D: A = base, B = variation, C = fill, D = sparse/break. The
  composer switches slots at phrase boundaries, as a human would.
- Ambient default is sparse: low hit counts, ghost velocities, optional full mute in intros.

### 3.7 Form / Timeline

Reuses the existing `Timeline` / `TimelineSection` idea: an ordered list of sections
(intro → build → peak → release → outro, or a loop for endless ambient). A section holds
*targets only*:

```rust
pub struct Section {
    pub bars: u32,
    pub mood_target: MoodAxes,
    pub density_target: f32,
    pub enabled_roles: RoleMask,
    pub macro_targets: Vec<MacroTarget>,
}
```

Sections never contain notes. They interpolate mood and macros over their length, so
transitions are smooth and cheap. A "drift" mode random-walks the targets slowly for endless
pieces with no fixed form.

### 3.8 Mood and macros

Mood is a small set of axes, each 0..1:

| Axis | Affects |
|---|---|
| **Energy** | Tempo feel, density budget, drum intensity, arp division |
| **Brightness** | Scale/mode (Lydian ↔ Aeolian), filter cutoff macros, register |
| **Tension** | Chord extensions, suspensions, harmonic movement rate |
| **Density** | Note/rest ratio, number of enabled roles |
| **Space** | Reverb/shimmer/crystallizer sends, gate lengths |

The axes map onto the existing `MacroParam` / `SceneMacro` machinery so patch-level effects
(cutoff, FX sends) move together with the musical rules. The user plays these knobs, with
MIDI learn available because they are ordinary parameters.

### 3.9 Autonomy and locks

The mechanism that makes the system playable as well as autonomous.

Per track:

| Level | Behavior |
|---|---|
| **Manual** | Composer never writes to this track |
| **Assist** | Composer proposes at phrase boundaries; user can accept or ignore |
| **Auto** | Composer writes freely every bar/phrase |

Per step / lane / bar there is also a **lock**. Any edit the user makes in the UI sets a lock
on that cell automatically; the composer composes around locked cells (treats them as fixed
context). A track can be **frozen**: stop composing, keep playing the current pattern.

---

## 4. Relationship to the Loop Grid (PR #36)

PR #36 records the **rendered audio** of the focused track into 12 bar-quantized pads. That
fits the composer well and the two should stay decoupled:

- The composer decides *what is played*; the loop grid captures *what was heard*.
- **Capture workflow.** While the composer plays a track, the user taps a pad; the audio loop
  freezes it. The composer can then evolve (or mute) that track while the loop keeps playing —
  a layer the composer cannot change. This is the Ableton-style "play, capture, build up" loop.
- **Commit to tempo and key.** Audio loops bake tempo and pitch. The composer therefore exposes
  `key_change_allowed` and quantizes any key change to phrase boundaries; while loops are
  playing, key changes are either disabled or restricted to consonant moves (relative
  major/minor), configurable.
- **Song-level length.** Pad length (1/2/4 bars) should match the composer's phrase length
  so loops always end on a phrase boundary.
- **Composer-triggered capture (later).** In Auto, the composer may arm a pad at the start of a
  phrase when a section peaks, so good moments are kept automatically.
- **MIDI capture (later).** Independently, log the notes the composer writes into a MIDI
  clip. That gives editable, tempo-independent clips and removes the limits of audio loops. It
  is a separate feature and not required for v1.

No code in the loop grid needs to change for v1; the composer only needs the pad `tap()`
atomics already exposed by `LoopGridAtomics`.

---

## 5. Threading and Timing

- The composer runs on a **control thread** (same pattern as the existing sequencer thread),
  ticked by the `BeatClock`. It composes the **next** bar when the current bar is at beat 3,
  so writes land before the sequencer reads them.
- Writes use the existing `Arc<Mutex<…State>>` handles for patterns (held briefly) and atomics
  for scalar settings. No new audio-thread state.
- **Race with user edits:** an edit sets a lock first, then writes; the composer reads the
  lock mask under the same mutex before writing.
- **Determinism:** each generator derives its RNG from `(seed, bar, role)`, so seeking or
  restarting reproduces the same bar.

---

## 6. Data and Persistence

- `ComposerState` (seed, mood axes, timeline, autonomy levels, locks, generator choices) is
  serialized with the scene/patch JSON used by the app, so a session can be saved, reloaded
  and re-seeded.
- The generated patterns themselves are already stored by the sequencers' existing
  persistence, so "freeze this piece" is just "set all tracks to Manual".

---

## 7. UI Sketch (Live mode)

- A **Composer strip** above or beside the loop grid: Start/Stop, seed (dice button to
  re-roll), section indicator with progress, and the five mood sliders.
- Per-track chip: Manual / Assist / Auto, plus a snowflake for freeze.
- Locked cells shown with a small lock glyph in each existing sequencer view.
- Design-system tokens only, like PR #36.

---

## 8. Phases

| Phase | Deliverable | Result you can hear |
|---|---|---|
| **0** | Wire `HarmonyContext` and `BeatClock` into a Composer thread with no generators | Nothing audible; plumbing and tests |
| **1** | Harmony Planner → `ChordSeqState` | Self-playing chord progressions |
| **2** | Pulse generator (arp) + Pad + Foundation roles | A full self-playing ambient bed |
| **3** | Mood axes → macros; autonomy and locks UI | Playable: tune the mood, take over tracks |
| **4** | Melody maker (motif engine) | Melodies that develop |
| **5** | Drum generator + pattern-slot switching | Sparse rhythm layer |
| **6** | Form/Timeline + drift | Pieces with an arc |
| **7** | Loop-grid integration (auto-capture) and MIDI capture | Build-up performances |

Each phase is shippable on its own and leaves the manual instrument unchanged.

---

## 9. Open Questions

1. Should the composer live in `forma-engine::generative` (reusing the existing, currently
   unwired code) or in a new `forma-composer` crate? Recommendation: new module in
   `forma-engine::generative`, reusing `HarmonicChain` / `MelodicChain` / `Timeline` types,
   behind the `RoleGenerator` trait.
2. Should PR #36 merge before phase 7? The composer phases 0–6 do not depend on it.
3. Tempo: should the composer ever change BPM, given audio loops commit to one tempo?
   Recommendation: no, tempo is user-owned.
4. How many tracks does Assist mode propose for at once? Recommendation: one per phrase
   to avoid noise.
5. Is MIDI capture (§4) wanted as a standalone feature before the loop integration?

---

## 10. Risks

| Risk | Mitigation |
|---|---|
| Melody sounds random | Motif reuse + chord-tone constraints; keep Melody optional |
| Tracks clash harmonically | Single `HarmonyContext`; all tonal output derived from it |
| Everything plays at once | Density budget with priorities |
| User edits overwritten | Auto-lock on edit; Manual/Assist levels |
| Audio loops conflict with key change | Phrase-quantized, restricted key changes while loops play |
| No audio device in CI/sandbox | Generators are pure functions of `(ctx, seed)`; test as data, check output by ear separately |

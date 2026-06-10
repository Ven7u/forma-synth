# Window Sizing & Global Scaling Strategy

This document covers three related but distinct concerns:
1. How the window opens and adapts to the screen it's on
2. How global zoom factor works as the master scale lever
3. How to migrate away from the current hardcoded font sizes

---

## The current problem

```
Current state:
  Window: 1400 × 860 px fixed (no min, no max, no screen-awareness)
  Zoom:   none (defaults to 1.0 with no user control)
  Fonts:  7–13 pt scattered as magic numbers (effectively hardcoded scaling)
```

The result:
- On a laptop with 150% OS scaling, logical screen width ≈ 1280 pt → window is
  wider than the screen before the user has done anything
- Fonts manually shrunk to 7–9 pt to fit content → unreadable on retina/HiDPI
- No way for the user to scale the UI (Ctrl +/− does nothing)
- No single place to change "the scale" if needed

---

## The fixed model: three independent scales

```
  [Screen logical size]          ← given by OS; you read it, don't fight it
         ↓
  [Global zoom_factor]           ← your master lever; user-adjustable; persisted
         ↓
  [Per-widget token sizes]       ← design intent; set once at comfortable values
```

Each layer has one job. Mixing them up (using per-widget sizes as a scale lever,
ignoring screen size) is the current situation.

---

## 1. Window opening strategy

### Rule: never open larger than the available screen

At startup:
1. Query the monitor's available area (screen size minus taskbar/dock).
2. Apply a preferred size (e.g. 1400 × 860), but clamp to 90% of available.
3. If the clamped size is smaller than the minimum viable layout, open
   maximized instead.

```
preferred = (1400, 860)
available = monitor.available_rect()
initial   = min(preferred, available * 0.90)

if initial.width < MIN_VIABLE_WIDTH or initial.height < MIN_VIABLE_HEIGHT:
    open maximized
else:
    open at initial size, centered
```

### Minimum viable window size

The smallest window where the app is still musically usable:
- Transport bar always visible: ~52 px
- One panel (oscillators or filter) visible: ~280 px
- Keyboard visible: ~100 px
- Total minimum height: **~480 px**
- Minimum width (one panel + oscilloscope): **~720 px**

Set these as `min_inner_size` in `eframe::NativeOptions`. Below this, the OS
prevents the user from shrinking further — content does not clip or overlap.

### Persisting window state

Save and restore:
- Last window size
- Last window position
- Last zoom factor

This is standard desktop app behavior. The window opens where the user left it.

---

## 2. Global zoom factor

### What it is

`egui::Context::set_pixels_per_point(ppp)` multiplies the entire UI scale.
At 1.0, one logical point = one pixel (on a 1× display). Setting it to 1.5
makes everything 50% larger; 0.8 makes everything 20% smaller.

Crucially: this is a **crisp rescale**, not a bitmap zoom. Text re-renders at
the new size, curves are redrawn, everything stays sharp.

### How zoom_factor works in practice

```
zoom_factor = user_preference (persisted setting, default 0.9)

At startup:
  ctx.set_pixels_per_point(zoom_factor * native_pixels_per_point)
                                         ↑ the OS DPI factor (1.0 on 96dpi,
                                           2.0 on retina, 1.5 on 150% scaling)

Result: font sizes in tokens are logical pt values that scale with both
        the OS display setting AND the user's zoom preference.
```

### Default zoom factor

Given the current content density, a default of **0.9** is recommended.
This gives ~10% more screen real estate than default egui sizing without
making anything too small, and leaves room for the user to zoom in (Ctrl +)
if they want larger controls.

### User zoom controls

| Action | Effect |
|--------|--------|
| Ctrl + `+` | zoom_factor += 0.05 (up to 1.4 max) |
| Ctrl + `-` | zoom_factor -= 0.05 (down to 0.7 min) |
| Ctrl + `0` | Reset to default (0.9) |

These shortcuts must be handled in the top-level event loop, not per-panel.

### Persistence

Store `zoom_factor` in the user's settings file (same place as window size).
It is a per-user preference, not a per-patch setting.

---

## 3. Font migration from magic numbers to tokens

### Current state

Fonts are hardcoded throughout the panel files:
```
// Scattered across oscillators.rs, modulation.rs, fx_chain.rs, etc.:
RichText::new("OSC 1").size(11.0).italics()
RichText::new("9.5 kHz").size(9.0)
RichText::new("A3").size(7.0)
FontId::proportional(10.0)
FontId::monospace(8.0)
```

### Target state

All font sizes come from the theme token:
```rust
// In any panel:
RichText::new("OSC 1").font(theme.font_heading())
RichText::new("9.5 kHz").font(theme.font_value())
RichText::new("A3").font(theme.font_micro())
```

Where `theme.font_heading()` returns `FontId::proportional(14.0)` etc.

### Migration mapping

| Current size | Old usage | New token | New base size |
|-------------|-----------|-----------|---------------|
| 7 pt | keyboard notes, tiny labels | `font_micro` | 9 pt |
| 8 pt | patch names, small labels | `font_small` | 10 pt |
| 9 pt | knob values, most labels | `font_value` / `font_body` | 11 / 12 pt |
| 10 pt | buttons, chips, headers | `font_body` | 12 pt |
| 11 pt | section titles | `font_heading` | 14 pt |
| 12–13 pt | rare large use | `font_heading` / `font_display` | 14 / 18 pt |

Note: base sizes are **larger** than what they replace. The global zoom factor
(default 0.9) compensates, and the user can adjust. The result is text that
is readable at actual size and correct at zoom-out.

---

## 4. Handling multiple monitor configurations

egui/eframe provides monitor information via `egui::ViewportInfo`. Use this
to set `pixels_per_point` at startup and update it on monitor change events
(when the user drags the window to a different screen):

```
On window move / monitor change:
  new_native_ppp = ctx.native_pixels_per_point().unwrap_or(1.0)
  ctx.set_pixels_per_point(zoom_factor * new_native_ppp)
```

This ensures the UI stays correctly scaled when moving between a laptop screen
and an external monitor with different DPI.

---

## 5. Summary checklist

- [ ] Set `min_inner_size` in NativeOptions (720 × 480)
- [ ] Clamp initial window size to 90% of available monitor area
- [ ] Open maximized if clamped < minimum viable
- [ ] Add `zoom_factor` field to user settings (default 0.9, range 0.7–1.4)
- [ ] Apply `ctx.set_pixels_per_point(zoom_factor * native_ppp)` at startup
- [ ] Handle Ctrl +/- /0 in the main event loop
- [ ] Persist zoom_factor and window geometry to settings file
- [ ] Add font token methods to SynthTheme (font_heading, font_body, etc.)
- [ ] Replace all hardcoded font sizes with token calls (per-file migration)
- [ ] Update `pixels_per_point` on monitor change events

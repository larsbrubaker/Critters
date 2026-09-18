# Critters — Rust port of Critter Stack

A faithful Rust port of *Critter Stack*, a physics stacking game (build the
tallest tower of forest critters without anyone falling off the stump),
rendered through agg-gui, simulated with box2d-rust, and shipped native +
WebAssembly (GitHub Pages). This file is the working charter: read it before
writing code.

## The reference implementation (read-only)

The JavaScript original is committed under `reference/` and is the ground
truth for every behaviour, colour, constant, and string:

- `reference/game.js` — game logic, rendering, camera, wind, scoring,
  synthesized audio (Matter.js physics)
- `reference/critters.js` — the hand-drawn critter faces and expressions
- `reference/homes.js` — the painted wood interiors of the shapes
- `reference/index.html`, `reference/style.css` — HUD, tray, overlay card
- `reference/README.md` — the original's rules and tuning notes

Always read the actual JavaScript when porting a function — never work from
memory of "how a stacking game works". The reference is read-only; never
edit it. When the port diverges from the original, instrument both sides
(the original exposes `window.__cs` with `?debug`) and diff, never guess.

## The three pillars

**No stubs, no shortcuts.** Every function must be complete and
production-ready. No `todo!()`, no `unimplemented!()`, no partial
implementations. If dependencies aren't ready, implement them first.

**Match the original.** Constants, colours, shape geometry, scoring, camera
easing, tier thresholds, wind, idle animations, expressions, sound
envelopes — all come from `reference/`. Where physics engines differ
(Matter.js vs box2d-rust), the Box2D translation of each Matter tuning
knob is documented next to the constant in `physics.rs` and the observable
gameplay rules (settle detection, lock/unlock, fall reasons, stability
cheat) are ported verbatim on top of it.

**Test-first bug fixing.** 1) Write a failing test that reproduces the bug.
2) Fix it. 3) Confirm the test passes. Never commit a bug fix that isn't
covered by a test.

## Rendering architecture

All UI through agg-gui — the strongest invariant. Platform shells
(`critters-native`, `critters-wasm`) create the window/canvas, forward
input, and get out of the way: no widget construction, no mode decisions,
no user-facing strings, no HTML/CSS UI beyond the bare canvas. The HUD,
hint, toast, overlay card, tray slots and mute button that were DOM
elements in the original are all painted by agg-gui inside the one
`GameWidget`.

The game keeps the original's logical coordinate system: a 480-unit-wide
stage whose height follows the aspect ratio, world y = 0 at the stump top
and y growing downward, 100 px = 1 m. The tray (112 logical px + padding)
sits below the stage exactly as the original's flex layout did.

## Audio

Sound effects are synthesized (no asset files), mirroring the original's
Web Audio graphs: `audio.rs` renders each effect to PCM samples on the
core side, and the shells only play buffers. The mute state and the best
score persist through agg-gui's storage abstraction (browser
`localStorage` on the web, a file on native).

## Local development uses agg-gui and box2d-rust as path deps — improve them as you go

`Cargo.toml` patches `agg-gui` to the sibling checkout `../agg-gui/` and
depends on `box2d-rust` at `../box2d-rust/`. When Critters needs a
capability that doesn't exist in either, add it to that library itself
(then Lars publishes a new version) — never a local workaround. CI clones
both siblings so the paths resolve there too.

## Build & test (Windows / PowerShell)

```powershell
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --package critters-core --package critters-native --package critters-wasm
cargo dev                          # hot-reload native shell (needs cargo-watch)
wasm-pack build critters-wasm --target web --out-dir ../demo/public/pkg --no-typescript
```

Run `scripts/pre-commit-check.ps1` before every commit.

## Forbidden patterns

- `todo!()`, `unimplemented!()`, `panic!()` for missing functionality
- Stub implementations; marking work complete while any test fails
- Weakening, `#[ignore]`-ing, or deleting tests to make them pass
- Guessing at divergences instead of instrumenting both sides
- Visible UI outside agg-gui (DOM elements, CSS overlays, native dialogs)
- Files over 800 lines (`file_line_count` test enforces this — split into
  real modules, never compress to squeak under)
- Editing `reference/`

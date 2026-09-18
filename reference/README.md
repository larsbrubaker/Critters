# Critter Stack

A physics stacking game: build the tallest tower of forest critters you can without anyone falling off the stump.

## Run

It is a static site, so open `index.html` from any local web server, for example:

```sh
python3 -m http.server 8000
```

then visit http://localhost:8000/.

## How to play

- Tap anywhere to dismiss the intro card, then tap or click a critter in the tray to pick it up. Its slot stays empty until you drop, then refills with a new critter.
- The held critter follows your pointer wherever it is. Move it over the tower, then tap or release to drop it, or drag straight up from the tray onto the stage and let go. Picking another slot while holding swaps them, but you only ever choose among the three critters you were offered.
- Play again starts a new round immediately, without the intro card.
- You can only drop once every piece on the tower has come to rest. While waiting, the held piece is dimmed and the hint says so.
- Scroll down to admire the tower with the mouse wheel, a vertical drag on the play area, or `↓` / `↑`. The view snaps back when you pick up or drop a piece.
- Keyboard: `1` / `2` / `3` pick a slot, `←` `→` move, `Space` drop or restart.
- The round ends when a piece slips below the stump top (nothing resting on the tower is ever lower than that), when its centre goes past the edge of the stump below the tower top, or when a piece that had come to rest tumbles more than 1.5 m down the tower. A freshly dropped piece may bump others and land anywhere over the stump, including into a gap. The game-over card says which critter fell and how. The tolerances are `BELOW_TOP`, `EDGE_TOLERANCE` and `FALL_DROP` at the top of `game.js`.

## Wind

In the Open Sky, Cloud Country and Stratosphere tiers (by tower height), gusts blow every 7 to 16 seconds. A gust lasts about two seconds, shows streaks across the screen, plays a whoosh, and draws the tower leaning, more so the higher up a piece sits, with the critters leaning too. It is purely visual: the physics bodies are never touched, so wind can never knock a piece off, and everything settles back into place as the gust fades. Tune it in the `WIND` object at the top of `game.js` (`lean` and `tilt` for how far the top appears to move, `sway` for shiver, `duration`, `minGap` / `maxGap`, and the tier range).

## Critter animations

Every shape is a cut of timber painted in `homes.js`, with a hollow or knot for its critter: logs side-on with grain and end rings, planks and trunks with bark grain and knots, stumps with a cut surface on top, a split-log wedge, and the hexagon and circle as log ends seen face-on with growth rings. The critters are drawn as vector faces in `critters.js`, each with five expressions: happy by default, curious while held, scared while falling or being jostled, worried while its piece is unsettled or in the wind, and relieved for a second after landing. They blink now and then. Every critter also has an idle animation (bunnies hop, squirrels twitch, owls tilt, foxes breathe, frogs puff, raccoons shuffle, hedgehogs rock, bears breathe, wolves sway, deer bob), squashes when its piece lands, bobs while held, and leans in the wind. The tray previews animate too.

## Scoring

- Every shape has a fixed resident critter, so you learn the pairings. Harder shapes carry more points:

  | Shape | Critter | Points |
  |---|---|---|
  | Log | Squirrel | 200 |
  | Block | Bunny | 200 |
  | Plank | Owl | 300 |
  | Wedge | Fox | 400 |
  | Stump | Frog | 300 |
  | Log end (hexagon) | Raccoon | 300 |
  | Round log | Hedgehog | 500 |
  | Great log (rare) | Bear | 1000 |
  | Trunk (rare) | Wolf | 800 |
  | Great stump (rare) | Deer | 600 |

- Each resting critter scores its points times a height multiplier of `1 + height in metres` (100 px = 1 m). A piece locks in its score once it settles and keeps it while the tower jostles, losing it only if it actually shifts.
- Your score is the best total the settled tower reached during the round. The all-time best is saved in the browser.

## Stability cheat

To make tall towers possible, pieces get denser, stickier and more damped the deeper they sit below the top of the tower, and very deep pieces freeze solid. Only pieces that are locked at rest and sitting over the stump get this treatment, and freezing is reserved for pieces well inside the stump's span (`FREEZE_INSET`), so a piece that is falling, hugging the edge, or has strayed off to the side is never frozen into an invisible ledge. The top of the tower always uses normal physics. The knobs live in the `STABILITY` object near the top of `game.js`:

| Knob | Meaning |
|---|---|
| `freeDepth` | metres below the top where pieces behave normally |
| `densityPerMetre`, `maxDensityMult` | how fast density grows with depth, and its cap |
| `frictionPerMetre` | extra surface friction per metre |
| `airPerMetre` | extra air damping per metre (kills wobble) |
| `freezeDepth` | depth at which pieces become static; 0 disables freezing |

Deeper pieces are drawn with a slight dark tint so you can see the effect while tuning. With `?debug` the object is reachable as `window.__cs.stability` for live tweaking.

## Altitude tiers

The background climbs with the tower: forest floor, treetops (4 m), open sky (8 m), cloud country (15 m), stratosphere (25 m) and outer space (40 m). Reaching a tier shows a toast and plays a chime. The upper tiers are there for future mechanics that make those heights reachable.

## Sound

Effects are synthesized with the Web Audio API, so there are no audio files. The speaker button in the corner mutes them and remembers the setting.

## Files

- `index.html` – page layout (canvas, HUD, tray, overlay)
- `style.css` – styling
- `critters.js` – the hand-drawn critter faces and their expressions
- `homes.js` – the painted wood interiors of the shapes
- `game.js` – game logic, rendering and audio, built on [Matter.js](https://brm.io/matter-js/) loaded from cdnjs

Add `?debug` to the URL to expose `window.__cs`, a small hook for driving the game from the console (tick the simulation, pick and drop pieces, lock the camera at an altitude, force a gust with `gust(dir)`, and tweak `stability` and `wind` live).

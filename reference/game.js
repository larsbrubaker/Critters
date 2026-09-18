(() => {
  const { Engine, Bodies, Body, Composite } = Matter;

  // ---------- Config ----------
  const W = 480;              // logical canvas width; height derived from aspect
  const STUMP_W = 260;        // width of the stump top (the platform)
  const STUMP_H = 140;        // stump height above the forest floor
  const FLOOR_Y = STUMP_H;    // world y of the forest floor (stump top is y = 0)
  const PX_PER_M = 100;       // 100 px = 1 m
  const HOVER_GAP = 150;      // how far above the tower the held piece hovers when picked up
  const HOVER_MARGIN = 40;    // minimum clearance kept between the held piece and any landed piece
  const SETTLE_FRAMES = 30;   // frames of stillness before a piece counts as resting
  const DROP_FALLBACK = 8000; // ms after a drop before dropping is allowed even if something never settles
  const EDGE_TOLERANCE = 8;   // px a piece's centre may hang past the stump edge before it counts as off the side
  const BELOW_TOP = 40;       // px a piece's bottom may dip below the stump top (heavy towers sink a little into the stump)
  const FREEZE_INSET = 25;    // pieces whose centre is within this distance of the stump edge are never frozen
  const FALL_DROP = 150;      // px a rested piece may drop below its highest resting spot before it counts as fallen

  // Wind gusts: only in the listed altitude tiers (by tower height). Purely visual: the tower is drawn
  // leaning during a gust and settles back as it fades. The physics bodies are never touched, so wind
  // can never knock a piece off.
  const WIND = {
    minTier: 2,        // Open Sky
    maxTier: 4,        // Stratosphere
    lean: 26,          // px the top of the tower appears to shift at full strength
    tilt: 0.09,        // radians the top of the tower appears to tilt at full strength
    sway: 3,           // px of extra shiver during the gust
    duration: 2200,    // ms a gust lasts
    minGap: 7000,      // ms between gusts (random in [minGap, maxGap])
    maxGap: 16000,
  };

  // Shapes. Every shape has a fixed resident critter (drawn by critters.js) so players learn the
  // pairings. Harder-to-stack shapes carry more points. `big` shapes are rare and carry the big animals.
  // faceSize is the approximate face height in px.
  const SHAPES = [
    { name: 'log',       kind: 'rect', w: 120, h: 36, color: '#a9744f', faceSize: 26, weight: 3,
      critter: 'squirrel', points: 200 },
    { name: 'block',     kind: 'rect', w: 60,  h: 60, color: '#c08a5a', faceSize: 34, weight: 3,
      critter: 'bunny', points: 200 },
    { name: 'plank',     kind: 'rect', w: 44,  h: 88, color: '#8c6a4a', faceSize: 28, weight: 2,
      critter: 'owl', points: 300 },
    { name: 'wedge',     kind: 'tri',  w: 88,  h: 64, color: '#b07f55', faceSize: 26, weight: 2,
      critter: 'fox', points: 400 },
    { name: 'stump',     kind: 'trap', w: 96, top: 56, h: 46, color: '#9c6a45', faceSize: 28, weight: 2,
      critter: 'frog', points: 300 },
    { name: 'log end',   kind: 'hex',  r: 36, color: '#d2a679', faceSize: 30, weight: 1.5,
      critter: 'raccoon', points: 300 },
    { name: 'round log', kind: 'round', r: 28, color: '#c99a67', faceSize: 30, weight: 1,
      critter: 'hedgehog', points: 500 },
    { name: 'great log', kind: 'rect', w: 180, h: 52, color: '#7a4f2e', faceSize: 40, weight: 0.5, big: true,
      critter: 'bear', points: 1000 },
    { name: 'trunk',     kind: 'rect', w: 100, h: 100, color: '#8a5d3b', faceSize: 54, weight: 0.4, big: true,
      critter: 'wolf', points: 800 },
    { name: 'great stump', kind: 'trap', w: 150, top: 104, h: 66, color: '#a0703f', faceSize: 44, weight: 0.4, big: true,
      critter: 'deer', points: 600 },
  ];

  // Altitude tiers (metres of settled tower height). Reaching one shows a toast.
  const TIERS = [
    { at: 0,  name: 'Forest Floor', icon: '\u{1F331}' },
    { at: 4,  name: 'Above the Treetops!', icon: '\u{1F332}' },
    { at: 8,  name: 'Open Sky!', icon: '\u{1F426}' },
    { at: 15, name: 'Cloud Country!', icon: '☁️' },
    { at: 25, name: 'Stratosphere!', icon: '\u{1F319}' },
    { at: 40, name: 'Outer Space!', icon: '\u{1F680}' },
  ];

  // Sky colours by camera altitude (metres): [alt, top, bottom]
  const SKY = [
    [0,  '#9fd3a8', '#e6efc4'],
    [4,  '#8ecae6', '#dff3fb'],
    [10, '#5aa9e6', '#bde0fe'],
    [20, '#1d4e89', '#5aa9e6'],
    [30, '#0b1530', '#1d3557'],
    [45, '#02030a', '#0b1530'],
  ];

  const BASE_DENSITY = 0.002;
  const BODY_OPTS = { friction: 0.9, frictionStatic: 1.5, restitution: 0, density: BASE_DENSITY, frictionAir: 0.01 };

  // Stability cheat: the deeper a piece sits below the top of the tower (the "active layer"),
  // the denser, stickier and more damped it becomes, so tall towers stop wobbling themselves apart.
  // Tune these for game balance. Depths are in metres below the highest settled piece.
  const STABILITY = {
    freeDepth: 0.8,        // pieces within this depth of the top behave normally
    densityPerMetre: 2.0,  // extra density multiplier per metre beyond freeDepth
    maxDensityMult: 12,    // cap on the density multiplier
    frictionPerMetre: 0.4, // extra surface friction per metre beyond freeDepth (capped at 2)
    airPerMetre: 0.06,     // extra air damping per metre beyond freeDepth (capped at 0.3)
    freezeDepth: 3.5,      // pieces deeper than this become solid (0 disables freezing)
  };

  // Only pieces resting on the tower (locked, and over the stump) get the treatment; a piece that is
  // still moving or has strayed off to the side must never be frozen into an invisible ledge.
  const overStump = b => Math.abs(b.position.x - W / 2) <= STUMP_W / 2;
  const wellInside = b => Math.abs(b.position.x - W / 2) <= STUMP_W / 2 - FREEZE_INSET;
  function applyStability(b) {
    if (!b.locked || !overStump(b)) { b.stabilityMult = 1; return; }
    const depth = (b.position.y - towerTop) / PX_PER_M;
    const ex = Math.max(0, depth - STABILITY.freeDepth);
    const mult = Math.min(STABILITY.maxDensityMult, 1 + STABILITY.densityPerMetre * ex);
    b.stabilityMult = mult;
    if (b.isStatic) return;
    // freezing is reserved for pieces well inside the stump's span; a piece hugging the edge stays live
    if (STABILITY.freezeDepth > 0 && depth > STABILITY.freezeDepth && wellInside(b)) { Body.setStatic(b, true); return; }
    const density = BASE_DENSITY * mult;
    if (Math.abs(density - b.density) / b.density > 0.02) Body.setDensity(b, density);
    b.friction = Math.min(2, 0.9 + STABILITY.frictionPerMetre * ex);
    b.frictionStatic = Math.min(3, 1.5 + STABILITY.frictionPerMetre * ex);
    b.frictionAir = Math.min(0.3, 0.01 + STABILITY.airPerMetre * ex);
  }

  // ---------- DOM ----------
  const canvas = document.getElementById('game');
  const ctx = canvas.getContext('2d');
  const slotEls = [...document.querySelectorAll('.slot')];
  const hudScore = document.getElementById('score');
  const hudHeight = document.getElementById('height');
  const hudBest = document.getElementById('best');
  const hintEl = document.getElementById('hint');
  const toastEl = document.getElementById('toast');
  const overlay = document.getElementById('overlay');
  const overlayTitle = document.getElementById('overlay-title');
  const overlayText = document.getElementById('overlay-text');
  const overlayBtn = document.getElementById('overlay-btn');
  const overlayDismiss = document.getElementById('overlay-dismiss');

  // ---------- State ----------
  let H = 720, scale = 1, dpr = 1;
  let engine, world;
  let placed = [];
  let slots = [null, null, null];
  let held = null;
  let heldX = W / 2;
  let heldBaseY = 0;         // hover height captured when the piece was picked up (only ever ratchets up)
  let heldSlot = -1;         // tray slot the held piece came from; it stays empty until the piece is dropped
  let dragging = false;
  let lastDropped = null, lastDropTime = 0;
  let towerTop = 0;          // world y of the highest settled piece (<= 0)
  let score = 0, roundBest = 0, roundBestHeight = 0, blocks = 0, tierReached = 0, beatBest = false;
  let bestScore = parseInt(localStorage.getItem('critterstack-best-score-v2') || '0', 10);
  let camY = 0, camY0 = 0;
  let state = 'start'; // start | play | over
  let time = 0, acc = 0;
  let toastTimer = null;
  let camLock = null; // debug only: pin the camera at a world y
  let viewOffset = 0; // how far (px) the player has scrolled the view down from the automatic camera position
  let wind = { active: false, dir: 1, start: 0, nextAt: 0, strength: 0 };
  const streaks = Array.from({ length: 26 }, () => ({ x: Math.random() * (W + 300), y: Math.random(), len: 40 + Math.random() * 90, v: 0.7 + Math.random() * 0.6 }));
  let hintUntil = 0; // a shaken hint message stays until this time
  let slotBodies = [null, null, null];

  // ---------- Scenery (generated once) ----------
  const seeded = (n, f) => Array.from({ length: n }, (_, i) => f(i));
  const mountains = seeded(9, i => ({ x: i * 70 - 40 + Math.random() * 30, h: 420 + Math.random() * 260, w: 260 + Math.random() * 120 }));
  const farTrees = seeded(34, () => ({ x: Math.random() * (W + 200) - 100, h: 170 + Math.random() * 120, w: 0.55 }));
  const midTrees = seeded(11, i => ({ x: i % 2 ? Math.random() * 150 - 60 : W - 90 + Math.random() * 150, h: 520 + Math.random() * 220, r: 48 + Math.random() * 26, t: 12 + Math.random() * 6 }));
  const nearTrees = seeded(4, i => ({ x: i < 2 ? -20 + Math.random() * 60 : W - 40 + Math.random() * 60, h: 760 + Math.random() * 240, r: 70 + Math.random() * 30, t: 30 + Math.random() * 14 }));
  const floorBits = seeded(40, () => ({ x: Math.random() * W, kind: Math.random(), s: 0.7 + Math.random() * 0.8 }));
  const clouds = seeded(16, i => ({ x: Math.random() * (W + 240) - 120, a: 1000 + i * 130 + Math.random() * 100, s: 0.6 + Math.random() * 0.9, v: 4 + Math.random() * 8 }));
  const birds = seeded(7, i => ({ x: Math.random() * W, a: 650 + i * 140 + Math.random() * 80, v: 18 + Math.random() * 22, dir: Math.random() < 0.5 ? -1 : 1, ph: Math.random() * 6 }));
  const stars = seeded(120, () => ({ x: Math.random() * W, y: Math.random() * 2000, r: Math.random() * 1.6 + 0.4, tw: Math.random() * 6 }));
  const planets = [
    { a: 2700, x: 90, r: 26, p: 0.35, color: '#e9e4d4', ring: false },          // moon
    { a: 4300, x: 380, r: 40, p: 0.45, color: '#e8b04b', ring: true },          // ringed planet
    { a: 5000, x: 140, r: 18, p: 0.4, color: '#c0563a', ring: false },          // red planet
  ];

  // ---------- Helpers ----------
  const rand = a => a[Math.floor(Math.random() * a.length)];
  const clampX = x => Math.max(30, Math.min(W - 30, x));
  const fmtM = m => `${m.toFixed(2)} m`;

  function pickShape(pool = SHAPES) {
    const total = pool.reduce((s, x) => s + x.weight, 0);
    let r = Math.random() * total;
    for (const s of pool) { r -= s.weight; if (r <= 0) return s; }
    return pool[0];
  }
  const specFor = shape => ({ shape, points: shape.points, name: shape.critter });
  const newSpec = () => specFor(pickShape());
  const flatSpec = () => specFor(pickShape(SHAPES.filter(s => !s.big && (s.kind === 'rect' || s.kind === 'trap'))));

  function makeBody(spec, x, y) {
    const s = spec.shape;
    let b;
    switch (s.kind) {
      case 'rect':
        b = Bodies.rectangle(x, y, s.w, s.h, { ...BODY_OPTS, chamfer: { radius: 4 } });
        break;
      case 'tri':
        b = Bodies.fromVertices(x, y, [{ x: -s.w / 2, y: s.h / 2 }, { x: s.w / 2, y: s.h / 2 }, { x: 0, y: -s.h / 2 }], BODY_OPTS);
        break;
      case 'trap':
        b = Bodies.fromVertices(x, y, [
          { x: -s.w / 2, y: s.h / 2 }, { x: s.w / 2, y: s.h / 2 },
          { x: s.top / 2, y: -s.h / 2 }, { x: -s.top / 2, y: -s.h / 2 },
        ], BODY_OPTS);
        break;
      case 'hex': {
        const v = [];
        for (let i = 0; i < 6; i++) {
          const a = (i * Math.PI) / 3; // flat top and bottom
          v.push({ x: Math.cos(a) * s.r, y: Math.sin(a) * s.r });
        }
        b = Bodies.fromVertices(x, y, v, BODY_OPTS);
        break;
      }
      case 'round':
        b = Bodies.circle(x, y, s.r, { ...BODY_OPTS, friction: 0.6 });
        break;
    }
    b.critter = { ...spec, baseMass: b.mass };
    b.anim = { phase: Math.random() * 1000, landAt: -1 };
    return b;
  }

  // A body counts as settled once it has been (nearly) motionless for a sustained stretch,
  // so the top of a bounce does not register as a resting height.
  const isSettled = b => b.isSleeping || (b.stillFrames || 0) >= SETTLE_FRAMES;
  function trackStillness(b) {
    if (b.speed < 0.3 && b.angularSpeed < 0.02) b.stillFrames = (b.stillFrames || 0) + 1;
    else b.stillFrames = 0;
  }

  // Once a piece settles it "locks": it keeps counting toward height and score even while a new
  // piece landing on it makes it twitch. It only unlocks if it actually moves a meaningful amount.
  const UNLOCK_DIST = 25, UNLOCK_ANGLE = 0.35;
  function trackLock(b) {
    if (b.locked) {
      const moved = Math.hypot(b.position.x - b.rest.x, b.position.y - b.rest.y) > UNLOCK_DIST ||
        Math.abs(b.angle - b.rest.angle) > UNLOCK_ANGLE;
      if (moved) { b.locked = false; b.stillFrames = 0; }
    }
    if (!b.locked && isSettled(b)) {
      b.locked = true;
      b.rest = { x: b.position.x, y: b.position.y, angle: b.angle, top: b.bounds.min.y };
      b.highestRestY = Math.min(b.highestRestY ?? Infinity, b.position.y);
    }
  }

  // Why a piece counts as lost, or null if it is still part of the tower. A freshly dropped piece may
  // bump things and land anywhere within the stump's span (e.g. into a gap); the tumble rule only
  // applies to pieces that had actually come to rest and then fell.
  function fallReason(b) {
    // nothing resting on the tower is ever meaningfully lower than the stump top
    if (b.bounds.max.y > BELOW_TOP) return 'slipped off the stump';
    if (b.bounds.max.y > 6 && !overStump(b)) return 'slipped off the stump';
    if (Math.abs(b.position.x - W / 2) > STUMP_W / 2 + EDGE_TOLERANCE && b.position.y > towerTop) return 'went over the side';
    if (b.highestRestY !== undefined && b.position.y > b.highestRestY + FALL_DROP) return 'tumbled down the tower';
    return null;
  }
  // Highest point of any piece that has actually landed on the tower (ignores the piece still falling).
  function landedTop() {
    let top = 0;
    for (const b of placed) if (b.hasLanded && b.bounds.min.y < top) top = b.bounds.min.y;
    return top;
  }
  // The held piece keeps the hover height it was picked up at, unless that would bring it within
  // HOVER_MARGIN of a landed piece, in which case it is lifted just enough (and stays lifted).
  function heldY() {
    if (!held) return towerTop - HOVER_GAP;
    const halfBelow = held.bounds.max.y - held.position.y;
    const clear = landedTop() - HOVER_MARGIN - halfBelow;
    if (clear < heldBaseY) heldBaseY = clear;
    return heldBaseY;
  }
  const towerSettled = () => placed.every(b => b.locked || b.bounds.max.y > BELOW_TOP);
  const canDrop = () => towerSettled() || time - lastDropTime > DROP_FALLBACK;

  // Points for one resting piece: its critter's value times a height multiplier.
  function pieceScore(b) {
    const hM = Math.max(0, -b.rest.y) / PX_PER_M;
    return b.critter.points * (1 + hM);
  }

  function setHint(text, shake) {
    if (hintEl.textContent !== text) hintEl.textContent = text;
    hintEl.classList.remove('shake');
    if (shake) { void hintEl.offsetWidth; hintEl.classList.add('shake'); hintUntil = time + 1400; }
  }

  function showToast(text) {
    toastEl.textContent = text;
    toastEl.classList.remove('hidden');
    toastEl.style.animation = 'none';
    void toastEl.offsetWidth;
    toastEl.style.animation = '';
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => toastEl.classList.add('hidden'), 2700);
  }

  // ---------- Audio (synthesized, no asset files) ----------
  let audioCtx = null;
  let muted = localStorage.getItem('critterstack-muted') === '1';
  const muteBtn = document.getElementById('mute');

  function audio() {
    if (!audioCtx) {
      const AC = window.AudioContext || window.webkitAudioContext;
      if (!AC) return null;
      audioCtx = new AC();
    }
    if (audioCtx.state === 'suspended') audioCtx.resume();
    return audioCtx;
  }

  // Play one tone: type, start/end frequency, duration (s), gain, optional delay (s).
  function tone(type, f0, f1, dur, gain = 0.2, delay = 0) {
    const ac = audio();
    if (!ac || muted) return;
    const t = ac.currentTime + delay;
    const o = ac.createOscillator();
    const g = ac.createGain();
    o.type = type;
    o.frequency.setValueAtTime(f0, t);
    o.frequency.exponentialRampToValueAtTime(Math.max(1, f1), t + dur);
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(gain, t + 0.01);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    o.connect(g).connect(ac.destination);
    o.start(t);
    o.stop(t + dur + 0.02);
  }

  // Filtered noise burst for thuds.
  function thump(strength) {
    const ac = audio();
    if (!ac || muted) return;
    const k = Math.max(0.15, Math.min(1, strength));
    const dur = 0.12 + 0.1 * k;
    const buf = ac.createBuffer(1, Math.ceil(ac.sampleRate * dur), ac.sampleRate);
    const d = buf.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = (Math.random() * 2 - 1) * (1 - i / d.length) ** 2;
    const src = ac.createBufferSource();
    src.buffer = buf;
    const lp = ac.createBiquadFilter();
    lp.type = 'lowpass';
    lp.frequency.value = 300 + 500 * k;
    const g = ac.createGain();
    g.gain.value = 0.5 * k;
    src.connect(lp).connect(g).connect(ac.destination);
    src.start();
    tone('sine', 110 + 40 * k, 60, dur, 0.35 * k);
  }

  const sfx = {
    pick() { tone('sine', 620, 920, 0.09, 0.15); },
    swap() { tone('sine', 920, 620, 0.09, 0.15); },
    drop() { tone('triangle', 420, 180, 0.16, 0.12); },
    land(strength) { thump(strength); },
    tier() { [523, 659, 784, 1047].forEach((f, i) => tone('sine', f, f, 0.28, 0.18, i * 0.11)); },
    best() { [784, 988, 1175].forEach((f, i) => tone('triangle', f, f, 0.2, 0.12, i * 0.09)); },
    wait() { tone('square', 200, 160, 0.08, 0.05); },
    over() { [330, 262, 208, 147].forEach((f, i) => tone('sawtooth', f, f * 0.94, 0.32, 0.12, i * 0.22)); },
    wind() {
      const ac = audio();
      if (!ac || muted) return;
      const dur = WIND.duration / 1000;
      const buf = ac.createBuffer(1, Math.ceil(ac.sampleRate * dur), ac.sampleRate);
      const d = buf.getChannelData(0);
      for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
      const src = ac.createBufferSource();
      src.buffer = buf;
      const bp = ac.createBiquadFilter();
      bp.type = 'bandpass';
      bp.Q.value = 1.2;
      const t = ac.currentTime;
      bp.frequency.setValueAtTime(250, t);
      bp.frequency.linearRampToValueAtTime(900, t + dur / 2);
      bp.frequency.linearRampToValueAtTime(250, t + dur);
      const g = ac.createGain();
      g.gain.setValueAtTime(0.0001, t);
      g.gain.exponentialRampToValueAtTime(0.25, t + dur / 2);
      g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
      src.connect(bp).connect(g).connect(ac.destination);
      src.start(t);
    },
  };

  function setMuted(m) {
    muted = m;
    localStorage.setItem('critterstack-muted', m ? '1' : '0');
    muteBtn.textContent = m ? '\u{1F507}' : '\u{1F50A}';
    muteBtn.setAttribute('aria-label', m ? 'Unmute' : 'Mute');
  }
  muteBtn.addEventListener('pointerdown', e => { e.stopPropagation(); e.preventDefault(); setMuted(!muted); if (!muted) sfx.pick(); });
  setMuted(muted);

  // ---------- Game flow ----------
  function init(skipIntro) {
    engine = Engine.create({ enableSleeping: true });
    engine.positionIterations = 10;
    engine.velocityIterations = 8;
    world = engine.world;
    Composite.add(world, [
      Bodies.rectangle(W / 2, STUMP_H / 2, STUMP_W, STUMP_H, { isStatic: true, friction: 1 }), // the stump
      Bodies.rectangle(W / 2, FLOOR_Y + 40, W * 4, 80, { isStatic: true, friction: 1 }),        // the forest floor
    ]);
    Matter.Events.on(engine, 'collisionStart', ev => {
      for (const pair of ev.pairs) {
        for (const b of [pair.bodyA, pair.bodyB]) {
          if (b.critter && !b.hasLanded) {
            b.hasLanded = true;
            b.anim.landAt = time;
            const v = Math.hypot(b.velocity.x, b.velocity.y);
            sfx.land(v / 9);
          }
        }
      }
    });
    placed = [];
    held = null;
    lastDropped = null;
    blocks = 0;
    score = 0;
    roundBest = 0;
    roundBestHeight = 0;
    tierReached = 0;
    beatBest = false;
    viewOffset = 0;
    wind = { active: false, dir: 1, start: 0, nextAt: time + WIND.minGap, strength: 0 };
    towerTop = 0;
    camY0 = -(H - 190); // stump top sits near the bottom of the view; the floor just below it
    camY = camY0;
    heldX = W / 2;
    heldSlot = -1;
    slots = [flatSpec(), flatSpec(), newSpec()];
    slots.forEach((_, i) => renderSlot(i));
    state = 'start';
    overlay.classList.remove('hidden', 'over');
    overlayTitle.textContent = 'Critter Stack';
    overlayText.textContent = 'Stack the forest critters as high as you can without letting anyone fall off the stump.';
    overlayBtn.classList.add('hidden');
    overlayDismiss.classList.remove('hidden');
    toastEl.classList.add('hidden');
    setHint('Pick a critter below to start');
    updateHud();
    if (skipIntro) beginPlay();
  }

  function beginPlay() {
    if (state !== 'start') return;
    state = 'play';
    overlay.classList.add('hidden');
    setHint('Pick a critter below');
  }

  function selectSlot(i) {
    if (state === 'over') return;
    const spec = slots[i];
    if (!spec) return;
    beginPlay();
    if (held) {
      slots[i] = specFor(held.critter.shape); // swap: the held critter goes back into this slot (hover height kept)
      sfx.swap();
    } else {
      slots[i] = null; // the slot stays empty until the piece is dropped, so you cannot fish for easier shapes
      heldSlot = i;
      heldBaseY = towerTop - HOVER_GAP;
      sfx.pick();
    }
    held = makeBody(spec, heldX, heldBaseY);
    viewOffset = 0;
    renderSlot(i);
    slotEls[i].classList.remove('pop');
    void slotEls[i].offsetWidth;
    slotEls[i].classList.add('pop');
    setHint('Move it over the tower, then tap or release to drop');
  }

  function drop() {
    if (!held || state !== 'play') return;
    if (!canDrop()) { setHint('Wait for the tower to settle…', true); sfx.wait(); return; }
    viewOffset = 0;
    sfx.drop();
    Composite.add(world, held);
    placed.push(held);
    lastDropped = held;
    lastDropTime = time;
    blocks++;
    held = null;
    if (heldSlot >= 0) { slots[heldSlot] = newSpec(); renderSlot(heldSlot); heldSlot = -1; }
    setHint('Pick the next critter');
    updateHud();
  }

  function gameOver(reason) {
    state = 'over';
    held = null;
    sfx.over();
    overlayTitle.textContent = 'Timber!';
    overlayText.textContent = `${reason ? reason + ' ' : ''}Score ${roundBest} · ${fmtM(roundBestHeight)} tall · ${blocks} critter${blocks === 1 ? '' : 's'}.` +
      (roundBest >= bestScore && roundBest > 0 ? ' New best!' : '');
    overlayBtn.classList.remove('hidden');
    overlayDismiss.classList.add('hidden');
    overlay.classList.remove('hidden');
    overlay.classList.add('over');
    setHint('');
  }

  function updateHud() {
    hudScore.textContent = score;
    hudHeight.textContent = fmtM(Math.max(0, -towerTop / PX_PER_M));
    hudBest.textContent = bestScore;
  }

  // ---------- Simulation ----------
  function step(dt) {
    time += dt;
    if (state !== 'start') {
      acc = Math.min(acc + dt, 100);
      while (acc >= 1000 / 60) {
        Engine.update(engine, 1000 / 60);
        acc -= 1000 / 60;
        for (const b of placed) trackStillness(b);
      }
    }

    let top = 0, s = 0;
    for (const b of placed) {
      trackLock(b);
      if (!b.locked) continue;
      if (b.rest.top < top) top = b.rest.top;
      s += pieceScore(b);
    }
    towerTop = top;
    score = Math.round(s);
    if (state === 'play') for (const b of placed) applyStability(b);

    if (state === 'play') {
      const h = -towerTop / PX_PER_M;
      if (h > roundBestHeight) roundBestHeight = h;
      if (score > roundBest) roundBest = score;
      if (roundBest > bestScore) {
        if (bestScore > 0 && !beatBest) { beatBest = true; sfx.best(); }
        bestScore = roundBest;
        localStorage.setItem('critterstack-best-score-v2', String(bestScore));
      }
      // tier toasts
      for (let i = TIERS.length - 1; i > tierReached; i--) {
        if (roundBestHeight >= TIERS[i].at) { tierReached = i; showToast(`${TIERS[i].icon} ${TIERS[i].name}`); sfx.tier(); break; }
      }
      for (const b of placed) {
        const why = fallReason(b);
        if (why) { gameOver(`The ${b.critter.name} ${why}!`); break; }
      }
      updateHud();
    }

    if (held) Body.setPosition(held, { x: heldX, y: heldY() });

    if (state === 'play') {
      updateWind();
      if (held && time > hintUntil) setHint(canDrop() ? 'Move it over the tower, then tap or release to drop' : 'Waiting for the tower to settle…');
    }

    const focus = held ? held.position.y : towerTop - HOVER_GAP;
    const autoTarget = Math.min(camY0, focus - H * 0.3);
    viewOffset = Math.max(0, Math.min(camY0 - autoTarget, viewOffset));
    const target = autoTarget + viewOffset;
    camY = camLock !== null ? camLock : camY + (target - camY) * 0.08;

    for (const c of clouds) { c.x += (c.v * dt) / 1000; if (c.x > W + 140) c.x = -140; }
    for (const b of birds) { b.x += (b.v * b.dir * dt) / 1000; if (b.x > W + 40) b.x = -40; if (b.x < -40) b.x = W + 40; }
  }

  const currentTier = () => {
    const h = -towerTop / PX_PER_M;
    let t = 0;
    for (let i = 0; i < TIERS.length; i++) if (h >= TIERS[i].at) t = i;
    return t;
  };

  function updateWind() {
    const tier = currentTier();
    const eligible = tier >= WIND.minTier && tier <= WIND.maxTier;
    if (!eligible) wind.nextAt = Math.max(wind.nextAt, time + WIND.minGap);
    if (!wind.active && eligible && time >= wind.nextAt) {
      wind.active = true;
      wind.dir = Math.random() < 0.5 ? -1 : 1;
      wind.start = time;
      sfx.wind();
    }
    if (!wind.active) return;
    const p = (time - wind.start) / WIND.duration;
    if (p >= 1) {
      wind.active = false;
      wind.strength = 0;
      wind.nextAt = time + WIND.minGap + Math.random() * (WIND.maxGap - WIND.minGap);
      return;
    }
    wind.strength = Math.sin(p * Math.PI);
  }

  // Visual lean for a piece during a gust: grows with its height up the tower (like a bending reed).
  function windLean(b) {
    if (wind.strength <= 0 || b.isStatic) return { dx: 0, rot: 0 };
    const span = Math.max(300, -towerTop);
    const f = Math.pow(Math.max(0, -b.position.y) / span, 1.5);
    const shiver = Math.sin(time / 90 + b.position.y * 0.02) * WIND.sway * f;
    return {
      dx: wind.dir * wind.strength * (WIND.lean * f + shiver),
      rot: wind.dir * wind.strength * WIND.tilt * f,
    };
  }

  // ---------- Rendering ----------
  const rise = () => camY0 - camY;                       // px the camera has climbed (>= 0)
  const altM = () => rise() / PX_PER_M;                  // camera altitude in metres
  const layerY = (worldY, p) => (worldY - camY0) - (camY - camY0) * p; // screen y with parallax p
  const skyY = (a, p) => H / 2 + (a - rise()) * p;      // screen y of a sky object centred when rise == a

  function draw() {
    ctx.setTransform(dpr * scale, 0, 0, dpr * scale, 0, 0);
    drawSky();
    drawSpace();
    drawMountains();
    drawFarTrees();
    drawBirdsAndClouds();
    drawMidTrees();
    drawNearTrees();
    drawWind();
    ctx.save();
    ctx.translate(0, -camY);
    drawFloor();
    drawStump();
    drawRuler();
    for (const b of placed) drawBody(ctx, b);
    if (held) {
      ctx.save();
      ctx.setLineDash([6, 8]);
      ctx.strokeStyle = 'rgba(255,255,255,0.5)';
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(held.position.x, held.bounds.max.y);
      ctx.lineTo(held.position.x, 0);
      ctx.stroke();
      ctx.restore();
      drawBody(ctx, held, !canDrop(), true);
    }
    ctx.restore();
    for (let i = 0; i < 3; i++) drawSlot(i);
  }

  function drawWind() {
    if (wind.strength <= 0) return;
    ctx.save();
    ctx.strokeStyle = `rgba(255,255,255,${0.55 * wind.strength})`;
    ctx.lineWidth = 2;
    ctx.lineCap = 'round';
    const travel = (time - wind.start) * 0.9 * wind.dir;
    for (const st of streaks) {
      const span = W + 300;
      const x = (((st.x + travel * st.v) % span) + span) % span - 150;
      const y = st.y * H + Math.sin((time / 400) + st.x) * 4;
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.lineTo(x - st.len * wind.dir, y);
      ctx.stroke();
    }
    ctx.restore();
  }

  function skyColors() {
    const a = altM();
    let i = 0;
    while (i < SKY.length - 2 && a > SKY[i + 1][0]) i++;
    const [a0, t0, b0] = SKY[i], [a1, t1, b1] = SKY[i + 1];
    const t = Math.max(0, Math.min(1, (a - a0) / (a1 - a0)));
    return [mix(t0, t1, t), mix(b0, b1, t)];
  }

  function drawSky() {
    const [top, bottom] = skyColors();
    const g = ctx.createLinearGradient(0, 0, 0, H);
    g.addColorStop(0, top);
    g.addColorStop(1, bottom);
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, W, H);

    // sun: sits low at the start and drifts up the sky as you climb, fading out into space
    const a = altM();
    const sunAlpha = Math.max(0, 1 - Math.max(0, a - 22) / 8);
    if (sunAlpha > 0) {
      ctx.globalAlpha = sunAlpha;
      ctx.fillStyle = '#ffd166';
      ctx.beginPath(); ctx.arc(W - 70, 150 - Math.min(a, 20) * 4, 34, 0, Math.PI * 2); ctx.fill();
      ctx.globalAlpha = 1;
    }
  }

  function drawSpace() {
    const a = altM();
    const starAlpha = Math.max(0, Math.min(1, (a - 12) / 10));
    if (starAlpha > 0) {
      for (const s of stars) {
        const y = ((s.y + rise() * 0.08) % H + H) % H;
        const tw = 0.6 + 0.4 * Math.sin(time / 600 + s.tw);
        ctx.fillStyle = `rgba(255,255,255,${starAlpha * tw})`;
        ctx.beginPath(); ctx.arc(s.x, y, s.r, 0, Math.PI * 2); ctx.fill();
      }
    }
    for (const p of planets) {
      const y = skyY(p.a, p.p);
      if (y < -80 || y > H + 80) continue;
      ctx.fillStyle = p.color;
      ctx.beginPath(); ctx.arc(p.x, y, p.r, 0, Math.PI * 2); ctx.fill();
      ctx.fillStyle = 'rgba(0,0,0,0.12)';
      ctx.beginPath(); ctx.arc(p.x - p.r * 0.3, y - p.r * 0.2, p.r * 0.25, 0, Math.PI * 2); ctx.fill();
      ctx.beginPath(); ctx.arc(p.x + p.r * 0.35, y + p.r * 0.3, p.r * 0.18, 0, Math.PI * 2); ctx.fill();
      if (p.ring) {
        ctx.strokeStyle = 'rgba(240,220,180,0.8)';
        ctx.lineWidth = 6;
        ctx.beginPath(); ctx.ellipse(p.x, y, p.r * 1.8, p.r * 0.45, -0.3, 0, Math.PI * 2); ctx.stroke();
      }
    }
  }

  function drawMountains() {
    const fade = 1 - Math.max(0, Math.min(1, (altM() - 14) / 10)); // gone by ~24 m
    if (fade <= 0) return;
    const base = layerY(FLOOR_Y, 0.2);
    ctx.save();
    ctx.globalAlpha = fade;
    ctx.fillStyle = 'rgba(110,140,165,0.7)';
    for (const m of mountains) {
      ctx.beginPath();
      ctx.moveTo(m.x - m.w / 2, base);
      ctx.lineTo(m.x, base - m.h);
      ctx.lineTo(m.x + m.w / 2, base);
      ctx.closePath();
      ctx.fill();
      ctx.fillStyle = 'rgba(255,255,255,0.75)';
      ctx.beginPath();
      ctx.moveTo(m.x - m.w * 0.12, base - m.h * 0.76);
      ctx.lineTo(m.x, base - m.h);
      ctx.lineTo(m.x + m.w * 0.12, base - m.h * 0.76);
      ctx.closePath();
      ctx.fill();
      ctx.fillStyle = 'rgba(110,140,165,0.7)';
    }
    ctx.restore();
  }

  function drawFarTrees() {
    const base = layerY(FLOOR_Y, 0.15);
    ctx.fillStyle = '#4d8f6f';
    for (const t of farTrees) {
      const tw = t.h * t.w;
      ctx.beginPath();
      ctx.moveTo(t.x - tw / 2, base);
      ctx.lineTo(t.x, base - t.h);
      ctx.lineTo(t.x + tw / 2, base);
      ctx.closePath();
      ctx.fill();
    }
    ctx.fillRect(0, base, W, H);
  }

  function drawLeafyTree(x, base, h, r, trunkW, trunkColor, leafColor) {
    ctx.fillStyle = trunkColor;
    ctx.fillRect(x - trunkW / 2, base - h, trunkW, h);
    ctx.fillStyle = leafColor;
    const top = base - h;
    ctx.beginPath();
    ctx.arc(x, top, r, 0, Math.PI * 2);
    ctx.arc(x - r * 0.8, top + r * 0.5, r * 0.75, 0, Math.PI * 2);
    ctx.arc(x + r * 0.8, top + r * 0.5, r * 0.75, 0, Math.PI * 2);
    ctx.arc(x, top + r * 0.9, r * 0.8, 0, Math.PI * 2);
    ctx.fill();
  }

  function drawMidTrees() {
    const base = layerY(FLOOR_Y, 0.35);
    for (const t of midTrees) drawLeafyTree(t.x, base, t.h, t.r, t.t, '#7a5a3c', '#4f9a5b');
    ctx.fillStyle = '#3f7a45';
    ctx.fillRect(0, base, W, H);
  }

  function drawNearTrees() {
    const base = layerY(FLOOR_Y, 0.65);
    for (const t of nearTrees) drawLeafyTree(t.x, base, t.h, t.r, t.t, '#4e3a2a', '#3a8a4a');
    ctx.fillStyle = '#356a3c';
    ctx.fillRect(0, base, W, H);
  }

  function drawBirdsAndClouds() {
    const cloudAlpha = Math.max(0, Math.min(1, (altM() - 4) / 3)) * 0.88;
    ctx.fillStyle = `rgba(255,255,255,${cloudAlpha})`;
    for (const c of clouds) {
      const y = skyY(c.a, 0.5);
      if (y < -80 || y > H + 80) continue;
      ctx.beginPath();
      ctx.arc(c.x, y, 22 * c.s, 0, Math.PI * 2);
      ctx.arc(c.x + 26 * c.s, y - 10 * c.s, 26 * c.s, 0, Math.PI * 2);
      ctx.arc(c.x + 54 * c.s, y, 20 * c.s, 0, Math.PI * 2);
      ctx.fill();
    }
    const birdAlpha = Math.max(0, Math.min(1, (altM() - 2.5) / 2));
    if (birdAlpha <= 0) return;
    ctx.strokeStyle = `rgba(40,40,40,${0.75 * birdAlpha})`;
    ctx.lineWidth = 2;
    for (const b of birds) {
      const y = skyY(b.a, 0.4);
      if (y < -20 || y > H + 20) continue;
      const flap = Math.sin(time / 120 + b.ph) * 4;
      ctx.beginPath();
      ctx.moveTo(b.x - 8, y + flap);
      ctx.quadraticCurveTo(b.x - 4, y - 3, b.x, y);
      ctx.quadraticCurveTo(b.x + 4, y - 3, b.x + 8, y + flap);
      ctx.stroke();
    }
  }

  function drawFloor() {
    // dirt, with a grass edge
    ctx.fillStyle = '#5b4431';
    ctx.fillRect(-W, FLOOR_Y, W * 3, 600);
    ctx.fillStyle = '#4f8a3f';
    ctx.fillRect(-W, FLOOR_Y - 6, W * 3, 10);
    for (const f of floorBits) {
      const x = f.x, y = FLOOR_Y - 4;
      if (f.kind < 0.45) {                       // grass tuft
        ctx.fillStyle = '#6a994e';
        ctx.beginPath(); ctx.moveTo(x - 6 * f.s, y); ctx.lineTo(x - 2 * f.s, y - 14 * f.s); ctx.lineTo(x, y); ctx.fill();
        ctx.beginPath(); ctx.moveTo(x, y); ctx.lineTo(x + 5 * f.s, y - 18 * f.s); ctx.lineTo(x + 8 * f.s, y); ctx.fill();
      } else if (f.kind < 0.7) {                 // mushroom
        ctx.fillStyle = '#f1e4c8';
        ctx.fillRect(x - 2 * f.s, y - 10 * f.s, 4 * f.s, 10 * f.s);
        ctx.fillStyle = '#c1666b';
        ctx.beginPath(); ctx.arc(x, y - 10 * f.s, 7 * f.s, Math.PI, 0); ctx.fill();
      } else if (f.kind < 0.85) {                // rock
        ctx.fillStyle = '#8d8a80';
        ctx.beginPath(); ctx.ellipse(x, y - 3 * f.s, 9 * f.s, 5 * f.s, 0, 0, Math.PI * 2); ctx.fill();
      } else {                                   // fern
        ctx.strokeStyle = '#3f7d4f';
        ctx.lineWidth = 2;
        for (let k = -2; k <= 2; k++) {
          ctx.beginPath(); ctx.moveTo(x, y); ctx.quadraticCurveTo(x + k * 6 * f.s, y - 12 * f.s, x + k * 10 * f.s, y - 16 * f.s); ctx.stroke();
        }
      }
    }
  }

  function drawStump() {
    const x = W / 2 - STUMP_W / 2;
    // roots
    ctx.fillStyle = '#5a3f2b';
    ctx.beginPath();
    ctx.moveTo(x, FLOOR_Y - 40); ctx.quadraticCurveTo(x - 10, FLOOR_Y, x - 50, FLOOR_Y); ctx.lineTo(x + 10, FLOOR_Y); ctx.fill();
    ctx.beginPath();
    ctx.moveTo(x + STUMP_W, FLOOR_Y - 40); ctx.quadraticCurveTo(x + STUMP_W + 10, FLOOR_Y, x + STUMP_W + 50, FLOOR_Y); ctx.lineTo(x + STUMP_W - 10, FLOOR_Y); ctx.fill();
    // bark
    ctx.fillStyle = '#6b4b35';
    ctx.fillRect(x, 0, STUMP_W, STUMP_H);
    ctx.fillStyle = 'rgba(0,0,0,0.12)';
    for (let i = 0; i < 6; i++) ctx.fillRect(x + 20 + i * 42, 16, 10, STUMP_H - 16);
    // top face with rings
    ctx.fillStyle = '#c69c6d';
    ctx.fillRect(x, -4, STUMP_W, 16);
    ctx.strokeStyle = 'rgba(90,60,40,0.5)';
    ctx.lineWidth = 2;
    for (let r = 20; r < STUMP_W / 2; r += 24) {
      ctx.beginPath();
      ctx.ellipse(W / 2, 4, r, 5, 0, 0, Math.PI * 2);
      ctx.stroke();
    }
    // moss
    ctx.fillStyle = '#6a994e';
    ctx.beginPath(); ctx.moveTo(x - 6, 0); ctx.lineTo(x + 10, -18); ctx.lineTo(x + 16, 0); ctx.fill();
    ctx.beginPath(); ctx.moveTo(x + STUMP_W - 16, 0); ctx.lineTo(x + STUMP_W - 10, -18); ctx.lineTo(x + STUMP_W + 6, 0); ctx.fill();
  }

  function drawRuler() {
    ctx.font = 'bold 12px sans-serif';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    const topVisible = camY - 20, bottomVisible = camY + H;
    for (let m = 1; m * PX_PER_M < -topVisible; m++) {
      const y = -m * PX_PER_M;
      if (y > bottomVisible) continue;
      ctx.strokeStyle = 'rgba(255,255,255,0.35)';
      ctx.lineWidth = 1;
      ctx.beginPath(); ctx.moveTo(8, y); ctx.lineTo(38, y); ctx.stroke();
      ctx.fillStyle = 'rgba(255,255,255,0.7)';
      ctx.fillText(`${m} m`, 42, y);
    }
  }

  // Idle animation per critter, plus a squash when the piece lands and a lean in the wind.
  function critterAnim(b, held) {
    const a = b.anim;
    const s = (time + a.phase) / 1000;
    let dx = 0, dy = 0, rot = 0, sx = 1, sy = 1;
    switch (b.critter.name) {
      case 'bunny': { const h = Math.max(0, Math.sin(s * 2.4)); dy = -h * h * 6; sy = 1 + h * 0.08; sx = 1 - h * 0.05; break; }
      case 'squirrel': { const burst = Math.sin(s * 0.7) > 0.7 ? 1 : 0; rot = Math.sin(s * 16) * 0.12 * burst; break; }
      case 'owl': rot = Math.sin(s * 1.1) * 0.14; break;
      case 'fox': { const br = Math.sin(s * 2.2); sy = 1 + br * 0.05; sx = 1 - br * 0.025; break; }
      case 'frog': { const h = Math.max(0, (Math.sin(s * 1.3) - 0.7) / 0.3); dy = -h * 7; sx = 1 + h * 0.12; sy = 1 - h * 0.08; break; }
      case 'raccoon': dx = Math.sin(s * 1.5) * 4; rot = Math.sin(s * 1.5) * 0.06; break;
      case 'hedgehog': rot = Math.sin(s * 2) * 0.3; break;
      case 'bear': { const br = Math.sin(s * 1.1); sy = 1 + br * 0.05; dy = -br; break; }
      case 'wolf': rot = Math.sin(s * 0.8) * 0.08; dy = Math.sin(s * 1.6) * 1.5; break;
      case 'deer': dy = Math.sin(s * 2.6) * 2; break;
    }
    if (a.landAt >= 0) {
      const e = (time - a.landAt) / 1000;
      if (e < 0.6) { const q = Math.exp(-e * 7) * Math.sin(e * 22); sy *= 1 - q * 0.35; sx *= 1 + q * 0.25; }
    }
    if (held) dy += Math.sin(time / 260) * 3;
    else if (wind.strength > 0 && !b.isStatic) { dx += wind.dir * wind.strength * 6; rot += wind.dir * wind.strength * 0.2; }
    return { dx, dy, rot, sx, sy };
  }

  // Which face a critter makes right now.
  function expressionFor(b, held) {
    if (held) return 'curious';
    if (!b.hasLanded) return 'scared';
    if (b.speed > 0.4 || b.angularSpeed > 0.03) return 'scared';
    if (b.anim.landAt >= 0 && time - b.anim.landAt < 1100) return 'relieved';
    if (b.hasLanded && !b.locked && !b.isStatic) return 'worried';
    if (wind.strength > 0.3 && !b.isStatic) return 'worried';
    return 'happy';
  }

  // Extents of a body's outline in its own unrotated local frame.
  function localExtents(b) {
    const s = b.critter.shape;
    if (s.kind === 'round') return { minX: -s.r, maxX: s.r, minY: -s.r, maxY: s.r, w: s.r * 2, h: s.r * 2 };
    const cos = Math.cos(-b.angle), sin = Math.sin(-b.angle);
    let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
    for (const v of b.vertices) {
      const dx = v.x - b.position.x, dy = v.y - b.position.y;
      const x = dx * cos - dy * sin, y = dx * sin + dy * cos;
      if (x < minX) minX = x; if (x > maxX) maxX = x; if (y < minY) minY = y; if (y > maxY) maxY = y;
    }
    return { minX, maxX, minY, maxY, w: maxX - minX, h: maxY - minY };
  }

  function drawBody(c, b, ghost, held) {
    const { shape } = b.critter;
    c.save();
    if (ghost) c.globalAlpha = 0.55;
    if (!held && c === ctx) {
      const lean = windLean(b);
      if (lean.dx || lean.rot) {
        c.translate(b.position.x + lean.dx, b.position.y);
        c.rotate(lean.rot);
        c.translate(-b.position.x, -b.position.y);
      }
    }
    c.beginPath();
    if (shape.kind === 'round') {
      c.arc(b.position.x, b.position.y, shape.r, 0, Math.PI * 2);
    } else {
      b.vertices.forEach((v, i) => (i ? c.lineTo(v.x, v.y) : c.moveTo(v.x, v.y)));
      c.closePath();
    }
    c.fillStyle = shape.color;
    c.fill();
    // paint the critter's home inside the shape
    c.save();
    c.clip();
    c.translate(b.position.x, b.position.y);
    c.rotate(b.angle);
    drawHome(c, shape.name, localExtents(b), shape.faceSize);
    c.restore();
    if (b.stabilityMult > 1) {
      c.fillStyle = `rgba(40,25,10,${Math.min(0.3, (b.stabilityMult - 1) * 0.03)})`;
      c.fill();
    }
    c.lineWidth = 3;
    c.strokeStyle = 'rgba(0,0,0,0.28)';
    c.stroke();
    c.lineWidth = 1.5;
    c.strokeStyle = 'rgba(255,255,255,0.25)';
    c.stroke();

    const an = critterAnim(b, held);
    c.translate(b.position.x, b.position.y);
    c.rotate(b.angle);
    c.translate(an.dx, an.dy);
    c.rotate(an.rot);
    c.scale(an.sx, an.sy);
    const expr = c === ctx ? expressionFor(b, held) : 'happy';
    const blink = expr !== 'scared' && ((time + b.anim.phase * 7) % 3900) < 120;
    drawCritter(c, b.critter.name, shape.faceSize * 1.05, expr, blink);
    c.restore();
  }

  const SLOT_W = 130, SLOT_H = 96;
  function renderSlot(i) {
    const el = slotEls[i];
    const cv = el.querySelector('canvas');
    cv.width = SLOT_W * dpr; cv.height = SLOT_H * dpr;
    const spec = slots[i];
    const pts = el.querySelector('.pts');
    slotBodies[i] = spec ? makeBody(spec, 0, 0) : null;
    el.classList.toggle('empty', !spec);
    if (!spec) { pts.textContent = ''; el.classList.remove('big'); drawSlot(i); return; }
    pts.textContent = `${spec.points} pt${spec.points === 1 ? '' : 's'}`;
    el.classList.toggle('big', !!spec.shape.big);
    drawSlot(i);
  }

  function drawSlot(i) {
    const b = slotBodies[i];
    const cv = slotEls[i].querySelector('canvas');
    const sc = cv.getContext('2d');
    sc.setTransform(1, 0, 0, 1, 0, 0);
    sc.clearRect(0, 0, cv.width, cv.height);
    if (!b) return;
    const bw = b.bounds.max.x - b.bounds.min.x, bh = b.bounds.max.y - b.bounds.min.y;
    const k = Math.min((SLOT_W - 16) / bw, (SLOT_H - 16) / bh, 1);
    const cx = (b.bounds.min.x + b.bounds.max.x) / 2, cy = (b.bounds.min.y + b.bounds.max.y) / 2;
    sc.setTransform(dpr * k, 0, 0, dpr * k, (SLOT_W / 2 - cx * k) * dpr, (SLOT_H / 2 - cy * k) * dpr);
    drawBody(sc, b, false);
  }

  function mix(a, b, t) {
    const pa = hex(a), pb = hex(b);
    const c = pa.map((v, i) => Math.round(v + (pb[i] - v) * t));
    return `rgb(${c[0]},${c[1]},${c[2]})`;
  }
  const hex = h => [1, 3, 5].map(i => parseInt(h.slice(i, i + 2), 16));

  // ---------- Input ----------
  function resize() {
    const rect = canvas.getBoundingClientRect();
    dpr = window.devicePixelRatio || 1;
    scale = rect.width / W;
    H = rect.height / scale;
    canvas.width = Math.round(rect.width * dpr);
    canvas.height = Math.round(rect.height * dpr);
    camY0 = -(H - 190);
    if (state === 'start') camY = camY0;
    slots.forEach((_, i) => renderSlot(i));
  }

  const toX = e => clampX((e.clientX - canvas.getBoundingClientRect().left) / scale);

  // Pointer on the canvas: horizontal movement steers the held piece, a mostly vertical drag scrolls
  // the view down the tower, and a release that was not a scroll drops the piece.
  let dragStart = null, dragLastY = 0, scrolling = false;
  canvas.addEventListener('pointermove', e => {
    if (dragging && dragStart) {
      const dx = e.clientX - dragStart.x, dy = e.clientY - dragStart.y;
      if (!scrolling && Math.abs(dy) > 14 && Math.abs(dy) > Math.abs(dx) * 1.5) scrolling = true;
      if (scrolling) { viewOffset -= (e.clientY - dragLastY) / scale; dragLastY = e.clientY; return; }
    }
    dragLastY = e.clientY;
  });
  // The held piece follows the pointer wherever it is, including over the tray.
  window.addEventListener('pointermove', e => { if (held && !scrolling) heldX = toX(e); });
  canvas.addEventListener('pointerdown', e => {
    e.preventDefault();
    if (state === 'over') return;
    if (state === 'start') { beginPlay(); return; }
    canvas.setPointerCapture(e.pointerId);
    dragStart = { x: e.clientX, y: e.clientY };
    dragLastY = e.clientY;
    scrolling = false;
    dragging = true;
    if (held) heldX = toX(e);
  });
  canvas.addEventListener('pointerup', e => {
    if (!dragging) return;
    dragging = false;
    const wasScroll = scrolling;
    scrolling = false;
    dragStart = null;
    if (wasScroll) return;
    if (held) { heldX = toX(e); drop(); }
    else if (state === 'play') setHint('Pick a critter from the tray first', true);
  });
  canvas.addEventListener('pointercancel', () => { dragging = false; scrolling = false; dragStart = null; });
  canvas.addEventListener('wheel', e => { e.preventDefault(); viewOffset += e.deltaY / scale; }, { passive: false });

  overlay.addEventListener('pointerdown', () => { if (state === 'start') beginPlay(); });
  slotEls.forEach((el, i) => {
    el.addEventListener('pointerdown', e => {
      e.preventDefault();
      if (!slots[i] || state === 'over') return;
      selectSlot(i);
      heldX = toX(e);
      // drag straight up onto the stage and release to drop
      const startY = e.clientY;
      const onUp = ev => {
        window.removeEventListener('pointerup', onUp);
        const stage = canvas.getBoundingClientRect();
        if (startY - ev.clientY > 24 && ev.clientY < stage.bottom) { heldX = toX(ev); drop(); }
      };
      window.addEventListener('pointerup', onUp);
    });
  });
  overlayBtn.addEventListener('click', () => init(true));

  window.addEventListener('pointerdown', () => audio(), { once: true });
  window.addEventListener('keydown', e => {
    audio();
    if (e.key >= '1' && e.key <= '3') selectSlot(+e.key - 1);
    else if (e.key === 'ArrowLeft') heldX = clampX(heldX - 12);
    else if (e.key === 'ArrowRight') heldX = clampX(heldX + 12);
    else if (e.key === 'ArrowDown') { e.preventDefault(); viewOffset += 60; }
    else if (e.key === 'ArrowUp') { e.preventDefault(); viewOffset -= 60; }
    else if (e.key === ' ' || e.key === 'Enter') {
      e.preventDefault();
      if (state === 'over') init(true);
      else if (state === 'start') beginPlay();
      else drop();
    } else if (state === 'start' && e.key === 'Escape') beginPlay();
  });

  window.addEventListener('resize', resize);

  // ---------- Loop ----------
  let last = performance.now();
  function frame(now) {
    const dt = Math.min(50, now - last);
    last = now;
    step(dt);
    draw();
    requestAnimationFrame(frame);
  }

  // Test hook (only with ?debug in the URL): drive the game without the animation loop.
  if (location.search.includes('debug')) {
    window.__cs = {
      get engine() { return engine; }, get placed() { return placed; }, get state() { return state; }, get held() { return held; },
      get slots() { return slots; }, shapes: SHAPES, stability: STABILITY,
      setSlot(i, shape) { slots[i] = specFor(shape); renderSlot(i); },
      drawPiece(c, shape, x, y, angle = 0) { const b = makeBody(specFor(shape), x, y); Body.setAngle(b, angle); drawBody(c, b, false, false); },
      tick(ms) { step(ms); draw(); }, select: selectSlot, drop, begin: beginPlay, setX(x) { heldX = x; },
      setCam(y) { camLock = y; }, wind: WIND, get windState() { return wind; },
      gust(dir) { wind.active = true; wind.dir = dir || 1; wind.start = time; },
      get viewOffset() { return viewOffset; }, get camY() { return camY; },
    };
  }

  resize();
  init();
  requestAnimationFrame(frame);
})();

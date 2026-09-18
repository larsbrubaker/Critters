// Shape interiors, all wood: every piece is a cut of timber with a hollow or knot for its critter.
// Drawn in the piece's local coordinates (origin at the body centre, unrotated) with the shape's
// outline already clipped. drawHome(ctx, shapeName, L, faceSize), L = { minX, maxX, minY, maxY, w, h }
(() => {
  const GRAIN = 'rgba(60,35,15,0.26)';
  const RING = 'rgba(80,50,30,0.5)';

  function ell(c, x, y, rx, ry, fill) { c.beginPath(); c.ellipse(x, y, rx, ry, 0, 0, Math.PI * 2); c.fillStyle = fill; c.fill(); }

  // horizontal grain (logs)
  function grainH(c, L, rows, amp = 1.5) {
    c.strokeStyle = GRAIN; c.lineWidth = 1.5; c.lineCap = 'round';
    for (let i = 1; i <= rows; i++) {
      const y = L.minY + (L.h * i) / (rows + 1);
      c.beginPath();
      for (let x = L.minX; x <= L.maxX; x += 6) c.lineTo(x, y + Math.sin(x * 0.15 + i) * amp);
      c.stroke();
    }
  }
  // vertical bark grain (planks, trunks, stumps)
  function grainV(c, L, cols, amp = 2) {
    c.strokeStyle = GRAIN; c.lineWidth = 2; c.lineCap = 'round';
    for (let i = 1; i <= cols; i++) {
      const x = L.minX + (L.w * i) / (cols + 1);
      c.beginPath();
      for (let y = L.minY; y <= L.maxY; y += 6) c.lineTo(x + Math.sin(y * 0.12 + i * 2) * amp, y);
      c.stroke();
    }
  }
  // end-grain discs at both ends of a side-on log
  function endRings(c, L, color) {
    const rx = L.h * 0.22, ry = L.h * 0.42;
    for (const x of [L.minX + rx * 0.9, L.maxX - rx * 0.9]) {
      ell(c, x, 0, rx, ry, color);
      c.strokeStyle = RING; c.lineWidth = 1.2;
      for (const k of [0.65, 0.35]) { c.beginPath(); c.ellipse(x, 0, rx * k, ry * k, 0, 0, Math.PI * 2); c.stroke(); }
    }
  }
  // growth rings filling a cross-cut face
  function crossCut(c, R, rings) {
    c.strokeStyle = RING; c.lineWidth = 1.3;
    for (let i = 1; i <= rings; i++) {
      const r = (R * i) / rings;
      c.beginPath();
      for (let a = 0; a <= Math.PI * 2 + 0.1; a += 0.25) c.lineTo(Math.cos(a) * r * (1 + Math.sin(a * 3 + i) * 0.03), Math.sin(a) * r * (1 + Math.cos(a * 2 + i) * 0.03));
      c.stroke();
    }
  }
  // lighter cut surface along the top of a stump, with a hint of rings
  function cutTop(c, L, color) {
    const h = L.h * 0.22;
    c.fillStyle = color; c.fillRect(L.minX - 6, L.minY - 6, L.w + 12, h + 6);
    c.strokeStyle = RING; c.lineWidth = 1.2;
    for (const k of [0.75, 0.45, 0.18]) { c.beginPath(); c.ellipse(0, L.minY + h * 0.5, (L.w / 2) * k, h * 0.32 * k, 0, 0, Math.PI * 2); c.stroke(); }
    c.strokeStyle = 'rgba(60,35,15,0.35)'; c.lineWidth = 1.5;
    c.beginPath(); c.moveTo(L.minX - 6, L.minY + h); c.lineTo(L.maxX + 6, L.minY + h); c.stroke();
  }
  // small dark knots
  function knots(c, pts) { for (const [x, y, r] of pts) { ell(c, x, y, r, r * 0.7, 'rgba(60,35,15,0.4)'); ell(c, x, y, r * 0.5, r * 0.35, 'rgba(40,22,10,0.5)'); } }
  // the hollow the critter lives in
  function hollow(c, rx, ry, dy = 0) {
    ell(c, 0, dy, rx * 1.08, ry * 1.08, 'rgba(60,35,15,0.45)');
    ell(c, 0, dy, rx, ry, '#3b2a1e');
    ell(c, 0, dy + ry * 0.15, rx * 0.85, ry * 0.8, '#22160d');
  }

  const HOMES = {
    'log'(c, L, f) {              // squirrel: side-on log with a knot hole
      grainH(c, L, 3);
      endRings(c, L, '#d2a679');
      hollow(c, f * 0.72, f * 0.5);
    },
    'block'(c, L, f) {            // bunny: squared-off chunk of timber
      grainV(c, L, 4, 1.5);
      knots(c, [[L.minX + L.w * 0.2, L.minY + L.h * 0.18, 4], [L.maxX - L.w * 0.18, L.maxY - L.h * 0.2, 3.5]]);
      hollow(c, f * 0.62, f * 0.62);
    },
    'plank'(c, L, f) {            // owl: bark trunk with a hollow
      grainV(c, L, 3, 2);
      knots(c, [[L.minX + L.w * 0.3, L.maxY - L.h * 0.12, 3]]);
      hollow(c, f * 0.62, f * 0.72);
    },
    'wedge'(c, L, f) {            // fox: split-log wedge, bark along the slanted sides
      grainH(c, L, 4, 1.2);
      c.strokeStyle = 'rgba(60,35,15,0.45)'; c.lineWidth = 6;
      c.beginPath(); c.moveTo(L.minX, L.maxY); c.lineTo(0, L.minY); c.lineTo(L.maxX, L.maxY); c.stroke();
      hollow(c, f * 0.66, f * 0.5, f * 0.08);
    },
    'stump'(c, L, f) {            // frog: tree stump, cut surface on top
      grainV(c, L, 5, 2);
      cutTop(c, L, '#d9b48a');
      hollow(c, f * 0.62, f * 0.5, f * 0.12);
    },
    'log end'(c, L, f) {          // raccoon: hollow log seen end-on
      crossCut(c, L.w * 0.55, 5);
      hollow(c, f * 0.62, f * 0.62);
    },
    'round log'(c, L, f) {        // hedgehog: round log end-on
      crossCut(c, L.w * 0.5, 4);
      hollow(c, f * 0.6, f * 0.6);
    },
    'great log'(c, L, f) {        // bear: big hollow log
      grainH(c, L, 4, 2);
      endRings(c, L, '#c99a67');
      hollow(c, f * 0.9, f * 0.72);
    },
    'trunk'(c, L, f) {            // wolf: thick trunk section with a big hollow
      grainV(c, L, 6, 2.5);
      knots(c, [[L.minX + L.w * 0.18, L.minY + L.h * 0.15, 5], [L.maxX - L.w * 0.15, L.maxY - L.h * 0.22, 4]]);
      hollow(c, f * 0.82, f * 0.74, f * 0.04);
    },
    'great stump'(c, L, f) {      // deer: wide stump
      grainV(c, L, 7, 2.5);
      cutTop(c, L, '#dcb890');
      hollow(c, f * 0.7, f * 0.58, f * 0.1);
    },
  };

  window.drawHome = function (c, name, L, faceSize) {
    const fn = HOMES[name];
    if (fn) fn(c, L, faceSize);
  };
})();

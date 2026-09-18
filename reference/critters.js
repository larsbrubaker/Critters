// Hand-drawn critter faces with expressions, rendered on canvas.
// drawCritter(ctx, name, size, expression, blink, t)
//   name: bunny | squirrel | owl | fox | frog | raccoon | hedgehog | bear | wolf | deer
//   size: approximate face height in px
//   expression: happy | curious | scared | worried | relieved
(() => {
  const OUTLINE = '#3a2718';

  const PALETTE = {
    bunny:    { body: '#f4f1ea', inner: '#f5a6b8', muzzle: '#ffffff', nose: '#e77c98' },
    squirrel: { body: '#c2743a', inner: '#e9b68a', muzzle: '#f0cfa5', nose: '#3a2718', tail: '#a85a25' },
    owl:      { body: '#8a6a4a', inner: '#d9b48a', muzzle: '#efe0c4', nose: '#f39c2b', iris: '#f2b632', ring: '#5a4030' },
    fox:      { body: '#ec7f2f', inner: '#3a2718', muzzle: '#fff5e8', nose: '#3a2718' },
    frog:     { body: '#63b552', inner: '#8fd67a', muzzle: '#a6dd8f', nose: '#2f6b2a' },
    raccoon:  { body: '#9a9a96', inner: '#c8c8c4', muzzle: '#e8e8e4', nose: '#3a2718', mask: '#3f3a36' },
    hedgehog: { body: '#d5aa7c', inner: '#f0d6b8', muzzle: '#f4e2cc', nose: '#3a2718', spikes: '#6b4a2e' },
    bear:     { body: '#8f5b2c', inner: '#c99a67', muzzle: '#d9b17e', nose: '#3a2718' },
    wolf:     { body: '#7d8a8c', inner: '#c5cdcf', muzzle: '#e3e8e8', nose: '#3a2718', iris: '#e8c03a' },
    deer:     { body: '#c68d55', inner: '#efd2ad', muzzle: '#f3e1c4', nose: '#3a2718', antler: '#7a5230' },
  };

  function line(c, x1, y1, x2, y2) { c.beginPath(); c.moveTo(x1, y1); c.lineTo(x2, y2); c.stroke(); }
  function ellipse(c, x, y, rx, ry, fill, stroke) {
    c.beginPath(); c.ellipse(x, y, rx, ry, 0, 0, Math.PI * 2);
    if (fill) { c.fillStyle = fill; c.fill(); }
    if (stroke) c.stroke();
  }
  function tri(c, x1, y1, x2, y2, x3, y3, fill, stroke) {
    c.beginPath(); c.moveTo(x1, y1); c.lineTo(x2, y2); c.lineTo(x3, y3); c.closePath();
    if (fill) { c.fillStyle = fill; c.fill(); }
    if (stroke) c.stroke();
  }

  // Things drawn behind the head: ears, tails, spikes, antlers, tufts.
  function drawBehind(c, name, u, P) {
    switch (name) {
      case 'bunny':
        // long ears, the right one flopped over
        for (const s of [-1, 1]) {
          c.save(); c.translate(s * 0.4 * u, -0.98 * u); c.rotate(s === -1 ? -0.14 : 0.55);
          ellipse(c, 0, 0, 0.23 * u, 0.7 * u, P.body, true);
          ellipse(c, 0, 0.06 * u, 0.11 * u, 0.5 * u, P.inner, false);
          c.restore();
        }
        // fur tuft on top
        for (const [x, y, r] of [[-0.18, -0.86, 0.14], [0.02, -0.94, 0.15], [0.2, -0.86, 0.13]]) ellipse(c, x * u, y * u, r * u, r * u, P.body, true);
        break;
      case 'fox':
      case 'wolf':
        for (const s of [-1, 1]) {
          tri(c, s * 0.3 * u, -0.55 * u, s * 0.95 * u, -0.45 * u, s * 0.7 * u, -1.25 * u, P.body, true);
          tri(c, s * 0.45 * u, -0.62 * u, s * 0.82 * u, -0.57 * u, s * 0.68 * u, -1.05 * u, P.inner, false);
        }
        break;
      case 'squirrel':
        c.save(); c.translate(0.55 * u, 0.1 * u);
        ellipse(c, 0.35 * u, -0.25 * u, 0.42 * u, 0.85 * u, P.tail, true);
        ellipse(c, 0.42 * u, -0.35 * u, 0.2 * u, 0.55 * u, P.inner, false);
        c.restore();
        for (const s of [-1, 1]) {
          ellipse(c, s * 0.62 * u, -0.7 * u, 0.24 * u, 0.26 * u, P.body, true);
          ellipse(c, s * 0.62 * u, -0.68 * u, 0.12 * u, 0.14 * u, P.inner, false);
        }
        break;
      case 'bear':
      case 'raccoon':
        for (const s of [-1, 1]) {
          ellipse(c, s * 0.68 * u, -0.6 * u, 0.3 * u, 0.3 * u, P.body, true);
          ellipse(c, s * 0.68 * u, -0.58 * u, 0.16 * u, 0.16 * u, P.inner, false);
        }
        break;
      case 'owl':
        for (const s of [-1, 1]) {
          tri(c, s * 0.25 * u, -0.72 * u, s * 0.8 * u, -0.62 * u, s * 0.62 * u, -1.25 * u, P.body, true);
          tri(c, s * 0.5 * u, -0.7 * u, s * 0.95 * u, -0.55 * u, s * 0.9 * u, -1.05 * u, P.body, true);
        }
        break;
      case 'frog':
        for (const s of [-1, 1]) ellipse(c, s * 0.48 * u, -0.72 * u, 0.34 * u, 0.34 * u, P.body, true);
        break;
      case 'hedgehog':
        for (let a = -165; a <= -15; a += 18) {
          const r = a * Math.PI / 180;
          const bx = Math.cos(r) * 0.78 * u, by = Math.sin(r) * 0.7 * u;
          const tx = Math.cos(r) * 1.35 * u, ty = Math.sin(r) * 1.25 * u;
          const px = -Math.sin(r) * 0.16 * u, py = Math.cos(r) * 0.16 * u;
          tri(c, bx - px, by - py, bx + px, by + py, tx, ty, P.spikes, true);
        }
        break;
      case 'deer':
        c.strokeStyle = P.antler; c.lineCap = 'round';
        c.save(); c.lineWidth = Math.max(1.5, u * 0.11);
        for (const s of [-1, 1]) {
          line(c, s * 0.4 * u, -0.7 * u, s * 0.55 * u, -1.35 * u);
          line(c, s * 0.48 * u, -1.05 * u, s * 0.85 * u, -1.25 * u);
          line(c, s * 0.52 * u, -1.2 * u, s * 0.3 * u, -1.45 * u);
        }
        c.restore();
        c.strokeStyle = OUTLINE;
        for (const s of [-1, 1]) {
          c.save(); c.translate(s * 0.85 * u, -0.45 * u); c.rotate(s * 0.9);
          ellipse(c, 0, 0, 0.2 * u, 0.42 * u, P.body, true);
          ellipse(c, 0, 0, 0.09 * u, 0.26 * u, P.inner, false);
          c.restore();
        }
        break;
    }
  }

  function drawMarkings(c, name, u, P) {
    switch (name) {
      case 'fox':
      case 'wolf':
        ellipse(c, 0, 0.32 * u, 0.62 * u, 0.48 * u, P.muzzle, false);
        break;
      case 'bear':
      case 'deer':
      case 'squirrel':
        ellipse(c, 0, 0.36 * u, 0.48 * u, 0.36 * u, P.muzzle, false);
        break;
      case 'raccoon':
        ellipse(c, 0, 0.4 * u, 0.5 * u, 0.36 * u, P.muzzle, false);
        ellipse(c, 0, -0.1 * u, 0.86 * u, 0.3 * u, P.mask, false);
        for (const s of [-1, 1]) ellipse(c, s * 0.34 * u, -0.1 * u, 0.24 * u, 0.2 * u, P.inner, false);
        break;
      case 'owl':
        // heart-shaped facial disc: two overlapping cream circles, one around each eye
        for (const s of [-1, 1]) ellipse(c, s * 0.36 * u, -0.04 * u, 0.46 * u, 0.5 * u, P.muzzle, false);
        c.save(); c.lineWidth = Math.max(1, u * 0.06); c.strokeStyle = P.ring;
        for (const s of [-1, 1]) { c.beginPath(); c.ellipse(s * 0.36 * u, -0.04 * u, 0.46 * u, 0.5 * u, 0, 0, Math.PI * 2); c.stroke(); }
        // chest feathers along the bottom of the head
        c.strokeStyle = 'rgba(60,40,25,0.55)'; c.lineWidth = Math.max(1, u * 0.07);
        for (const x of [-0.42, -0.14, 0.14, 0.42]) { c.beginPath(); c.arc(x * u, 0.62 * u, 0.14 * u, Math.PI * 0.1, Math.PI * 0.9); c.stroke(); }
        c.restore();
        break;
      case 'hedgehog':
        ellipse(c, 0, 0.3 * u, 0.55 * u, 0.42 * u, P.muzzle, false);
        break;
    }
  }

  function drawEye(c, x, y, r, expr, blink, P) {
    c.save();
    c.lineWidth = Math.max(1.2, r * 0.32);
    c.lineCap = 'round';
    c.strokeStyle = OUTLINE;
    if (expr === 'relieved') {
      c.beginPath(); c.arc(x, y + r * 0.45, r * 0.85, Math.PI * 1.15, Math.PI * 1.85); c.stroke();
      c.restore(); return;
    }
    if (blink) { line(c, x - r * 0.8, y, x + r * 0.8, y); c.restore(); return; }
    const k = expr === 'scared' ? 1.3 : expr === 'curious' ? 1.15 : 1;
    const pr = expr === 'scared' ? 0.4 : expr === 'worried' ? 0.52 : 0.6;
    const py = expr === 'curious' ? -r * 0.18 : expr === 'worried' ? r * 0.1 : 0;
    ellipse(c, x, y, r * k, r * k, '#ffffff', false);
    c.lineWidth = Math.max(1, r * 0.18); c.stroke();
    ellipse(c, x, y + py, r * k * pr, r * k * pr, P.iris || OUTLINE, false);
    if (P.iris) ellipse(c, x, y + py, r * k * pr * 0.55, r * k * pr * 0.55, OUTLINE, false);
    ellipse(c, x - r * k * pr * 0.35, y + py - r * k * pr * 0.35, r * k * 0.16, r * k * 0.16, '#ffffff', false);
    ellipse(c, x + r * k * pr * 0.3, y + py + r * k * pr * 0.3, r * k * 0.07, r * k * 0.07, '#ffffff', false);
    c.restore();
  }

  function drawBrows(c, ex, ey, r, expr) {
    if (expr !== 'scared' && expr !== 'worried') return;
    c.save();
    c.lineWidth = Math.max(1.2, r * 0.3); c.lineCap = 'round'; c.strokeStyle = OUTLINE;
    const lift = expr === 'scared' ? 2.1 : 1.8;
    for (const s of [-1, 1]) line(c, s * ex - s * r * 0.9, ey - r * 1.35, s * ex + s * r * 0.5, ey - r * lift);
    c.restore();
  }

  function drawMouth(c, name, u, P, expr) {
    c.save();
    c.lineWidth = Math.max(1.2, u * 0.09); c.lineCap = 'round'; c.strokeStyle = OUTLINE;
    const my = name === 'frog' ? 0.28 * u : 0.42 * u;
    const w = name === 'frog' ? 0.5 * u : 0.26 * u;
    if (name === 'bunny' && expr === 'happy') {
      // split "ω" lip under the nose with two buck teeth
      c.lineWidth = Math.max(1.2, u * 0.08);
      line(c, 0, 0.27 * u, 0, 0.36 * u);
      c.beginPath(); c.arc(-0.11 * u, 0.33 * u, 0.11 * u, Math.PI * 0.1, Math.PI * 0.9); c.stroke();
      c.beginPath(); c.arc(0.11 * u, 0.33 * u, 0.11 * u, Math.PI * 0.1, Math.PI * 0.9); c.stroke();
      c.fillStyle = '#fff';
      c.fillRect(-0.085 * u, 0.36 * u, 0.08 * u, 0.17 * u);
      c.fillRect(0.005 * u, 0.36 * u, 0.08 * u, 0.17 * u);
      c.lineWidth = Math.max(1, u * 0.045);
      c.strokeRect(-0.085 * u, 0.36 * u, 0.17 * u, 0.17 * u);
      line(c, 0, 0.36 * u, 0, 0.53 * u);
      c.restore();
      return;
    }
    switch (expr) {
      case 'happy':
        c.beginPath(); c.arc(0, my - w * 0.5, w, Math.PI * 0.15, Math.PI * 0.85); c.stroke();
        break;
      case 'relieved':
        c.beginPath(); c.arc(0, my - w * 0.3, w * 1.25, Math.PI * 0.1, Math.PI * 0.9); c.stroke();
        c.fillStyle = 'rgba(240,120,140,0.45)';
        for (const s of [-1, 1]) ellipse(c, s * 0.58 * u, 0.22 * u, 0.14 * u, 0.09 * u, 'rgba(240,120,140,0.45)', false);
        break;
      case 'curious':
        ellipse(c, 0, my + 0.05 * u, 0.09 * u, 0.1 * u, OUTLINE, false);
        break;
      case 'scared':
        ellipse(c, 0, my + 0.08 * u, w * 0.75, w * 0.62, '#4a2323', true);
        ellipse(c, 0, my + 0.25 * u, w * 0.4, w * 0.2, '#e07a8a', false);
        break;
      case 'worried':
        c.beginPath();
        c.moveTo(-w, my + 0.04 * u);
        c.quadraticCurveTo(-w * 0.5, my - 0.12 * u, 0, my + 0.04 * u);
        c.quadraticCurveTo(w * 0.5, my + 0.2 * u, w, my + 0.04 * u);
        c.stroke();
        break;
    }
    c.restore();
  }

  function drawNose(c, name, u, P) {
    if (name === 'bunny') {
      // tiny pink nose, slightly heart-shaped
      c.beginPath();
      c.moveTo(-0.1 * u, 0.16 * u); c.lineTo(0.1 * u, 0.16 * u); c.lineTo(0, 0.27 * u); c.closePath();
      c.fillStyle = P.nose; c.fill();
      ellipse(c, -0.05 * u, 0.16 * u, 0.055 * u, 0.045 * u, P.nose, false);
      ellipse(c, 0.05 * u, 0.16 * u, 0.055 * u, 0.045 * u, P.nose, false);
      return;
    }
    if (name === 'owl') { tri(c, -0.11 * u, 0.1 * u, 0.11 * u, 0.1 * u, 0, 0.42 * u, P.nose, true); return; }
    if (name === 'frog') { for (const s of [-1, 1]) ellipse(c, s * 0.12 * u, 0.02 * u, 0.04 * u, 0.03 * u, P.nose, false); return; }
    ellipse(c, 0, 0.14 * u, 0.1 * u, 0.075 * u, P.nose, false);
  }

  function drawCritter(c, name, size, expr, blink) {
    const P = PALETTE[name];
    if (!P) return;
    const u = size / 2;
    c.save();
    c.lineWidth = Math.max(1.2, u * 0.085);
    c.strokeStyle = OUTLINE;
    c.lineJoin = 'round';
    drawBehind(c, name, u, P);
    // head
    if (name === 'bunny') ellipse(c, 0, 0.03 * u, 0.98 * u, 0.86 * u, P.body, true);
    else ellipse(c, 0, 0, 0.9 * u, 0.82 * u, P.body, true);
    drawMarkings(c, name, u, P);
    if (name === 'bunny') {
      // rosy cheeks and whiskers
      for (const s of [-1, 1]) {
        ellipse(c, s * 0.6 * u, 0.32 * u, 0.17 * u, 0.1 * u, 'rgba(245,150,170,0.55)', false);
        c.save(); c.strokeStyle = 'rgba(90,70,60,0.5)'; c.lineWidth = Math.max(1, u * 0.04); c.lineCap = 'round';
        line(c, s * 0.5 * u, 0.3 * u, s * 0.96 * u, 0.2 * u);
        line(c, s * 0.5 * u, 0.36 * u, s * 0.96 * u, 0.42 * u);
        c.restore();
      }
    }
    // eyes
    let ex = 0.33 * u, ey = -0.12 * u, er = 0.13 * u;
    if (name === 'bunny') { ex = 0.37 * u; ey = -0.04 * u; er = 0.165 * u; }
    if (name === 'owl') { ex = 0.36 * u; ey = -0.06 * u; er = 0.25 * u; }
    if (name === 'frog') { ex = 0.48 * u; ey = -0.72 * u; er = 0.17 * u; }
    drawEye(c, -ex, ey, er, expr, blink, P);
    drawEye(c, ex, ey, er, expr, blink, P);
    drawBrows(c, ex, ey, er, expr);
    drawNose(c, name, u, P);
    drawMouth(c, name, u, P, expr);
    c.restore();
  }

  window.drawCritter = drawCritter;
  window.CRITTER_NAMES = Object.keys(PALETTE);
})();

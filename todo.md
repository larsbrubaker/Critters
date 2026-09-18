# Critters — remaining work

This document tracks ONLY work that remains. As items complete, delete
them in the same commit that finishes the work.

- Compare the feel of tall towers (stability cheat, freezing) against the
  original side by side and tune the Box2D translation in `physics.rs` if
  towers topple earlier or later than in Matter.js.
- Mobile: verify touch drag-to-drop and the on-screen tray on a phone via
  the Pages deploy.
- agg-gui: `WgpuGfxCtx::save/restore` only stacks transform + clip; global
  alpha, dash, colours and line width leak past `restore()` (the software
  `GfxCtx` restores them). Critters works around it in `render/canvas.rs`;
  the backends should agree.

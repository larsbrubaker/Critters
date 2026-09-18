//! One critter piece: the Matter.js body fields the original game logic
//! reads (`position`, `angle`, `speed`, `bounds`, `isSleeping`, …) plus the
//! bookkeeping `game.js` hung on the body (`stillFrames`, `locked`, `rest`,
//! `hasLanded`, `anim`, `stabilityMult`). The gameplay rules that act on a
//! single body — `trackStillness`, `trackLock`, `fallReason`, `pieceScore`,
//! `localExtents`, `applyStability` — are ported here verbatim. Pieces in
//! the world mirror a Box2D body through `physics.rs`; held and tray pieces
//! have no body and are positioned directly.

use crate::config::{
    ShapeDef, ShapeKind, BASE_DENSITY, BASE_FRICTION, BASE_FRICTION_AIR, BASE_FRICTION_STATIC,
    FLOOR_Y, FREEZE_INSET, PX_PER_M, SETTLE_FRAMES, STABILITY, STUMP_W, UNLOCK_ANGLE, UNLOCK_DIST,
    W,
};
use crate::geometry::{local_outline, world_vertices, Bounds, P};
use crate::physics::{BodyHandle, Physics};

/// Why a piece counts as lost. The original had three rules (slipped below
/// the stump top, centre past the stump edge, tumbled 1.5 m); the port keeps
/// only the one that is visibly a fall — see `zoom.rs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallReason {
    HitTheGround,
}

impl FallReason {
    pub fn text(self) -> &'static str {
        match self {
            FallReason::HitTheGround => "hit the ground",
        }
    }
}

/// `b.rest`: where a piece settled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rest {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    pub top: f64,
}

/// Extents of a body's outline in its own unrotated frame (`localExtents`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Extents {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub w: f64,
    pub h: f64,
}

pub struct Piece {
    pub spec: ShapeDef,
    pub body: Option<BodyHandle>,
    /// Outline in the piece frame (None for the round log).
    pub local: Option<Vec<P>>,
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    /// Velocity in px per 1/60 s step (Matter's `body.velocity`).
    pub velocity: P,
    pub speed: f64,
    pub angular_speed: f64,
    pub is_sleeping: bool,
    pub is_static: bool,
    /// World-space outline (empty for the round log).
    pub vertices: Vec<P>,
    pub bounds: Bounds,
    pub still_frames: u32,
    pub locked: bool,
    pub rest: Option<Rest>,
    pub highest_rest_y: Option<f64>,
    pub has_landed: bool,
    /// Touched the forest floor (contact event from `game.rs`).
    pub hit_ground: bool,
    pub anim_phase: f64,
    /// Time the piece landed, or -1.
    pub land_at: f64,
    pub stability_mult: f64,
    density: f64,
    friction: f64,
    friction_static: f64,
    friction_air: f64,
}

impl Piece {
    /// `makeBody(spec, x, y)` without adding it to the world.
    pub fn new(spec: ShapeDef, x: f64, y: f64, anim_phase: f64) -> Self {
        let mut piece = Self {
            spec,
            body: None,
            local: local_outline(spec.kind),
            x,
            y,
            angle: 0.0,
            velocity: P::default(),
            speed: 0.0,
            angular_speed: 0.0,
            is_sleeping: false,
            is_static: false,
            vertices: Vec::new(),
            bounds: Bounds::default(),
            still_frames: 0,
            locked: false,
            rest: None,
            highest_rest_y: None,
            has_landed: false,
            hit_ground: false,
            anim_phase,
            land_at: -1.0,
            stability_mult: 1.0,
            density: BASE_DENSITY,
            friction: BASE_FRICTION,
            friction_static: BASE_FRICTION_STATIC,
            friction_air: BASE_FRICTION_AIR,
        };
        piece.refresh_geometry();
        piece
    }

    pub fn kind(&self) -> ShapeKind {
        self.spec.kind
    }

    fn refresh_geometry(&mut self) {
        match (&self.local, self.spec.kind) {
            (Some(local), _) => {
                self.vertices = world_vertices(local, self.x, self.y, self.angle);
                self.bounds = Bounds::of(&self.vertices, self.velocity);
            }
            (None, ShapeKind::Round { r }) => {
                self.vertices.clear();
                let corners = [
                    P::new(self.x - r, self.y - r),
                    P::new(self.x + r, self.y + r),
                ];
                self.bounds = Bounds::of(&corners, self.velocity);
            }
            (None, _) => {}
        }
    }

    /// `Body.setPosition(b, pos)` for a piece outside the world (held or in a
    /// tray slot). Velocity stays zero, like the original's non-updating call.
    pub fn set_position(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
        self.refresh_geometry();
    }

    /// `Body.setAngle` for the debug preview.
    pub fn set_angle(&mut self, angle: f64) {
        self.angle = angle;
        self.refresh_geometry();
    }

    /// `Composite.add(world, held)`: create the Box2D body at the current spot.
    pub fn add_to_world(&mut self, physics: &mut Physics, index: usize) {
        let handle = physics.add_piece(
            self.spec.kind,
            self.local.as_deref(),
            self.x,
            self.y,
            index as u64,
        );
        self.body = Some(handle);
        self.sync_from(physics);
    }

    /// Pull the body's state after a physics step.
    pub fn sync_from(&mut self, physics: &Physics) {
        let Some(handle) = self.body else {
            return;
        };
        let s = physics.state(handle);
        self.x = s.x;
        self.y = s.y;
        self.angle = s.angle;
        self.velocity = P::new(s.vx / 60.0, s.vy / 60.0);
        self.speed = (self.velocity.x * self.velocity.x + self.velocity.y * self.velocity.y).sqrt();
        self.angular_speed = (s.omega / 60.0).abs();
        self.is_sleeping = s.sleeping;
        self.is_static = s.is_static;
        self.refresh_geometry();
    }

    /// `isSettled`: asleep, or nearly motionless for a sustained stretch, so
    /// the top of a bounce does not register as a resting height.
    pub fn is_settled(&self) -> bool {
        self.is_sleeping || self.still_frames >= SETTLE_FRAMES
    }

    /// `trackStillness`, called once per physics step.
    pub fn track_stillness(&mut self) {
        if self.speed < 0.3 && self.angular_speed < 0.02 {
            self.still_frames += 1;
        } else {
            self.still_frames = 0;
        }
    }

    /// `trackLock`: once a piece settles it "locks" and keeps counting toward
    /// height and score even while a new piece landing on it makes it twitch.
    /// It only unlocks if it actually moves a meaningful amount.
    pub fn track_lock(&mut self) {
        if let Some(rest) = self.rest.filter(|_| self.locked) {
            let dist = ((self.x - rest.x).powi(2) + (self.y - rest.y).powi(2)).sqrt();
            let moved =
                dist > UNLOCK_DIST || wrap_angle(self.angle - rest.angle).abs() > UNLOCK_ANGLE;
            if moved {
                self.locked = false;
                self.still_frames = 0;
            }
        }
        if !self.locked && self.is_settled() {
            self.locked = true;
            self.rest = Some(Rest {
                x: self.x,
                y: self.y,
                angle: self.angle,
                top: self.bounds.min.y,
            });
            self.highest_rest_y = Some(match self.highest_rest_y {
                Some(h) => h.min(self.y),
                None => self.y,
            });
        }
    }

    /// `overStump`: centre within the stump's span.
    pub fn over_stump(&self) -> bool {
        (self.x - W / 2.0).abs() <= STUMP_W / 2.0
    }

    /// `wellInside`: centre well inside the stump's span (freeze eligibility).
    pub fn well_inside(&self) -> bool {
        (self.x - W / 2.0).abs() <= STUMP_W / 2.0 - FREEZE_INSET
    }

    /// `fallReason`, with the port's rule: a piece is lost only when it hits
    /// the ground. Overhangs, pieces resting past the stump's edge and pieces
    /// that tumble but are caught by the tower are all still in play. The
    /// floor contact event is the signal; the bounds check is a backstop for
    /// a contact that begins and ends inside one step.
    pub fn fall_reason(&self) -> Option<FallReason> {
        (self.hit_ground || self.bounds.max.y >= FLOOR_Y - 1.0).then_some(FallReason::HitTheGround)
    }

    /// `pieceScore`: the critter's value times a height multiplier.
    pub fn score(&self) -> f64 {
        let rest_y = self.rest.map(|r| r.y).unwrap_or(0.0);
        let h_m = (-rest_y).max(0.0) / PX_PER_M;
        self.spec.points as f64 * (1.0 + h_m)
    }

    /// `localExtents`.
    pub fn local_extents(&self) -> Extents {
        if let ShapeKind::Round { r } = self.spec.kind {
            return Extents {
                min_x: -r,
                max_x: r,
                min_y: -r,
                max_y: r,
                w: r * 2.0,
                h: r * 2.0,
            };
        }
        let (sin, cos) = (-self.angle).sin_cos();
        let mut e = Extents {
            min_x: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            min_y: f64::INFINITY,
            max_y: f64::NEG_INFINITY,
            w: 0.0,
            h: 0.0,
        };
        for v in &self.vertices {
            let dx = v.x - self.x;
            let dy = v.y - self.y;
            let x = dx * cos - dy * sin;
            let y = dx * sin + dy * cos;
            e.min_x = e.min_x.min(x);
            e.max_x = e.max_x.max(x);
            e.min_y = e.min_y.min(y);
            e.max_y = e.max_y.max(y);
        }
        e.w = e.max_x - e.min_x;
        e.h = e.max_y - e.min_y;
        e
    }

    /// `applyStability`: only pieces resting on the tower (locked, and over the
    /// stump) get the treatment; a piece that is still moving or has strayed
    /// off to the side must never be frozen into an invisible ledge.
    pub fn apply_stability(&mut self, physics: &mut Physics, tower_top: f64) {
        if !self.locked || !self.over_stump() {
            self.stability_mult = 1.0;
            return;
        }
        let depth = (self.y - tower_top) / PX_PER_M;
        let ex = (depth - STABILITY.free_depth).max(0.0);
        let mult = (1.0 + STABILITY.density_per_metre * ex).min(STABILITY.max_density_mult);
        self.stability_mult = mult;
        if self.is_static {
            return;
        }
        let Some(handle) = self.body else {
            return;
        };
        // freezing is reserved for pieces well inside the stump's span; a piece hugging the edge stays live
        if STABILITY.freeze_depth > 0.0 && depth > STABILITY.freeze_depth && self.well_inside() {
            physics.set_static(handle);
            self.is_static = true;
            return;
        }
        let density = BASE_DENSITY * mult;
        if (density - self.density).abs() / self.density > 0.02 {
            physics.set_density(handle, density);
            self.density = density;
        }
        let friction = (0.9 + STABILITY.friction_per_metre * ex).min(2.0);
        if friction != self.friction {
            physics.set_friction(handle, friction);
            self.friction = friction;
        }
        // Box2D has no static coefficient; kept for parity with the original's bookkeeping.
        self.friction_static = (1.5 + STABILITY.friction_per_metre * ex).min(3.0);
        let air = (0.01 + STABILITY.air_per_metre * ex).min(0.3);
        if air != self.friction_air {
            physics.set_friction_air(handle, air);
            self.friction_air = air;
        }
    }
}

/// Difference of two angles wrapped to `(-π, π]` — Box2D reports rotations
/// wrapped, whereas Matter accumulated them, so the unlock test compares the
/// shortest way round.
pub fn wrap_angle(a: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    let mut a = (a + std::f64::consts::PI) % tau;
    if a < 0.0 {
        a += tau;
    }
    a - std::f64::consts::PI
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SHAPES;

    fn block() -> Piece {
        Piece::new(SHAPES[1], W / 2.0, -30.0, 0.0)
    }

    #[test]
    fn settle_requires_thirty_still_frames_or_sleep() {
        let mut p = block();
        for _ in 0..SETTLE_FRAMES - 1 {
            p.track_stillness();
            assert!(!p.is_settled());
        }
        p.track_stillness();
        assert!(p.is_settled());
        p.speed = 1.0;
        p.track_stillness();
        assert_eq!(p.still_frames, 0);
        p.is_sleeping = true;
        assert!(p.is_settled());
    }

    #[test]
    fn lock_records_rest_and_unlocks_on_real_movement() {
        let mut p = block();
        p.is_sleeping = true;
        p.track_lock();
        assert!(p.locked);
        let rest = p.rest.unwrap();
        assert_eq!(rest.y, -30.0);
        assert_eq!(rest.top, -60.0);
        assert_eq!(p.highest_rest_y, Some(-30.0));
        // a twitch keeps the lock
        p.set_position(W / 2.0 + 10.0, -30.0);
        p.track_lock();
        assert!(p.locked);
        // a real slide unlocks it (and, awake, it stays unlocked)
        p.is_sleeping = false;
        p.set_position(W / 2.0 + 30.0, -30.0);
        p.track_lock();
        assert!(!p.locked);
        assert_eq!(p.still_frames, 0);
        // spinning a full turn is "no movement" once wrapped
        let mut q = block();
        q.is_sleeping = true;
        q.track_lock();
        q.set_angle(std::f64::consts::TAU);
        q.track_lock();
        assert!(q.locked);
    }

    #[test]
    fn score_uses_resting_height_in_metres() {
        let mut p = block();
        p.set_position(W / 2.0, -250.0);
        p.is_sleeping = true;
        p.track_lock();
        assert!((p.score() - 200.0 * 3.5).abs() < 1e-9);
    }

    #[test]
    fn only_the_ground_loses_the_round() {
        let mut p = block();
        assert_eq!(p.fall_reason(), None);
        // resting far past the stump's edge, level with the tower: legal
        p.set_position(W / 2.0 + STUMP_W / 2.0 + 200.0, -100.0);
        assert_eq!(p.fall_reason(), None);
        // hanging below the stump top beside the stump, not touching the floor: legal
        p.set_position(W / 2.0 + STUMP_W / 2.0 + 40.0, 60.0);
        assert_eq!(p.fall_reason(), None);
        // a rested piece that slid a long way down but was caught: legal
        let mut t = block();
        t.set_position(W / 2.0, -900.0);
        t.is_sleeping = true;
        t.track_lock();
        t.set_position(W / 2.0, -300.0);
        assert_eq!(t.fall_reason(), None);
        // floor contact, or bounds reaching the floor, ends it
        t.hit_ground = true;
        assert_eq!(t.fall_reason(), Some(FallReason::HitTheGround));
        let mut q = block();
        q.set_position(W / 2.0 + 300.0, FLOOR_Y - 30.0);
        assert_eq!(q.fall_reason(), Some(FallReason::HitTheGround));
    }

    #[test]
    fn local_extents_undo_the_rotation() {
        let mut p = Piece::new(SHAPES[0], 100.0, 100.0, 0.0); // 120 × 36 log
        p.set_angle(1.2);
        let e = p.local_extents();
        assert!((e.w - 120.0).abs() < 1e-9);
        assert!((e.h - 36.0).abs() < 1e-9);
        assert!((e.min_x + 60.0).abs() < 1e-9);
        let round = Piece::new(SHAPES[6], 0.0, 0.0, 0.0);
        assert_eq!(round.local_extents().w, 56.0);
    }

    #[test]
    fn stability_leaves_the_active_layer_alone_and_freezes_deep_pieces() {
        let mut physics = Physics::new();
        let mut p = block();
        p.add_to_world(&mut physics, 0);
        p.locked = true;
        p.rest = Some(Rest {
            x: p.x,
            y: p.y,
            angle: 0.0,
            top: -60.0,
        });
        // top of the tower: no treatment
        p.apply_stability(&mut physics, -30.0);
        assert_eq!(p.stability_mult, 1.0);
        assert!(!p.is_static);
        // 2 m below the top: denser but live
        p.apply_stability(&mut physics, -230.0);
        assert!((p.stability_mult - (1.0 + 2.0 * 1.2)).abs() < 1e-9);
        assert!(!p.is_static);
        // 4 m below the top, well inside: frozen
        p.apply_stability(&mut physics, -430.0);
        assert!(p.is_static);
        assert!(physics.state(p.body.unwrap()).is_static);
        // an unlocked piece is never treated
        let mut q = block();
        q.add_to_world(&mut physics, 1);
        q.apply_stability(&mut physics, -1000.0);
        assert_eq!(q.stability_mult, 1.0);
        assert!(!q.is_static);
    }
}

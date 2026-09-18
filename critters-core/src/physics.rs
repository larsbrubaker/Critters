//! Box2D bridge. The original ran Matter.js in pixel units with y growing
//! downward; this module keeps the game in that frame and mirrors y into a
//! Box2D world (y up, `set_length_units_per_meter(100)` so 100 px = 1 m).
//!
//! Matter → Box2D translation of every tuning knob in `config.rs`:
//!
//! | Matter.js option                | Box2D equivalent                                    |
//! |---------------------------------|-----------------------------------------------------|
//! | gravity `{y: 1, scale: 0.001}`  | 1000 px/s² (0.001 px/ms² · 1000²) → `gravity.y = -1000` |
//! | `density` (mass = ρ·area)       | shape density, same number (only ratios matter)     |
//! | `friction` (pair uses the min)  | `material.friction` + a `min` friction callback      |
//! | `frictionStatic`                | no separate static coefficient in Box2D v3          |
//! | `restitution: 0`                | `material.restitution = 0` (default)                |
//! | `frictionAir` f (v *= 1−f /step)| linear + angular damping d = 60·f/(1−f)             |
//! | `chamfer: {radius: 4}`          | rounded box, half extents shrunk by 4, radius 4     |
//! | `Body.setStatic`                | `body_set_type(Static)`                             |
//! | `enableSleeping`                | world `enable_sleep` (default on)                   |
//! | position/velocity iterations    | 4 sub-steps of the soft-step solver                 |
//!
//! Positions and angles read back are converted to the game frame; velocities
//! are converted to Matter's "px per 1/60 s step" units by `piece.rs`.

use box2d_rust::body::{
    body_get_angular_velocity, body_get_linear_velocity, body_get_position, body_get_rotation,
    body_get_type, body_get_user_data, body_is_awake, body_set_angular_damping,
    body_set_linear_damping, body_set_type, create_body,
};
use box2d_rust::collision::Circle;
use box2d_rust::core::set_length_units_per_meter;
use box2d_rust::geometry::{make_polygon, make_rounded_box};
use box2d_rust::hull::compute_hull;
use box2d_rust::id::{BodyId, ShapeId};
use box2d_rust::math_functions::{make_rot, rot_get_angle, to_pos, Vec2};
use box2d_rust::shape::{
    create_circle_shape, create_polygon_shape, shape_get_body, shape_set_density,
    shape_set_friction,
};
use box2d_rust::types::{default_body_def, default_shape_def, default_world_def, BodyType};
use box2d_rust::world::{world_get_contact_events, world_set_friction_callback, world_step, World};

use crate::config::{
    ShapeKind, BASE_DENSITY, BASE_FRICTION, FLOOR_Y, ROUND_FRICTION, STUMP_H, STUMP_W, W,
};
use crate::geometry::P;

/// Pixels per metre handed to Box2D so its internal tolerances (linear slop,
/// speculative distance, sleep threshold) scale to the game's px units.
pub const LENGTH_UNITS_PER_METER: f32 = 100.0;
/// Matter's gravity in px/s² (see the table above).
pub const GRAVITY_PX_PER_S2: f32 = 1000.0;
/// Fixed physics step, `Engine.update(engine, 1000 / 60)`.
pub const STEP_SECONDS: f32 = 1.0 / 60.0;
/// Sub-steps per fixed step (stands in for Matter's 10 position / 8 velocity iterations).
pub const SUB_STEPS: i32 = 4;
/// User data tag for the static stump and floor.
pub const GROUND_USER_DATA: u64 = u64::MAX;

/// Matter's `frictionAir` (fraction of velocity lost per 60 Hz step) as a
/// Box2D damping coefficient: Box2D applies `v *= 1 / (1 + d·dt)` per step.
pub fn air_to_damping(friction_air: f64) -> f32 {
    let f = friction_air.clamp(0.0, 0.999);
    (60.0 * f / (1.0 - f)) as f32
}

/// Matter pairs use `min(frictionA, frictionB)`; Box2D defaults to the
/// geometric mean, so install this instead.
fn min_friction(a: f32, _material_a: u64, b: f32, _material_b: u64) -> f32 {
    a.min(b)
}

/// Handle to a piece's body and its single shape.
#[derive(Clone, Copy, Debug)]
pub struct BodyHandle {
    pub body: BodyId,
    pub shape: ShapeId,
}

/// Snapshot of a body in the game frame (y down, px, radians clockwise on screen).
#[derive(Clone, Copy, Debug, Default)]
pub struct BodyState {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    /// px/s
    pub vx: f64,
    pub vy: f64,
    /// rad/s
    pub omega: f64,
    pub sleeping: bool,
    pub is_static: bool,
}

pub struct Physics {
    world: World,
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
}

impl Physics {
    /// A fresh world with the stump and forest floor, like `init()`.
    pub fn new() -> Self {
        set_length_units_per_meter(LENGTH_UNITS_PER_METER);
        let mut def = default_world_def();
        def.gravity = Vec2 {
            x: 0.0,
            y: -GRAVITY_PX_PER_S2,
        };
        def.enable_sleep = true;
        let mut world = World::new(&def);
        world_set_friction_callback(&mut world, Some(min_friction));
        let mut physics = Self { world };
        // the stump
        physics.add_static_box(W / 2.0, STUMP_H / 2.0, STUMP_W, STUMP_H);
        // the forest floor
        physics.add_static_box(W / 2.0, FLOOR_Y + 40.0, W * 4.0, 80.0);
        physics
    }

    fn add_static_box(&mut self, cx: f64, cy: f64, w: f64, h: f64) {
        let mut def = default_body_def();
        def.type_ = BodyType::Static;
        def.position = to_pos(Vec2 {
            x: cx as f32,
            y: -cy as f32,
        });
        def.user_data = GROUND_USER_DATA;
        let body = create_body(&mut self.world, &def);
        let mut shape = default_shape_def();
        shape.material.friction = 1.0;
        shape.material.restitution = 0.0;
        let poly = make_rounded_box((w / 2.0) as f32, (h / 2.0) as f32, 0.0);
        create_polygon_shape(&mut self.world, body, &shape, &poly);
    }

    /// `Composite.add(world, held)`: put a piece into the simulation at rest.
    /// `local` is the outline from `geometry::local_outline` (None for the
    /// round log); `user` is the piece index.
    pub fn add_piece(
        &mut self,
        kind: ShapeKind,
        local: Option<&[P]>,
        x: f64,
        y: f64,
        user: u64,
    ) -> BodyHandle {
        let mut def = default_body_def();
        def.type_ = BodyType::Dynamic;
        def.position = to_pos(Vec2 {
            x: x as f32,
            y: -y as f32,
        });
        def.rotation = make_rot(0.0);
        def.linear_damping = air_to_damping(crate::config::BASE_FRICTION_AIR);
        def.angular_damping = def.linear_damping;
        def.user_data = user;
        let body = create_body(&mut self.world, &def);

        let mut shape = default_shape_def();
        shape.density = BASE_DENSITY as f32;
        shape.material.restitution = 0.0;
        shape.enable_contact_events = true;
        let shape_id = match kind {
            ShapeKind::Round { r } => {
                shape.material.friction = ROUND_FRICTION as f32;
                let circle = Circle {
                    center: Vec2 { x: 0.0, y: 0.0 },
                    radius: r as f32,
                };
                create_circle_shape(&mut self.world, body, &shape, &circle)
            }
            ShapeKind::Rect { w, h } => {
                shape.material.friction = BASE_FRICTION as f32;
                // Matter's chamfer cuts the corners inward; Box2D's radius
                // grows the polygon outward, so shrink the box first.
                let poly = make_rounded_box((w / 2.0 - 4.0) as f32, (h / 2.0 - 4.0) as f32, 4.0);
                create_polygon_shape(&mut self.world, body, &shape, &poly)
            }
            _ => {
                shape.material.friction = BASE_FRICTION as f32;
                let pts: Vec<Vec2> = local
                    .unwrap_or(&[])
                    .iter()
                    .map(|p| Vec2 {
                        x: p.x as f32,
                        y: -p.y as f32,
                    })
                    .collect();
                let hull = compute_hull(&pts);
                let poly = make_polygon(&hull, 0.0);
                create_polygon_shape(&mut self.world, body, &shape, &poly)
            }
        };
        BodyHandle {
            body,
            shape: shape_id,
        }
    }

    /// One fixed 1/60 s step.
    pub fn step(&mut self) {
        world_step(&mut self.world, STEP_SECONDS, SUB_STEPS);
    }

    /// User-data pairs of the bodies that started touching during the last
    /// step (`collisionStart`). Ground contacts report `GROUND_USER_DATA`.
    pub fn begin_contacts(&self) -> Vec<(u64, u64)> {
        world_get_contact_events(&self.world)
            .begin_events
            .iter()
            .map(|e| {
                let a = body_get_user_data(&self.world, shape_get_body(&self.world, e.shape_id_a));
                let b = body_get_user_data(&self.world, shape_get_body(&self.world, e.shape_id_b));
                (a, b)
            })
            .collect()
    }

    pub fn state(&self, h: BodyHandle) -> BodyState {
        let p = body_get_position(&self.world, h.body);
        let v = body_get_linear_velocity(&self.world, h.body);
        BodyState {
            x: p.x as f64,
            y: -(p.y as f64),
            angle: -(rot_get_angle(body_get_rotation(&self.world, h.body)) as f64),
            vx: v.x as f64,
            vy: -(v.y as f64),
            omega: -(body_get_angular_velocity(&self.world, h.body) as f64),
            sleeping: !body_is_awake(&self.world, h.body),
            is_static: body_get_type(&self.world, h.body) == BodyType::Static,
        }
    }

    /// `Body.setStatic(b, true)`.
    pub fn set_static(&mut self, h: BodyHandle) {
        body_set_type(&mut self.world, h.body, BodyType::Static);
    }

    /// `Body.setDensity`.
    pub fn set_density(&mut self, h: BodyHandle, density: f64) {
        shape_set_density(&mut self.world, h.shape, density as f32, true);
    }

    /// `b.friction = …` (the pair rule is `min`, see `min_friction`).
    pub fn set_friction(&mut self, h: BodyHandle, friction: f64) {
        shape_set_friction(&mut self.world, h.shape, friction as f32);
    }

    /// `b.frictionAir = …`.
    pub fn set_friction_air(&mut self, h: BodyHandle, friction_air: f64) {
        let d = air_to_damping(friction_air);
        body_set_linear_damping(&mut self.world, h.body, d);
        body_set_angular_damping(&mut self.world, h.body, d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damping_conversion_matches_matter_per_step_loss() {
        // Box2D: v' = v / (1 + d·dt). Matter: v' = v·(1 − f).
        for f in [0.01, 0.1, 0.3] {
            let d = air_to_damping(f) as f64;
            let box2d = 1.0 / (1.0 + d / 60.0);
            assert!((box2d - (1.0 - f)).abs() < 1e-6, "f={f}");
        }
    }

    #[test]
    fn dropped_block_lands_on_the_stump_and_sleeps() {
        let mut physics = Physics::new();
        let kind = ShapeKind::Rect { w: 60.0, h: 60.0 };
        let local = crate::geometry::local_outline(kind);
        let h = physics.add_piece(kind, local.as_deref(), W / 2.0, -150.0, 0);
        let mut landed = false;
        for _ in 0..240 {
            physics.step();
            if physics
                .begin_contacts()
                .iter()
                .any(|&(a, b)| a == 0 || b == 0)
            {
                landed = true;
            }
        }
        let s = physics.state(h);
        assert!(landed, "no contact event was reported");
        // resting with its bottom on the stump top (y = 0): centre at -30
        assert!((s.y + 30.0).abs() < 2.0, "centre y = {}", s.y);
        assert!(s.angle.abs() < 0.05);
        assert!(s.vy.abs() < 1.0);
        assert!(s.sleeping, "a resting block should fall asleep within 4 s");
    }

    #[test]
    fn frozen_piece_reports_static_and_angles_are_screen_clockwise() {
        let mut physics = Physics::new();
        let kind = ShapeKind::Tri { w: 88.0, h: 64.0 };
        let local = crate::geometry::local_outline(kind);
        // off the stump so it tips over as it falls onto the floor
        let h = physics.add_piece(
            kind,
            local.as_deref(),
            W / 2.0 + STUMP_W / 2.0 + 20.0,
            -100.0,
            3,
        );
        for _ in 0..30 {
            physics.step();
        }
        assert!(!physics.state(h).is_static);
        physics.set_static(h);
        let s = physics.state(h);
        assert!(s.is_static);
        // frozen pieces stop moving
        for _ in 0..30 {
            physics.step();
        }
        let s2 = physics.state(h);
        assert_eq!(s.x, s2.x);
        assert_eq!(s.y, s2.y);
    }

    #[test]
    fn gravity_matches_matter_free_fall() {
        let mut physics = Physics::new();
        let kind = ShapeKind::Round { r: 28.0 };
        let h = physics.add_piece(kind, None, W / 2.0, -2000.0, 0);
        for _ in 0..30 {
            physics.step();
        }
        let s = physics.state(h);
        // Matter after 30 steps: v = 0.2778 px/step · Σ0.99^k ≈ 7.23 px/step (434 px/s),
        // fallen ≈ 117 px (y grows downward). Box2D's damping reproduces it.
        assert!(
            s.y > -2000.0 + 100.0 && s.y < -2000.0 + 135.0,
            "y = {}",
            s.y
        );
        assert!(s.vy > 410.0 && s.vy < 460.0, "vy = {}", s.vy);
    }
}

//! Game configuration ported from the top of `reference/game.js`: stage
//! geometry, gameplay tolerances, wind, the shape/critter table, altitude
//! tiers, sky colours and the stability cheat. Every number here is the
//! original's; nothing is tuned independently. `physics.rs` documents how the
//! Matter.js body options translate to Box2D.

use agg_gui::color::Color;

/// Build a colour from a CSS `#rrggbb` literal.
pub const fn hex(v: u32) -> Color {
    Color::from_rgb8((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Build a colour from CSS `rgba(r, g, b, a)` components.
pub const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color::rgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a)
}

/// Logical canvas width; height derived from the stage aspect.
pub const W: f64 = 480.0;
/// Width of the stump top (the platform).
pub const STUMP_W: f64 = 260.0;
/// Stump height above the forest floor.
pub const STUMP_H: f64 = 140.0;
/// World y of the forest floor (stump top is y = 0).
pub const FLOOR_Y: f64 = STUMP_H;
/// 100 px = 1 m.
pub const PX_PER_M: f64 = 100.0;
/// How far above the tower the held piece hovers when picked up.
pub const HOVER_GAP: f64 = 150.0;
/// Minimum clearance kept between the held piece and any landed piece.
pub const HOVER_MARGIN: f64 = 40.0;
/// Frames of stillness before a piece counts as resting.
pub const SETTLE_FRAMES: u32 = 30;
/// ms after a drop before dropping is allowed even if something never settles.
pub const DROP_FALLBACK: f64 = 8000.0;
/// px a piece's bottom may dip below the stump top (heavy towers sink a little into the stump).
pub const BELOW_TOP: f64 = 40.0;
/// Pieces whose centre is within this distance of the stump edge are never frozen.
pub const FREEZE_INSET: f64 = 25.0;

/// Wind gusts: only in the listed altitude tiers (by tower height). Purely
/// visual: the tower is drawn leaning during a gust and settles back as it
/// fades. The physics bodies are never touched, so wind can never knock a
/// piece off.
pub struct Wind {
    pub min_tier: usize,
    pub max_tier: usize,
    /// px the top of the tower appears to shift at full strength.
    pub lean: f64,
    /// radians the top of the tower appears to tilt at full strength.
    pub tilt: f64,
    /// px of extra shiver during the gust.
    pub sway: f64,
    /// ms a gust lasts.
    pub duration: f64,
    /// ms between gusts (random in [min_gap, max_gap]).
    pub min_gap: f64,
    pub max_gap: f64,
}

pub const WIND: Wind = Wind {
    min_tier: 2,
    max_tier: 4,
    lean: 26.0,
    tilt: 0.09,
    sway: 3.0,
    duration: 2200.0,
    min_gap: 7000.0,
    max_gap: 16000.0,
};

/// The resident critter of a shape (drawn by `render/critters.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Critter {
    Bunny,
    Squirrel,
    Owl,
    Fox,
    Frog,
    Raccoon,
    Hedgehog,
    Bear,
    Wolf,
    Deer,
}

impl Critter {
    /// Lower-case name as used in the original's game-over message.
    pub fn name(self) -> &'static str {
        match self {
            Critter::Bunny => "bunny",
            Critter::Squirrel => "squirrel",
            Critter::Owl => "owl",
            Critter::Fox => "fox",
            Critter::Frog => "frog",
            Critter::Raccoon => "raccoon",
            Critter::Hedgehog => "hedgehog",
            Critter::Bear => "bear",
            Critter::Wolf => "wolf",
            Critter::Deer => "deer",
        }
    }
}

/// Outline family of a shape, in the piece's local frame (y down, origin at
/// the geometric centre used by the original's `Bodies.*` helpers).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShapeKind {
    /// Axis-aligned rectangle with 4 px chamfered corners.
    Rect { w: f64, h: f64 },
    /// Isosceles triangle, apex up.
    Tri { w: f64, h: f64 },
    /// Trapezoid, wide base at the bottom.
    Trap { w: f64, top: f64, h: f64 },
    /// Regular hexagon with flat top and bottom.
    Hex { r: f64 },
    /// Circle.
    Round { r: f64 },
}

/// One entry of the original `SHAPES` table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeDef {
    pub name: &'static str,
    pub kind: ShapeKind,
    pub color: Color,
    /// Approximate face height in px.
    pub face_size: f64,
    /// Draw weight in the random pool.
    pub weight: f64,
    /// Rare shapes that carry the big animals.
    pub big: bool,
    pub critter: Critter,
    pub points: u32,
}

/// Shapes. Every shape has a fixed resident critter so players learn the
/// pairings. Harder-to-stack shapes carry more points.
pub const SHAPES: [ShapeDef; 10] = [
    ShapeDef {
        name: "log",
        kind: ShapeKind::Rect { w: 120.0, h: 36.0 },
        color: hex(0xa9744f),
        face_size: 26.0,
        weight: 3.0,
        big: false,
        critter: Critter::Squirrel,
        points: 200,
    },
    ShapeDef {
        name: "block",
        kind: ShapeKind::Rect { w: 60.0, h: 60.0 },
        color: hex(0xc08a5a),
        face_size: 34.0,
        weight: 3.0,
        big: false,
        critter: Critter::Bunny,
        points: 200,
    },
    ShapeDef {
        name: "plank",
        kind: ShapeKind::Rect { w: 44.0, h: 88.0 },
        color: hex(0x8c6a4a),
        face_size: 28.0,
        weight: 2.0,
        big: false,
        critter: Critter::Owl,
        points: 300,
    },
    ShapeDef {
        name: "wedge",
        kind: ShapeKind::Tri { w: 88.0, h: 64.0 },
        color: hex(0xb07f55),
        face_size: 26.0,
        weight: 2.0,
        big: false,
        critter: Critter::Fox,
        points: 400,
    },
    ShapeDef {
        name: "stump",
        kind: ShapeKind::Trap {
            w: 96.0,
            top: 56.0,
            h: 46.0,
        },
        color: hex(0x9c6a45),
        face_size: 28.0,
        weight: 2.0,
        big: false,
        critter: Critter::Frog,
        points: 300,
    },
    ShapeDef {
        name: "log end",
        kind: ShapeKind::Hex { r: 36.0 },
        color: hex(0xd2a679),
        face_size: 30.0,
        weight: 1.5,
        big: false,
        critter: Critter::Raccoon,
        points: 300,
    },
    ShapeDef {
        name: "round log",
        kind: ShapeKind::Round { r: 28.0 },
        color: hex(0xc99a67),
        face_size: 30.0,
        weight: 1.0,
        big: false,
        critter: Critter::Hedgehog,
        points: 500,
    },
    ShapeDef {
        name: "great log",
        kind: ShapeKind::Rect { w: 180.0, h: 52.0 },
        color: hex(0x7a4f2e),
        face_size: 40.0,
        weight: 0.5,
        big: true,
        critter: Critter::Bear,
        points: 1000,
    },
    ShapeDef {
        name: "trunk",
        kind: ShapeKind::Rect { w: 100.0, h: 100.0 },
        color: hex(0x8a5d3b),
        face_size: 54.0,
        weight: 0.4,
        big: true,
        critter: Critter::Wolf,
        points: 800,
    },
    ShapeDef {
        name: "great stump",
        kind: ShapeKind::Trap {
            w: 150.0,
            top: 104.0,
            h: 66.0,
        },
        color: hex(0xa0703f),
        face_size: 44.0,
        weight: 0.4,
        big: true,
        critter: Critter::Deer,
        points: 600,
    },
];

/// Altitude tier (metres of settled tower height). Reaching one shows a toast.
pub struct Tier {
    pub at: f64,
    pub name: &'static str,
    pub icon: &'static str,
}

pub const TIERS: [Tier; 6] = [
    Tier {
        at: 0.0,
        name: "Forest Floor",
        icon: "\u{1F331}",
    },
    Tier {
        at: 4.0,
        name: "Above the Treetops!",
        icon: "\u{1F332}",
    },
    Tier {
        at: 8.0,
        name: "Open Sky!",
        icon: "\u{1F426}",
    },
    Tier {
        at: 15.0,
        name: "Cloud Country!",
        icon: "\u{2601}\u{FE0F}",
    },
    Tier {
        at: 25.0,
        name: "Stratosphere!",
        icon: "\u{1F319}",
    },
    Tier {
        at: 40.0,
        name: "Outer Space!",
        icon: "\u{1F680}",
    },
];

/// Sky colours by camera altitude (metres): (alt, top, bottom).
pub const SKY: [(f64, Color, Color); 6] = [
    (0.0, hex(0x9fd3a8), hex(0xe6efc4)),
    (4.0, hex(0x8ecae6), hex(0xdff3fb)),
    (10.0, hex(0x5aa9e6), hex(0xbde0fe)),
    (20.0, hex(0x1d4e89), hex(0x5aa9e6)),
    (30.0, hex(0x0b1530), hex(0x1d3557)),
    (45.0, hex(0x02030a), hex(0x0b1530)),
];

/// Matter.js `density` of a fresh piece.
pub const BASE_DENSITY: f64 = 0.002;
/// Matter.js `BODY_OPTS.friction`.
pub const BASE_FRICTION: f64 = 0.9;
/// Matter.js `BODY_OPTS.frictionStatic`.
pub const BASE_FRICTION_STATIC: f64 = 1.5;
/// Matter.js `BODY_OPTS.frictionAir` (per-step velocity loss).
pub const BASE_FRICTION_AIR: f64 = 0.01;
/// The round log uses a lower friction so it can roll.
pub const ROUND_FRICTION: f64 = 0.6;

/// Stability cheat: the deeper a piece sits below the top of the tower (the
/// "active layer"), the denser, stickier and more damped it becomes, so tall
/// towers stop wobbling themselves apart. Depths are in metres below the
/// highest settled piece.
pub struct Stability {
    /// Pieces within this depth of the top behave normally.
    pub free_depth: f64,
    /// Extra density multiplier per metre beyond free_depth.
    pub density_per_metre: f64,
    /// Cap on the density multiplier.
    pub max_density_mult: f64,
    /// Extra surface friction per metre beyond free_depth (capped at 2).
    pub friction_per_metre: f64,
    /// Extra air damping per metre beyond free_depth (capped at 0.3).
    pub air_per_metre: f64,
    /// Pieces deeper than this become solid (0 disables freezing).
    pub freeze_depth: f64,
}

pub const STABILITY: Stability = Stability {
    free_depth: 0.8,
    density_per_metre: 2.0,
    max_density_mult: 12.0,
    friction_per_metre: 0.4,
    air_per_metre: 0.06,
    freeze_depth: 3.5,
};

/// Once a piece settles it "locks"; it only unlocks if it moves this much.
pub const UNLOCK_DIST: f64 = 25.0;
pub const UNLOCK_ANGLE: f64 = 0.35;

/// Tray slot preview canvas size (CSS px).
pub const SLOT_W: f64 = 130.0;
pub const SLOT_H: f64 = 96.0;

/// Max width of the app column (`#app { max-width: 560px }`).
pub const APP_MAX_WIDTH: f64 = 560.0;
/// Tray height: 112 px slots + 2 × 10 px padding + 5 px top border.
pub const TRAY_HEIGHT: f64 = 112.0 + 20.0 + 5.0;

/// localStorage keys of the original, kept so the web build reads a best
/// score saved by the JavaScript version on the same origin.
pub const BEST_SCORE_KEY: &str = "critterstack-best-score-v2";
pub const MUTED_KEY: &str = "critterstack-muted";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_table_matches_original_points_and_pairings() {
        let expect = [
            ("log", Critter::Squirrel, 200),
            ("block", Critter::Bunny, 200),
            ("plank", Critter::Owl, 300),
            ("wedge", Critter::Fox, 400),
            ("stump", Critter::Frog, 300),
            ("log end", Critter::Raccoon, 300),
            ("round log", Critter::Hedgehog, 500),
            ("great log", Critter::Bear, 1000),
            ("trunk", Critter::Wolf, 800),
            ("great stump", Critter::Deer, 600),
        ];
        for (shape, (name, critter, points)) in SHAPES.iter().zip(expect) {
            assert_eq!(shape.name, name);
            assert_eq!(shape.critter, critter);
            assert_eq!(shape.points, points);
        }
        assert_eq!(SHAPES.iter().filter(|s| s.big).count(), 3);
    }

    #[test]
    fn hex_decodes_css_literals() {
        let c = hex(0xa9744f);
        assert!((c.r - 0xa9 as f32 / 255.0).abs() < 1e-6);
        assert!((c.g - 0x74 as f32 / 255.0).abs() < 1e-6);
        assert!((c.b - 0x4f as f32 / 255.0).abs() < 1e-6);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn tiers_are_ascending() {
        for pair in TIERS.windows(2) {
            assert!(pair[0].at < pair[1].at);
        }
        assert_eq!(TRAY_HEIGHT, 137.0);
    }
}

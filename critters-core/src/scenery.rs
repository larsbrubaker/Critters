//! Background scenery generated once at startup (the `seeded(...)` tables
//! near the top of `reference/game.js`): mountains, three tree layers,
//! forest-floor bits, clouds, birds, stars, planets and the wind streaks.
//! `render/background.rs` draws these; `game.rs` advances the clouds, birds
//! and streaks.

use agg_gui::color::Color;

use crate::config::{hex, W};
use crate::rng::Rng;

pub struct Mountain {
    pub x: f64,
    pub h: f64,
    pub w: f64,
}

pub struct FarTree {
    pub x: f64,
    pub h: f64,
    /// Width as a fraction of height.
    pub w: f64,
}

/// A leafy tree in the mid or near layer.
pub struct LeafyTree {
    pub x: f64,
    pub h: f64,
    pub r: f64,
    /// Trunk width.
    pub t: f64,
}

pub struct FloorBit {
    pub x: f64,
    /// Random in [0, 1): < 0.45 grass tuft, < 0.7 mushroom, < 0.85 rock, else fern.
    pub kind: f64,
    pub s: f64,
}

pub struct Cloud {
    pub x: f64,
    /// Camera rise (px) at which the cloud is centred on screen.
    pub a: f64,
    pub s: f64,
    /// Drift speed, px/s.
    pub v: f64,
}

pub struct Bird {
    pub x: f64,
    pub a: f64,
    pub v: f64,
    pub dir: f64,
    pub ph: f64,
}

pub struct Star {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub tw: f64,
}

pub struct Planet {
    pub a: f64,
    pub x: f64,
    pub r: f64,
    /// Parallax factor.
    pub p: f64,
    pub color: Color,
    pub ring: bool,
}

pub struct Streak {
    pub x: f64,
    /// Fraction of the stage height.
    pub y: f64,
    pub len: f64,
    pub v: f64,
}

pub struct Scenery {
    pub mountains: Vec<Mountain>,
    pub far_trees: Vec<FarTree>,
    pub mid_trees: Vec<LeafyTree>,
    pub near_trees: Vec<LeafyTree>,
    pub floor_bits: Vec<FloorBit>,
    pub clouds: Vec<Cloud>,
    pub birds: Vec<Bird>,
    pub stars: Vec<Star>,
    pub planets: Vec<Planet>,
    pub streaks: Vec<Streak>,
}

impl Scenery {
    pub fn generate(rng: &mut Rng) -> Self {
        let mountains = (0..9)
            .map(|i| Mountain {
                x: i as f64 * 70.0 - 40.0 + rng.random() * 30.0,
                h: 420.0 + rng.random() * 260.0,
                w: 260.0 + rng.random() * 120.0,
            })
            .collect();
        let far_trees = (0..34)
            .map(|_| FarTree {
                x: rng.random() * (W + 200.0) - 100.0,
                h: 170.0 + rng.random() * 120.0,
                w: 0.55,
            })
            .collect();
        let mid_trees = (0..11)
            .map(|i| LeafyTree {
                x: if i % 2 == 1 {
                    rng.random() * 150.0 - 60.0
                } else {
                    W - 90.0 + rng.random() * 150.0
                },
                h: 520.0 + rng.random() * 220.0,
                r: 48.0 + rng.random() * 26.0,
                t: 12.0 + rng.random() * 6.0,
            })
            .collect();
        let near_trees = (0..4)
            .map(|i| LeafyTree {
                x: if i < 2 {
                    -20.0 + rng.random() * 60.0
                } else {
                    W - 40.0 + rng.random() * 60.0
                },
                h: 760.0 + rng.random() * 240.0,
                r: 70.0 + rng.random() * 30.0,
                t: 30.0 + rng.random() * 14.0,
            })
            .collect();
        let floor_bits = (0..40)
            .map(|_| FloorBit {
                x: rng.random() * W,
                kind: rng.random(),
                s: 0.7 + rng.random() * 0.8,
            })
            .collect();
        let clouds = (0..16)
            .map(|i| Cloud {
                x: rng.random() * (W + 240.0) - 120.0,
                a: 1000.0 + i as f64 * 130.0 + rng.random() * 100.0,
                s: 0.6 + rng.random() * 0.9,
                v: 4.0 + rng.random() * 8.0,
            })
            .collect();
        let birds = (0..7)
            .map(|i| Bird {
                x: rng.random() * W,
                a: 650.0 + i as f64 * 140.0 + rng.random() * 80.0,
                v: 18.0 + rng.random() * 22.0,
                dir: rng.sign(),
                ph: rng.random() * 6.0,
            })
            .collect();
        let stars = (0..120)
            .map(|_| Star {
                x: rng.random() * W,
                y: rng.random() * 2000.0,
                r: rng.random() * 1.6 + 0.4,
                tw: rng.random() * 6.0,
            })
            .collect();
        let planets = vec![
            Planet {
                a: 2700.0,
                x: 90.0,
                r: 26.0,
                p: 0.35,
                color: hex(0xe9e4d4),
                ring: false,
            }, // moon
            Planet {
                a: 4300.0,
                x: 380.0,
                r: 40.0,
                p: 0.45,
                color: hex(0xe8b04b),
                ring: true,
            }, // ringed planet
            Planet {
                a: 5000.0,
                x: 140.0,
                r: 18.0,
                p: 0.4,
                color: hex(0xc0563a),
                ring: false,
            }, // red planet
        ];
        let streaks = (0..26)
            .map(|_| Streak {
                x: rng.random() * (W + 300.0),
                y: rng.random(),
                len: 40.0 + rng.random() * 90.0,
                v: 0.7 + rng.random() * 0.6,
            })
            .collect();
        Self {
            mountains,
            far_trees,
            mid_trees,
            near_trees,
            floor_bits,
            clouds,
            birds,
            stars,
            planets,
            streaks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_match_the_original_tables() {
        let s = Scenery::generate(&mut Rng::seeded(1));
        assert_eq!(s.mountains.len(), 9);
        assert_eq!(s.far_trees.len(), 34);
        assert_eq!(s.mid_trees.len(), 11);
        assert_eq!(s.near_trees.len(), 4);
        assert_eq!(s.floor_bits.len(), 40);
        assert_eq!(s.clouds.len(), 16);
        assert_eq!(s.birds.len(), 7);
        assert_eq!(s.stars.len(), 120);
        assert_eq!(s.planets.len(), 3);
        assert_eq!(s.streaks.len(), 26);
    }

    #[test]
    fn trees_hug_the_stage_edges() {
        let s = Scenery::generate(&mut Rng::seeded(3));
        for (i, t) in s.mid_trees.iter().enumerate() {
            if i % 2 == 1 {
                assert!(t.x >= -60.0 && t.x < 90.0);
            } else {
                assert!(t.x >= W - 90.0 && t.x < W + 60.0);
            }
        }
        for b in &s.birds {
            assert!(b.dir == 1.0 || b.dir == -1.0);
        }
    }
}

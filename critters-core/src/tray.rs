//! The tray's shape offering — `pickShape`, `newSpec`, `flatSpec`,
//! `makeBody` (for pieces outside the world) and `renderSlot` from
//! `reference/game.js`. Shapes are drawn from the weighted `SHAPES` pool; a
//! new round starts with two flat, easy shapes and one from the full pool.

use crate::config::{ShapeDef, ShapeKind, SHAPES};
use crate::game::Game;
use crate::piece::Piece;

impl Game {
    pub(crate) fn pick_shape(&mut self, pool: &[ShapeDef]) -> ShapeDef {
        let total: f64 = pool.iter().map(|s| s.weight).sum();
        let mut r = self.rng.random() * total;
        for s in pool {
            r -= s.weight;
            if r <= 0.0 {
                return *s;
            }
        }
        pool[0]
    }

    pub(crate) fn new_spec(&mut self) -> ShapeDef {
        self.pick_shape(&SHAPES)
    }

    /// The first two slots of a round only offer easy, flat shapes.
    pub(crate) fn flat_spec(&mut self) -> ShapeDef {
        let pool: Vec<ShapeDef> = SHAPES
            .iter()
            .copied()
            .filter(|s| !s.big && matches!(s.kind, ShapeKind::Rect { .. } | ShapeKind::Trap { .. }))
            .collect();
        self.pick_shape(&pool)
    }

    pub(crate) fn make_piece(&mut self, spec: ShapeDef, x: f64, y: f64) -> Piece {
        let phase = self.rng.random() * 1000.0;
        Piece::new(spec, x, y, phase)
    }

    /// `renderSlot(i)`: rebuild the preview body for a slot.
    pub fn render_slot(&mut self, i: usize) {
        self.slot_pieces[i] = self.slots[i].map(|spec| {
            let phase = self.rng.random() * 1000.0;
            Piece::new(spec, 0.0, 0.0, phase)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_pick_covers_the_whole_pool() {
        let mut g = Game::new(720.0, 0, false);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..2000 {
            seen.insert(g.new_spec().name);
        }
        for s in SHAPES.iter() {
            assert!(seen.contains(s.name), "{} never offered", s.name);
        }
        for _ in 0..200 {
            let f = g.flat_spec();
            assert!(!f.big);
            assert!(matches!(
                f.kind,
                ShapeKind::Rect { .. } | ShapeKind::Trap { .. }
            ));
        }
    }

    #[test]
    fn render_slot_builds_a_preview_only_for_filled_slots() {
        let mut g = Game::new(720.0, 0, false);
        g.slots[1] = None;
        g.render_slot(1);
        assert!(g.slot_pieces[1].is_none());
        g.slots[1] = Some(SHAPES[0]);
        g.render_slot(1);
        assert_eq!(g.slot_pieces[1].as_ref().unwrap().spec, SHAPES[0]);
    }
}

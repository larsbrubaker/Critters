//! The DOM chrome of the original, painted by agg-gui: `#hud`, `#toast`,
//! `#hint`, `#mute`, `#overlay` with its card, and `#tray` with the three
//! slots (`reference/index.html` + `reference/style.css`). Everything is in
//! CSS pixels relative to the app column, with the stylesheet's paddings,
//! radii, colours and keyframe animations reproduced by hand. The overlay
//! button's rectangle is published to `game.input` for hit testing.

use agg_gui::color::Color;

use crate::config::{hex, rgba, SLOT_H, SLOT_W, TRAY_HEIGHT};
use crate::game::{Game, State, INTRO_SMALL, TOAST_ANIMATION};
use crate::input::{Layout, Rect};
use crate::render::canvas::{wrap_lines, Cx};
use crate::render::world::{draw_body, slot_background};

const WHITE: Color = Color::white();
const TEXT_SHADOW: Color = rgba(0, 0, 0, 0.6);

/// CSS `ease-out` (cubic-bezier(0, 0, 0.58, 1)), close enough as 1-(1-t)².
fn ease_out(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t)
}

fn rounded_fill(c: &mut Cx, r: Rect, radius: f64, color: Color) {
    c.fill_style(color);
    c.begin_path();
    c.ctx.rounded_rect(r.x, r.y, r.w, r.h, radius);
    c.fill();
}

/// Text with the stylesheet's `text-shadow: 0 1px 3px rgba(0,0,0,.6)`.
fn shadow_text(c: &mut Cx, text: &str, x: f64, baseline: f64, size: f64, bold: bool, color: Color) {
    c.fill_style(TEXT_SHADOW);
    c.fill_text(text, x, baseline + 1.0, size, bold);
    c.fill_style(color);
    c.fill_text(text, x, baseline, size, bold);
}

/// Uppercase label with `letter-spacing: 1px`.
fn spaced_width(c: &mut Cx, text: &str, size: f64) -> f64 {
    c.measure(text, size, false) + text.chars().count() as f64
}

fn spaced_text(c: &mut Cx, text: &str, x: f64, baseline: f64, size: f64, color: Color) {
    let mut cx = x;
    for ch in text.chars() {
        let s = ch.to_string();
        shadow_text(c, &s, cx, baseline, size, false, color);
        cx += c.measure(&s, size, false) + 1.0;
    }
}

fn draw_hud(c: &mut Cx, game: &Game, layout: &Layout) {
    let stats = [
        ("SCORE", game.score.to_string()),
        (
            "HEIGHT",
            Game::fmt_m((-game.tower_top / crate::config::PX_PER_M).max(0.0)),
        ),
        ("BEST", game.best_score.to_string()),
    ];
    let label_size = 11.0;
    let value_size = 22.0;
    let label_lh = label_size * 1.36;
    let value_lh = value_size * 1.36;
    let box_h = 6.0 + label_lh + value_lh + 6.0;
    let widths: Vec<f64> = stats
        .iter()
        .map(|(l, v)| {
            let lw = spaced_width(c, l, label_size);
            let vw = c.measure(v, value_size, true);
            (lw.max(vw) + 24.0).max(96.0)
        })
        .collect();
    let left = 12.0;
    let right = layout.app_w - 12.0;
    let gap = ((right - left) - widths.iter().sum::<f64>()) / 2.0;
    let mut x = left;
    for (i, (label, value)) in stats.iter().enumerate() {
        let w = widths[i];
        let r = Rect {
            x,
            y: 10.0,
            w,
            h: box_h,
        };
        rounded_fill(c, r, 10.0, rgba(20, 40, 28, 0.55));
        let cx = x + w / 2.0;
        let lw = spaced_width(c, label, label_size);
        let (la, ld) = c.metrics(label_size, false);
        let label_base = 10.0 + 6.0 + (label_lh - (la + ld)) / 2.0 + la;
        spaced_text(
            c,
            label,
            cx - lw / 2.0,
            label_base,
            label_size,
            rgba(255, 255, 255, 0.85),
        );
        let vw = c.measure(value, value_size, true);
        let (va, vd) = c.metrics(value_size, true);
        let value_base = 10.0 + 6.0 + label_lh + (value_lh - (va + vd)) / 2.0 + va;
        shadow_text(c, value, cx - vw / 2.0, value_base, value_size, true, WHITE);
        x += w + gap;
    }
}

fn draw_toast(c: &mut Cx, game: &Game, layout: &Layout) {
    let Some(toast) = &game.toast else {
        return;
    };
    let p = ((game.time - toast.shown_at) / TOAST_ANIMATION).clamp(0.0, 1.0);
    // keyframes: 0% {op 0; ty 12; s .9} 12% {op 1; ty 0; s 1.05} 20% {s 1} 80% {op 1} 100% {op 0; ty -10}
    let (opacity, ty, s) = if p < 0.12 {
        let t = ease_out(p / 0.12);
        (t, 12.0 - 12.0 * t, 0.9 + 0.15 * t)
    } else if p < 0.2 {
        let t = ease_out((p - 0.12) / 0.08);
        (1.0, 0.0, 1.05 - 0.05 * t)
    } else {
        let t = ease_out((p - 0.2) / 0.8);
        let op = if p < 0.8 {
            1.0
        } else {
            1.0 - ease_out((p - 0.8) / 0.2)
        };
        (op, -10.0 * t, 1.0)
    };
    let size = 18.0;
    let tw = c.measure(&toast.text, size, true);
    let (a, d) = c.metrics(size, true);
    let h = 8.0 + (a + d) + 8.0;
    let w = tw + 36.0;
    let cx = layout.app_w / 2.0;
    let top = 70.0;
    c.save();
    c.global_alpha(opacity * 0.95);
    c.translate(cx, top + ty + h / 2.0);
    c.scale(s, s);
    let r = Rect {
        x: -w / 2.0,
        y: -h / 2.0,
        w,
        h,
    };
    rounded_fill(c, r, 999.0, hex(0xfff8ec));
    c.fill_style(hex(0x5a3f2b));
    c.fill_text(&toast.text, -tw / 2.0, -h / 2.0 + 8.0 + a, size, true);
    c.restore();
}

fn draw_hint(c: &mut Cx, game: &Game, layout: &Layout) {
    if game.hint.is_empty() {
        return;
    }
    let size = 16.0;
    let mut color = WHITE;
    let mut dx = 0.0;
    if let Some(at) = game.hint_shake_at {
        let t = (game.time - at) / 350.0;
        if t < 1.0 {
            color = hex(0xffd166);
            dx = if t < 0.25 {
                -6.0 * (t / 0.25)
            } else if t < 0.75 {
                -6.0 + 12.0 * ((t - 0.25) / 0.5)
            } else {
                6.0 - 6.0 * ((t - 0.75) / 0.25)
            };
        } else {
            color = hex(0xffd166);
        }
    }
    let tw = c.measure(&game.hint, size, true);
    let (_a, d) = c.metrics(size, true);
    let baseline = layout.stage_h - 12.0 - d - (size * 1.36 - size) / 2.0;
    shadow_text(
        c,
        &game.hint,
        layout.app_w / 2.0 - tw / 2.0 + dx,
        baseline,
        size,
        true,
        color,
    );
}

const FA_EXPAND: &str = "\u{f065}";
const FA_COMPRESS: &str = "\u{f066}";

/// Full-screen toggle, touch devices only (see `Layout::fullscreen_rect`).
fn draw_fullscreen(c: &mut Cx, game: &Game, layout: &Layout) {
    if !Layout::shows_fullscreen_button() {
        return;
    }
    let r = layout.fullscreen_rect();
    let hover = game
        .input
        .pointer
        .map(|(x, y)| r.contains(x, y))
        .unwrap_or(false);
    c.fill_style(if hover {
        rgba(20, 40, 28, 0.8)
    } else {
        rgba(20, 40, 28, 0.55)
    });
    c.begin_path();
    c.arc(
        r.x + r.w / 2.0,
        r.y + r.h / 2.0,
        r.w / 2.0,
        0.0,
        std::f64::consts::TAU,
    );
    c.fill();
    let glyph = if agg_gui::fullscreen::is_active() {
        FA_COMPRESS
    } else {
        FA_EXPAND
    };
    c.fill_style(WHITE);
    c.icon_centered(glyph, r.x + r.w / 2.0, r.y + r.h / 2.0, 18.0);
}

fn draw_mute(c: &mut Cx, game: &Game, layout: &Layout) {
    let r = layout.mute_rect();
    let hover = game
        .input
        .pointer
        .map(|(x, y)| r.contains(x, y))
        .unwrap_or(false);
    let bg = if hover {
        rgba(20, 40, 28, 0.8)
    } else {
        rgba(20, 40, 28, 0.55)
    };
    c.fill_style(bg);
    c.begin_path();
    c.arc(
        r.x + r.w / 2.0,
        r.y + r.h / 2.0,
        r.w / 2.0,
        0.0,
        std::f64::consts::TAU,
    );
    c.fill();
    let glyph = if game.muted { "\u{1F507}" } else { "\u{1F50A}" };
    let size = 20.0;
    let w = c.measure(glyph, size, false);
    let base = c.middle_baseline(r.y + r.h / 2.0, size, false);
    c.fill_style(WHITE);
    c.fill_text(glyph, r.x + r.w / 2.0 - w / 2.0, base, size, false);
}

/// One paragraph line-wrapped at `line_h`; returns the y after it.
#[allow(clippy::too_many_arguments)]
fn paragraph(
    c: &mut Cx,
    lines: &[String],
    cx: f64,
    mut y: f64,
    size: f64,
    bold: bool,
    line_h: f64,
    color: Color,
) -> f64 {
    let (a, d) = c.metrics(size, bold);
    for line in lines {
        let w = c.measure(line, size, bold);
        c.fill_style(color);
        c.fill_text(
            line,
            cx - w / 2.0,
            y + (line_h - (a + d)) / 2.0 + a,
            size,
            bold,
        );
        y += line_h;
    }
    y
}

fn draw_overlay(c: &mut Cx, game: &mut Game, layout: &Layout) {
    if !game.overlay.visible {
        game.input.overlay_button = None;
        return;
    }
    c.fill_style(rgba(10, 25, 15, 0.55));
    c.fill_rect(0.0, 0.0, layout.app_w, layout.stage_h);

    let card_w = 360.0_f64.min(layout.app_w - 32.0);
    let content_w = card_w - 52.0;
    let title_lines = wrap_lines(c, &game.overlay.title, content_w, 34.0, true);
    let text_lines = wrap_lines(c, &game.overlay.text, content_w, 16.0, false);
    let small_lines: Vec<String> = INTRO_SMALL
        .iter()
        .flat_map(|p| wrap_lines(c, p, content_w, 13.0, false))
        .collect();
    let title_h = title_lines.len() as f64 * 34.0 * 1.2;
    let text_h = text_lines.len() as f64 * 16.0 * 1.4;
    let small_h = small_lines.len() as f64 * 13.0 * 1.4;
    let button_h = 18.0 * 1.2 + 20.0;
    let dim_h = 13.0 * 1.4;
    let mut card_h = 22.0 + title_h + 8.0 + text_h + 8.0 + small_h;
    if game.overlay.over {
        card_h += 8.0 + 10.0 + button_h;
    } else {
        card_h += 14.0 + dim_h;
    }
    card_h += 22.0;
    let cx = layout.app_w / 2.0;
    let card = Rect {
        x: cx - card_w / 2.0,
        y: (layout.stage_h - card_h) / 2.0,
        w: card_w,
        h: card_h,
    };
    // box-shadow 0 10px 30px rgba(0,0,0,.4)
    rounded_fill(
        c,
        Rect {
            y: card.y + 10.0,
            ..card
        },
        18.0,
        rgba(0, 0, 0, 0.18),
    );
    rounded_fill(c, card, 18.0, hex(0xfff8ec));

    let mut y = card.y + 22.0;
    y = paragraph(
        c,
        &title_lines,
        cx,
        y,
        34.0,
        true,
        34.0 * 1.2,
        hex(0x5a3f2b),
    );
    y += 8.0;
    y = paragraph(
        c,
        &text_lines,
        cx,
        y,
        16.0,
        false,
        16.0 * 1.4,
        hex(0x3b2a1e),
    );
    y += 8.0;
    y = paragraph(
        c,
        &small_lines,
        cx,
        y,
        13.0,
        false,
        13.0 * 1.4,
        hex(0x3b2a1e).with_alpha(0.8),
    );
    if game.overlay.over {
        y += 8.0 + 10.0;
        let label = "Play again";
        let lw = c.measure(label, 18.0, true);
        let bw = lw + 52.0;
        let pressed = game.input.button_armed
            && game
                .input
                .pointer
                .zip(game.input.overlay_button)
                .map(|((px, py), r)| r.contains(px, py))
                .unwrap_or(false);
        let lift = if pressed { 2.0 } else { 0.0 };
        let btn = Rect {
            x: cx - bw / 2.0,
            y: y + lift,
            w: bw,
            h: button_h,
        };
        // box-shadow 0 4px 0 #4a6f36 (2px when pressed)
        let shadow = Rect {
            y: btn.y + 4.0 - lift,
            ..btn
        };
        rounded_fill(c, shadow, 999.0, hex(0x4a6f36));
        rounded_fill(c, btn, 999.0, hex(0x6a994e));
        let (a, d) = c.metrics(18.0, true);
        c.fill_style(WHITE);
        c.fill_text(
            label,
            cx - lw / 2.0,
            btn.y + (button_h - (a + d)) / 2.0 + a,
            18.0,
            true,
        );
        game.input.overlay_button = Some(Rect { y, ..btn });
    } else {
        y += 14.0;
        let dim = ["Tap anywhere to begin".to_string()];
        paragraph(
            c,
            &dim,
            cx,
            y,
            13.0,
            false,
            dim_h,
            hex(0x3b2a1e).with_alpha(0.55),
        );
        game.input.overlay_button = None;
    }
}

/// HUD, toast, hint, mute button and overlay — everything inside `#stage`
/// that is not the canvas.
pub fn draw_stage_chrome(c: &mut Cx, game: &mut Game, layout: &Layout) {
    c.save();
    c.ctx.clip_rect(0.0, 0.0, layout.app_w, layout.stage_h);
    draw_hud(c, game, layout);
    draw_toast(c, game, layout);
    draw_hint(c, game, layout);
    draw_mute(c, game, layout);
    draw_fullscreen(c, game, layout);
    draw_overlay(c, game, layout);
    c.restore();
}

/// `#tray` with its three `.slot`s.
pub fn draw_tray(c: &mut Cx, game: &Game, layout: &Layout) {
    let top = layout.stage_h;
    c.fill_style(hex(0x3b2a1e));
    c.fill_rect(0.0, top, layout.app_w, TRAY_HEIGHT);
    c.fill_style(hex(0x5a3f2b));
    c.fill_rect(0.0, top, layout.app_w, 5.0);

    for i in 0..3 {
        let r = layout.slot_rect(i);
        let spec = game.slots[i];
        let empty = spec.is_none();
        let big = spec.map(|s| s.big).unwrap_or(false);
        let hover = !empty
            && game
                .input
                .pointer
                .map(|(x, y)| r.contains(x, y))
                .unwrap_or(false);
        // transform: :active scale(.96); .pop keyframes .6 → 1.08 (70%) → 1
        let mut s = 1.0;
        let pop_t = (game.time - game.slot_pop_at[i]) / 250.0;
        if (0.0..1.0).contains(&pop_t) {
            s = if pop_t < 0.7 {
                0.6 + 0.48 * (pop_t / 0.7)
            } else {
                1.08 - 0.08 * ((pop_t - 0.7) / 0.3)
            };
        } else if game.input.slot_active == Some(i) && !empty {
            s = 0.96;
        }
        c.save();
        c.translate(r.x + r.w / 2.0, r.y + r.h / 2.0);
        c.scale(s, s);
        c.translate(-(r.x + r.w / 2.0), -(r.y + r.h / 2.0));
        rounded_fill(c, r, 14.0, slot_background(big, empty));
        let border = if empty {
            rgba(255, 255, 255, 0.15)
        } else if hover {
            rgba(255, 209, 102, 0.6)
        } else if big {
            rgba(255, 209, 102, 0.35)
        } else {
            rgba(255, 255, 255, 0.08)
        };
        c.stroke_style(border);
        c.line_width(3.0);
        if empty {
            c.set_line_dash(&[9.0, 6.0]);
        }
        c.begin_path();
        c.ctx
            .rounded_rect(r.x + 1.5, r.y + 1.5, r.w - 3.0, r.h - 3.0, 12.5);
        c.stroke();
        c.set_line_dash(&[]);

        if let Some(piece) = &game.slot_pieces[i] {
            // .slot canvas { width: 130px; height: 96px; max-width: 100% }
            let cw = SLOT_W.min(r.w - 6.0);
            let ch = SLOT_H;
            let cx = r.x + r.w / 2.0;
            let cy = r.y + r.h / 2.0;
            c.save();
            c.ctx.clip_rect(cx - cw / 2.0, cy - ch / 2.0, cw, ch);
            let bw = piece.bounds.max.x - piece.bounds.min.x;
            let bh = piece.bounds.max.y - piece.bounds.min.y;
            let k = ((SLOT_W - 16.0) / bw).min((SLOT_H - 16.0) / bh).min(1.0);
            let pcx = (piece.bounds.min.x + piece.bounds.max.x) / 2.0;
            let pcy = (piece.bounds.min.y + piece.bounds.max.y) / 2.0;
            c.translate(cx - pcx * k, cy - pcy * k);
            c.scale(k, k);
            draw_body(c, game, piece, false, false, false);
            c.restore();
        }

        if let Some(spec) = spec {
            // .pts badge
            let text = format!(
                "{} pt{}",
                spec.points,
                if spec.points == 1 { "" } else { "s" }
            );
            let tw = c.measure(&text, 12.0, true);
            let (a, d) = c.metrics(12.0, true);
            let bh = 2.0 + (a + d) + 2.0;
            let bw = tw + 16.0;
            let badge = Rect {
                x: r.x + r.w - 8.0 - bw,
                y: r.y + r.h - 6.0 - bh,
                w: bw,
                h: bh,
            };
            rounded_fill(c, badge, 999.0, rgba(255, 209, 102, 0.9));
            c.fill_style(hex(0x3b2a1e));
            c.fill_text(&text, badge.x + 8.0, badge.y + 2.0 + a, 12.0, true);
        }
        // .key
        let key = (i + 1).to_string();
        let (a, _) = c.metrics(11.0, true);
        c.fill_style(rgba(255, 255, 255, 0.5));
        c.fill_text(&key, r.x + 8.0, r.y + 5.0 + a, 11.0, true);
        c.restore();
    }
    if game.state == State::Over {
        // slots stay visible but inert; nothing extra to draw
    }
}

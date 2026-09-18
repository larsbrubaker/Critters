//! Fonts bundled into the binary so native and web render identically:
//! Nunito (regular + bold) stands in for the original's
//! `"Trebuchet MS", "Segoe UI", system-ui` stack, Noto Emoji is chained as
//! the fallback for the tier-toast and mute-button glyphs, a two-glyph
//! subset of Liberation Sans supplies the ← → arrows of the intro card, and
//! Font Awesome is available for icons. `render/` and `widget.rs` take a
//! [`Fonts`].

use std::sync::Arc;

use agg_gui::text::Font;

pub const NUNITO_REGULAR: &[u8] = include_bytes!("../assets/Nunito_Regular.ttf");
pub const NUNITO_BOLD: &[u8] = include_bytes!("../assets/Nunito_Bold.ttf");
pub const NOTO_EMOJI: &[u8] = include_bytes!("../assets/NotoEmoji-Regular.ttf");
pub const FONT_AWESOME: &[u8] = include_bytes!("../assets/fa.ttf");
/// `pyftsubset LiberationSans-Regular.ttf --unicodes=U+2190,U+2192` (SIL OFL).
pub const ARROWS: &[u8] = include_bytes!("../assets/LiberationSans-Arrows.ttf");

#[derive(Clone)]
pub struct Fonts {
    pub regular: Arc<Font>,
    pub bold: Arc<Font>,
    pub icons: Arc<Font>,
}

impl Fonts {
    /// Parse the embedded fonts, chaining the emoji fallback onto the text faces.
    pub fn load() -> Result<Self, &'static str> {
        let arrows = Arc::new(Font::from_slice(ARROWS)?);
        let emoji = Arc::new(Font::from_slice(NOTO_EMOJI)?.with_fallback(arrows));
        let regular = Font::from_slice(NUNITO_REGULAR)?.with_fallback(Arc::clone(&emoji));
        let bold = Font::from_slice(NUNITO_BOLD)?.with_fallback(emoji);
        let icons = Font::from_slice(FONT_AWESOME)?;
        Ok(Self {
            regular: Arc::new(regular),
            bold: Arc::new(bold),
            icons: Arc::new(icons),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonts_parse_and_cover_the_glyphs_the_game_draws() {
        let fonts = Fonts::load().expect("bundled fonts parse");
        let chars = fonts.regular.characters();
        for ch in "Critter Stack 0123456789 m·→←…".chars() {
            assert!(chars.contains(&ch), "missing {ch:?}");
        }
        // emoji fallback: tier icons and the speaker
        for ch in [
            '\u{1F331}',
            '\u{1F332}',
            '\u{1F426}',
            '\u{2601}',
            '\u{1F319}',
            '\u{1F680}',
            '\u{1F50A}',
            '\u{1F507}',
        ] {
            assert!(chars.contains(&ch), "missing emoji {ch:?}");
        }
        assert!(fonts.bold.characters().contains(&'T'));
    }
}

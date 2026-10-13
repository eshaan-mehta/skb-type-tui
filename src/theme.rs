//! Colors. These come from the fixed part of the 256-color palette, so they look the same
//! in light and dark terminal themes (themes remap the 16 basic colors, which is how
//! "yellow" ends up mustard on some of them).

use std::sync::OnceLock;

use ratatui::style::Color;

/// Next key to press, focus letter, highlighted numbers. #0087ff
pub const ACCENT: Color = Color::Indexed(33);
/// Keys to hold for the next character (shift, layer keys). #af5fff
pub const HOLD: Color = Color::Indexed(135);
/// Correct keypress flash, learned letters. #00af5f
pub const CORRECT: Color = Color::Indexed(35);
/// Mistake flash, mistyped text. #d70000
pub const MISTAKE: Color = Color::Indexed(160);
/// A key being held down. #a8a8a8
pub const PRESSED: Color = Color::Indexed(248);
/// "Your voyager is missing" and other warnings. #d75f00
pub const WARNING: Color = Color::Indexed(166);
/// Keys that changed in a new layout revision. #d75fd7
pub const CHANGED: Color = Color::Indexed(170);
/// Keys the user edited by hand. #00afaf
pub const EDITED: Color = Color::Indexed(37);
/// Text on colored fills.
pub const ON_FILL: Color = Color::Indexed(231);
/// Text on the light gray "pressed" fill.
pub const ON_PRESSED: Color = Color::Indexed(16);

/// The terminal's own text and background colors, if it said (see `detect`).
static TERMINAL: OnceLock<Option<Rgb>> = OnceLock::new();

type Rgb = ((u8, u8, u8), (u8, u8, u8));

/// Opacity of faded text (what's behind a menu).
const FADED_ALPHA: f64 = 0.1;
/// Opacity of letters progressive mode hasn't unlocked yet.
const LOCKED_ALPHA: f64 = 0.2;

/// Ask the terminal what its text and background colors are, with the standard OSC 10/11
/// queries most terminals answer. Call it before taking over the terminal, since the answer
/// comes back as input.
pub fn detect() {
    let colors = terminal_colorsaurus::color_palette(terminal_colorsaurus::QueryOptions::default())
        .ok()
        .map(|p| (p.foreground.scale_to_8bit(), p.background.scale_to_8bit()));
    let _ = TERMINAL.set(colors);
}

/// Text faded into the background, behind a menu (`FADED_ALPHA`).
pub fn faded() -> Option<Color> {
    see_through(FADED_ALPHA)
}

/// Letters not unlocked yet (`LOCKED_ALPHA`).
pub fn locked() -> Option<Color> {
    see_through(LOCKED_ALPHA)
}

/// Text at `alpha` opacity, in light and dark themes alike. Terminals can't draw see-through
/// text, so this mixes the terminal's text color into its background color. None if the
/// terminal didn't say what those are.
fn see_through(alpha: f64) -> Option<Color> {
    let (text, background) = (*TERMINAL.get()?)?;
    Some(blend(background, text, alpha))
}

/// `share` of the way from color `a` to color `b`.
fn blend(a: (u8, u8, u8), b: (u8, u8, u8), share: f64) -> Color {
    let mix = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * share).round() as u8;
    Color::Rgb(mix(a.0, b.0), mix(a.1, b.1), mix(a.2, b.2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blends_toward_the_text() {
        // Light theme: just a little darker than the background.
        assert_eq!(blend((0xfb, 0xf7, 0xf0), (0, 0, 0), FADED_ALPHA), Color::Rgb(226, 222, 216));
        // Dark theme: just a little lighter.
        assert_eq!(blend((0x1e, 0x1e, 0x1e), (0xd4, 0xd4, 0xd4), FADED_ALPHA), Color::Rgb(48, 48, 48));
    }
}

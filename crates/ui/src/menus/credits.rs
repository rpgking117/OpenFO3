//! The credits (`CreditsMenu`, class 1047, `menus\options\credits_menu.xml`),
//! from `Fallout3.exe` 1.7.0.3: the main menu's Crew, and `PlayCredits`.
//!
//! * The roll (`0063c810`) is read from `Data\Credits.txt`, line by line: a
//!   line starting with `*` is a comment; a line that starts with a carriage
//!   return (an empty line) moves down by the font's `+0x2c` (the
//!   [`crate::font::Font::descent`] here); `<X:n>` sets x, `<X:+n>` /
//!   `<X:-n>` move it; `<Y:n>` sets y, `<Y:+n>` moves down, `<Y:-n>` up;
//!   `<J:L|C|R>` justifies (1, 2, 4); `<F:n>` picks font n; `<C:rrr,ggg,bbb,aaa>`
//!   colours (fixed widths: the numbers are read at offsets 3, 7, 11, 15 from
//!   the `<`); `<I:wwww,hhhh,aaa,file>` places a picture from
//!   `Textures\Interface\Credits\` (offsets 3, 8, 13, the name from 17) and
//!   moves down by its height; other text up to a `<`, `>`, `*` or carriage
//!   return is a line of text at (x, y), which then moves down by the text's
//!   height. Characters below a space are skipped.
//! * Opening (`0063ce90`): the roll starts 720 below (`0xc4340000`), moved
//!   214 right when the screen isn't 4:3.
//! * Each frame (`CreditsMenu::vf0B`): the roll rises by
//!   `fCreditsScrollSpeed` × the frame's seconds; past twice its bound
//!   radius, or on a click, Escape or another key (`vf0E`: codes outside
//!   -1..4), the menu closes (`0063c120`).
//!
//! Positions here are in menu units with y down: the game's y (up) negated.

use crate::draw::{DrawItem, DrawKind};
use crate::font::{extra_line_gap, Font};
use crate::text::GlyphQuad;
use crate::tile::TileId;

/// The menu's file and class.
pub const FILE: &str = "menus\\options\\credits_menu.xml";
pub const CLASS: i32 = 1047;
/// The roll (a loose file under `Data`).
pub const TEXT: &str = "credits.txt";
/// The pictures' folder, under `textures\`.
pub const PICTURES: &str = "textures\\interface\\credits\\";
/// How far below the roll starts (`0063c810`: `0xc4340000`).
pub const START_BELOW: f32 = 720.0;
/// The roll moved right on a screen that isn't 4:3 (`0063ce90`).
pub const WIDESCREEN_SHIFT: f32 = 214.0;
/// `[Menu] fCreditsScrollSpeed`: the INI's 40 (the exe's own default is 55,
/// `00ce7a30`).
pub const SCROLL_SPEED: f32 = 40.0;
/// The original rises faster than the setting says: recorded on Nate's PC
/// with `fCreditsScrollSpeed` 40, the roll rose 58.4 menu units a second (a
/// line 578 pixels at 1920 x 1080 over 8.8 s), 1.46 times it. `vf0B`
/// multiplies the setting by the frame's time (`01090bac`); where the rest
/// comes from isn't traced, so this is measured, not read.
pub const MEASURED_SPEED_FACTOR: f32 = 1.46;

/// Justification (`<J:..>`): the text tiles' numbers.
pub const LEFT: i32 = 1;
pub const CENTER: i32 = 2;
pub const RIGHT: i32 = 4;

/// One thing in the roll.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Text {
        text: String,
        /// 1 to 8.
        font: usize,
        justify: i32,
        /// Red, green, blue and alpha, 0 to 255.
        color: [u8; 4],
    },
    Picture {
        /// Under `textures\`, lower case.
        file: String,
        width: f32,
        height: f32,
        alpha: u8,
    },
}

/// One thing and where it is: x across from the roll's origin, y down.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub kind: Kind,
    pub x: f32,
    pub y: f32,
}

/// A number the way `atol` reads it at `at`: spaces, a sign, digits.
fn atol(bytes: &[u8], at: usize) -> i64 {
    let mut i = at;
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    let mut sign = 1;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut n: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        n = n * 10 + i64::from(bytes[i] - b'0');
        i += 1;
    }
    sign * n
}

/// Text from `at` up to a carriage return, `<`, `>` or `*` (`0063c2c0`),
/// and where it stopped.
fn run(bytes: &[u8], mut at: usize) -> (String, usize) {
    let start = at;
    while at < bytes.len() && !matches!(bytes[at], b'\r' | b'<' | b'>' | b'*') {
        at += 1;
    }
    (esm::text::decode_cp1252(&bytes[start..at]), at)
}

/// Reads the roll (`0063c810`). `blank(font)` is how far an empty line moves
/// down; `height(font, text)` a line of text's height.
// Translated from 0063c810 (decompiled, Fallout3.exe 1.7.0.3)
pub fn parse(
    bytes: &[u8],
    blank: impl Fn(usize) -> f32,
    height: impl Fn(usize, &str) -> f32,
) -> Vec<Entry> {
    let mut out = Vec::new();
    let (mut x, mut y) = (0i64, 0i64);
    let mut font = 1usize;
    let mut justify = LEFT;
    let mut color = [255u8; 4];
    let n = bytes.len();
    let mut pos = 0usize;
    while pos < n {
        if bytes[pos] == b'\r' {
            y -= blank(font).round() as i64;
        }
        while pos < n && bytes[pos] != b'\n' {
            let c = bytes[pos];
            if c == b'*' {
                while pos < n && bytes[pos] != b'\n' {
                    pos += 1;
                }
            } else if c == b'<' {
                let tag = bytes.get(pos + 1).copied().unwrap_or(0);
                let sub = bytes.get(pos + 3).copied().unwrap_or(0);
                match tag {
                    b'C' => {
                        for (k, off) in [3, 7, 11, 15].into_iter().enumerate() {
                            color[k] = atol(bytes, pos + off).clamp(0, 255) as u8;
                        }
                    }
                    b'F' => font = usize::from(sub.saturating_sub(b'0')).max(1),
                    b'J' => {
                        justify = match sub {
                            b'C' => CENTER,
                            b'R' => RIGHT,
                            _ => LEFT,
                        }
                    }
                    b'X' => match sub {
                        b'+' => x += atol(bytes, pos + 4),
                        b'-' => x -= atol(bytes, pos + 4),
                        _ => x = atol(bytes, pos + 3),
                    },
                    b'Y' => match sub {
                        b'+' => y -= atol(bytes, pos + 4),
                        b'-' => y += atol(bytes, pos + 4),
                        _ => y = atol(bytes, pos + 3),
                    },
                    b'I' => {
                        let width = atol(bytes, pos + 3);
                        let h = atol(bytes, pos + 8);
                        let alpha = atol(bytes, pos + 13).clamp(0, 255) as u8;
                        let (name, end) = run(bytes, (pos + 17).min(n));
                        out.push(Entry {
                            kind: Kind::Picture {
                                file: format!("{PICTURES}{}", name.trim().to_ascii_lowercase()),
                                width: width as f32,
                                height: h as f32,
                                alpha,
                            },
                            x: x as f32,
                            y: -(y as f32),
                        });
                        y -= h;
                        pos = end;
                    }
                    _ => {}
                }
                // On to the tag's end unless already there (a picture's name
                // stops at it), then past it.
                while pos < n && bytes[pos] != b'>' {
                    pos += 1;
                }
                pos += 1;
            } else if c < 0x20 {
                pos += 1;
            } else {
                let (text, end) = run(bytes, pos);
                out.push(Entry {
                    kind: Kind::Text {
                        text: text.clone(),
                        font,
                        justify,
                        color,
                    },
                    x: x as f32,
                    y: -(y as f32),
                });
                y -= height(font, &text).round() as i64;
                pos = end;
            }
        }
        pos += 1;
    }
    out
}

/// How far the roll reaches down from its origin (the last thing's bottom).
pub fn length(entries: &[Entry], height: impl Fn(usize, &str) -> f32) -> f32 {
    entries
        .iter()
        .map(|e| {
            e.y + match &e.kind {
                Kind::Text { font, text, .. } => height(*font, text),
                Kind::Picture { height, .. } => *height,
            }
        })
        .fold(0.0, f32::max)
}

/// Where the roll's origin is, `seconds` after it started: from
/// [`START_BELOW`] up at `speed` units a second; `None` once it has gone
/// (`vf0B`: past the roll's own size).
pub fn scroll(seconds: f32, speed: f32, length: f32) -> Option<f32> {
    let risen = speed * seconds - START_BELOW;
    (risen <= length).then_some(-risen)
}

/// The wrap width the game gives each line (`0063c810`: `0x500`).
pub const WRAP: i32 = 1280;

/// A line of text laid out as a text tile lays it out
/// (`crate::tile::Ui::layout`): the font's line height, its extra gap, the
/// justification.
fn lay_out(font: &Font, index: usize, text: &str, justify: i32) -> (Vec<GlyphQuad>, f32) {
    let line_height = font.line_height;
    let extra = extra_line_gap(index);
    let bytes: Vec<u8> = text.chars().map(|c| u8::try_from(c as u32).unwrap_or(b'?')).collect();
    let prepared = crate::text::prepare(font, line_height, extra, &bytes, WRAP, 0);
    let (quads, _) = crate::text::glyphs(font, line_height, extra, &prepared, justify);
    (quads, prepared.height as f32)
}

/// How far the roll moves down after an empty line and after a line of
/// text: the font's `+0x2c` ([`Font::descent`]) for both, from the loaded
/// fonts (1 to 8). (Compared with the original: font 7's 48 gives its 138
/// units from one credit's heading to the next.)
pub fn heights(fonts: &[Option<Font>]) -> (impl Fn(usize) -> f32 + '_, impl Fn(usize, &str) -> f32 + '_) {
    let font = move |i: usize| fonts.get(i.saturating_sub(1)).and_then(|f| f.as_ref());
    (
        move |i: usize| font(i).map_or(0.0, |f| f.descent),
        move |i: usize, _text: &str| font(i).map_or(0.0, |f| f.descent),
    )
}

/// The roll's pictures and text with its origin at `origin` (menu units),
/// those on the screen (`height` tall) only.
pub fn draw_items(
    fonts: &[Option<Font>],
    entries: &[Entry],
    origin: [f32; 2],
    height: f32,
    tile: TileId,
    depth: f32,
) -> Vec<DrawItem> {
    let mut out = Vec::new();
    for e in entries {
        let x = origin[0] + e.x;
        let y = origin[1] + e.y;
        if y > height || y < -200.0 {
            continue;
        }
        match &e.kind {
            Kind::Text {
                text,
                font,
                justify,
                color,
            } => {
                let Some(f) = fonts.get(font.saturating_sub(1)).and_then(|f| f.as_ref()) else {
                    continue;
                };
                let (quads, _) = lay_out(f, *font, text, *justify);
                let glyphs = quads
                    .iter()
                    .filter(|q| q.right > q.left)
                    .map(|q| {
                        (
                            [x + q.left, y + q.top, q.right - q.left, q.bottom - q.top],
                            q.uv,
                            q.texture,
                        )
                    })
                    .collect();
                out.push(DrawItem {
                    tile,
                    depth,
                    color: color.map(|c| f32::from(c) / 255.0),
                    kind: DrawKind::Text {
                        font: *font,
                        glyphs,
                    },
                });
            }
            Kind::Picture {
                file,
                width,
                height: h,
                alpha,
            } => out.push(DrawItem {
                tile,
                depth,
                color: [1.0, 1.0, 1.0, f32::from(*alpha) / 255.0],
                kind: DrawKind::Image {
                    texture: file.clone(),
                    rect: [x, y, *width, *h],
                    uv: [0.0, 0.0, 1.0, 1.0],
                    repeat_u: false,
                    scroll: None,
                },
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROLL: &[u8] = b"*\r\n* a comment <X:9>\r\n<X:-530><Y:0><J:L><F:7>\r\n\r\n<I:0256,0064,255,FalloutLogo.dds>\r\n<C:199,255,165,255>GAME DESIGN<Y:-6>\r\n<C:248,255,217,255>Bethesda Game Studios\r\n<J:C><X:+30>Todd Howard\r\n";

    fn parsed() -> Vec<Entry> {
        parse(ROLL, |_| 20.0, |_, _| 30.0)
    }

    /// Comments are skipped; tags set the place, font, justification and
    /// colour; an empty line moves down by the font's gap, a picture by its
    /// height, a line of text by its own.
    #[test]
    fn the_roll_reads_as_the_game_reads_it() {
        let e = parsed();
        assert_eq!(e.len(), 4);
        assert_eq!(
            e[0].kind,
            Kind::Picture {
                file: "textures\\interface\\credits\\falloutlogo.dds".into(),
                width: 256.0,
                height: 64.0,
                alpha: 255
            }
        );
        // After the empty line.
        assert_eq!((e[0].x, e[0].y), (-530.0, 20.0));
        assert_eq!(
            e[1].kind,
            Kind::Text {
                text: "GAME DESIGN".into(),
                font: 7,
                justify: LEFT,
                color: [199, 255, 165, 255]
            }
        );
        assert_eq!(e[1].y, 84.0);
        // 30 for the line, then `<Y:-6>` up.
        assert_eq!(e[2].y, 84.0 + 30.0 - 6.0);
        assert!(matches!(e[2].kind, Kind::Text { color: [248, 255, 217, 255], .. }));
        assert_eq!((e[3].x, e[3].y), (-500.0, 138.0));
        assert!(matches!(e[3].kind, Kind::Text { justify: CENTER, .. }));
    }

    /// The roll starts below and rises until it has gone past its length.
    #[test]
    fn the_roll_rises_and_ends() {
        assert_eq!(scroll(0.0, 40.0, 100.0), Some(720.0));
        assert_eq!(scroll(18.0, 40.0, 100.0), Some(0.0));
        assert!(scroll(30.0, 40.0, 100.0).is_none());
        assert_eq!(length(&parsed(), |_, _| 30.0), 168.0);
    }
}

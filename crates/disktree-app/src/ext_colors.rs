//! File-type colours, read from `disktree.ext.colors.txt`.
//!
//! One `extension=#rrggbb` line colours every file of that type. A directory
//! carries no extension of its own, so it takes the colour of the extension
//! it holds the most bytes of ([`disktree_core::classify::dominant_extension`]).
//! An extension with no line keeps its old category colour, so a partly
//! filled file is safe.
//!
//! The file is found like a language file — beside the executable, in the
//! working directory, then under the user's config directory — and read once
//! at startup. It is read again whenever the *File type colors* toggle is
//! switched on, so an edit takes effect without a restart.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, RwLock};

use gpui_kit::{Hsla, Rgba};

/// The one file name this reads.
pub const FILE_NAME: &str = "disktree.ext.colors.txt";

/// The configured colours, keyed by lowercased extension without its dot.
static COLORS: LazyLock<RwLock<HashMap<String, Hsla>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Read the colour file, if there is one. Called once, at startup.
pub fn load() {
    reload();
}

/// Read the colour file again from the usual directories, replacing what is
/// known. Returns whether a file was found.
pub fn reload() -> bool {
    for directory in crate::i18n::search_dirs() {
        if reload_from(&directory) {
            return true;
        }
    }
    install(HashMap::new());
    false
}

/// Read `<directory>/disktree.ext.colors.txt` and install it, returning
/// whether the file was there and readable. Split out so a test can point it
/// at a directory of its own.
pub fn reload_from(directory: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(directory.join(FILE_NAME)) else {
        return false;
    };
    install(parse(&text));
    true
}

/// The colour configured for `extension` (lowercased, no dot).
pub fn color(extension: &str) -> Option<Hsla> {
    COLORS.read().ok()?.get(extension).copied()
}

/// Every configured colour, sorted by extension, for the legend.
pub fn entries() -> Vec<(String, Hsla)> {
    let Ok(colors) = COLORS.read() else {
        return Vec::new();
    };
    let mut all: Vec<(String, Hsla)> = colors
        .iter()
        .map(|(extension, color)| (extension.clone(), *color))
        .collect();
    all.sort_by(|left, right| left.0.cmp(&right.0));
    all
}

/// Whether any colour is configured.
pub fn is_empty() -> bool {
    match COLORS.read() {
        Ok(colors) => colors.is_empty(),
        Err(_) => true,
    }
}

fn install(colors: HashMap<String, Hsla>) {
    if let Ok(mut current) = COLORS.write() {
        *current = colors;
    }
}

/// One `extension=#rrggbb` per line. A line starting with `#` is a comment,
/// blank lines and lines without an `=` are skipped, and the extension is
/// lowercased without a leading dot.
fn parse(text: &str) -> HashMap<String, Hsla> {
    let mut colors = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((extension, value)) = line.split_once('=') else {
            continue;
        };
        let extension = extension
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase();
        if extension.is_empty() {
            continue;
        }
        if let Some(color) = parse_hex(value.trim()) {
            colors.insert(extension, color);
        }
    }
    colors
}

/// `#rrggbb`, or the three-digit `#rgb`, with the `#` optional.
fn parse_hex(value: &str) -> Option<Hsla> {
    let digits = value.strip_prefix('#').unwrap_or(value);
    if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let (red, green, blue) = match digits.len() {
        6 => (
            u8::from_str_radix(&digits[0..2], 16).ok()?,
            u8::from_str_radix(&digits[2..4], 16).ok()?,
            u8::from_str_radix(&digits[4..6], 16).ok()?,
        ),
        3 => {
            // `#f80` is `#ff8800`: each digit doubles as a nibble.
            let nibble = |at: usize| {
                Some(u8::from_str_radix(&digits[at..=at], 16).ok()? * 17)
            };
            (nibble(0)?, nibble(1)?, nibble(2)?)
        }
        _ => return None,
    };
    Some(Hsla::from(Rgba {
        r: f32::from(red) / 255.0,
        g: f32::from(green) / 255.0,
        b: f32::from(blue) / 255.0,
        a: 1.0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_becomes_an_extension_and_a_colour() {
        let colors = parse(
            "# a comment\n\nmp3=#ff8800\n.rs = 00FF00\nbad line\nnone=\nbad=#zzz\n",
        );
        assert_eq!(colors.len(), 2);
        assert_eq!(colors.get("mp3"), parse_hex("#ff8800").as_ref());
        assert_eq!(colors.get("rs"), parse_hex("#00ff00").as_ref());
    }

    #[test]
    fn hex_takes_six_or_three_digits_with_or_without_the_hash() {
        assert!((parse_hex("#ffffff").expect("white").l - 1.0).abs() < 1e-6);
        let shorthand = parse_hex("#f80").expect("shorthand");
        let long = parse_hex("ff8800").expect("long form");
        assert!((shorthand.l - long.l).abs() < 1e-6);
        assert!((shorthand.h - long.h).abs() < 1e-6);
        assert!(parse_hex("#12345").is_none());
        assert!(parse_hex("nope").is_none());
    }

    /// The one test that touches the shared map: switching *File type colors*
    /// on must pick up an edit on disk without a restart.
    #[test]
    fn reading_again_picks_up_an_edited_file() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        std::fs::write(temp.path().join(FILE_NAME), "mp3=#ff0000\n")
            .expect("write");
        assert!(reload_from(temp.path()));
        let first = color("mp3").expect("configured");

        std::fs::write(temp.path().join(FILE_NAME), "mp3=#00ff00\n")
            .expect("rewrite");
        assert!(reload_from(temp.path()));
        let second = color("mp3").expect("configured again");

        assert_ne!(first, second, "the second read replaced the first");
        assert_eq!(color("wav"), None, "an unlisted extension has no colour");
    }
}

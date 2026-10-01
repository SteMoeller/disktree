//! The interface language, read from files instead of baked into the binary.
//!
//! The English text is the key: a call site asks for `i18n::t("Mark")`, and a
//! `disktree.de.i18n.txt` line `Mark=Markieren` translates it. A key no file
//! defines comes back unchanged, so a partly translated file can never render
//! an empty or a `some.key` label — and a string can be moved out of the code
//! one at a time.
//!
//! Files are looked up once at startup, in this order: beside the executable,
//! the working directory, then the user's config directory. The first file
//! found for a language wins. English is built in; an external
//! `disktree.en.i18n.txt` overrides it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The English strings, compiled in so a missing file still reads. The keys
/// are the English text itself, so this file lists them, and an external
/// `disktree.en.i18n.txt` may override any of them.
const EMBEDDED_EN: &str = include_str!("../../../disktree.en.i18n.txt");

/// The shape of a language file: `disktree.<code>.i18n.txt`.
const FILE_PREFIX: &str = "disktree.";
const FILE_SUFFIX: &str = ".i18n.txt";

struct Language {
    code: String,
    strings: HashMap<&'static str, &'static str>,
}

struct Catalog {
    languages: Vec<Language>,
    current: AtomicUsize,
}

static CATALOG: OnceLock<Catalog> = OnceLock::new();

/// Read the language files and install the catalog. Called once, at startup.
pub fn load() {
    let mut languages = vec![Language {
        code: "en".to_string(),
        strings: leak(parse(EMBEDDED_EN)),
    }];
    let mut seen: Vec<String> = Vec::new();
    let mut external_en = false;

    for dir in search_dirs() {
        for (code, strings) in files_in(&dir) {
            if code == "en" {
                if external_en {
                    continue;
                }
                external_en = true;
                languages[0].strings = leak(strings);
                continue;
            }
            if seen.iter().any(|name| name == &code) {
                continue;
            }
            seen.push(code.clone());
            languages.push(Language {
                code,
                strings: leak(strings),
            });
        }
    }
    languages[1..].sort_by(|left, right| left.code.cmp(&right.code));

    let _ = CATALOG.set(Catalog {
        languages,
        current: AtomicUsize::new(0),
    });
}

/// The key's text in the current language, else English, else the key.
///
/// Returns `&'static str` so a translated label fits every call site the
/// English literal fitted — a `&str` parameter as well as `Into<SharedString>`.
/// The strings are leaked once at [`load`], which is bounded by the files.
pub fn t(key: &'static str) -> &'static str {
    let Some(catalog) = CATALOG.get() else {
        return key;
    };
    let index = catalog.current.load(Ordering::Relaxed);
    if let Some(language) = catalog.languages.get(index)
        && let Some(text) = language.strings.get(key).copied()
    {
        return text;
    }
    if let Some(english) = catalog.languages.first()
        && let Some(text) = english.strings.get(key).copied()
    {
        return text;
    }
    key
}

/// [`t`], with `{0}`, `{1}`, … filled from `args` in order.
pub fn tf(key: &'static str, args: &[&dyn std::fmt::Display]) -> String {
    let mut text = t(key).to_string();
    for (index, arg) in args.iter().enumerate() {
        text = text.replace(&format!("{{{index}}}"), &arg.to_string());
    }
    text
}

/// The language codes found, English first.
pub fn languages() -> Vec<String> {
    CATALOG.get().map_or_else(
        || vec!["en".to_string()],
        |catalog| {
            catalog
                .languages
                .iter()
                .map(|language| language.code.clone())
                .collect()
        },
    )
}

/// The current language code.
pub fn current() -> String {
    let Some(catalog) = CATALOG.get() else {
        return "en".to_string();
    };
    let index = catalog.current.load(Ordering::Relaxed);
    catalog
        .languages
        .get(index)
        .map_or_else(|| "en".to_string(), |language| language.code.clone())
}

/// Move to the next found language and return its code.
pub fn cycle() -> String {
    let Some(catalog) = CATALOG.get() else {
        return "en".to_string();
    };
    let count = catalog.languages.len().max(1);
    let next = (catalog.current.load(Ordering::Relaxed) + 1) % count;
    catalog.current.store(next, Ordering::Relaxed);
    current()
}

/// Where language files are looked for, in order: the executable's directory,
/// the working directory, then the user's config directory. Also where
/// [`crate::ext_colors`] looks for its file, so the search order is one thing.
pub fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        dirs.push(dir.to_path_buf());
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd);
    }
    if let Some(dir) = config_dir() {
        dirs.push(dir);
    }
    dirs
}

/// `%APPDATA%\disktree` on Windows, `$XDG_CONFIG_HOME/disktree` or
/// `~/.config/disktree` elsewhere.
fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(|base| PathBuf::from(base).join("disktree"))
    }
    #[cfg(not(windows))]
    {
        if let Some(base) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(base).join("disktree"));
        }
        std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(".config/disktree"))
    }
}

/// Every `disktree.<code>.i18n.txt` in `dir`, as `(code, strings)`.
fn files_in(dir: &Path) -> Vec<(String, HashMap<String, String>)> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(code) = name
            .strip_prefix(FILE_PREFIX)
            .and_then(|rest| rest.strip_suffix(FILE_SUFFIX))
        else {
            continue;
        };
        if code.is_empty() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        found.push((code.to_ascii_lowercase(), parse(&text)));
    }
    found.sort_by(|left, right| left.0.cmp(&right.0));
    found
}

/// Keep a parsed file's strings for the life of the process. The files are read
/// once, so leaking them is bounded, and it lets [`t`] hand out `&'static str`.
fn leak(
    strings: HashMap<String, String>,
) -> HashMap<&'static str, &'static str> {
    strings
        .into_iter()
        .map(|(key, value)| {
            let key: &'static str = Box::leak(key.into_boxed_str());
            let value: &'static str = Box::leak(value.into_boxed_str());
            (key, value)
        })
        .collect()
}

/// `key=text` lines; `#` starts a comment, blank lines and lines without an
/// `=` are skipped. The first `=` separates key from text.
fn parse(text: &str) -> HashMap<String, String> {
    let mut strings = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // A tab separates key and text when the key itself holds an `=`, as
        // the key-hint lines do; otherwise the first `=` separates them.
        let Some((key, value)) =
            line.split_once('\t').or_else(|| line.split_once('='))
        else {
            continue;
        };
        let key = key.trim_end();
        if key.is_empty() {
            continue;
        }
        strings.insert(key.to_string(), value.trim().to_string());
    }
    strings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_becomes_a_key_and_its_text() {
        let strings = parse(
            "# a comment\n\nMark=Markieren\nno equals sign\nPad = spaced \n",
        );
        assert_eq!(strings.get("Mark").map(String::as_str), Some("Markieren"));
        assert_eq!(strings.get("Pad").map(String::as_str), Some("spaced"));
        assert!(!strings.contains_key("no equals sign"));
    }

    #[test]
    fn a_key_may_hold_an_equals_sign_via_a_tab() {
        // The key-hint lines read `- / = / 0`; a tab separates them.
        let strings = parse("- / = / 0\t- / = / 0\n");
        assert_eq!(
            strings.get("- / = / 0").map(String::as_str),
            Some("- / = / 0")
        );
    }

    #[test]
    fn a_key_without_a_file_comes_back_unchanged() {
        // English is the key, so nothing is ever rendered empty.
        assert_eq!(t("Whatever this is"), "Whatever this is");
    }

    #[test]
    fn placeholders_are_filled_in_order() {
        let a = "1 GiB";
        let b = "2 GiB";
        assert_eq!(tf("{0} free of {1}", &[&a, &b]), "1 GiB free of 2 GiB");
    }
}

//! What a directory *is*, and whether its space can be had back.
//!
//! Colour in the treemap means a kind of data, and a hatch means reclaimable
//! space, so the two questions a cleanup tool exists to answer — "what is it"
//! and "can I delete it" — can be read off a tile at a glance.
//!
//! Both come from names. A short lookup of well-known directory names covers
//! most of a home directory; anything unmatched takes its parent's kind, and a
//! top-level directory with an unknown name takes the kind of its largest
//! recognisable child (`~/world` is mostly `.git`, so it is git). Reclaimable
//! space is the same idea, plus a sibling check where a name alone is too
//! common to trust: `target` is only a build directory beside a `Cargo.toml`.

use std::borrow::Cow;
use std::collections::HashMap;

use rayon::prelude::*;

use crate::tree::{Node, NodeKind};

/// A kind of data, for colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Category {
    /// Source code and checkouts.
    Code,
    /// Space agents write into: worktrees, sandboxes, experiments.
    AgentScratch,
    /// Compilers, package managers and their installs.
    Toolchain,
    /// Folders a sync client owns.
    Synced,
    /// Version-control object stores.
    Git,
    /// Pictures, music, video, games and models.
    Media,
    /// Documents, downloads and the desktop.
    Documents,
    /// Caches and other regenerable state.
    Cache,
    /// Nothing recognisable.
    #[default]
    Other,
}

impl Category {
    /// The categories the legend lists, in its order.
    pub const LEGEND: [Self; 8] = [
        Self::Code,
        Self::AgentScratch,
        Self::Toolchain,
        Self::Synced,
        Self::Git,
        Self::Media,
        Self::Documents,
        Self::Cache,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Code => "Code",
            Self::AgentScratch => "Agent scratch",
            Self::Toolchain => "Toolchains",
            Self::Synced => "Synced",
            Self::Git => "Git",
            Self::Media => "Media",
            Self::Documents => "Documents",
            Self::Cache => "Cache",
            Self::Other => "Other",
        }
    }
}

/// Why a directory's space can be had back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reclaim {
    /// A cache: whatever wrote it will write it again.
    Regenerable,
    /// A sync client's old versions of files.
    SyncHistory,
    /// A package manager's content store.
    PackageStore,
    /// Compiler or bundler output beside its sources.
    BuildOutput,
    /// Installed dependencies beside their manifest.
    Reinstallable,
    /// Container or sandbox image layers.
    SandboxLayers,
    /// Sandbox or VM snapshots.
    Snapshots,
    /// Already deleted, still on disk.
    Trash,
    /// Scratch space meant to be thrown away.
    Temporary,
}

impl Reclaim {
    /// The reason, as the "Worth a look" list says it.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Regenerable => "regenerable",
            Self::SyncHistory => "sync history",
            Self::PackageStore => "package store",
            Self::BuildOutput => "build output",
            Self::Reinstallable => "reinstallable",
            Self::SandboxLayers => "sandbox layers",
            Self::Snapshots => "snapshots",
            Self::Trash => "trash",
            Self::Temporary => "temporary",
        }
    }
}

/// Bytes of a name lowercased on the stack.
const LOWERED: usize = 32;

/// `name` lowercased, on the stack when it fits in [`LOWERED`] bytes: a
/// directory on a whole disk is looked up twice for each of hundreds of
/// thousands, and a string allocated each time was most of what
/// classifying cost. A longer name is rare enough to allocate.
fn lowered<'a>(name: &str, buffer: &'a mut [u8; LOWERED]) -> Cow<'a, str> {
    let Some(bytes) = buffer.get_mut(..name.len()) else {
        return Cow::Owned(name.to_ascii_lowercase());
    };
    bytes.copy_from_slice(name.as_bytes());
    bytes.make_ascii_lowercase();
    // ASCII lowercasing keeps UTF-8.
    Cow::Borrowed(std::str::from_utf8(bytes).unwrap_or_default())
}

/// The kind a directory name announces on its own, if any.
pub fn category_of_name(name: &str) -> Option<Category> {
    let mut buffer = [0; LOWERED];
    let lower = lowered(name, &mut buffer);
    // Windows: a work or school account's folder carries the organisation,
    // `OneDrive - Contoso`, and Dropbox's does the same, `Dropbox (Contoso)`.
    if lower.starts_with("onedrive - ") || lower.starts_with("dropbox (") {
        return Some(Category::Synced);
    }
    let category = match &*lower {
        "src" | "code" | "projects" | "repos" | "dev" | "work"
        | "workspace" | "workspaces" | "github.com" | "gitlab.com"
        | "sites" | "development" => Category::Code,
        ".codex" | ".claude" | ".herdr" | ".pi" | ".cursor" | ".aider"
        | ".gemini" | ".continue" | ".windsurf" | ".microsandbox" | ".omp"
        | ".agents" | ".openai" | "tries" | "worktrees" | "experiments"
        | "scratch" | "playground" => Category::AgentScratch,
        ".cargo" | ".rustup" | ".local" | ".npm" | ".pnpm-store" | "pnpm"
        | ".bun" | ".deno" | "go" | ".gradle" | ".m2" | ".platformio"
        | "mise" | ".mise" | ".pyenv" | ".nvm" | ".gem" | "gem" | ".rbenv"
        | ".espressif" | ".arduino15" | ".config" | ".vscode" | ".zig"
        | ".rye" | ".conda" | "anaconda3" | "miniconda3" | ".opam"
        | ".ghcup" | ".stack" | ".julia" | ".dotnet" | ".android"
        | ".sdkman" | ".volta" | ".yarn" | ".java" | ".nuget"
        // macOS: Xcode's and the simulators' state in ~/Library/Developer.
        // Not `Developer` itself: ~/Developer is where Apple puts projects.
        | "xcode" | "coresimulator" => Category::Toolchain,
        "sync" | "dropbox" | "nextcloud" | "google drive" | "onedrive"
        | "pclouddrive" | "mega" | ".stversions"
        // macOS: iCloud Drive, and the File Provider clients (Dropbox,
        // Google Drive, OneDrive) since macOS 12.
        | "mobile documents" | "cloudstorage"
        // Windows: iCloud for Windows.
        | "iclouddrive" => Category::Synced,
        ".git" => Category::Git,
        "pictures" | "photos" | "music" | "videos" | "movies" | "steam"
        | "steamlibrary" | "steamapps" | "emulation"
        | "models" | ".ollama" | ".lmstudio" | "games" | "wineprefix" => {
            Category::Media
        }
        "documents" | "desktop" | "downloads" | "books" | "notes"
        | "obsidian" | "public" | "templates" => Category::Documents,
        ".cache" | "cache" | "caches" | ".ccache" | ".sccache" | "_cacache"
        | "__pycache__" | "node_modules" | "trash" | ".trash" | "tmp"
        | ".tmp" | "deriveddata" | "ios devicesupport"
        | "watchos devicesupport"
        // Windows: see `reclaim_of`.
        | "temp" | "$recycle.bin" | "npm-cache" | "v3-cache" | "inetcache"
        | "d3dscache" | "dxcache" | "glcache" | "crashdumps" => Category::Cache,
        _ => return None,
    };
    Some(category)
}

/// Whether a directory's space can be had back, judged from its name, the
/// kind of the directory holding it, and its siblings' names.
pub fn reclaim_of(
    name: &str,
    parent: Category,
    has_sibling: impl Fn(&str) -> bool,
) -> Option<Reclaim> {
    let mut buffer = [0; LOWERED];
    let lower = lowered(name, &mut buffer);
    let reclaim = match &*lower {
        ".cache" | "cache" | "caches" | ".ccache" | ".sccache" | "_cacache"
        // Windows' own caches in AppData\Local: npm's, NuGet's downloads,
        // the browser engine's, and compiled shaders, Direct3D's and the
        // graphics driver's, all rebuilt as they are needed.
        | "npm-cache" | "v3-cache" | "inetcache" | "d3dscache" | "dxcache"
        | "glcache"
        // Symbols Xcode copies off a device it meets, and copies again the
        // next time that device is plugged in.
        | "ios devicesupport" | "watchos devicesupport" => Reclaim::Regenerable,
        ".stversions" => Reclaim::SyncHistory,
        ".pnpm-store" | "pnpm" => Reclaim::PackageStore,
        "__pycache__" | ".pytest_cache" | ".mypy_cache" | ".ruff_cache"
        | ".next" | ".turbo" | ".parcel-cache"
        // Xcode's build products and indexes, rebuilt on the next build.
        | "deriveddata" => Reclaim::BuildOutput,
        // ~/Library/Logs, told apart from a project's logs by its neighbour.
        "logs" if has_sibling("Application Support") => Reclaim::Temporary,
        // Too common to trust alone: only a build directory beside a manifest.
        "target" if has_sibling("Cargo.toml") => Reclaim::BuildOutput,
        "node_modules" if has_sibling("package.json") => Reclaim::Reinstallable,
        // Layers and snapshots are only disposable inside sandbox state.
        "layers" if parent == Category::AgentScratch => Reclaim::SandboxLayers,
        "snapshots" if parent == Category::AgentScratch => Reclaim::Snapshots,
        "trash" | ".trash" | "$recycle.bin" => Reclaim::Trash,
        // `Temp` is where Windows puts temporary files, in AppData\Local;
        // `CrashDumps` beside it holds dumps of programs that crashed.
        "tmp" | ".tmp" | "temp" | "crashdumps" => Reclaim::Temporary,
        _ => return None,
    };
    Some(reclaim)
}

/// Assign a category and a reclaim reason to every node beneath `root`.
///
/// Top-down: a node's own name wins, otherwise it inherits. Reclaimable
/// space is inherited too, so everything under a cache is hatched.
pub fn classify(root: &mut Node) {
    root.category = Category::Other;
    root.reclaim = None;
    for index in 0..root.children.len() {
        let siblings = &root.children;
        let has_sibling =
            |wanted: &str| siblings.iter().any(|name| &*name.name == wanted);
        let child = &siblings[index];
        // A top-level directory with an unknown name takes the kind of its
        // largest recognisable child: `~/world` is mostly `.git`.
        let category = category_of_name(&child.name)
            .or_else(|| is_git_store(child).then_some(Category::Git))
            .or_else(|| dominant_child_category(child))
            .unwrap_or(Category::Other);
        let reclaim = child
            .is_dir()
            .then(|| reclaim_of(&child.name, Category::Other, has_sibling))
            .flatten();
        classify_below(&mut root.children[index], category, reclaim, 1);
    }
}

fn classify_below(
    node: &mut Node,
    category: Category,
    reclaim: Option<Reclaim>,
    depth: usize,
) {
    node.category = category;
    node.reclaim = reclaim;
    // Most directories hold only files, which take this one's kind as
    // they are: no list of kinds to build, no call per file.
    if !node.children.iter().any(Node::is_dir) {
        for child in &mut node.children {
            child.category = category;
            child.reclaim = reclaim;
        }
        return;
    }
    // See `tree::PARALLEL_LEVELS`: parallel only near the top. Deeper, the
    // children are decided and descended one at a time, with no list of
    // decisions: there are half a million directories to get through.
    if depth < crate::tree::PARALLEL_LEVELS {
        let kinds: Vec<(Category, Option<Reclaim>)> = (0..node.children.len())
            .map(|index| kind_of(&node.children, index, category, reclaim))
            .collect();
        node.children.par_iter_mut().zip(kinds).for_each(
            |(child, (category, reclaim))| {
                classify_below(child, category, reclaim, depth + 1);
            },
        );
    } else {
        for index in 0..node.children.len() {
            let (category, reclaim) =
                kind_of(&node.children, index, category, reclaim);
            classify_below(
                &mut node.children[index],
                category,
                reclaim,
                depth + 1,
            );
        }
    }
}

/// What the child at `index` is, given what its parent is: its own name
/// wins, otherwise it inherits. A file always inherits.
fn kind_of(
    siblings: &[Node],
    index: usize,
    category: Category,
    reclaim: Option<Reclaim>,
) -> (Category, Option<Reclaim>) {
    let child = &siblings[index];
    if !child.is_dir() {
        return (category, reclaim);
    }
    let has_sibling =
        |wanted: &str| siblings.iter().any(|name| &*name.name == wanted);
    let child_category = category_of_name(&child.name)
        .or_else(|| is_git_store(child).then_some(Category::Git))
        .unwrap_or(category);
    let child_reclaim =
        reclaim.or_else(|| reclaim_of(&child.name, category, has_sibling));
    (child_category, child_reclaim)
}

/// The kind of an unknown directory, from what fills it: the first
/// recognisable name down the largest children, a few levels deep.
fn dominant_child_category(node: &Node) -> Option<Category> {
    let mut node = node;
    for _ in 0..3 {
        if let Some(category) = node
            .children
            .iter()
            .filter(|child| child.is_dir())
            .find_map(|child| {
                category_of_name(&child.name)
                    .or_else(|| is_git_store(child).then_some(Category::Git))
            })
        {
            return Some(category);
        }
        node = node.children.iter().find(|child| child.is_dir())?;
    }
    None
}

/// A git object store by its shape, whatever it is called: a bare
/// repository, or a `.git` directory, has `objects`, `refs` and `HEAD`.
pub fn is_git_store(node: &Node) -> bool {
    let has =
        |wanted: &str| node.children.iter().any(|child| &*child.name == wanted);
    node.is_dir() && has("objects") && has("refs") && has("HEAD")
}

/// A file name's extension, lowercased without its dot, or `None`.
///
/// `None` for a name with no dot (`notes`), a trailing dot (`notes.`) and a
/// dotfile whose whole name is the extension (`.gitignore`). A dot later on
/// still counts, so `.config.json` is `json` and `archive.tar.gz` is `gz`.
/// Already-lowercase extensions borrow the name rather than allocating: this
/// runs once per file, and a directory can hold hundreds of thousands.
pub fn file_extension(name: &str) -> Option<Cow<'_, str>> {
    let dot = name.rfind('.')?;
    let extension = name.get(dot + 1..)?;
    if extension.is_empty() || dot == 0 {
        return None;
    }
    if extension.bytes().any(|byte| byte.is_ascii_uppercase()) {
        Some(Cow::Owned(extension.to_ascii_lowercase()))
    } else {
        Some(Cow::Borrowed(extension))
    }
}

/// The file extension holding the most bytes in `node`'s subtree, files
/// only: what a directory is mostly made of. `None` when no file beneath it
/// has an extension.
///
/// Bytes, not file count: twenty gigabytes of music beside a hundred of film
/// make a media directory the film's kind, and hardlink de-duplication has
/// already zeroed the duplicates [`crate::tree::aggregate`] left them.
pub fn dominant_extension(node: &Node) -> Option<String> {
    let mut tally: HashMap<Cow<'_, str>, u64> = HashMap::new();
    tally_extensions(node, &mut tally);
    tally
        .into_iter()
        .max_by(|left, right| {
            // Largest first; a tie is broken on the name, so the answer is
            // the same whatever order the map iterates in.
            left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0))
        })
        .map(|(extension, _)| extension.into_owned())
}

fn tally_extensions<'a>(
    node: &'a Node,
    tally: &mut HashMap<Cow<'a, str>, u64>,
) {
    if node.is_dir() {
        for child in &node.children {
            tally_extensions(child, tally);
        }
        return;
    }
    // Only real files count: what a deletion frees is the claim, and a
    // symlink or device is not the bytes this directory is mostly made of.
    if node.kind != NodeKind::File {
        return;
    }
    let Some(extension) = file_extension(&node.name) else {
        return;
    };
    *tally.entry(extension).or_default() += node.bytes;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{Metric, NodeKind, aggregate};

    fn file(name: &str, bytes: u64) -> Node {
        Node::entry(name, NodeKind::File, bytes)
    }

    fn dir(name: &str, children: Vec<Node>) -> Node {
        let mut node = Node::directory(name);
        node.children = children;
        node
    }

    fn home() -> Node {
        let mut home = dir(
            "tobi",
            vec![
                dir(
                    "src",
                    vec![dir(
                        "tries",
                        vec![dir("2026-09-01", vec![file("a", 1)])],
                    )],
                ),
                dir(
                    ".cache",
                    vec![dir("kache", vec![dir("store", vec![file("b", 1)])])],
                ),
                dir(
                    "world",
                    vec![
                        dir(".git", vec![dir("objects", vec![file("c", 9)])]),
                        file("README", 1),
                    ],
                ),
                dir(
                    "rust-thing",
                    vec![
                        file("Cargo.toml", 1),
                        dir("target", vec![file("d", 5)]),
                    ],
                ),
                dir("js-thing", vec![dir("target", vec![file("e", 5)])]),
                dir(
                    ".microsandbox",
                    vec![
                        dir("cache", vec![dir("layers", vec![file("f", 1)])]),
                        dir("snapshots", vec![file("g", 1)]),
                    ],
                ),
                dir("Sync", vec![dir(".stversions", vec![file("h", 1)])]),
                dir("mystery", vec![file("i", 1)]),
                dir(
                    "monorepo",
                    vec![dir(
                        "git",
                        vec![
                            file("HEAD", 1),
                            dir("refs", vec![]),
                            dir("objects", vec![file("pack", 50)]),
                        ],
                    )],
                ),
            ],
        );
        aggregate(&mut home, Metric::Bytes);
        classify(&mut home);
        home
    }

    fn named<'a>(node: &'a Node, path: &[&str]) -> &'a Node {
        let mut node = node;
        for part in path {
            node = node
                .children
                .iter()
                .find(|child| &*child.name == *part)
                .unwrap_or_else(|| panic!("no {part}"));
        }
        node
    }

    /// Regression: a Steam library on a second drive was named by the
    /// `temp` folder Steam keeps in `steamapps`, so a disk of games showed
    /// as cache.
    #[test]
    fn a_steam_library_on_another_drive_is_media_not_its_temp_folder() {
        let mut root = dir(
            "data",
            vec![dir(
                "SteamLibrary",
                vec![dir(
                    "steamapps",
                    vec![
                        dir("temp", vec![file("partial", 1)]),
                        dir("common", vec![file("game.pak", 1_000)]),
                    ],
                )],
            )],
        );
        aggregate(&mut root, Metric::Bytes);
        classify(&mut root);
        let library = &root.children[0];
        assert_eq!(library.category, Category::Media);
        let common = library.children[0].child_named("common").expect("common");
        assert_eq!(common.category, Category::Media);
        assert_eq!(common.reclaim, None, "installed games are not hatched");
    }

    #[test]
    fn steam_libraries_and_emulation_are_media_not_disposable_caches() {
        for name in ["SteamLibrary", "steamapps", "Emulation"] {
            assert_eq!(category_of_name(name), Some(Category::Media));
            assert_eq!(reclaim_of(name, Category::Other, |_| false), None);
        }
    }

    #[test]
    fn names_announce_their_kind_and_children_inherit_it() {
        let home = home();
        assert_eq!(named(&home, &["src"]).category, Category::Code);
        // `tries` is agent scratch even inside code: it is what agents write.
        assert_eq!(
            named(&home, &["src", "tries"]).category,
            Category::AgentScratch
        );
        assert_eq!(
            named(&home, &["src", "tries", "2026-09-01"]).category,
            Category::AgentScratch,
            "an unknown name inherits"
        );
        assert_eq!(
            named(&home, &["src", "tries", "2026-09-01", "a"]).category,
            Category::AgentScratch,
            "files inherit too"
        );
    }

    #[test]
    fn an_unknown_top_level_directory_takes_its_largest_known_child() {
        let home = home();
        assert_eq!(named(&home, &["world"]).category, Category::Git);
        assert_eq!(named(&home, &["mystery"]).category, Category::Other);
        // A bare repository is git by its shape, whatever it is called.
        assert_eq!(named(&home, &["monorepo"]).category, Category::Git);
        assert_eq!(named(&home, &["monorepo", "git"]).category, Category::Git);
    }

    #[test]
    fn caches_are_reclaimable_all_the_way_down() {
        let home = home();
        assert_eq!(
            named(&home, &[".cache"]).reclaim,
            Some(Reclaim::Regenerable)
        );
        assert_eq!(
            named(&home, &[".cache", "kache", "store"]).reclaim,
            Some(Reclaim::Regenerable)
        );
        assert_eq!(
            named(&home, &["Sync", ".stversions"]).reclaim,
            Some(Reclaim::SyncHistory)
        );
        assert_eq!(named(&home, &["src"]).reclaim, None);
    }

    #[test]
    fn target_is_build_output_only_beside_a_cargo_manifest() {
        let home = home();
        assert_eq!(
            named(&home, &["rust-thing", "target"]).reclaim,
            Some(Reclaim::BuildOutput)
        );
        assert_eq!(named(&home, &["js-thing", "target"]).reclaim, None);
    }

    #[test]
    fn sandbox_layers_and_snapshots_are_reclaimable_only_in_sandbox_state() {
        let home = home();
        // .microsandbox/cache is already a regenerable cache, so its layers
        // inherit that; the snapshots are judged on their own.
        assert!(
            named(&home, &[".microsandbox", "cache", "layers"])
                .reclaim
                .is_some()
        );
        assert_eq!(
            named(&home, &[".microsandbox", "snapshots"]).reclaim,
            Some(Reclaim::Snapshots)
        );
        assert_eq!(
            reclaim_of("snapshots", Category::Documents, |_| false),
            None
        );
    }

    #[test]
    fn macos_developer_leftovers_are_reclaimable_and_its_risks_are_not() {
        let none = |_: &str| false;
        assert_eq!(
            reclaim_of("DerivedData", Category::Toolchain, none),
            Some(Reclaim::BuildOutput)
        );
        assert_eq!(
            reclaim_of("iOS DeviceSupport", Category::Toolchain, none),
            Some(Reclaim::Regenerable)
        );
        let library = |name: &str| name == "Application Support";
        assert_eq!(
            reclaim_of("Logs", Category::Other, library),
            Some(Reclaim::Temporary)
        );
        assert_eq!(reclaim_of("logs", Category::Code, none), None);
        // Big, but not safe to offer: Archives hold the symbols crash
        // reports need, and Backup is an iPhone's only backup.
        assert_eq!(reclaim_of("Archives", Category::Toolchain, none), None);
        assert_eq!(reclaim_of("Backup", Category::Other, none), None);
        assert_eq!(
            category_of_name("Mobile Documents"),
            Some(Category::Synced)
        );
        // ~/Developer holds a user's projects, not a toolchain.
        assert_eq!(category_of_name("Developer"), None);
        assert_eq!(category_of_name("Xcode"), Some(Category::Toolchain));
    }

    #[test]
    fn windows_caches_and_sync_folders_are_recognized() {
        let none = |_: &str| false;
        for (name, reclaim) in [
            ("Temp", Reclaim::Temporary),
            ("CrashDumps", Reclaim::Temporary),
            ("npm-cache", Reclaim::Regenerable),
            ("v3-cache", Reclaim::Regenerable),
            ("INetCache", Reclaim::Regenerable),
            ("D3DSCache", Reclaim::Regenerable),
            ("DXCache", Reclaim::Regenerable),
            ("$Recycle.Bin", Reclaim::Trash),
        ] {
            assert_eq!(
                reclaim_of(name, Category::Other, none),
                Some(reclaim),
                "{name}"
            );
            assert_eq!(category_of_name(name), Some(Category::Cache), "{name}");
        }
        for name in [
            "OneDrive - Contoso",
            // Past the stack buffer for lowercasing.
            "OneDrive - Contoso Pharmaceuticals International",
            "Dropbox (Contoso)",
            "iCloudDrive",
        ] {
            assert_eq!(
                category_of_name(name),
                Some(Category::Synced),
                "{name}"
            );
        }
        assert_eq!(category_of_name(".nuget"), Some(Category::Toolchain));
        assert_eq!(category_of_name("OneDriveSetup"), None);
    }

    #[test]
    fn the_legend_lists_every_named_category_once() {
        let mut seen = std::collections::HashSet::new();
        for category in Category::LEGEND {
            assert!(seen.insert(category));
            assert_ne!(category, Category::Other);
            assert!(!category.label().is_empty());
        }
    }

    #[test]
    fn an_extension_is_the_part_after_the_last_dot() {
        assert_eq!(file_extension("notes").as_deref(), None);
        assert_eq!(file_extension("notes.").as_deref(), None);
        // A dotfile's whole name is the "extension", which is not one.
        assert_eq!(file_extension(".gitignore").as_deref(), None);
        assert_eq!(file_extension("song.MP3").as_deref(), Some("mp3"));
        assert_eq!(file_extension(".config.json").as_deref(), Some("json"));
        assert_eq!(file_extension("archive.tar.gz").as_deref(), Some("gz"));
    }

    #[test]
    fn a_directory_is_coloured_by_its_heaviest_extension() {
        let mut root = dir(
            "music",
            vec![
                file("a.mp3", 20),
                file("b.MKV", 100),
                file("c.mkv", 5),
                dir("sub", vec![file("d.mp3", 1)]),
                file("README", 50),
            ],
        );
        aggregate(&mut root, Metric::Bytes);
        // Case-insensitive: MKV and mkv pool, and beat the mp3s together.
        assert_eq!(dominant_extension(&root).as_deref(), Some("mkv"));

        // A tie is broken on the name, so it does not depend on map order.
        let mut tied = dir("t", vec![file("a.aaa", 10), file("b.bbb", 10)]);
        aggregate(&mut tied, Metric::Bytes);
        assert_eq!(dominant_extension(&tied).as_deref(), Some("aaa"));

        // Nothing with an extension: nothing to colour by.
        let mut plain = dir("p", vec![file("README", 1)]);
        aggregate(&mut plain, Metric::Bytes);
        assert_eq!(dominant_extension(&plain), None);
    }
}

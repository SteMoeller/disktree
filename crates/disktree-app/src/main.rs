//! `disktree`: find what is eating a volume, mark it, and remove it.
//!
//! The window opens on a treemap of the scanned root — the home directory
//! unless another path is given — with a breadcrumb bar, a selection line, and a
//! live free-space meter. Marking is non-destructive until the review screen
//! is confirmed.

// A window, not a console program: on Windows, opening it from Explorer or
// the Start menu should not bring a console window along. `main` attaches to
// the console of a terminal it was started from, so `--help` and errors still
// reach one. Ignored elsewhere.
#![windows_subsystem = "windows"]

mod app_menu;
mod appearance;
mod ext_colors;
mod git;
mod i18n;
mod marks;
mod palette;
mod state;
#[cfg(test)]
mod tests;
mod treemap_view;
mod ui;
mod views;
mod widgets;

use std::path::PathBuf;
use std::rc::Rc;
#[cfg(target_os = "macos")]
use std::{
    io::IsTerminal as _,
    os::unix::process::CommandExt as _,
    process::{Command, Stdio},
};

use anyhow::{Context as _, Result};
use disktree_core::scan::ScanOptions;
use gpui_kit::{AppContext as _, PlatformDisplay, WindowOptions, px, size};
use state::Disktree;

/// What the command line asked for.
#[derive(Debug)]
struct Args {
    root: PathBuf,
    options: ScanOptions,
    depth: u32,
}

const USAGE: &str = "\
disktree — a treemap of what is using your disk

usage: disktree [OPTIONS] [PATH]

arguments:
  PATH              directory to scan (default: the home directory)

The window opens on a treemap of the root, largest first. Space marks the
selected tile, Enter opens it, c reviews the marked list, ? lists every key.

options:
  -a, --apparent-size   measure apparent length instead of allocated blocks
  -l, --follow-links    follow symlinks
  -H, --no-hidden       skip dotfiles and dot-directories
  -D, --disk            scan the whole disk the home directory is on
  -X, --cross-filesystems
                        also measure other disks, network shares and pseudo
                        filesystems mounted below PATH (off by default)
  -d, --depth N         how many levels to draw at once (1-6, default 3)
      --metric files    rank by file count instead of bytes
  -h, --help            show this help
";

fn main() -> Result<()> {
    #[cfg(windows)]
    console::attach();
    let outcome = run();
    #[cfg(windows)]
    console::detach();
    outcome
}

fn run() -> Result<()> {
    // The interface language comes from files found at startup; see `i18n`.
    i18n::load();
    // File-type colours, likewise; see `ext_colors`.
    ext_colors::load();
    let args = parse_args(std::env::args_os().skip(1))?;

    // When the app executable is reached through the command-line symlink,
    // cmux sends SIGTERM to its foreground process group as AppKit takes
    // focus. Spawn once into a separate group before AppKit starts. Restrict
    // this to interactive cmux sessions so scripts retain normal foreground
    // lifetime; the marker prevents the child from spawning recursively.
    #[cfg(target_os = "macos")]
    if std::io::stdin().is_terminal()
        && std::env::var_os("CMUX_SURFACE_ID").is_some()
        && std::env::var_os("DISKTREE_CMUX_DETACHED").is_none()
    {
        Command::new(std::env::current_exe().context("find disktree")?)
            .args(std::env::args_os().skip(1))
            .env("DISKTREE_CMUX_DETACHED", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            // Zero makes the child the leader of a new process group.
            .process_group(0)
            .spawn()
            .context("start disktree")?;
        return Ok(());
    }

    let root = args.root.clone();
    let depth = args.depth;
    let title_root = root.clone();

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_omarchy::init(cx);
            app_menu::install(cx);
            let home = std::env::home_dir();
            let native_look = appearance::follows_system(home.as_deref());
            if native_look {
                appearance::apply(cx.window_appearance(), cx);
            }
            let options = args.options.clone();
            let root_for_app = root.clone();
            // Open centred in the work area of the monitor the app was
            // started on, rather than at a fixed global point that only ever
            // lands on the primary monitor.
            let wanted = size(px(1440.), px(900.));
            let work = start_display(cx).map_or_else(
                || {
                    gpui_kit::Bounds::new(
                        gpui_kit::point(px(0.), px(0.)),
                        wanted,
                    )
                },
                |display| display.visible_bounds(),
            );
            let window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(gpui_kit::WindowBounds::Windowed(
                            centered_in(work, wanted),
                        )),
                        titlebar: Some(gpui_kit::TitlebarOptions {
                            title: Some(
                                format!(
                                    "disktree · {}",
                                    marks::display_path(
                                        &title_root,
                                        home.as_deref(),
                                    )
                                )
                                .into(),
                            ),
                            ..Default::default()
                        }),
                        // Wayland app id. Hyprland reports it as the window
                        // class, and the desktop entry's StartupWMClass and
                        // the documented window rule both match `disktree`.
                        // Left unset, the class is empty and that rule never
                        // matches.
                        app_id: Some("disktree".to_owned()),
                        // Below this the treemap stops being readable, so ask
                        // the compositor not to go there.
                        window_min_size: Some(size(px(900.), px(600.))),
                        ..Default::default()
                    },
                    move |window, cx| {
                        if native_look {
                            appearance::follow(window);
                        }
                        cx.new(|cx| {
                            Disktree::new(
                                root_for_app.clone(),
                                options.clone(),
                                depth,
                                cx,
                            )
                        })
                    },
                )
                .expect("open the disktree window");

            // The treemap owns the keyboard from the first frame; there is no
            // text field to focus first.
            let _ = window.update(cx, |this, window, cx| {
                let focus = this.focus.clone();
                window.focus(&focus, cx);
            });
            cx.activate(true);
        });
    Ok(())
}

/// Where to place a window of `wanted` size so it sits centred in `work`, a
/// monitor's work area: the screen minus its taskbar, dock or menu bar.
///
/// Shrunk to the work area when the window would not fit, so it is never put
/// partly off the monitor; the window's own minimum size still applies on top
/// of that. The area's offset is kept, so a monitor to the right of or below
/// the primary centres on its own origin.
fn centered_in(
    work: gpui_kit::Bounds<gpui_kit::Pixels>,
    wanted: gpui_kit::Size<gpui_kit::Pixels>,
) -> gpui_kit::Bounds<gpui_kit::Pixels> {
    let width = if wanted.width < work.size.width {
        wanted.width
    } else {
        work.size.width
    };
    let height = if wanted.height < work.size.height {
        wanted.height
    } else {
        work.size.height
    };
    gpui_kit::Bounds::new(
        gpui_kit::point(
            work.origin.x + (work.size.width - width) * 0.5,
            work.origin.y + (work.size.height - height) * 0.5,
        ),
        size(width, height),
    )
}

/// The monitor the app was started on: the one under the pointer, so the
/// window opens where the user is; the primary monitor when the platform
/// cannot say.
#[cfg(not(windows))]
fn start_display(cx: &gpui_kit::App) -> Option<Rc<dyn PlatformDisplay>> {
    cx.primary_display()
}

/// Windows: the cursor's monitor through Win32, matched to a GPUI display.
/// A `DisplayId` holds the platform's `HMONITOR` there, which is exactly
/// what `MonitorFromPoint` returns.
#[cfg(windows)]
#[allow(
    unsafe_code,
    reason = "two Win32 calls: one fills a point, one reads it"
)]
fn start_display(cx: &gpui_kit::App) -> Option<Rc<dyn PlatformDisplay>> {
    use gpui_kit::DisplayId;
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        MONITOR_DEFAULTTONEAREST, MonitorFromPoint,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut cursor = POINT { x: 0, y: 0 };
    // SAFETY: `cursor` is ours and the call only writes into it.
    if unsafe { GetCursorPos(&raw mut cursor) } == 0 {
        return cx.primary_display();
    }
    // SAFETY: takes the point by value and returns an opaque handle.
    let monitor = unsafe { MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return cx.primary_display();
    }
    cx.find_display(DisplayId::new(monitor as u64))
        .or_else(|| cx.primary_display())
}

/// Read the command line, program name already skipped.
fn parse_args(
    mut args: impl Iterator<Item = std::ffi::OsString>,
) -> Result<Args> {
    let mut root: Option<PathBuf> = None;
    let mut options = ScanOptions::default();
    let mut depth = 3_u32;
    let mut disk = false;
    // `std::env::args` panics on a name that is not Unicode, and a path is
    // any name: a restart as administrator hands the root back exactly as
    // it was, so the caller passes `args_os`.
    let text = |value: Option<std::ffi::OsString>, need: &str| {
        value
            .and_then(|value| value.into_string().ok())
            .with_context(|| need.to_owned())
    };

    while let Some(arg) = args.next() {
        match arg.to_str().unwrap_or_default() {
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "-a" | "--apparent-size" => options.apparent_size = true,
            "-l" | "--follow-links" => options.follow_links = true,
            "-H" | "--no-hidden" => options.include_hidden = false,
            // Staying on one volume is the default; the flag is kept so
            // old invocations still work.
            "-x" | "--one-filesystem" => options.one_filesystem = true,
            "-X" | "--cross-filesystems" => options.one_filesystem = false,
            "-D" | "--disk" => disk = true,
            "-d" | "--depth" => {
                let value = text(args.next(), "--depth needs a number")?;
                depth = value.parse().context("--depth needs a number")?;
                anyhow::ensure!(
                    (1..=6).contains(&depth),
                    "--depth must be 1 to 6"
                );
            }
            "--metric" => {
                let value = text(args.next(), "--metric needs a value")?;
                options.metric = match value.as_str() {
                    "files" => disktree_core::tree::Metric::Files,
                    "bytes" | "size" => disktree_core::tree::Metric::Bytes,
                    other => anyhow::bail!(
                        "unknown metric {other}; try bytes or files"
                    ),
                };
            }
            // Launch Services added a process serial number when opening an
            // app from Finder until OS X 10.9, and some launchers still do.
            other if other.starts_with("-psn_") => {}
            other if other.starts_with('-') => {
                anyhow::bail!("unknown option {other}\n\n{USAGE}");
            }
            _ => {
                anyhow::ensure!(root.is_none(), "only one path can be scanned");
                root = Some(PathBuf::from(arg));
            }
        }
    }

    anyhow::ensure!(
        !(disk && root.is_some()),
        "--disk and a PATH cannot be combined"
    );
    let home = std::env::home_dir();
    let root = match root {
        _ if disk => home
            .as_deref()
            .and_then(disktree_core::space::volume_root_for)
            .unwrap_or_else(|| PathBuf::from("/")),
        Some(root) => root,
        None => home.context("no path given and no home directory")?,
    };
    // Store the depth as the initial view setting rather than a scan option: it
    // is a display choice the run-time `[` and `]` keys also change.
    // Canonical, so a later widening recognises this tree in the wider walk;
    // through dunce, so Windows gets `C:\Users\…` rather than the `\\?\C:\…`
    // form nothing else is written in.
    let root = dunce::canonicalize(&root).unwrap_or(root);
    let metadata = std::fs::metadata(&root)
        .with_context(|| format!("cannot read {}", root.display()))?;
    anyhow::ensure!(metadata.is_dir(), "{} is not a directory", root.display());

    Ok(Args {
        root,
        options,
        depth: depth.clamp(1, 6),
    })
}

/// The console of the terminal disktree was started from, if any: a
/// windowed program on Windows gets none of its own.
#[cfg(windows)]
mod console {
    #![allow(
        unsafe_code,
        reason = "two Win32 calls that take no pointers to get wrong"
    )]

    use windows_sys::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, FreeConsole,
    };

    /// Borrow the parent's console so printed text reaches it. Does
    /// nothing when started from Explorer, which has none.
    pub fn attach() {
        // SAFETY: takes a process id by value, and failure only means
        // there was no console to attach to.
        unsafe {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }

    /// Let go of it again, so the shell redraws its prompt.
    pub fn detach() {
        // SAFETY: no arguments; a process without a console is left as is.
        unsafe {
            FreeConsole();
        }
    }
}

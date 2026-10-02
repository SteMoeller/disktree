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
#[cfg(target_os = "macos")]
use std::{
    io::IsTerminal as _,
    os::unix::process::CommandExt as _,
    process::{Command, Stdio},
};

use anyhow::{Context as _, Result};
use disktree_core::scan::ScanOptions;
use gpui_kit::{AppContext as _, WindowOptions, px, size};
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
            let work = start_work_area()
                .or_else(|| {
                    cx.primary_display().map(|display| display.visible_bounds())
                })
                .unwrap_or_else(|| {
                    gpui_kit::Bounds::new(
                        gpui_kit::point(px(0.), px(0.)),
                        wanted,
                    )
                });
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

/// The work area — the screen minus its taskbar — of the monitor disktree
/// was started on, in logical pixels.
///
/// The launcher is what says where the user is: the console window the
/// process was started from, when it has a visible one (Windows Terminal's
/// is hidden, and reporting it would point at the wrong screen), else the
/// window in front, else the pointer. `None` when none of them can be
/// resolved, so the caller falls back to the primary monitor.
///
/// The rectangle and the DPI come straight from Win32, then divided the way
/// gpui divides a display's, so the numbers are gpui's logical pixels without
/// having to find the same display again by id.
#[cfg(windows)]
#[allow(
    unsafe_code,
    reason = "a handful of Win32 calls that only write into locals"
)]
fn start_work_area() -> Option<gpui_kit::Bounds<gpui_kit::Pixels>> {
    use gpui_kit::{Bounds, point, px, size};
    use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO,
        MonitorFromPoint, MonitorFromWindow,
    };
    use windows_sys::Win32::System::Console::GetConsoleWindow;
    use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetForegroundWindow, IsWindowVisible,
    };

    // SAFETY: every call takes either nothing or an opaque handle Windows
    // itself handed us, and each out pointer aims at a live local.
    let monitor = unsafe {
        let console: HWND = GetConsoleWindow();
        let reference = if !console.is_null() && IsWindowVisible(console) != 0 {
            Some(console)
        } else {
            let foreground = GetForegroundWindow();
            (!foreground.is_null()).then_some(foreground)
        };
        if let Some(window) = reference {
            MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST)
        } else {
            let mut cursor = POINT { x: 0, y: 0 };
            if GetCursorPos(&raw mut cursor) == 0 {
                return None;
            }
            MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST)
        }
    };
    if monitor.is_null() {
        return None;
    }

    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = u32::try_from(std::mem::size_of::<MONITORINFO>()).ok()?;
    // SAFETY: `monitor` is live, and `info` is a live, writable MONITORINFO
    // with the size the API asks to be set first.
    if unsafe { GetMonitorInfoW(monitor, &raw mut info) } == 0 {
        return None;
    }

    let (mut dpi_x, mut dpi_y) = (0_u32, 0_u32);
    // SAFETY: `monitor` is live, the constant is the documented one, and both
    // out pointers aim at live u32s.
    let dpi = unsafe {
        GetDpiForMonitor(
            monitor,
            MDT_EFFECTIVE_DPI,
            &raw mut dpi_x,
            &raw mut dpi_y,
        )
    };
    // The same source gpui takes a display's scale factor from, so these are
    // the logical pixels it expects; a failure falls back to 1:1.
    let scale = if dpi == 0 && dpi_x > 0 {
        dpi_x as f32 / 96.0
    } else {
        1.0
    };

    let RECT {
        left,
        top,
        right,
        bottom,
    } = info.rcWork;
    let (left, top, right, bottom) =
        (left as f32, top as f32, right as f32, bottom as f32);
    Some(Bounds::new(
        point(px(left / scale), px(top / scale)),
        size(px((right - left) / scale), px((bottom - top) / scale)),
    ))
}

/// Everything but Windows leaves placement to the platform, which centres
/// the window on the primary display itself.
#[cfg(not(windows))]
fn start_work_area() -> Option<gpui_kit::Bounds<gpui_kit::Pixels>> {
    None
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

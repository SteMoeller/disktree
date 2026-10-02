//! One scan of `ROOT` with a fixed pool of `WORKERS`, the way a Power
//! Efficiency preset runs it, reported as one JSON line. Driven by
//! `scripts/bench-scan-workers.py`, which times it and samples the machine.
//!
//! ```sh
//! cargo run --release -p disktree-core --example scan_workers -- ROOT WORKERS
//! ```

use std::path::PathBuf;
use std::time::{Duration, Instant};

use disktree_core::scan::{ScanHandle, ScanOptions};
use disktree_core::scan_threads::ScanThreads;

fn main() {
    let mut args = std::env::args_os().skip(1);
    let (Some(root), Some(workers)) = (args.next(), args.next()) else {
        eprintln!("usage: scan_workers ROOT WORKERS");
        std::process::exit(2);
    };
    let Some(workers) = workers
        .to_str()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|&value| value > 0)
    else {
        eprintln!("WORKERS must be a positive integer");
        std::process::exit(2);
    };
    let options = ScanOptions {
        // What `PowerEfficiency::policy` sets: a fixed pool, no governor.
        threads: ScanThreads {
            max_threads: workers,
            adaptive: false,
            system_cpu_limit: None,
            ..ScanThreads::default()
        },
        ..ScanOptions::default()
    };
    let start = Instant::now();
    let handle = ScanHandle::spawn(PathBuf::from(root), options);
    let node = loop {
        if let Some(result) = handle.poll() {
            match result {
                Ok(node) => break node,
                Err(error) => {
                    eprintln!("scan failed: {error}");
                    std::process::exit(1);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let elapsed = start.elapsed().as_secs_f64();
    let progress = handle.progress.snapshot();
    println!(
        "{{\"workers\":{workers},\"pool\":{},\"elapsed\":{elapsed},\
         \"files\":{},\"dirs\":{},\"bytes\":{},\"errors\":{}}}",
        progress.threads, node.files, node.dirs, node.bytes, progress.errors
    );
}

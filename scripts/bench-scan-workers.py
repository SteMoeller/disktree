#!/usr/bin/env python3
"""Time whole scans at several fixed worker counts and print the PR table.

Each run is one `scan_workers` example process under `/usr/bin/time`, so CPU
time is the scan's own user + system seconds. Whole-machine busy time is
sampled every half second while it runs (macOS `host_statistics`, Linux
`/proc/stat`). Rounds rotate the order so drift in the machine or the cache
does not favour one count. The first scan is an untimed warm-up.

    scripts/bench-scan-workers.py /System/Volumes/Data --workers 18 8 4 2

Hold the machine quiet: a VM, an indexer or a build skews every column.
"""

import argparse
import ctypes
import fcntl
import json
import os
import pathlib
import platform
import re
import statistics
import subprocess
import sys
import time

REPO = pathlib.Path(__file__).resolve().parent.parent
EXAMPLE = REPO / "target/release/examples/scan_workers"


def cpu_ticks():
    """(busy, total) ticks for the whole machine since boot."""
    if platform.system() == "Darwin":
        lib = ctypes.CDLL("/usr/lib/libSystem.B.dylib")
        lib.mach_host_self.restype = ctypes.c_uint
        ticks = (ctypes.c_uint * 4)()
        count = ctypes.c_uint(4)
        # HOST_CPU_LOAD_INFO: user, system, idle, nice.
        if lib.host_statistics(lib.mach_host_self(), 3, ticks, ctypes.byref(count)):
            raise OSError("host_statistics failed")
        return sum(ticks) - ticks[2], sum(ticks)
    fields = [int(x) for x in open("/proc/stat").readline().split()[1:]]
    idle = fields[3] + fields[4]
    return sum(fields) - idle, sum(fields)


def percent(value):
    return "n/a" if value is None else f"{value:.0f}%"


def busy_percent(before, after):
    busy, total = (a - b for a, b in zip(after, before))
    return 100 * busy / total if total else None


def cpu_seconds(time_output):
    """User + system seconds from BSD or GNU `/usr/bin/time` output."""
    user = re.search(r"([\d.]+) user|User time \(seconds\): ([\d.]+)", time_output)
    system = re.search(r"([\d.]+) sys|System time \(seconds\): ([\d.]+)", time_output)
    return sum(float(next(g for g in m.groups() if g)) for m in (user, system))


def scan(root, workers):
    flag = "-l" if platform.system() == "Darwin" else "-v"
    before = cpu_ticks()
    started = time.monotonic()
    process = subprocess.Popen(
        ["/usr/bin/time", flag, str(EXAMPLE), root, str(workers)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        # Its own group, so an interrupt never leaves a scan running.
        start_new_session=True,
    )
    try:
        out, err = process.communicate()
    except BaseException:
        os.killpg(process.pid, 9)
        raise
    if process.returncode:
        sys.exit(f"scan failed:\n{err}")
    run = json.loads(out.strip().splitlines()[-1])
    run["wall"] = time.monotonic() - started
    run["cpu_s"] = cpu_seconds(err)
    run["machine"] = busy_percent(before, cpu_ticks())
    return run


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("root", help="directory or volume to scan")
    parser.add_argument("--workers", type=int, nargs="+", default=[18, 8, 4, 2])
    parser.add_argument("--rounds", type=int, default=3)
    parser.add_argument("--out", type=pathlib.Path, help="write every run as JSON here")
    parser.add_argument("--lock", help="flock this file for the whole block")
    args = parser.parse_args()

    subprocess.run(
        ["cargo", "build", "--release", "-p", "disktree-core", "--example", "scan_workers"],
        cwd=REPO,
        check=True,
    )
    lock = open(args.lock, "w") if args.lock else None
    if lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
    cores = os.cpu_count() or 1
    base = max(args.workers)
    print(f"warm-up: {base} workers", file=sys.stderr)
    scan(args.root, base)
    runs = []
    for round_ in range(args.rounds):
        shift = round_ % len(args.workers)
        order = args.workers[shift:] + args.workers[:shift]
        if round_ % 2:
            order.reverse()
        for workers in order:
            run = scan(args.root, workers)
            run["round"] = round_ + 1
            runs.append(run)
            print(
                f"round {round_ + 1}, {workers} workers: {run['elapsed']:.1f} s, "
                f"{run['cpu_s']:.0f} CPU-s, machine {percent(run['machine'])}",
                file=sys.stderr,
            )
    if args.out:
        args.out.write_text(json.dumps(runs, indent=1))

    def median(workers, key):
        values = [r[key] for r in runs if r["workers"] == workers and r[key] is not None]
        return statistics.median(values) if values else None

    # The scan's own share of every core, which background load cannot move.
    for run in runs:
        run["share"] = 100 * run["cpu_s"] / (run["elapsed"] * cores)
    b_time, b_cpu, b_share = (median(base, k) for k in ("elapsed", "cpu_s", "share"))
    print(f"\nMedian of {args.rounds}; Δ against {base} workers; {cores} cores.\n")
    print("| Workers | Scan time | Δ time | CPU time | Δ CPU time | Machine used | Δ machine | Whole machine |")
    print("| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
    for workers in sorted(args.workers, reverse=True):
        t, c, s, m = (median(workers, k) for k in ("elapsed", "cpu_s", "share", "machine"))
        first = workers == base
        pct = lambda x, y: "baseline" if first else f"{(x - y) / y * 100:+.0f}%"
        pts = "baseline" if first else f"{s - b_share:+.0f} pts"
        print(
            f"| {workers} | {t:.0f} s | {pct(t, b_time)} | {c:,.0f} CPU-s | {pct(c, b_cpu)} "
            f"| {s:.0f}% | {pts} | {percent(m)} |"
        )


if __name__ == "__main__":
    main()

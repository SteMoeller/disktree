//! Scan-local admission: queued directories never occupy Rayon workers.

use std::sync::{Mutex, MutexGuard, PoisonError};

/// Filesystem concurrency for one scan, independent of the global Rayon pool.
/// A scan crossing several volumes shares this budget; separate scans do not.
#[derive(Clone, Copy, Debug)]
pub struct ScanThreads {
    /// Fixed scan workers, capped by available CPUs; `usize::MAX` uses the
    /// shared platform-default pool. With `adaptive`, maximum admitted directory
    /// jobs instead, including jobs awaiting a Rayon worker.
    pub max_threads: usize,
    /// Search below the cap and respond to whole-system CPU pressure.
    pub adaptive: bool,
    /// Fraction of initial entries/second a candidate must retain.
    pub retained_throughput: f64,
    /// Best-effort whole-system CPU ceiling; `None` disables the governor.
    pub system_cpu_limit: Option<f64>,
}

impl Default for ScanThreads {
    fn default() -> Self {
        Self {
            // Preserve core callers' pool-sized fixed concurrency. The app
            // supplies the user's Power Efficiency preset before scanning.
            max_threads: usize::MAX,
            // Adaptive admission remains an explicit experimental opt-in.
            adaptive: false,
            retained_throughput: 0.80,
            system_cpu_limit: Some(0.80),
        }
    }
}

impl ScanThreads {
    pub(crate) fn normalized(self) -> Self {
        Self {
            max_threads: self.max_threads.max(1),
            retained_throughput: if (0.0..=1.0)
                .contains(&self.retained_throughput)
            {
                self.retained_throughput
            } else {
                0.80
            },
            system_cpu_limit: self.system_cpu_limit.filter(|limit| {
                limit.is_finite() && *limit > 0.0 && *limit <= 1.0
            }),
            ..self
        }
    }
}

pub(crate) struct Admission<T> {
    state: Mutex<Queue<T>>,
}

struct Queue<T> {
    pending: Vec<T>,
    active: usize,
    limit: usize,
    epoch: u64,
    starved: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Sample {
    pub(crate) active: usize,
    pub(crate) limit: usize,
    pub(crate) epoch: u64,
    pub(crate) starved: bool,
}

impl Sample {
    pub(crate) const fn settled(self) -> bool {
        self.active <= self.limit
    }
}

impl<T> Admission<T> {
    pub(crate) fn new(root: T, limit: usize) -> Self {
        Self {
            state: Mutex::new(Queue {
                pending: vec![root],
                active: 0,
                limit: limit.max(1),
                epoch: 0,
                starved: false,
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Queue<T>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn start(&self) -> Vec<T> {
        Self::admit(&mut self.lock())
    }

    /// Each submitted job hands back its slot exactly once, after its own
    /// parent-completion signal. Its children already hold pending tokens.
    pub(crate) fn complete(&self, children: Vec<T>) -> Vec<T> {
        let mut state = self.lock();
        state.active -= 1;
        state.pending.extend(children);
        let jobs = Self::admit(&mut state);
        drop(state);
        jobs
    }

    fn admit(state: &mut Queue<T>) -> Vec<T> {
        let count = state
            .limit
            .saturating_sub(state.active)
            .min(state.pending.len());
        let split = state.pending.len() - count;
        let jobs = state.pending.split_off(split);
        state.active += jobs.len();
        state.starved |= state.active < state.limit;
        jobs
    }

    /// The same lock guards dispatch and changes, so a settled snapshot
    /// cannot precede a late dispatch made using the old limit.
    pub(crate) fn set_limit(&self, limit: usize) {
        let mut state = self.lock();
        let limit = limit.max(1);
        if state.limit != limit {
            state.limit = limit;
            state.epoch += 1;
        }
    }

    pub(crate) fn sample(&self) -> Sample {
        let mut state = self.lock();
        Sample {
            active: state.active,
            limit: state.limit,
            epoch: state.epoch,
            // A long directory can remain undersupplied for many samples.
            // Clearing the history must not erase the current condition,
            // including an increase awaiting the next dispatch boundary.
            starved: std::mem::take(&mut state.starved)
                || state.active < state.limit,
        }
    }
}

/// One reference, then a bounded accepted/rejected bracket. This targets a
/// sampled rate, not a guarantee about an evolving tree's completion time.
pub(crate) struct Search {
    reference: Option<f64>,
    accepted: usize,
    rejected: usize,
    candidate: usize,
    passes: u8,
    failures: u8,
    windows: u8,
    retained: f64,
    pub(crate) done: bool,
}

impl Search {
    pub(crate) const fn new(initial: usize, retained: f64) -> Self {
        Self {
            reference: None,
            accepted: initial,
            rejected: 0,
            candidate: initial,
            passes: 0,
            failures: 0,
            windows: 0,
            retained,
            done: initial <= 1,
        }
    }

    pub(crate) fn sample(&mut self, rate: Option<f64>) -> usize {
        if self.done {
            return self.accepted;
        }
        let Some(reference) = self.reference else {
            if let Some(rate) = rate.filter(|rate| *rate > 0.0) {
                self.reference = Some(rate);
                self.candidate = (self.accepted / 2).max(1);
                self.windows = 0;
            } else {
                self.windows += 1;
                self.done = self.windows == 3;
            }
            return self.candidate;
        };
        self.windows += 1;
        if let Some(rate) = rate {
            if rate >= reference * self.retained {
                self.passes += 1;
            } else {
                self.failures += 1;
            }
        }
        if self.passes == 2 || self.failures == 2 || self.windows == 3 {
            if self.passes == 2 {
                self.accepted = self.candidate;
            } else {
                self.rejected = self.candidate;
            }
            self.done = self.accepted - self.rejected <= 1;
            self.candidate = if self.done {
                self.accepted
            } else {
                self.rejected + (self.accepted - self.rejected) / 2
            };
            self.passes = 0;
            self.failures = 0;
            self.windows = 0;
        }
        self.candidate
    }
}

pub(crate) struct Governor {
    ceiling: Option<f64>,
    cap: usize,
    high: u8,
    low: u8,
    pub(crate) pressured: bool,
}

impl Governor {
    pub(crate) const fn new(cap: usize, ceiling: Option<f64>) -> Self {
        Self {
            ceiling,
            cap,
            high: 0,
            low: 0,
            pressured: false,
        }
    }

    pub(crate) fn admitted(&self, desired: usize) -> usize {
        if self.ceiling.is_none() {
            desired
        } else {
            desired.min(self.cap)
        }
    }

    pub(crate) fn sample(
        &mut self,
        usage: Option<f64>,
        admitted: usize,
        desired: usize,
        settled: bool,
    ) -> usize {
        let Some(ceiling) = self.ceiling else {
            return desired;
        };
        if let Some(usage) = usage {
            self.pressured = usage > ceiling;
            if self.pressured {
                self.low = 0;
                self.high = self.high.saturating_add(1).min(2);
                if self.high >= 2 && settled {
                    self.cap = (admitted / 2).max(1);
                    self.high = 0;
                }
            } else if usage < (ceiling - 0.05).max(0.0) {
                self.high = 0;
                self.low = self.low.saturating_add(1).min(4);
                if self.low >= 4 && settled {
                    self.cap = (self.cap + 1).min(desired.max(self.cap));
                    self.low = 0;
                }
            } else {
                self.high = 0;
                self.low = 0;
            }
        } else {
            // Missing measurements cannot establish spare capacity.
            self.high = 0;
            self.low = 0;
        }
        desired.min(self.cap)
    }
}

use std::io;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::scan::ScanProgress;

pub(crate) struct ControllerGuard {
    stop: Arc<(Mutex<bool>, Condvar)>,
    worker: Option<JoinHandle<()>>,
}

impl ControllerGuard {
    pub(crate) fn spawn<T: Send + 'static>(
        options: ScanThreads,
        admission: Arc<Admission<T>>,
        progress: Arc<ScanProgress>,
    ) -> io::Result<Option<Self>> {
        if !options.adaptive {
            return Ok(None);
        }
        let stop = Arc::new((Mutex::new(false), Condvar::new()));
        let wake = Arc::clone(&stop);
        let worker = thread::Builder::new()
            .name("disktree-admission".into())
            .spawn(move || {
            control(options, &admission, &progress, &wake);
        })?;
        Ok(Some(Self {
            stop,
            worker: Some(worker),
        }))
    }
}

impl Drop for ControllerGuard {
    fn drop(&mut self) {
        *self.stop.0.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.stop.1.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn control<T>(
    options: ScanThreads,
    admission: &Admission<T>,
    progress: &ScanProgress,
    stop: &(Mutex<bool>, Condvar),
) {
    let mut cpu = options.system_cpu_limit.map(|_| CpuSampler::new());
    control_with(
        options,
        admission,
        progress,
        stop,
        Duration::from_millis(250),
        || cpu.as_mut().and_then(CpuSampler::sample),
    );
}

fn control_with<T>(
    options: ScanThreads,
    admission: &Admission<T>,
    progress: &ScanProgress,
    stop: &(Mutex<bool>, Condvar),
    interval: Duration,
    mut sample_cpu: impl FnMut() -> Option<f64>,
) {
    // Two 250 ms votes distinguish a candidate from a transient burst. CPU
    // counters are sampled less often because they have coarser resolution.
    let mut search =
        Search::new(options.max_threads, options.retained_throughput);
    let mut governor =
        Governor::new(options.max_threads, options.system_cpu_limit);
    let mut desired = options.max_threads;
    let mut cpu_at = Instant::now();
    let mut start = Instant::now();
    let mut entries = progress.entries();
    let mut epoch = 0;
    let mut measuring = false;
    loop {
        let (stopped, _) = stop
            .1
            .wait_timeout_while(
                stop.0.lock().unwrap_or_else(PoisonError::into_inner),
                interval,
                |stopped| !*stopped,
            )
            .unwrap_or_else(PoisonError::into_inner);
        if *stopped || progress.is_cancelled() {
            return;
        }
        drop(stopped);
        let sample = admission.sample();
        let mut limit = sample.limit;
        if cpu_at.elapsed() >= interval * 2 {
            limit = governor.sample(
                sample_cpu(),
                sample.limit,
                desired,
                sample.settled(),
            );
            cpu_at = Instant::now();
        }
        let limited = governor.pressured || limit < desired;
        progress
            .system_cpu_limited
            .store(limited, Ordering::Relaxed);
        progress
            .threads_settled
            .store(sample.settled(), Ordering::Relaxed);
        let current = progress.entries();
        let now = Instant::now();
        if limited || !sample.settled() || epoch != sample.epoch || !measuring {
            measuring = !limited && sample.settled();
        } else if !search.done {
            let rate = (!sample.starved && current > entries).then(|| {
                (current - entries) as f64
                    / now.duration_since(start).as_secs_f64()
            });
            desired = search.sample(rate);
            limit = governor.admitted(desired);
        }
        if limit != sample.limit {
            admission.set_limit(limit);
            progress.threads.store(limit, Ordering::Relaxed);
            progress.worker_transitions.fetch_add(1, Ordering::Relaxed);
            progress.threads_settled.store(false, Ordering::Relaxed);
            measuring = false;
        }
        progress
            .thread_tuning_complete
            .store(search.done, Ordering::Relaxed);
        entries = current;
        start = now;
        epoch = sample.epoch;
    }
}

struct CpuSampler {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    system: sysinfo::System,
}

#[cfg_attr(
    not(any(target_os = "macos", target_os = "linux")),
    allow(
        clippy::missing_const_for_fn,
        clippy::unused_self,
        reason = "The fallback mirrors the stateful, non-const native API."
    )
)]
impl CpuSampler {
    fn new() -> Self {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let mut system = sysinfo::System::new();
            system.refresh_cpu_usage();
            Self { system }
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        Self {}
    }

    fn sample(&mut self) -> Option<f64> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            self.system.refresh_cpu_usage();
            let usage = f64::from(self.system.global_cpu_usage()) / 100.0;
            // sysinfo reports zero on some native read failures. Conservatively
            // ignore zero too: it must not be evidence for restoring workers.
            (!self.system.cpus().is_empty() && usage > 0.0 && usage <= 1.0)
                .then_some(usage)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn search_refines_one_reference_without_compounding_loss() {
        let mut search = Search::new(8, 0.8);
        assert_eq!(search.sample(Some(100.0)), 4);
        assert_eq!(search.sample(Some(85.0)), 4);
        assert_eq!(search.sample(Some(85.0)), 2);
        assert_eq!(search.sample(Some(70.0)), 2);
        assert_eq!(search.sample(Some(70.0)), 3);
        assert_eq!(search.sample(Some(81.0)), 3);
        assert_eq!(search.sample(Some(81.0)), 3);
        assert!(search.done);
    }

    #[test]
    fn long_directory_never_becomes_a_fully_supplied_window() {
        let admission = Admission::new(0, 8);
        assert_eq!(admission.start(), vec![0]);
        let mut search = Search::new(8, 0.8);
        for _ in 0..6 {
            let sample = admission.sample();
            assert!(sample.settled());
            assert!(sample.starved);
            assert_eq!(search.sample((!sample.starved).then_some(100.0)), 8);
        }
        assert!(search.reference.is_none());
        assert!(search.done);
    }

    #[test]
    fn increased_limit_is_not_supplied_until_jobs_are_dispatched() {
        let admission = Admission::new(0, 1);
        assert_eq!(admission.start(), vec![0]);
        assert_eq!(admission.complete((1..=8).collect()).len(), 1);
        admission.set_limit(8);
        let mut search = Search::new(16, 0.8);
        assert_eq!(search.sample(Some(100.0)), 8);
        for _ in 0..3 {
            let sample = admission.sample();
            assert!(sample.settled());
            assert!(sample.starved);
            search.sample((!sample.starved).then_some(100.0));
        }
        // Even an apparently fast candidate cannot be accepted while only
        // one job is admitted and the other ready jobs remain queued.
        assert_eq!(search.accepted, 16);
        assert_eq!(search.rejected, 8);
        assert_eq!(admission.complete(vec![9]).len(), 8);
        assert!(!admission.sample().starved);
    }

    #[test]
    fn inconclusive_windows_cannot_accept_a_reduction() {
        let mut search = Search::new(4, 0.8);
        assert_eq!(search.sample(Some(100.0)), 2);
        for _ in 0..2 {
            assert_eq!(search.sample(None), 2);
        }
        assert_eq!(search.sample(None), 3);
        assert_eq!(search.sample(Some(90.0)), 3);
        assert_eq!(search.sample(None), 3);
        assert_eq!(search.sample(None), 4);
        assert!(search.done);
    }

    #[test]
    fn governor_waits_for_retirement_and_recovers_without_exceeding_target() {
        let mut governor = Governor::new(8, Some(0.8));
        assert_eq!(governor.sample(Some(0.9), 8, 8, true), 8);
        assert_eq!(governor.sample(Some(0.9), 8, 8, true), 4);
        for _ in 0..300 {
            assert_eq!(governor.sample(Some(0.9), 4, 8, false), 4);
        }
        assert_eq!(governor.sample(Some(0.9), 4, 8, true), 2);
        assert_eq!(governor.sample(None, 2, 8, true), 2);
        for _ in 0..3 {
            assert_eq!(governor.sample(Some(0.5), 2, 8, true), 2);
        }
        assert_eq!(governor.sample(Some(0.5), 2, 8, true), 3);
        // A throughput decision does not manufacture a CPU-pressure cap.
        let mut governor = Governor::new(8, Some(0.8));
        for _ in 0..4 {
            assert_eq!(governor.sample(Some(0.5), 2, 2, true), 2);
        }
        assert_eq!(governor.admitted(8), 8);
    }

    fn wait_until(mut ready: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !ready() {
            assert!(Instant::now() < deadline, "controller timed out");
            thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn controller_retires_recovers_after_search_and_cancels() {
        let admission = Arc::new(Admission::new(0, 4));
        assert_eq!(admission.start(), vec![0]);
        assert_eq!(admission.complete(vec![1, 2, 3, 4]).len(), 4);
        let progress = Arc::new(ScanProgress::default());
        // 0 is missing, 1 is below the ceiling, 2 is above it.
        let cpu = Arc::new(std::sync::atomic::AtomicUsize::new(1));
        let controller_admission = Arc::clone(&admission);
        let controller_progress = Arc::clone(&progress);
        let controller_cpu = Arc::clone(&cpu);
        let controller = thread::spawn(move || {
            control_with(
                ScanThreads {
                    max_threads: 4,
                    adaptive: true,
                    ..ScanThreads::default()
                },
                &controller_admission,
                &controller_progress,
                &(Mutex::new(false), Condvar::new()),
                Duration::from_millis(2),
                || match controller_cpu.load(Ordering::Relaxed) {
                    1 => Some(0.5),
                    2 => Some(0.95),
                    _ => None,
                },
            );
        });
        // No entries gives an inconclusive bounded search. The governor must
        // remain active after that search completes.
        wait_until(|| progress.snapshot().thread_tuning_complete);
        cpu.store(2, Ordering::Relaxed);
        wait_until(|| admission.sample().limit == 2);
        thread::sleep(Duration::from_millis(30));
        assert_eq!(admission.sample().limit, 2);
        assert!(!progress.snapshot().threads_settled);
        assert!(admission.complete(Vec::new()).is_empty());
        assert!(admission.complete(Vec::new()).is_empty());
        wait_until(|| admission.sample().limit == 1);
        cpu.store(0, Ordering::Relaxed);
        assert!(admission.complete(Vec::new()).is_empty());
        thread::sleep(Duration::from_millis(30));
        assert_eq!(admission.sample().limit, 1);
        cpu.store(1, Ordering::Relaxed);
        wait_until(|| admission.sample().limit == 4);
        assert!(progress.snapshot().worker_transitions >= 5);
        progress.cancel();
        controller.join().expect("cancelled controller");
        assert!(admission.complete(Vec::new()).is_empty());
        assert_eq!(admission.sample().active, 0);
    }

    fn execute(
        scope: &rayon::Scope<'_>,
        jobs: Vec<usize>,
        admission: &Arc<Admission<usize>>,
        releases: &Arc<Mutex<Vec<Option<mpsc::Receiver<()>>>>>,
        started: &mpsc::Sender<usize>,
        completed: &mpsc::Sender<usize>,
    ) {
        for job in jobs {
            let admission = Arc::clone(admission);
            let releases = Arc::clone(releases);
            let started = started.clone();
            let completed = completed.clone();
            scope.spawn(move |scope| {
                let children = if job == 0 {
                    (1..=4).collect()
                } else if job <= 4 {
                    started.send(job).expect("started");
                    let release = releases.lock().expect("releases")[job]
                        .take()
                        .expect("one receiver");
                    release
                        .recv_timeout(Duration::from_secs(5))
                        .expect("release");
                    vec![job + 4]
                } else {
                    Vec::new()
                };
                let next = admission.complete(children);
                completed.send(job).expect("completed");
                execute(
                    scope, next, &admission, &releases, &started, &completed,
                );
            });
        }
    }

    #[test]
    fn real_pool_retires_at_job_boundaries_and_drains_queued_descendants() {
        let admission = Arc::new(Admission::new(0, 4));
        let (started, starts) = mpsc::channel();
        let (completed, completions) = mpsc::channel();
        let (done, finished) = mpsc::channel();
        let mut senders = Vec::new();
        let mut receivers = Vec::new();
        for _ in 0..=4 {
            let (send, receive) = mpsc::channel();
            senders.push(send);
            receivers.push(Some(receive));
        }
        let releases = Arc::new(Mutex::new(receivers));
        let worker_admission = Arc::clone(&admission);
        let worker = thread::spawn(move || {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(4)
                .build()
                .expect("pool");
            pool.scope(|scope| {
                execute(
                    scope,
                    worker_admission.start(),
                    &worker_admission,
                    &releases,
                    &started,
                    &completed,
                );
            });
            done.send(()).expect("done");
        });
        assert_eq!(
            completions
                .recv_timeout(Duration::from_secs(5))
                .expect("root"),
            0
        );
        let mut running = Vec::new();
        for _ in 0..4 {
            running.push(
                starts.recv_timeout(Duration::from_secs(5)).expect("start"),
            );
        }
        admission.set_limit(1);
        assert!(!admission.sample().settled());
        for (index, &job) in running[..3].iter().enumerate() {
            senders[job].send(()).expect("release");
            assert_eq!(
                completions
                    .recv_timeout(Duration::from_secs(5))
                    .expect("done"),
                job
            );
            let sample = admission.sample();
            assert_eq!(sample.active, 3 - index);
            assert_eq!(sample.settled(), index == 2);
        }
        // The last admitted job is still running; its siblings' descendants
        // must remain ready rather than occupying blocked Rayon workers.
        assert_eq!(admission.lock().pending.len(), 3);
        admission.set_limit(2);
        senders[running[3]].send(()).expect("last release");
        finished
            .recv_timeout(Duration::from_secs(5))
            .expect("scope drained");
        worker.join().expect("worker");
        let mut remaining: Vec<_> = completions.try_iter().collect();
        remaining.sort_unstable();
        let mut expected = vec![running[3], 5, 6, 7, 8];
        expected.sort_unstable();
        assert_eq!(remaining, expected);
        assert_eq!(admission.sample().active, 0);
    }
}

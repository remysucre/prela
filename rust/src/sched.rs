use std::cell::{Cell, RefCell, UnsafeCell};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

static BEAT: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
pub fn beat() -> u64 {
    BEAT.load(Ordering::Relaxed)
}

pub fn start_clock(interval: Duration) {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        std::thread::Builder::new()
            .name("prela-clock".into())
            .spawn(move || {
                let mut next = Instant::now() + interval;
                loop {
                    std::thread::sleep(next.saturating_duration_since(Instant::now()));
                    BEAT.fetch_add(1, Ordering::Relaxed);
                    next += interval;
                }
            })
            .unwrap();
    });
}

/// get heartbeat interval from env
fn beat_interval() -> Duration {
    let us = std::env::var("PRELA_BEAT_US")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);
    Duration::from_micros(us)
}

/// task states
const PENDING: u8 = 0;
const TAKEN: u8 = 1;
const DONE: u8 = 2;

/// try-runnable abstraction over tasks
trait Job: Send + Sync {
    fn try_run(&self) -> bool;
}

/// a unit of (potentially parallel) work
struct Task<'a, T> {
    state: AtomicU8,
    job: UnsafeCell<Option<Box<dyn FnOnce() -> T + Send + 'a>>>,
    out: UnsafeCell<Option<T>>,
}

/// safe bc Task is only touched by the (unique) thread that wins the claim
unsafe impl<T: Send> Sync for Task<'_, T> {}

impl<T: Send> Job for Task<'_, T> {
    /// try to claim task, and run it if successful. Result stored in task.out
    fn try_run(&self) -> bool {
        if self
            .state
            .compare_exchange(PENDING, TAKEN, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return false;
        }
        let f = unsafe { (*self.job.get()).take().unwrap() };
        let r = catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_| std::process::abort());
        unsafe { *self.out.get() = Some(r) };
        self.state.store(DONE, Ordering::Release);
        true
    }
}

/// shared deque since promotions are sparse
pub struct Pool {
    queue: Mutex<VecDeque<Arc<dyn Job>>>,
    ready: Condvar,
    threads: usize,
}

impl Pool {
    /// init Pool with fresh deque and spawn `threads - 1` threads
    pub fn new(threads: usize) -> &'static Pool {
        let threads = threads.max(1);
        let pool: &'static Pool = Box::leak(Box::new(Pool {
            queue: Mutex::new(VecDeque::new()),
            ready: Condvar::new(),
            threads,
        }));
        for i in 1..threads {
            std::thread::Builder::new()
                .name(format!("prela-worker-{i}"))
                .spawn(move || pool.work())
                .unwrap();
        }
        pool
    }

    pub fn threads(&self) -> usize {
        self.threads
    }

    /// sleep while deque empty; when non-empty, take oldest task and try to run it; repeat
    fn work(&self) {
        loop {
            let job = {
                let mut q = self.queue.lock().unwrap();
                loop {
                    match q.pop_front() {
                        Some(j) => break j,
                        None => q = self.ready.wait(q).unwrap(),
                    }
                }
            };
            job.try_run();
        }
    }

    /// try to take oldest queued task without waiting (steal)
    fn pop(&self) -> Option<Arc<dyn Job>> {
        self.queue.lock().unwrap().pop_front()
    }

    /// add f to the work-sharing queue; return a handle to the task
    pub unsafe fn spawn<'a, T, F>(&'static self, f: F) -> Handle<'a, T>
    where
        T: Send + 'a,
        F: FnOnce() -> T + Send + 'a,
    {
        let task = Arc::new(Task {
            state: AtomicU8::new(PENDING),
            job: UnsafeCell::new(Some(Box::new(f) as Box<dyn FnOnce() -> T + Send + 'a>)),
            out: UnsafeCell::new(None),
        });
        let erased: Arc<dyn Job + 'a> = task.clone();
        let erased: Arc<dyn Job + 'static> = unsafe { std::mem::transmute(erased) };
        self.queue.lock().unwrap().push_back(erased);
        self.ready.notify_one();
        Handle {
            task,
            pool: self,
            _a: PhantomData,
        }
    }

    /// queue fb, run fa, join fb
    pub fn join2<A, B, FA, FB>(&'static self, fa: FA, fb: FB) -> (A, B)
    where
        A: Send,
        B: Send,
        FA: FnOnce() -> A + Send,
        FB: FnOnce() -> B + Send,
    {
        let h = unsafe { self.spawn(fb) };
        let a = fa();
        (a, h.join())
    }
}

/// global pool; starts clock
pub fn pool() -> &'static Pool {
    static POOL: OnceLock<&'static Pool> = OnceLock::new();
    POOL.get_or_init(|| {
        let n = std::env::var("PRELA_THREADS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        if n > 1 {
            start_clock(beat_interval());
        }
        Pool::new(n)
    })
}

/// joinable wrapper around a task for the spawner to wait on
pub struct Handle<'a, T: Send> {
    task: Arc<Task<'a, T>>,
    pool: &'static Pool,
    _a: PhantomData<&'a ()>,
}

impl<T: Send> Handle<'_, T> {
    /// run task if unclaimed, otherwise try running other queued tasks
    fn wait(&self) {
        if self.task.try_run() {
            return;
        }
        while self.task.state.load(Ordering::Acquire) != DONE {
            match self.pool.pop() {
                Some(j) => {
                    j.try_run();
                }
                None => std::hint::spin_loop(),
            }
        }
    }

    /// wait on task and take result
    pub fn join(self) -> T {
        self.wait();
        unsafe { (*self.task.out.get()).take().unwrap() }
    }
}

/// dropping waits so task doesn't outlive handle
impl<T: Send> Drop for Handle<'_, T> {
    fn drop(&mut self) {
        self.wait();
    }
}

pub trait Reduce<A>: Sync {
    fn init(&self) -> A;
    fn merge(&self, acc: &mut A, later: A);
}

/// a pair of closures is a reduction
impl<A, I: Fn() -> A + Sync, M: Fn(&mut A, A) + Sync> Reduce<A> for (I, M) {
    fn init(&self) -> A {
        (self.0)()
    }
    fn merge(&self, acc: &mut A, later: A) {
        (self.1)(acc, later)
    }
}

/// splittable: a running loop
trait Split {
    fn try_split(&self) -> bool;
}

thread_local! {
    /// running `Interval`s, from outermost to innermost
    static LOOPS: RefCell<Vec<*const dyn Split>> = const { RefCell::new(Vec::new()) };
    /// The last beat this thread acted on, so nested loops split once per beat, not once each.
    static LAST: Cell<u64> = const { Cell::new(0) };
}

/// total number of splits across all threads
static SPLITS: AtomicUsize = AtomicUsize::new(0);

pub fn splits() -> usize {
    SPLITS.load(Ordering::Relaxed)
}

/// a piece of the loop.
/// `pending` stores the pieces handed off so far
struct Interval<'a, A: Send, Rd, S, B> {
    cur: Cell<usize>,
    hi: Cell<usize>,
    pending: RefCell<Vec<Handle<'a, A>>>,
    pool: &'static Pool,
    rd: &'a Rd,
    start: &'a S,
    body: &'a B,
}

/// intervals are splittable
impl<'a, A, Rd, C, S, B> Split for Interval<'a, A, Rd, S, B>
where
    A: Send + 'a,
    Rd: Reduce<A>,
    S: Fn(usize) -> C + Sync,
    B: Fn(&mut A, &mut C, usize) + Sync,
{
    fn try_split(&self) -> bool {
        let (cur, hi) = (self.cur.get(), self.hi.get());
        if cur >= hi {
            return false;
        }
        let mid = cur + (hi - cur) / 2;
        self.hi.set(mid); // keep lower half
        let (pool, rd, start, body) = (self.pool, self.rd, self.start, self.body);
        // handle for upper half task
        let h = unsafe {
            pool.spawn(move || {
                let mut a = rd.init(); // acc set to reduction init
                run_interval_in(pool, rd, &mut a, mid, hi, start, body);
                a
            })
        };
        self.pending.borrow_mut().push(h);
        SPLITS.fetch_add(1, Ordering::Relaxed);
        true
    }
}

/// registers an interval on thread-local `LOOPS` stack and pops it when `Registered` goes out of
/// scope
struct Registered;

impl Registered {
    /// push `frame` onto thread-local `LOOPS` stack
    fn new<'a>(frame: &'a (dyn Split + 'a)) -> Registered {
        let p: *const (dyn Split + 'a) = frame;
        let p: *const (dyn Split + 'static) = unsafe { std::mem::transmute(p) };
        LOOPS.with(|ls| ls.borrow_mut().push(p));
        Registered
    }
}

impl Drop for Registered {
    fn drop(&mut self) {
        LOOPS.with(|ls| ls.borrow_mut().pop());
    }
}

/// split the outermost loop with work left
/// checks whether LAST == b so we don't split more than once
#[cold]
fn on_beat(b: u64) {
    if LAST.with(|l| l.replace(b)) == b {
        return;
    }
    LOOPS.with(|ls| {
        for &f in ls.borrow().iter() {
            if unsafe { (*f).try_split() } {
                break;
            }
        }
    });
}

/// runs `body` over `lo..hi`, splitting on beats and reducing `acc` with `rd`
pub fn run_interval<A, Rd, C, S, B>(rd: &Rd, acc: &mut A, lo: usize, hi: usize, start: S, body: B)
where
    A: Send,
    Rd: Reduce<A>,
    S: Fn(usize) -> C + Sync,
    B: Fn(&mut A, &mut C, usize) + Sync,
{
    run_interval_in(pool(), rd, acc, lo, hi, &start, &body);
}

/// loops over the `lo..hi` interval
/// creates a frame for the current interval, registers it in LOOPS
/// applies `body` to `i` in `cur..high` checking for a beat at each iteration
/// if beat, split
/// once done with the interval, join all pending frames
fn run_interval_in<A, Rd, C, S, B>(
    pool: &'static Pool,
    rd: &Rd,
    acc: &mut A,
    lo: usize,
    hi: usize,
    start: &S,
    body: &B,
) where
    A: Send,
    Rd: Reduce<A>,
    S: Fn(usize) -> C + Sync,
    B: Fn(&mut A, &mut C, usize) + Sync,
{
    let frame = Interval {
        cur: Cell::new(lo),
        hi: Cell::new(hi),
        pending: RefCell::new(Vec::new()),
        pool,
        rd,
        start,
        body,
    };
    let mut c = start(lo);
    {
        let _reg = Registered::new(&frame);
        let mut seen = beat();
        loop {
            let i = frame.cur.get();
            if i >= frame.hi.get() {
                break;
            }
            frame.cur.set(i + 1);
            body(acc, &mut c, i);
            let b = beat();
            if b != seen {
                seen = b;
                on_beat(b);
            }
        }
    }
    for h in frame.pending.take().into_iter().rev() {
        rd.merge(acc, h.join());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn clock_ticks_near_interval() {
        start_clock(Duration::from_micros(100));
        let (b0, t0) = (beat(), Instant::now());
        std::thread::sleep(Duration::from_millis(200));
        let (n, dt) = (beat() - b0, t0.elapsed());
        let us = dt.as_secs_f64() * 1e6 / n as f64;
        eprintln!("{n} beats in {dt:?}: {us:.1}µs per beat");
        assert!(n > 400, "{n} beats in {dt:?}");
    }

    fn sum(pool: &'static Pool, xs: &[u64]) -> u64 {
        if xs.len() <= 1024 {
            return xs.iter().sum();
        }
        let (l, r) = xs.split_at(xs.len() / 2);
        let (a, b) = pool.join2(|| sum(pool, l), || sum(pool, r));
        a + b
    }

    #[test]
    fn recursive_sum_matches_sequential() {
        let xs: Vec<u64> = (1..=1_000_000).collect();
        let want: u64 = xs.iter().sum();
        assert_eq!(sum(Pool::new(4), &xs), want);
        assert_eq!(sum(Pool::new(1), &xs), want);
    }

    #[test]
    fn tasks_run_on_other_threads() {
        let pool = Pool::new(4);
        let seen = Mutex::new(HashSet::new());
        let hs: Vec<_> = (0..8)
            .map(|_| unsafe {
                pool.spawn(|| {
                    std::thread::sleep(Duration::from_millis(10));
                    seen.lock().unwrap().insert(std::thread::current().id());
                })
            })
            .collect();
        for h in hs {
            h.join();
        }
        assert!(seen.lock().unwrap().len() > 1);
    }

    #[test]
    fn dropped_handle_waits_for_task() {
        let pool = Pool::new(2);
        let mut x = 0u64;
        {
            let _h = unsafe { pool.spawn(|| x = 42) };
        }
        assert_eq!(x, 42);
    }
}

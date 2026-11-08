use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

// Task states: waiting in the queue, claimed by some thread, finished with a result.
const PENDING: u8 = 0;
const TAKEN: u8 = 1;
const DONE: u8 = 2;

// A type-erased task as the queue sees it: anyone may try to run it, only the first claimer does.
trait Job: Send + Sync {
    fn try_run(&self) -> bool;
}

// One unit of work: its state, the closure to run (taken once), and the slot its result lands in.
struct Task<'a, T> {
    state: AtomicU8,
    job: UnsafeCell<Option<Box<dyn FnOnce() -> T + Send + 'a>>>,
    out: UnsafeCell<Option<T>>,
}

// Sharing is safe because `job` and `out` are only touched by the single thread that won the claim.
unsafe impl<T: Send> Sync for Task<'_, T> {}

impl<T: Send> Job for Task<'_, T> {
    // Claim the task (PENDING → TAKEN), run it, store the result, mark DONE; false if already claimed.
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

// Worker threads plus the shared queue they take tasks from; `threads` counts the calling thread too.
pub struct Pool {
    queue: Mutex<VecDeque<Arc<dyn Job>>>,
    ready: Condvar,
    threads: usize,
}

impl Pool {
    // Leak a pool and start `threads - 1` workers on it (the caller is the remaining one).
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

    // Total thread count, including the caller.
    pub fn threads(&self) -> usize {
        self.threads
    }

    // Worker loop: sleep until the queue is non-empty, take the oldest task, try to run it, repeat.
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

    // Take the oldest queued task without waiting, for a joiner that wants to help.
    fn pop(&self) -> Option<Arc<dyn Job>> {
        self.queue.lock().unwrap().pop_front()
    }

    // Queue `f` and return its handle; unsafe because the handle must not be leaked while `f` borrows `'a`.
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

    // Safe fork-join: offer `fb` to the pool, run `fa` here, then join `fb`.
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

// The global pool, created on first use with `PRELA_THREADS` threads (default 1); starts the clock if > 1.
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

// The spawner's side of a task; joining or dropping it waits until the task is done.
pub struct Handle<'a, T: Send> {
    task: Arc<Task<'a, T>>,
    pool: &'static Pool,
    _a: PhantomData<&'a ()>,
}

impl<T: Send> Handle<'_, T> {
    // Run the task here if nobody claimed it; otherwise run other queued tasks (or spin) until it's done.
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

    // Wait for the task and take its result.
    pub fn join(self) -> T {
        self.wait();
        unsafe { (*self.task.out.get()).take().unwrap() }
    }
}

// Dropping without joining still waits, so the task can't outlive what it borrows.
impl<T: Send> Drop for Handle<'_, T> {
    fn drop(&mut self) {
        self.wait();
    }
}

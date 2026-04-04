// Lesson 14: Work-Stealing Scheduler
//
// Multi-threaded runtime where idle workers steal from busy ones.
// Run with: cargo run -p async-lessons --bin 14-work-stealing -- <command>
//
// Commands:
//   two-workers      Run 2 worker threads, verify both do work
//   steal-half       Demonstrate stealing half of a busy queue
//   benchmark        Compare single-thread vs work-stealing throughput
//   all              Run all demos

use clap::{Parser, Subcommand};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;
use std::time::{Duration, Instant};

// ============================================================
// Task (same as Lesson 10, but with a global-queue variant)
// ============================================================

type TaskQueue = Arc<Mutex<VecDeque<Arc<Task>>>>;

struct Task {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    global_queue: TaskQueue,
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        self.global_queue.lock().unwrap().push_back(self.clone());
    }
}

// ============================================================
// Work-Stealing Runtime
// ============================================================

/// A multi-threaded work-stealing runtime.
///
/// TODO: Implement the worker loop and spawn.
///
/// Architecture:
///   - N worker threads, each with a local VecDeque
///   - One global queue for overflow and cross-thread spawns
///   - Workers: pop local → pop global → steal from random peer → park
struct WorkStealingRuntime {
    /// Shared global queue
    global_queue: TaskQueue,
    /// Per-worker local queues (indexed by worker_id)
    local_queues: Vec<Arc<Mutex<VecDeque<Arc<Task>>>>>,
    /// Worker thread handles
    handles: Vec<thread::JoinHandle<()>>,
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
    /// Number of workers
    num_workers: usize,
}

impl WorkStealingRuntime {
    /// Create a new runtime with `num_workers` threads.
    ///
    /// TODO: Implement this.
    ///   1. Create global_queue
    ///   2. Create num_workers local queues
    ///   3. Spawn num_workers threads, each running worker_loop
    ///   4. Store thread handles for join on shutdown
    fn new(num_workers: usize) -> Self {
        let global_queue: TaskQueue = Arc::new(Mutex::new(VecDeque::new()));
        let local_queues: Vec<_> = (0..num_workers)
            .map(|_| Arc::new(Mutex::new(VecDeque::new())))
            .collect();
        let shutdown = Arc::new(AtomicBool::new(false));

        let handles = Vec::new();
        // TODO: spawn worker threads here
        // Each thread runs worker_loop(worker_id, local_queues, global_queue, shutdown)

        Self {
            global_queue,
            local_queues,
            handles,
            shutdown,
            num_workers,
        }
    }

    /// Spawn a task onto the global queue.
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            global_queue: self.global_queue.clone(),
        });
        self.global_queue.lock().unwrap().push_back(task);
    }

    /// Signal shutdown and wait for workers to finish.
    fn shutdown(mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        // Unpark all workers so they see the shutdown flag
        for handle in &self.handles {
            handle.thread().unpark();
        }
        for handle in self.handles.drain(..) {
            handle.join().unwrap();
        }
    }
}

/// The worker loop — run on each worker thread.
///
/// TODO: Implement this.
///   1. Loop until shutdown flag is set
///   2. Try pop from local queue
///   3. If empty → try pop from global queue
///   4. If empty → try steal from a random peer's local queue
///   5. If got a task → poll it
///   6. If nothing → park (will be unparked when new task arrives)
fn worker_loop(
    _worker_id: usize,
    _local_queues: &[Arc<Mutex<VecDeque<Arc<Task>>>>],
    _global_queue: &TaskQueue,
    _shutdown: &AtomicBool,
) {
    todo!("Implement worker_loop")
}

// ============================================================
// Single-threaded executor (for benchmark comparison)
// ============================================================

fn run_single_threaded(tasks: Vec<Pin<Box<dyn Future<Output = ()> + Send>>>) {
    let queue: TaskQueue = Arc::new(Mutex::new(VecDeque::new()));
    for future in tasks {
        let task = Arc::new(Task {
            future: Mutex::new(future),
            global_queue: queue.clone(),
        });
        queue.lock().unwrap().push_back(task);
    }

    loop {
        let task = queue.lock().unwrap().pop_front();
        match task {
            Some(task) => {
                let waker = Waker::from(task.clone());
                let mut cx = Context::from_waker(&waker);
                let mut future = task.future.lock().unwrap();
                let _ = future.as_mut().poll(&mut cx);
            }
            None => break,
        }
    }
}

// ============================================================
// CLI
// ============================================================

#[derive(Parser)]
#[command(name = "work-stealing", about = "Lesson 14: Work-Stealing Scheduler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run 2 workers, verify both process tasks
    TwoWorkers,
    /// Demo stealing from a busy worker
    StealHalf,
    /// Benchmark single-thread vs work-stealing
    Benchmark,
    /// Run all demos
    All,
}

fn demo_two_workers() {
    println!("=== Two-Worker Runtime ===");
    println!("Spawning 100 tasks across 2 worker threads.");
    println!();

    // Track which thread processed each task
    let thread_ids: Arc<Mutex<Vec<thread::ThreadId>>> = Arc::new(Mutex::new(Vec::new()));

    let rt = WorkStealingRuntime::new(2);

    for i in 0..100 {
        let ids = thread_ids.clone();
        rt.spawn(async move {
            ids.lock().unwrap().push(thread::current().id());
            // Simulate some work
            let _ = i * i;
        });
    }

    // Give workers time to process
    thread::sleep(Duration::from_millis(500));
    rt.shutdown();

    let ids = thread_ids.lock().unwrap();
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    println!("  Total tasks processed: {}", ids.len());
    println!("  Unique threads used: {}", unique.len());
    println!("  Thread IDs: {:?}", unique);
    println!();
    if unique.len() >= 2 {
        println!("  Both workers did work!");
    } else {
        println!("  TODO: implement worker_loop so both threads process tasks");
    }
}

fn demo_steal_half() {
    println!("=== Steal Half ===");
    println!("50 tasks on Worker 0, none on Worker 1.");
    println!("Worker 1 should steal half and process them.");
    println!();
    println!("TODO: implement worker_loop with steal-from-peer logic.");
    println!("After implementation, Worker 1 should process ~25 tasks.");
}

fn demo_benchmark() {
    println!("=== Benchmark: Single-Thread vs Work-Stealing ===");
    println!();

    let num_tasks = 10_000;
    let counter = Arc::new(AtomicU32::new(0));

    // Single-threaded
    let c = counter.clone();
    let tasks: Vec<_> = (0..num_tasks)
        .map(|_| {
            let c = c.clone();
            Box::pin(async move {
                c.fetch_add(1, Ordering::Relaxed);
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        })
        .collect();

    counter.store(0, Ordering::SeqCst);
    let start = Instant::now();
    run_single_threaded(tasks);
    let single_time = start.elapsed();
    let single_count = counter.load(Ordering::SeqCst);

    println!("  Single-threaded: {} tasks in {:?} ({:.0} tasks/sec)",
        single_count, single_time,
        single_count as f64 / single_time.as_secs_f64());

    // Work-stealing (4 workers)
    counter.store(0, Ordering::SeqCst);
    let c = counter.clone();
    let rt = WorkStealingRuntime::new(4);
    let start = Instant::now();

    for _ in 0..num_tasks {
        let c = c.clone();
        rt.spawn(async move {
            c.fetch_add(1, Ordering::Relaxed);
        });
    }

    thread::sleep(Duration::from_millis(500));
    rt.shutdown();
    let ws_time = start.elapsed();
    let ws_count = counter.load(Ordering::SeqCst);

    println!("  Work-stealing (4 workers): {} tasks in {:?} ({:.0} tasks/sec)",
        ws_count, ws_time,
        ws_count as f64 / ws_time.as_secs_f64());

    println!();
    println!("  TODO: once worker_loop is implemented, work-stealing should be faster.");
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::TwoWorkers => demo_two_workers(),
        Command::StealHalf => demo_steal_half(),
        Command::Benchmark => demo_benchmark(),
        Command::All => {
            demo_two_workers();
            println!("\n");
            demo_steal_half();
            println!("\n");
            demo_benchmark();
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_threaded_completes_all() {
        let counter = Arc::new(AtomicU32::new(0));
        let tasks: Vec<_> = (0..50).map(|_| {
            let c = counter.clone();
            Box::pin(async move { c.fetch_add(1, Ordering::SeqCst); })
                as Pin<Box<dyn Future<Output = ()> + Send>>
        }).collect();

        run_single_threaded(tasks);
        assert_eq!(counter.load(Ordering::SeqCst), 50);
    }

    #[test]
    fn runtime_creates_without_panic() {
        let rt = WorkStealingRuntime::new(2);
        rt.shutdown();
    }
}

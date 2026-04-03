// Lesson 4: A Minimal Executor
//
// Build a single-threaded executor that can run multiple futures.
// Run with: cargo run -p async-lessons --bin 4-executor -- <command>
//
// Commands:
//   block-on           Run a single future with block_on
//   multi              Run multiple futures concurrently
//   delay <secs>       Run a real-time DelayFuture
//   concurrent-delays  Spawn multiple delays, watch them complete concurrently
//   all                Run all demos

use clap::{Parser, Subcommand};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::time::{Duration, Instant};

// ============================================================
// CountdownFuture (reused from Lessons 1-3)
// ============================================================

struct CountdownFuture {
    name: String,
    count: u32,
}

impl Future for CountdownFuture {
    type Output = String;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<String> {
        if self.count == 0 {
            Poll::Ready(format!("{} done!", self.name))
        } else {
            self.count -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

// ============================================================
// DelayFuture: a future that completes after a real time delay
// ============================================================

/// A future that returns Ready after a real wall-clock delay.
/// Uses a background thread to call waker.wake() when the deadline passes.
///
/// TODO: Implement Future for DelayFuture.
///   - First poll: if deadline hasn't passed, spawn a thread that sleeps
///     until deadline then calls waker.wake(). Set waker_set=true. Return Pending.
///   - Subsequent polls before deadline: just return Pending (thread already spawned).
///   - Poll after deadline: return Ready(message).
struct DelayFuture {
    message: String,
    deadline: Instant,
    waker_set: bool,
}

impl DelayFuture {
    fn new(delay: Duration, message: &str) -> Self {
        Self {
            message: message.to_string(),
            deadline: Instant::now() + delay,
            waker_set: false,
        }
    }
}

impl Future for DelayFuture {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<String> {
        // TODO:
        // 1. If Instant::now() >= self.deadline → return Ready(message.clone())
        // 2. If !self.waker_set:
        //    a. Clone the waker: let waker = cx.waker().clone();
        //    b. Get the deadline: let deadline = self.deadline;
        //    c. Spawn a thread: std::thread::spawn(move || { ... })
        //       Inside the thread: sleep until deadline, then call waker.wake()
        //    d. Set self.waker_set = true
        // 3. Return Pending
        todo!("Implement DelayFuture::poll")
    }
}

// ============================================================
// block_on: runs a single future on the current thread
// ============================================================

/// Run a single future to completion on the current thread.
///
/// TODO: Implement this function.
///   1. Get current thread handle: std::thread::current()
///   2. Build a thread-parking waker (wake = thread.unpark())
///      Hint: reuse your Lesson 3 thread waker, or build one inline
///   3. Create Context from the waker
///   4. Pin the future
///   5. Loop:
///      a. Poll the future
///      b. If Ready → return the value
///      c. If Pending → std::thread::park() (sleep until waker fires)
fn block_on<F: Future>(future: F) -> F::Output {
    todo!("Implement block_on")
}

// ============================================================
// Multi-task Executor
// ============================================================

/// A task is a boxed future + the shared queue reference (for waking).
///
/// TODO: Define the Task struct:
///   - future: Pin<Box<dyn Future<Output = ()>>>
///   - queue: Arc<Mutex<VecDeque<Arc<Task>>>>
///
/// TODO: Implement a function to create a Waker for a Task:
///   - The waker's wake() should push the Arc<Task> back into the queue
///   - The waker's data pointer stores Arc<Task>

struct Executor {
    queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
}

struct Task {
    // TODO: add fields
    // future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    // queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
    _placeholder: (),
}

impl Executor {
    fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Add a future to the executor's task queue.
    ///
    /// TODO: Implement spawn.
    ///   1. Box and pin the future
    ///   2. Create a Task with the future and a clone of the queue
    ///   3. Wrap in Arc
    ///   4. Push to queue
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        todo!("Implement Executor::spawn")
    }

    /// Run all spawned tasks to completion.
    ///
    /// TODO: Implement run.
    ///   1. Loop: pop a task from the queue
    ///   2. Build a Waker for that task (wake = push Arc<Task> back to queue)
    ///   3. Poll the task's future
    ///   4. If Ready → task is done (don't re-queue)
    ///   5. If Pending → waker will re-queue when ready
    ///   6. If queue is empty → park thread (a waker will unpark when a task is ready)
    ///   7. When all tasks are done → return
    fn run(&self) {
        todo!("Implement Executor::run")
    }
}

// ============================================================
// CLI
// ============================================================

#[derive(Parser)]
#[command(name = "executor", about = "Lesson 4: A minimal executor")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a single CountdownFuture with block_on
    BlockOn,
    /// Run multiple CountdownFutures concurrently with the multi-task executor
    Multi,
    /// Run a real-time DelayFuture with block_on
    Delay {
        #[arg(default_value = "2")]
        secs: u64,
    },
    /// Spawn concurrent delays, watch them finish in time order
    ConcurrentDelays,
    /// Run all demos
    All,
}

fn demo_block_on() {
    println!("=== block_on (single future) ===");
    println!("Running CountdownFuture(5) with block_on.");
    println!("The executor parks after each Pending, waker unparks it.");
    println!();

    let start = Instant::now();
    let result = block_on(CountdownFuture { name: "task".into(), count: 5 });
    println!("  Result: {result}");
    println!("  Time: {:?} (should be near-instant — no real waiting)", start.elapsed());
    println!();
    println!("Takeaway: block_on is the simplest executor — poll, park, repeat.");
}

fn demo_multi() {
    println!("=== Multi-task Executor ===");
    println!("Spawning 3 CountdownFutures with different counts.");
    println!("They run concurrently (interleaved), not sequentially.");
    println!();

    let executor = Executor::new();
    executor.spawn(async {
        let msg = CountdownFuture { name: "A (count=3)".into(), count: 3 }.await;
        println!("  {msg}");
    });
    executor.spawn(async {
        let msg = CountdownFuture { name: "B (count=1)".into(), count: 1 }.await;
        println!("  {msg}");
    });
    executor.spawn(async {
        let msg = CountdownFuture { name: "C (count=5)".into(), count: 5 }.await;
        println!("  {msg}");
    });

    executor.run();

    println!();
    println!("Takeaway: B finishes first (count=1), then A (3), then C (5).");
    println!("All on one thread, interleaved. This is cooperative multitasking.");
}

fn demo_delay(secs: u64) {
    println!("=== DelayFuture (real timer) ===");
    println!("Waiting {secs} seconds using a background thread + waker.");
    println!("The executor thread sleeps (0% CPU) until the delay fires.");
    println!();

    let start = Instant::now();
    let result = block_on(DelayFuture::new(
        Duration::from_secs(secs),
        &format!("hello after {secs}s!"),
    ));
    println!("  Result: \"{result}\"");
    println!("  Time: {:?}", start.elapsed());
    println!();
    println!("Takeaway: the executor didn't busy-poll. It parked the thread,");
    println!("and the background thread's waker.wake() unparked it after {secs}s.");
}

fn demo_concurrent_delays() {
    println!("=== Concurrent Delays ===");
    println!("Spawning 3 delays: 3s, 1s, 2s. They should complete in ~3s total.");
    println!();

    let start = Instant::now();
    let executor = Executor::new();

    executor.spawn(async {
        let msg = DelayFuture::new(Duration::from_secs(3), "slow (3s)").await;
        println!("  [{:.1}s] {msg}", Instant::now().duration_since(Instant::now() - Duration::from_secs(3)).as_secs_f64());
    });
    executor.spawn(async {
        let msg = DelayFuture::new(Duration::from_secs(1), "fast (1s)").await;
        println!("  {msg}");
    });
    executor.spawn(async {
        let msg = DelayFuture::new(Duration::from_secs(2), "medium (2s)").await;
        println!("  {msg}");
    });

    executor.run();

    let total = start.elapsed();
    println!();
    println!("  Total time: {:?}", total);
    println!("  (Should be ~3s, NOT 6s — delays ran concurrently!)");
    println!();
    println!("Takeaway: concurrent execution on a single thread.");
    println!("Three 'sleeps' overlapped because the executor multiplexed them.");
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::BlockOn => demo_block_on(),
        Command::Multi => demo_multi(),
        Command::Delay { secs } => demo_delay(secs),
        Command::ConcurrentDelays => demo_concurrent_delays(),
        Command::All => {
            demo_block_on();
            println!("\n");
            demo_multi();
            println!("\n");
            demo_delay(2);
            println!("\n");
            demo_concurrent_delays();
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
    fn block_on_countdown() {
        let result = block_on(CountdownFuture { name: "test".into(), count: 5 });
        assert_eq!(result, "test done!");
    }

    #[test]
    fn block_on_delay() {
        let start = Instant::now();
        let result = block_on(DelayFuture::new(Duration::from_millis(100), "done"));
        let elapsed = start.elapsed();
        assert_eq!(result, "done");
        assert!(elapsed >= Duration::from_millis(90), "Should wait ~100ms");
        assert!(elapsed < Duration::from_millis(500), "Shouldn't take too long");
    }

    #[test]
    fn multi_executor_completes_all() {
        let done = Arc::new(AtomicBool::new(false));
        let done2 = done.clone();

        let executor = Executor::new();
        executor.spawn(async {
            CountdownFuture { name: "a".into(), count: 3 }.await;
        });
        executor.spawn(async {
            CountdownFuture { name: "b".into(), count: 1 }.await;
        });
        executor.spawn(async move {
            CountdownFuture { name: "c".into(), count: 5 }.await;
            done2.store(true, Ordering::SeqCst);
        });

        executor.run();
        assert!(done.load(Ordering::SeqCst), "All tasks should complete");
    }

    #[test]
    fn concurrent_delays_are_concurrent() {
        let start = Instant::now();
        let executor = Executor::new();

        executor.spawn(async {
            DelayFuture::new(Duration::from_millis(300), "a").await;
        });
        executor.spawn(async {
            DelayFuture::new(Duration::from_millis(100), "b").await;
        });
        executor.spawn(async {
            DelayFuture::new(Duration::from_millis(200), "c").await;
        });

        executor.run();
        let elapsed = start.elapsed();
        // Should take ~300ms (max of all delays), not 600ms (sum)
        assert!(elapsed < Duration::from_millis(500), "Delays should be concurrent");
    }
}

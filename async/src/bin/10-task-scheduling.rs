// Lesson 10: Task Scheduling
//
// Build a proper task scheduler with spawn, JoinHandle, and fairness.
// Run with: cargo run -p async-lessons --bin 10-task-scheduling -- <command>
//
// Commands:
//   spawn-join         Spawn tasks, await JoinHandles
//   fairness           Show FIFO fairness with greedy + quick tasks
//   starvation         Show what happens when a task never yields
//   all                Run all demos

use clap::{Parser, Subcommand};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

// ============================================================
// Task
// ============================================================

struct Task {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        self.queue.lock().unwrap().push_back(self.clone());
    }
}

// ============================================================
// JoinState + JoinHandle
// ============================================================

/// Shared state between a spawned task and its JoinHandle.
struct JoinState<T> {
    result: Option<T>,
    waker: Option<Waker>,
}

/// A future that resolves to the output of a spawned task.
///
/// TODO: Implement Future for JoinHandle<T>.
///   - poll(): lock state, check result
///     - Some → return Ready(result)
///     - None → store waker, return Pending
pub struct JoinHandle<T> {
    state: Arc<Mutex<JoinState<T>>>,
}

impl<T: Unpin> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        // TODO:
        // 1. Lock self.state
        // 2. If result is Some → take it, return Ready
        // 3. If result is None → store cx.waker().clone(), return Pending
        todo!("Implement JoinHandle::poll")
    }
}

// ============================================================
// Executor with spawn + run
// ============================================================

struct Executor {
    queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
}

impl Executor {
    fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Spawn a future as a task, return a JoinHandle to get the result.
    ///
    /// TODO: Implement this.
    ///   1. Create JoinState { result: None, waker: None }
    ///   2. Wrap in Arc<Mutex<JoinState<T>>>
    ///   3. Create a wrapper future that:
    ///      a. Runs the inner future to completion
    ///      b. Stores the result in JoinState
    ///      c. Wakes the JoinHandle's waker
    ///   4. Create a Task with the wrapper future
    ///   5. Push the task to the queue
    ///   6. Return JoinHandle { state }
    fn spawn<T: Send + 'static>(
        &self,
        _future: impl Future<Output = T> + Send + 'static,
    ) -> JoinHandle<T> {
        todo!("Implement Executor::spawn")
    }

    /// Spawn a fire-and-forget task (no JoinHandle).
    fn spawn_detached(&self, future: impl Future<Output = ()> + Send + 'static) {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            queue: self.queue.clone(),
        });
        self.queue.lock().unwrap().push_back(task);
    }

    /// Run all tasks until the queue is empty.
    fn run(&self) {
        loop {
            let task = self.queue.lock().unwrap().pop_front();
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
}

// ============================================================
// Helper futures
// ============================================================

/// A future that yields N times then returns the total poll count.
struct YieldNTimes {
    remaining: u32,
    poll_count: Arc<AtomicU32>,
    label: String,
}

impl Future for YieldNTimes {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        let count = self.poll_count.fetch_add(1, Ordering::SeqCst) + 1;
        if self.remaining == 0 {
            println!("  [{}] Ready after {} polls", self.label, count);
            Poll::Ready(count)
        } else {
            self.remaining -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

// ============================================================
// CLI
// ============================================================

#[derive(Parser)]
#[command(name = "task-scheduling", about = "Lesson 10: Task Scheduling")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Spawn tasks and await JoinHandles for results
    SpawnJoin,
    /// Demonstrate FIFO fairness
    Fairness,
    /// Show cooperative multitasking in action
    Starvation,
    /// Run all demos
    All,
}

fn demo_spawn_join() {
    println!("=== Spawn + JoinHandle ===");
    println!("Spawning 3 tasks, each returns a number. Awaiting results.");
    println!();
    println!("TODO: implement spawn() and JoinHandle::poll, then uncomment:");
    println!("  let a = executor.spawn(async {{ 10 }});");
    println!("  let b = executor.spawn(async {{ 20 }});");
    println!("  let c = executor.spawn(async {{ 30 }});");
    println!("  // a.await + b.await + c.await == 60");
    println!();
    println!("Takeaway: JoinHandle lets you get results from spawned tasks.");
}

fn demo_fairness() {
    println!("=== FIFO Fairness ===");
    println!("Spawning a greedy task (100 yields) and a quick task (1 yield).");
    println!();

    let executor = Executor::new();
    let greedy_polls = Arc::new(AtomicU32::new(0));
    let quick_polls = Arc::new(AtomicU32::new(0));

    let gp = greedy_polls.clone();
    executor.spawn_detached(async move {
        YieldNTimes { remaining: 100, poll_count: gp, label: "greedy".into() }.await;
    });

    let qp = quick_polls.clone();
    executor.spawn_detached(async move {
        YieldNTimes { remaining: 1, poll_count: qp, label: "quick ".into() }.await;
    });

    executor.run();

    println!();
    println!("  Greedy: {} polls, Quick: {} polls",
        greedy_polls.load(Ordering::SeqCst),
        quick_polls.load(Ordering::SeqCst));
    println!("  Quick finished early — FIFO ensures it gets a turn!");
    println!();
    println!("Takeaway: tasks go to the BACK of the queue after yielding.");
}

fn demo_starvation() {
    println!("=== Cooperative Scheduling Demo ===");
    println!("Two tasks: one yields 10 times, one yields 0 times.");
    println!();

    let executor = Executor::new();
    let order = Arc::new(Mutex::new(Vec::<String>::new()));

    let o1 = order.clone();
    executor.spawn_detached(async move {
        YieldNTimes {
            remaining: 5,
            poll_count: Arc::new(AtomicU32::new(0)),
            label: "slow".into(),
        }.await;
        o1.lock().unwrap().push("slow".into());
    });

    let o2 = order.clone();
    executor.spawn_detached(async move {
        YieldNTimes {
            remaining: 0,
            poll_count: Arc::new(AtomicU32::new(0)),
            label: "fast".into(),
        }.await;
        o2.lock().unwrap().push("fast".into());
    });

    executor.run();

    let result = order.lock().unwrap().clone();
    println!();
    println!("  Completion order: {:?}", result);
    println!("  Fast completed before slow — not starved!");
    println!();
    println!("Takeaway: cooperative scheduling works because tasks YIELD.");
    println!("A task that never returns Pending would block everyone.");
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::SpawnJoin => demo_spawn_join(),
        Command::Fairness => demo_fairness(),
        Command::Starvation => demo_starvation(),
        Command::All => {
            demo_spawn_join();
            println!("\n");
            demo_fairness();
            println!("\n");
            demo_starvation();
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
    fn wake_requeues_task() {
        let queue: Arc<Mutex<VecDeque<Arc<Task>>>> = Arc::new(Mutex::new(VecDeque::new()));
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(async {})),
            queue: queue.clone(),
        });

        assert_eq!(queue.lock().unwrap().len(), 0);
        task.wake();
        assert_eq!(queue.lock().unwrap().len(), 1);
    }

    #[test]
    fn fifo_order() {
        let executor = Executor::new();
        let order = Arc::new(Mutex::new(Vec::new()));

        let o1 = order.clone();
        executor.spawn_detached(async move {
            YieldNTimes {
                remaining: 3,
                poll_count: Arc::new(AtomicU32::new(0)),
                label: "A".into(),
            }.await;
            o1.lock().unwrap().push('A');
        });

        let o2 = order.clone();
        executor.spawn_detached(async move {
            YieldNTimes {
                remaining: 0,
                poll_count: Arc::new(AtomicU32::new(0)),
                label: "B".into(),
            }.await;
            o2.lock().unwrap().push('B');
        });

        executor.run();
        let result = order.lock().unwrap().clone();
        assert_eq!(result, vec!['B', 'A'], "B (0 yields) should finish before A (3 yields)");
    }

    #[test]
    fn all_tasks_complete() {
        let executor = Executor::new();
        let count = Arc::new(AtomicU32::new(0));

        for _ in 0..5 {
            let c = count.clone();
            executor.spawn_detached(async move {
                YieldNTimes {
                    remaining: 2,
                    poll_count: Arc::new(AtomicU32::new(0)),
                    label: "t".into(),
                }.await;
                c.fetch_add(1, Ordering::SeqCst);
            });
        }

        executor.run();
        assert_eq!(count.load(Ordering::SeqCst), 5);
    }
}

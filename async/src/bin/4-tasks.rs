// Lesson 4: Tasks
//
// Understand what a Task is and build one from scratch.
// Run with: cargo run -p async-lessons --bin 4-tasks -- <command>
//
// Commands:
//   build-task         Create a Task struct wrapping a future
//   task-waker         Create a waker that re-queues a task
//   lifecycle          Full task lifecycle: spawn → poll → wake → poll → done
//   join-handle        Implement JoinHandle to get a task's result
//   all                Run all demos

use clap::{Parser, Subcommand};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

// ============================================================
// CountdownFuture (reused from previous lessons)
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
// Task: a future wrapped with executor metadata
// ============================================================

type TaskQueue = Arc<Mutex<VecDeque<Arc<Task>>>>;

/// A task wraps a future with everything the executor needs.
///
/// TODO: Uncomment and understand each field:
///   - future: the actual work (pinned, boxed, type-erased, behind a Mutex)
///   - queue: where to push this task when waker.wake() is called
struct Task {
    // TODO: add these fields:
    // future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    // queue: TaskQueue,
    _placeholder: (),
}

impl Task {
    /// Create a new task from a future and a queue reference.
    ///
    /// TODO: Implement this.
    ///   1. Box::pin the future (puts it on heap + pins it)
    ///   2. Wrap in Mutex (for interior mutability)
    ///   3. Store the queue reference
    fn new(future: impl Future<Output = ()> + Send + 'static, queue: TaskQueue) -> Arc<Self> {
        todo!("Implement Task::new")
    }

    /// Create a Waker for this task.
    /// When wake() is called, it pushes Arc<Task> back into the queue.
    ///
    /// TODO: Implement the vtable functions:
    ///   - clone: Arc::from_raw → clone → Arc::into_raw both
    ///   - wake: Arc::from_raw → push to queue → (Arc drops)
    ///   - wake_by_ref: Arc::from_raw → push to queue → Arc::into_raw (don't drop)
    ///   - drop: Arc::from_raw (let it drop)
    fn waker(self: &Arc<Self>) -> Waker {
        todo!("Implement Task::waker")
    }
}

// ============================================================
// JoinHandle: get the result of a spawned task
// ============================================================

/// A future that resolves to the output of a spawned task.
///
/// TODO: Implement this struct and Future for it.
///   - Shared state: Arc<Mutex<JoinState<T>>>
///   - JoinState holds: Option<T> (result) and Option<Waker> (to wake the handle)
///   - When the task completes, it stores the result and wakes the handle
///   - When the handle is polled, it checks if the result is available
struct JoinHandle<T> {
    // TODO: add shared state
    _marker: std::marker::PhantomData<T>,
}

// ============================================================
// Helpers
// ============================================================

/// Spawn a task: wrap future in Task, push to queue.
///
/// TODO: This will work once you implement Task::new.
fn spawn(future: impl Future<Output = ()> + Send + 'static, queue: &TaskQueue) {
    todo!("Implement spawn — calls Task::new and pushes to queue")
}

/// Poll one task from the queue. Returns Some(true) if Ready, Some(false) if Pending.
///
/// TODO: Implement this once Task has a `future` field and `waker()` method.
///   1. Pop a task from the queue
///   2. Create a waker via task.waker()
///   3. Lock the future, poll it
///   4. Return Ready/Pending
fn poll_one(queue: &TaskQueue) -> Option<bool> {
    todo!("Implement poll_one — pops task, builds waker, polls future")
}

// ============================================================
// CLI
// ============================================================

#[derive(Parser)]
#[command(name = "tasks", about = "Lesson 4: Tasks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a Task struct wrapping a CountdownFuture
    BuildTask,
    /// Create a waker that re-queues a task on wake()
    TaskWaker,
    /// Full lifecycle: spawn → poll → wake → poll → done
    Lifecycle,
    /// Implement JoinHandle to get a task's result
    JoinHandle,
    /// Run all demos
    All,
}

fn demo_build_task() {
    println!("=== Build a Task ===");
    println!("Wrapping a CountdownFuture in a Task struct.");
    println!();
    println!("TODO: Implement Task::new, then this demo will work.");
    println!("  1. Add `future` and `queue` fields to the Task struct");
    println!("  2. Implement Task::new — Box::pin the future, store queue ref");
    println!("  3. Run this demo again to see the task created and queued");
    // TODO: uncomment when Task is implemented:
    // let queue: TaskQueue = Arc::new(Mutex::new(VecDeque::new()));
    // let task = Task::new(
    //     async {
    //         let _ = CountdownFuture { name: "wrapped".into(), count: 3 }.await;
    //     },
    //     queue.clone(),
    // );
    // println!("  Task created: Arc<Task> with strong_count = {}", Arc::strong_count(&task));
    // queue.lock().unwrap().push_back(task);
    // println!("  Queue length after push: {}", queue.lock().unwrap().len());
    // println!();
    // println!("Takeaway: a Task wraps a future with a queue reference.");
}

fn demo_task_waker() {
    println!("=== Task Waker ===");
    println!("Creating a waker whose wake() pushes the task back to the queue.");
    println!();
    println!("TODO: Implement Task::waker(), then this demo will work.");
    println!("  1. Build a RawWaker vtable where wake() pushes Arc<Task> to the queue");
    println!("  2. Use Arc::into_raw / Arc::from_raw for the data pointer");
    println!("  3. Run this demo to see the task re-queued after Pending");
}

fn demo_lifecycle() {
    println!("=== Task Lifecycle ===");
    println!("Spawn → queue → poll → Pending → wake → queue → poll → Ready");
    println!();
    println!("TODO: Implement Task::new, Task::waker, spawn, and poll_one.");
    println!("Then this demo will show the full lifecycle:");
    println!("  1. Spawn a CountdownFuture(3) into the queue");
    println!("  2. Pop task, poll → Pending, waker re-queues it");
    println!("  3. Pop task, poll → Pending, waker re-queues it");
    println!("  4. Pop task, poll → Ready, task is done");
    println!("  5. Queue empty — all tasks complete");
}

fn demo_join_handle() {
    println!("=== JoinHandle ===");
    println!("Getting a result from a spawned task.");
    println!();
    println!("TODO: implement JoinHandle<T> and spawn_with_handle()");
    println!("See Exercise 4 in 4-tasks.md");
    // TODO: when JoinHandle is implemented, demo it here
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::BuildTask => demo_build_task(),
        Command::TaskWaker => demo_task_waker(),
        Command::Lifecycle => demo_lifecycle(),
        Command::JoinHandle => demo_join_handle(),
        Command::All => {
            demo_build_task();
            println!("\n");
            demo_task_waker();
            println!("\n");
            demo_lifecycle();
            println!("\n");
            demo_join_handle();
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
    fn task_creation() {
        let queue: TaskQueue = Arc::new(Mutex::new(VecDeque::new()));
        let task = Task::new(async {}, queue.clone());
        assert_eq!(Arc::strong_count(&task), 1);
    }

    #[test]
    fn waker_requeues_task() {
        let queue: TaskQueue = Arc::new(Mutex::new(VecDeque::new()));
        spawn(
            async {
                let _ = CountdownFuture { name: "t".into(), count: 1 }.await;
            },
            &queue,
        );

        assert_eq!(queue.lock().unwrap().len(), 1);
        poll_one(&queue); // Pending → waker pushes back
        assert_eq!(queue.lock().unwrap().len(), 1, "Task should be re-queued after Pending");
    }

    #[test]
    fn task_completes_and_leaves_queue() {
        let queue: TaskQueue = Arc::new(Mutex::new(VecDeque::new()));
        spawn(
            async {
                let _ = CountdownFuture { name: "t".into(), count: 0 }.await;
            },
            &queue,
        );

        // CountdownFuture(0) is Ready on first poll
        // But the async block wrapping it may need 2 polls
        while !queue.lock().unwrap().is_empty() {
            poll_one(&queue);
        }
        assert_eq!(queue.lock().unwrap().len(), 0, "Queue should be empty after completion");
    }

    #[test]
    fn multiple_tasks_all_complete() {
        let queue: TaskQueue = Arc::new(Mutex::new(VecDeque::new()));
        spawn(async { let _ = CountdownFuture { name: "a".into(), count: 2 }.await; }, &queue);
        spawn(async { let _ = CountdownFuture { name: "b".into(), count: 1 }.await; }, &queue);
        spawn(async { let _ = CountdownFuture { name: "c".into(), count: 3 }.await; }, &queue);

        let mut polls = 0;
        while !queue.lock().unwrap().is_empty() {
            poll_one(&queue);
            polls += 1;
            if polls > 100 { panic!("Too many polls — possible infinite loop"); }
        }
    }
}

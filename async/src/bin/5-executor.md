# Lesson 5: A Minimal Executor

> **Prerequisites**: Lesson 4 (Tasks) — you should understand what a Task is before building the executor that drives them.

## What is a Task? (quick recap)

We've been using the word "task" loosely. Let's define it precisely.

**A future** is a state machine that can be polled. It's just a struct that implements `Future`. It doesn't know about executors, queues, or scheduling. It's passive — it sits there until someone calls `poll()`.

**A task** is a future that has been **spawned** onto an executor. It's the executor's unit of work — a future wrapped with everything the executor needs to manage it.

```
Future alone:                    Task (future + executor metadata):
┌──────────────────┐             ┌──────────────────────────────┐
│  impl Future     │             │  Task                        │
│                  │             │                              │
│  poll() → Ready  │             │  future: Pin<Box<dyn Future>>│
│        → Pending │             │  waker: Waker                │
│                  │             │  state: Running | Completed  │
└──────────────────┘             │  queue: Arc<Mutex<VecDeque>> │
                                 │                              │
 Just a struct.                  └──────────────────────────────┘
 Can't run itself.
                                  Knows how to re-schedule itself.
                                  The executor polls this.
```

### Analogy

- **Future** = a recipe (instructions for making a dish)
- **Task** = a kitchen ticket (recipe + order number + "notify table 5 when done" + position in the queue)

The chef (executor) works with tickets, not raw recipes. The ticket tracks everything needed to manage the order.

### At the Rust code level

Here's what a `Task` looks like in a real executor:

```rust
struct Task {
    /// The future this task is driving to completion.
    /// Pinned because futures may be self-referential (Lesson 5).
    /// Boxed because different tasks hold different future types.
    /// Mutex because the executor and waker may access it from different contexts.
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,

    /// A reference to the executor's task queue.
    /// When waker.wake() is called, the task pushes itself back into this queue.
    queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
}
```

And `spawn` creates a task from a future:

```rust
fn spawn(future: impl Future<Output = ()> + Send + 'static) -> Arc<Task> {
    let task = Arc::new(Task {
        future: Mutex::new(Box::pin(future)),
        queue: queue.clone(),
    });
    queue.lock().unwrap().push_back(task.clone());
    task
}
```

### The lifecycle of a task

```
1. spawn(my_future)
   │
   ▼
2. Task created: wraps future + gets a queue reference
   │
   ▼
3. Task pushed to executor's queue
   │
   ▼
4. Executor pops task, builds a Waker for it, calls task.future.poll(cx)
   │
   ├── Poll::Ready → task is done, drop it
   │
   └── Poll::Pending → task is NOT in the queue
       │
       ▼
5. ... time passes, I/O event or timer fires ...
   │
   ▼
6. waker.wake() → pushes Arc<Task> back into the queue
   │
   ▼
7. Executor pops it again → back to step 4
```

The key insight: **the waker closes over the task**. When you call `waker.wake()`, it pushes `Arc<Task>` back into the queue. This is the connection between the future world (poll/wake) and the executor world (queue/schedule).

### Task vs Future: when people say "task"

In async Rust conversations:
- "Spawn a task" = call `tokio::spawn(future)` — wraps the future in a task and schedules it
- "The task is blocked" = the task returned `Pending` and is waiting for a wake
- "Task-local storage" = per-task data (like thread-local, but per task)
- "Task dump" = list all tasks and what state they're in (debugging)

Every `tokio::spawn()` creates one task. Every `.await` inside that task is a state transition within the same task — NOT a new task.

```rust
tokio::spawn(async {        // ← this is ONE task
    let a = foo().await;    // ← state transition within the task
    let b = bar().await;    // ← another state transition, same task
    a + b
});

tokio::spawn(async {        // ← this is a SECOND task
    baz().await;
});
```

## Real-life analogy: the project manager

A project manager doesn't do the work — they coordinate. They have a list of tasks and a team:

```
Project Manager (executor):
  ┌─────────────────────────────────────┐
  │ Task list:                          │
  │   [ ] Design mockups (waiting)      │
  │   [ ] Write API (waiting)           │
  │   [→] Review PR (in progress)       │
  │   [✓] Deploy staging (done)         │
  └─────────────────────────────────────┘

Loop:
  1. Pick the next task that needs attention
  2. Ask: "are you done yet?" (poll)
  3. If done → mark complete, move on
  4. If not → task says "I'll ping you when ready" (waker)
  5. If nothing needs attention → take a nap (park)
  6. Get pinged (waker.wake()) → wake up, go to step 1
```

That's an executor. It's a loop that polls futures and sleeps when there's nothing to do.

## Two levels of executor

### Level 1: `block_on` (runs one future)

The simplest possible executor. Runs a single future on the current thread:

```
block_on(future):
  ┌──────────────────────────────────────┐
  │  loop {                              │
  │      poll(future)                    │
  │      if Ready → return result        │
  │      if Pending → park thread        │
  │      ... waker fires → unpark ...    │
  │  }                                   │
  └──────────────────────────────────────┘
```

This is what `tokio::runtime::Runtime::block_on()` does at its core.

### Level 2: Multi-task executor (runs many futures)

Adds a task queue. `spawn()` adds futures to the queue. The executor polls them round-robin:

```
Executor:
  ┌──────────────────────────────────────────────┐
  │                                              │
  │  Task Queue: [task_1, task_2, task_3]        │
  │                                              │
  │  loop {                                      │
  │      task = queue.pop()                      │
  │      poll(task)                              │
  │      if Ready → done, don't re-queue         │
  │      if Pending → waker will re-queue it     │
  │      if queue empty → park thread            │
  │  }                                           │
  └──────────────────────────────────────────────┘
```

The key insight: the `Waker` for each task pushes it back into the queue when `wake()` is called. The executor only polls tasks that are ready to make progress.

```
Task 1 returns Pending
  → future stores waker
  → task is NOT in the queue (nothing to do)
  → ... time passes ...
  → I/O event fires → waker.wake()
  → task is pushed back into queue
  → executor pops it, polls it → Ready!
```

## The waker-queue connection

This is the part that makes executors work:

```
spawn(future):
  1. Wrap future in a Task (Arc<Task>)
  2. Create a Waker whose wake() pushes Arc<Task> to the queue
  3. Push task to queue

poll(task):
  1. Pop task from queue
  2. Build Context with the task's waker
  3. Call future.poll(cx)
  4. If Pending → nothing (waker will re-queue when ready)
  5. If Ready → done
```

```
┌─────────────┐      ┌──────────────────┐
│  Executor   │      │  Task            │
│             │      │                  │
│  queue: ────┤      │  future: ...     │
│  [t1,t2,t3] │      │  waker: ────────┐│
│             │      │                 ││
└─────────────┘      └─────────────────┘│
       ▲                                │
       │            wake() pushes       │
       └────────── task back to queue ──┘
```

## The DelayFuture: a real timer

Now that you have an executor with a real waker (not noop), you can build a future that actually waits for real time:

```rust
struct DelayFuture {
    message: String,
    deadline: Instant,
    waker_set: bool,
}
```

How it works:
1. First poll: spawn a background thread that sleeps until the deadline, then calls `waker.wake()`
2. Return `Pending`
3. Background thread wakes up → calls `waker.wake()` → executor re-polls
4. Second poll: deadline has passed → return `Ready(message)`

```
Executor                    DelayFuture               Background Thread
   │                           │                           │
   ├── poll() ───────────►     │                           │
   │                           │── spawn thread ──────────►│
   │                           │   (sleeps 2 seconds)      │
   │  ◄── Pending ─────────────┤                           │
   │                           │                           │
   ├── park() (sleeping)       │                    (sleeping)
   │                           │                           │
   │                           │              ... 2 sec ...│
   │                           │                           │
   │                           │  ◄── waker.wake() ────────┤
   │  ◄── unpark! ─────────────────────────────────────────┤
   │                           │                           │
   ├── poll() ───────────►     │                           │
   │  ◄── Ready("done!") ──────┤                           │
```

This is how `tokio::time::sleep` works — except tokio uses a timer wheel instead of spawning a thread per timer.

## Exercises

### Exercise 1: block_on

Implement `block_on<F: Future>(future: F) -> F::Output`:
1. Build a thread-parking waker (from Lesson 3)
2. Loop: poll the future
3. If `Ready` → return the value
4. If `Pending` → `thread::park()` (waker will unpark)

Test with `CountdownFuture`.

### Exercise 2: Multi-task executor

Implement an `Executor` with:
- `spawn(future)` — wraps the future in an `Arc<Task>`, adds to queue
- `run()` — pops tasks, polls them, sleeps when empty

The tricky part: building a waker whose `wake()` pushes the `Arc<Task>` back into a shared queue (`Arc<Mutex<VecDeque<Arc<Task>>>>`).

Test: spawn 3 `CountdownFuture`s with different counts. Print when each completes. They should interleave.

### Exercise 3: DelayFuture (real timer)

Implement `DelayFuture`:
1. First poll: clone the waker, spawn a thread that sleeps then calls `waker.wake()`
2. Return `Pending`
3. Second poll: check if deadline passed → `Ready(message)`

Test with `block_on`:
```rust
block_on(DelayFuture::new(Duration::from_secs(2), "hello from the future!"))
```

This should print after exactly 2 seconds — proving that the executor slept efficiently (not busy-polling).

### Exercise 4: Spawn DelayFutures concurrently

Using your multi-task executor, spawn three delays:
```rust
executor.spawn(DelayFuture::new(Duration::from_secs(3), "slow"));
executor.spawn(DelayFuture::new(Duration::from_secs(1), "fast"));
executor.spawn(DelayFuture::new(Duration::from_secs(2), "medium"));
```

They should complete in order: fast (1s), medium (2s), slow (3s) — all finishing within ~3 seconds total (concurrent), not 6 seconds (sequential).

### Exercise 5: JoinHandle

Make `spawn()` return a `JoinHandle<T>` — a future that resolves to the spawned task's output.

```rust
let handle = executor.spawn(async { 42 });
let result = block_on(handle);
assert_eq!(result, 42);
```

Implement with `Arc<Mutex<Option<T>>>` shared between the task and the handle, plus a waker for the handle to be notified when the task completes.

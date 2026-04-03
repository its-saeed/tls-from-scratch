# Lesson 4: A Minimal Executor

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

# Lesson 4: Tasks

## Real-life analogy: the post office

A **letter** (future) is just content — it can't deliver itself. To get it somewhere, you put it in an **envelope** (task) with:
- The letter inside (the future)
- A return address (the waker — how to notify when done)
- A tracking number (so the system can find it)
- A destination queue (which mailbag it goes in)

The **postal worker** (executor) doesn't handle raw letters. They handle envelopes — because envelopes have all the metadata needed for delivery.

```
Letter (Future):                    Envelope (Task):
┌──────────────┐                    ┌───────────────────────────┐
│ Dear Bob,    │                    │ To: task queue            │
│ ...content...│                    │ Return addr: waker        │
│              │                    │ Tracking: Arc<Task>       │
└──────────────┘                    │ ┌──────────────┐         │
                                    │ │ Dear Bob,    │         │
 Can't deliver itself.              │ │ ...content...│         │
                                    │ └──────────────┘         │
                                    └───────────────────────────┘
                                     The system can route this.
```

## Future vs Task

A **future** is a struct implementing `Future`. It's passive — it just sits there until someone calls `poll()`.

A **task** is a future **wrapped with executor metadata**: a waker, a queue reference, and shared ownership via `Arc`. It's the unit of work the executor manages.

```
spawn(future) → Task → pushed to queue → executor polls it
```

You write futures. The executor manages tasks.

## What a Task looks like in Rust

```rust
struct Task {
    /// The future, pinned and boxed.
    /// - Box: because different tasks hold different future types (type erasure)
    /// - Pin: because futures may be self-referential (Lesson 6)
    /// - Mutex: because the executor thread and waker may access concurrently
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,

    /// Reference to the executor's task queue.
    /// The waker uses this to push the task back when wake() is called.
    queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
}
```

Why each part:
- **`dyn Future<Output = ()>`** — type erasure. The executor holds many different future types in one queue.
- **`Box`** — puts the future on the heap (required for `dyn`).
- **`Pin`** — prevents moving the future after first poll (required by the `Future` trait).
- **`Mutex`** — interior mutability. We need `&mut` access to poll, but the task is shared via `Arc`.
- **`Send`** — the task might move between threads (multi-threaded executor).
- **`Arc<Task>`** — shared ownership. Both the executor queue and the waker hold references.

## The lifecycle of a task

```
                        spawn(my_future)
                              │
                              ▼
               ┌──────────────────────────┐
               │ Task created             │
               │  future = Box::pin(f)    │
               │  queue = executor.queue  │
               └────────────┬─────────────┘
                            │
                            ▼
               ┌──────────────────────────┐
               │ Pushed to executor queue │
               │  queue: [... , task]     │
               └────────────┬─────────────┘
                            │
                            ▼
               ┌──────────────────────────┐
               │ Executor pops task       │◄──────────────────┐
               │ Builds waker for it      │                   │
               │ Calls task.future.poll() │                   │
               └────────────┬─────────────┘                   │
                            │                                 │
                   ┌────────┴────────┐                        │
                   │                 │                        │
                   ▼                 ▼                        │
            Poll::Ready        Poll::Pending                  │
               │                    │                         │
               ▼                    ▼                         │
           Task done.        Task NOT in queue.               │
           Drop it.          Waiting for event.               │
                                    │                         │
                                    ▼                         │
                             Event fires                      │
                             waker.wake()  ───────────────────┘
                             pushes Arc<Task> back to queue
```

## The waker-task connection

This is the critical piece. Each task gets a waker whose `wake()` pushes the task back into the queue:

```rust
fn create_waker_for_task(task: Arc<Task>) -> Waker {
    // The waker's data pointer is the Arc<Task>
    // wake() does: task.queue.lock().push_back(task.clone())
}
```

When a future inside a task calls `cx.waker().wake_by_ref()`:
1. The waker grabs its `Arc<Task>`
2. Pushes the `Arc<Task>` into the executor's queue
3. The executor wakes up (if parked) and polls the task again

```
Future calls:          cx.waker().wake_by_ref()
                              │
Waker does:            queue.lock().push_back(arc_task.clone())
                              │
Executor sees:         queue is non-empty → pop task → poll it
```

## The `'static` requirement

When you call `tokio::spawn(future)`, the future must be `'static`. Why?

```rust
fn bad_example() {
    let data = vec![1, 2, 3];

    tokio::spawn(async {
        println!("{:?}", data);  // ERROR: `data` doesn't live long enough
    });

    // `data` is dropped here, but the task might still be running!
}
```

The task lives independently — it might outlive the function that spawned it. So it can't borrow local variables. It must own everything it needs.

Fix: move ownership into the task:

```rust
fn good_example() {
    let data = vec![1, 2, 3];

    tokio::spawn(async move {  // `move` transfers ownership
        println!("{:?}", data);  // task owns `data`
    });
    // `data` has been moved, can't use it here
}
```

## The `Send` requirement

For multi-threaded executors, tasks must be `Send` — they might be polled on different threads.

```rust
// This WON'T compile with tokio::spawn:
let rc = Rc::new(42);  // Rc is !Send
tokio::spawn(async move {
    println!("{}", rc);  // ERROR: Rc cannot be sent between threads
});

// Fix: use Arc instead of Rc
let arc = Arc::new(42);  // Arc is Send
tokio::spawn(async move {
    println!("{}", arc);  // OK
});
```

A future is `Send` if all values it holds across `.await` points are `Send`. If you hold a `MutexGuard` (which is `!Send`) across an `.await`, the future becomes `!Send` and can't be spawned.

## `.await` is NOT a new task

A common confusion:

```rust
tokio::spawn(async {        // ← ONE task
    let a = foo().await;    // ← state transition within the task
    let b = bar().await;    // ← another state transition, same task
    a + b                   // all inside one task
});

tokio::spawn(async {        // ← SECOND task (independent)
    baz().await;
});
```

`spawn()` creates a task. `.await` is a yield point within a task. Two awaits in one async block = one task with two state transitions. Two `spawn()` calls = two tasks that run concurrently.

## JoinHandle: getting a result from a task

`spawn()` returns a `JoinHandle` — a future that resolves when the task completes:

```rust
let handle = tokio::spawn(async { 42 });
let result = handle.await.unwrap();  // 42
```

Internally, `JoinHandle` is:
```rust
struct JoinHandle<T> {
    result: Arc<Mutex<Option<T>>>,  // shared with the task
    waker: Arc<Mutex<Option<Waker>>>,  // notified when task completes
}
```

When the task finishes, it stores the result and wakes the `JoinHandle`'s waker. When you `.await` the handle, it checks if the result is ready.

## Exercises

### Exercise 1: Build a Task struct

Define a `Task` struct with:
- A pinned, boxed, type-erased future
- A reference to a shared task queue

Create a task from a `CountdownFuture`. Don't poll it yet — just verify you can construct it.

### Exercise 2: Create a waker for a task

Build a `Waker` whose `wake()` pushes `Arc<Task>` back into the queue.

1. Store `Arc<Task>` as the waker's data pointer (`Arc::into_raw`)
2. `wake()` recovers the Arc, locks the queue, pushes the task
3. Poll a task using this waker. Verify the task re-appears in the queue after returning `Pending`.

### Exercise 3: Task lifecycle

Implement the full lifecycle:
1. Create a task queue (`Arc<Mutex<VecDeque<Arc<Task>>>>`)
2. Spawn a `CountdownFuture(3)` into it
3. Loop: pop task, create waker, poll, check queue
4. Print the queue state after each poll
5. Verify: task appears in queue after Pending, disappears after Ready

### Exercise 4: JoinHandle

Implement a simple `JoinHandle<T>`:
1. `spawn()` returns a `JoinHandle` alongside the task
2. Both share an `Arc<Mutex<Option<T>>>`
3. When the task's future completes, store the result
4. `JoinHandle` implements `Future` — polls check if result is available
5. Test: spawn a future that returns 42, await the handle, assert 42

### Exercise 5: 'static and Send verification

Write futures that fail to compile and understand why:
1. A future that borrows a local variable → fails `'static`
2. A future that holds `Rc<T>` across an await → fails `Send`
3. Fix both — use `move` and `Arc` respectively

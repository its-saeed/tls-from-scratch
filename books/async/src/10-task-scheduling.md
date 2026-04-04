# Lesson 10: Task Scheduling

> **Prerequisites**: Lesson 4 (Tasks), Lesson 5 (Executor), Lesson 9 (Reactor). You need to understand what a task is and how the reactor wakes them.

## Real-life analogy: the hospital ER

An emergency room triages patients:

```
┌──────────────────────────────────────────────────────┐
│  ER Waiting Room (Run Queue)                         │
│                                                      │
│  [Patient A: broken arm]  ← arrived first            │
│  [Patient B: headache]    ← arrived second           │
│  [Patient C: chest pain]  ← arrived third            │
│                                                      │
│  Triage Nurse (Scheduler):                           │
│    FIFO: treat A first (arrived first)               │
│    Priority: treat C first (most urgent)             │
│    Fair: give each patient 5 minutes, rotate         │
│                                                      │
│  Doctor (Executor):                                  │
│    Takes next patient from queue                     │
│    Examines them (poll)                              │
│    If done → discharge (Ready)                       │
│    If not → send back to waiting room (Pending)      │
│    If needs X-ray → nurse will call them (waker)     │
└──────────────────────────────────────────────────────┘
```

Different scheduling strategies:
- **FIFO**: first in, first out. Simple, fair-ish, what we build first.
- **Priority**: urgent tasks first. Used for I/O vs timers.
- **Round-robin**: each task gets a time slice. Prevents starvation.
- **Work-stealing**: multiple doctors, idle ones steal from busy queues. Lesson 14.

## What is a scheduler?

The scheduler decides **which task to poll next**. In Lesson 5, we had a simple loop:

```rust
// Lesson 5: naive approach
while let Some(task) = queue.pop_front() {
    poll(task);  // poll whatever's next in the queue
}
```

This works but has problems:
- A task that always returns `Pending` and immediately wakes itself monopolizes the CPU
- No way to prioritize I/O-ready tasks over timer tasks
- No fairness guarantee

## The run queue

The run queue is where tasks wait to be polled. When `waker.wake()` is called, the task is pushed to the back of the queue:

```
                    Run Queue (VecDeque<Arc<Task>>)
                    ┌───────────────────────────────┐
        push back → │ task_C │ task_A │ task_B │    │ ← pop front
                    └───────────────────────────────┘

Executor loop:
  1. Pop task_B from front
  2. Poll it
  3. If Pending → waker will push it back later
  4. If Ready → done, drop the task
  5. Pop next (task_A)
  6. ...
```

### Fairness

A task that wakes itself immediately goes to the **back** of the queue. This gives other tasks a chance to run:

```
Queue: [A, B, C]
  Poll A → Pending, wakes self → queue: [B, C, A]
  Poll B → Pending, wakes self → queue: [C, A, B]
  Poll C → Ready → queue: [A, B]
  Poll A → Ready → queue: [B]
  Poll B → Ready → queue: []
```

Each task gets one turn per cycle. This is cooperative multitasking — tasks must **yield** (return Pending) to let others run. A task that never yields starves everyone.

## The ArcWake pattern

In Lesson 3, we built wakers manually with `RawWaker`. There's a cleaner way — implement the `Wake` trait:

```rust
use std::task::Wake;

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        // Push ourselves back into the queue
        self.queue.lock().unwrap().push_back(self.clone());
    }
}
```

Then create a waker from an Arc<Task>:

```rust
let waker = Waker::from(task.clone());  // calls task.wake() when woken
```

No unsafe code. The `Wake` trait handles the vtable for you.

```
Lesson 3 (manual):                    Lesson 10 (Wake trait):
  RawWaker { data, vtable }            impl Wake for Task {
  unsafe fn clone/wake/drop                fn wake(self: Arc<Self>) { ... }
  → error-prone, lots of unsafe        }
                                        Waker::from(arc_task)
                                        → safe, clean, idiomatic
```

## JoinHandle: getting results from tasks

When you `spawn()`, you want to get the result back:

```rust
let handle = spawn(async { 42 });
let result = handle.await;  // 42
```

`JoinHandle<T>` is a future that resolves when the task completes:

```
┌─────────────────────────────────────────────────────┐
│  Shared state (Arc<Mutex<JoinState>>)               │
│                                                     │
│  result: Option<T>       ← None until task finishes │
│  waker: Option<Waker>   ← set by JoinHandle::poll  │
│                                                     │
│  Task writes result, wakes JoinHandle               │
│  JoinHandle checks result                           │
└──────────────────┬──────────────────────────────────┘
                   │
          shared by both sides
                   │
    ┌──────────────┴──────────────┐
    │                             │
    ▼                             ▼
┌─────────┐                ┌──────────────┐
│  Task   │                │  JoinHandle  │
│         │                │  (a Future)  │
│ runs    │                │              │
│ future  │   completes    │ poll():      │
│ stores  │ ──────────────►│  result set? │
│ result  │                │  yes → Ready │
│ wakes   │                │  no → Pending│
│ handle  │                │              │
└─────────┘                └──────────────┘
```

Implementation:

```rust
struct JoinState<T> {
    result: Option<T>,
    waker: Option<Waker>,
}

struct JoinHandle<T> {
    state: Arc<Mutex<JoinState<T>>>,
}

impl<T> Future for JoinHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<T> {
        let mut state = self.state.lock().unwrap();
        if let Some(result) = state.result.take() {
            Poll::Ready(result)
        } else {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
```

## Putting it together: the full executor

```rust
struct Executor {
    queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
    reactor: Reactor,
}

impl Executor {
    fn spawn<T>(&self, future: impl Future<Output=T> + Send + 'static) -> JoinHandle<T> {
        // 1. Create shared JoinState
        // 2. Wrap future: when it completes, store result and wake handle
        // 3. Create Task with the wrapped future
        // 4. Push to queue
        // 5. Return JoinHandle
    }

    fn run(&mut self) {
        loop {
            // 1. Drain queue, poll each task
            let tasks: Vec<_> = self.queue.lock().unwrap().drain(..).collect();

            if tasks.is_empty() {
                // 2. Nothing to do → block on reactor
                self.reactor.wait(None);
                continue;
            }

            for task in tasks {
                let waker = Waker::from(task.clone());
                let mut cx = Context::from_waker(&waker);
                let mut future = task.future.lock().unwrap();
                let _ = future.as_mut().poll(&mut cx);
                // If Ready → task dropped (not re-queued)
                // If Pending → waker will re-queue later
            }
        }
    }
}
```

## Exercises

### Exercise 1: Spawn and join

Implement `spawn()` returning a `JoinHandle<T>`. Spawn three tasks that each return a number. Await all three handles, assert the sum.

```rust
let a = executor.spawn(async { 10 });
let b = executor.spawn(async { 20 });
let c = executor.spawn(async { 30 });
// a.await + b.await + c.await == 60
```

### Exercise 2: Task fairness

Spawn a "greedy" task that yields 1000 times (returns Pending + wakes each time) and a "quick" task that completes in one poll. Track the order of completions. The quick task should complete within a few polls, not after 1000 — proving FIFO fairness.

### Exercise 3: Starvation detection

Spawn a task that never yields (infinite loop inside poll). Show that other tasks never run. Then add a yield point (`YieldOnce` from Lesson 2). Show that fairness is restored.

This demonstrates why async is **cooperative** — tasks must yield voluntarily.

### Exercise 4: JoinHandle cancellation

Drop a `JoinHandle` before the task completes. Add a `cancelled` flag to `JoinState` — when the handle is dropped, set the flag. The task checks the flag on each poll and aborts early if cancelled.

Test: spawn a task that increments a counter each poll. Drop the handle after 3 polls. Assert the counter is 3, not more.

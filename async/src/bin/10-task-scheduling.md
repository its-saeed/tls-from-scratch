# Lesson 9: Task Scheduling

Define the `Task` struct, `JoinHandle`, `spawn()`, and a run queue that drives
futures to completion. This is where your executor stops being a toy loop and
starts looking like a real runtime.

## What you'll build

- A `Task` that wraps a `BoxFuture<'static, T>` behind an `Arc` so it can be
  woken from anywhere
- A `JoinHandle<T>` that is itself a future -- awaiting it yields the task's
  output
- A `spawn()` function that boxes the future, allocates a task, and pushes it
  onto a run queue
- A FIFO run queue (start with `VecDeque`, upgrade to `crossbeam` later)
- A `block_on()` that drains the queue until the root future completes

## Key concepts

- **Task lifecycle** -- spawned -> queued -> polling -> (pending | ready)
- **ArcWake pattern** -- implement `Wake` for your `Task` so the waker just
  re-enqueues the Arc
- **Run queue** -- simple FIFO queue of tasks that are ready to be polled
- **JoinHandle** -- shared state between spawner and joiner; needs a slot for
  the result and a waker for the joiner
- **Cancellation** -- dropping the JoinHandle can optionally cancel the task

## Exercises

1. **Spawn and join** -- implement `spawn()` and `JoinHandle`. Spawn three
   tasks that each return a number and assert the sum from the joining side.

2. **Task fairness** -- spawn a task that yields 1000 times (using a manual
   `Poll::Pending` + re-wake). Verify it does not starve a second task that
   only needs one poll.

3. **JoinHandle drop = cancel** -- implement cancellation on JoinHandle drop.
   Spawn a task that writes to a shared flag, drop the handle before the task
   runs, and assert the flag is never set.

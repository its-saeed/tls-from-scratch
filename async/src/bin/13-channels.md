# Lesson 12: Channels

Build async oneshot and mpsc channels that use waker integration instead of
thread blocking. These are the fundamental communication primitives between
async tasks.

## What you'll build

- **Oneshot channel** -- a `(Sender<T>, Receiver<T>)` pair where the receiver
  is a future that resolves when the sender sends a value or is dropped
- **MPSC channel** -- multiple-producer, single-consumer with a bounded buffer;
  `send()` is a future that waits for capacity, `recv()` is a future that waits
  for a message
- Shared inner state protected by `Mutex` (or lock-free if you're ambitious)
- Waker storage so the receiver wakes when data arrives and the sender wakes
  when buffer space opens

## Key concepts

- **Waker hand-off** -- the receiver stores its `Waker` in shared state; the
  sender calls `wake()` after depositing data
- **Closed detection** -- dropping one half signals the other; `RecvError` /
  `SendError` carry this information
- **Bounded vs unbounded** -- bounded channels provide backpressure; unbounded
  channels risk OOM
- **Fairness** -- when multiple senders contend, who gets notified first when
  space opens?
- **Lock granularity** -- hold the lock only to swap data and waker; never hold
  it across an await

## Exercises

1. **Oneshot** -- implement oneshot, spawn two tasks: one sends a value after a
   delay, the other awaits it. Assert correctness.

2. **Bounded MPSC** -- implement a bounded(4) mpsc. Spawn 8 producers that each
   send one message and 1 consumer. Verify all 8 messages arrive and that
   producers actually wait when the buffer is full.

3. **Select over channels** -- using the select from Lesson 14 (or a manual
   poll loop), receive from two mpsc channels and print whichever delivers
   first.

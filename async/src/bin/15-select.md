# Lesson 14: Select Internals

Race multiple futures against each other. When one completes, cancel the rest.
Understand the cancellation and drop semantics that make `select!` tricky.

## What you'll build

- A `select(a, b)` function that polls two futures and returns whichever
  finishes first, dropping the loser
- A `select!` macro that generalizes to N branches with pattern matching
- Demonstration of cancellation safety: what state is lost when a future is
  dropped mid-await
- A `Fuse` wrapper that makes a future safe to poll after completion (returns
  `Pending` forever)

## Key concepts

- **Poll both, return first** -- each call to the select future polls all
  branches; the first `Ready` wins
- **Drop = cancellation** -- the losing future is dropped immediately; any
  in-progress I/O is abandoned
- **Cancellation safety** -- a future is cancellation-safe if dropping it after
  `Pending` loses no data (e.g., `recv()` is safe, `read_exact()` is not)
- **Bias** -- polling order matters; naive top-to-bottom polling starves lower
  branches; randomize or rotate
- **Fuse** -- once a future returns `Ready`, polling it again is UB in general;
  `Fuse` guards against this
- **Pin projection** -- select needs to pin each branch; understand pin
  projections for struct fields

## Exercises

1. **Binary select** -- implement `select(fut_a, fut_b) -> Either<A, B>`. Test
   with two sleeps of different durations and assert the shorter one wins.

2. **Cancellation demo** -- create a future that increments a counter each time
   it is polled. Select it against a ready future. Assert the counter shows it
   was polled exactly once before being dropped.

3. **Select loop** -- use your select in a loop: receive from a channel OR a
   timer. Print "tick" on timer and "msg: ..." on channel. Exit when the
   channel closes.

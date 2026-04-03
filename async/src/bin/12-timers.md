# Lesson 11: Timers

Add time awareness to your runtime: a timer heap that tracks deadlines, a
`sleep()` future, and integration with the reactor's poll timeout.

## What you'll build

- A `TimerHeap` backed by `BinaryHeap` that stores `(Instant, Waker)` entries
- A `Sleep` future returned by `sleep(duration)` -- registers a deadline and
  returns `Pending` until the deadline passes
- Integration with the reactor: before calling `mio::Poll::poll()`, compute the
  timeout from the nearest deadline in the heap
- A `timeout()` combinator: wraps any future and returns an error if it does
  not complete within a duration
- A `Interval` stream that yields at regular intervals

## Key concepts

- **Min-heap** -- the smallest deadline sits at the top; after each poll loop
  iteration, pop all expired entries and wake them
- **Poll timeout** -- the reactor blocks in `poll(timeout)`; set timeout to
  time-until-next-deadline so timers fire promptly
- **Cancellation** -- if a `Sleep` is dropped, its entry should be removed (or
  lazily ignored on expiry)
- **Clock sources** -- `Instant::now()` vs test clocks; abstracting time for
  deterministic tests
- **Thundering herd** -- many timers expiring at once; batch wakeups

## Exercises

1. **sleep + print** -- implement `sleep()` and run three tasks that each sleep
   for a different duration and print when they wake. Verify ordering.

2. **timeout combinator** -- implement `timeout(dur, future)`. Wrap a sleep(5s)
   in a timeout(1s) and assert you get a `TimedOut` error.

3. **Interval stream** -- implement `interval(dur)` that yields `()` every
   `dur`. Use it to print a heartbeat message every 500ms while another task
   does work.

# Lesson 13: Work-Stealing Scheduler

Build a multi-threaded runtime where each worker thread has a local run queue
and can steal tasks from other workers when idle. This is the architecture
behind tokio's multi-threaded scheduler.

## What you'll build

- **Worker threads** -- N threads (one per core), each running its own poll
  loop
- **Local queues** -- each worker has a bounded local deque; newly spawned tasks
  go to the local queue first
- **Global queue** -- overflow and cross-thread spawns land in a shared MPMC
  queue
- **Stealing** -- an idle worker picks a random peer and steals half of its
  local queue
- **Shutdown** -- a shared flag that workers check; drain remaining tasks on
  shutdown

## Key concepts

- **Cache locality** -- tasks that spawn children tend to run on the same core,
  keeping data in L1/L2
- **Chase-Lev deque** -- the classic data structure for work-stealing: push/pop
  from one end (owner), steal from the other (thieves)
- **Randomized stealing** -- picking a random victim avoids contention hotspots
- **Parking / unparking** -- idle workers park (condvar or eventfd); a new task
  unparks one worker
- **LIFO slot** -- tokio optimization: the most recently spawned task gets a
  fast-path slot polled before the queue

## Exercises

1. **Two-thread runtime** -- build a minimal work-stealing runtime with 2
   workers. Spawn 100 tasks that each record `thread::current().id()` and
   verify both threads did work.

2. **Steal half** -- implement the "steal half" policy. Spawn 50 tasks on
   worker 0, none on worker 1. Assert that worker 1 eventually runs some of
   them.

3. **Benchmark** -- compare throughput (tasks/sec) of your work-stealing
   runtime vs the single-threaded executor from Lesson 9. Spawn 10k trivial
   tasks and measure wall-clock time.

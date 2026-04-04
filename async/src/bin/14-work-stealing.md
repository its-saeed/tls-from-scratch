# Lesson 14: Work-Stealing Scheduler

> **Prerequisites**: Lesson 10 (Task Scheduling), Lesson 13 (Channels). You need a working single-threaded executor before going multi-threaded.

## Real-life analogy: supermarket checkout lanes

```
Single-threaded executor = one checkout lane:
┌──────────────────────────────────────────┐
│  Lane 1: [customer] [customer] [customer]│  One cashier.
│                                          │  Long line.
└──────────────────────────────────────────┘

Multi-threaded (no stealing) = N lanes, unbalanced:
┌──────────────────────────────────────────┐
│  Lane 1: [customer] [customer] [customer]│  Busy!
│  Lane 2: [customer]                      │  Almost idle.
│  Lane 3:                                 │  Empty!
│  Lane 4: [customer] [customer]           │
└──────────────────────────────────────────┘
  Some lanes are jammed while others are empty.

Work-stealing = lanes help each other:
┌──────────────────────────────────────────┐
│  Lane 1: [customer] [customer]           │
│  Lane 2: [customer] ←── stole from lane 1│
│  Lane 3: [customer] ←── stole from lane 1│
│  Lane 4: [customer]                      │
└──────────────────────────────────────────┘
  Idle cashiers walk to busy lanes and take customers.
  Result: balanced load, shorter wait times.
```

## Why work-stealing?

With N worker threads, you need a strategy for distributing tasks:

```
Strategy 1: Shared queue (simple, bad)
  All workers pop from ONE queue.
  Problem: lock contention. N threads fighting over one Mutex.

  ┌────────┐  ┌────────┐  ┌────────┐
  │Worker 0│  │Worker 1│  │Worker 2│
  └───┬────┘  └───┬────┘  └───┬────┘
      │           │           │
      └─────┬─────┴─────┬────┘
            │            │
       ┌────▼────────────▼────┐
       │  Shared Queue (Mutex)│  ← contention!
       └──────────────────────┘

Strategy 2: Local queues + work stealing (complex, fast)
  Each worker has its own queue. No contention for local work.
  When idle, steal from a random busy worker.

  ┌────────┐  ┌────────┐  ┌────────┐
  │Worker 0│  │Worker 1│  │Worker 2│
  │[t1,t2] │  │[t3]    │  │[]      │ ← empty, will steal
  └────────┘  └────────┘  └───┬────┘
                               │ steal half from Worker 0
                               ▼
  ┌────────┐  ┌────────┐  ┌────────┐
  │Worker 0│  │Worker 1│  │Worker 2│
  │[t1]    │  │[t3]    │  │[t2]    │ ← balanced!
  └────────┘  └────────┘  └────────┘
```

## Architecture

```
┌────────────────────────────────────────────────────────┐
│  Work-Stealing Runtime                                 │
│                                                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Global Queue (for overflow + cross-thread spawn)│  │
│  │  Arc<Mutex<VecDeque<Arc<Task>>>>                  │  │
│  └──────────┬──────────────┬──────────┬─────────────┘  │
│             │              │          │                 │
│  ┌──────────▼───┐  ┌──────▼──────┐  ┌▼────────────┐  │
│  │  Worker 0    │  │  Worker 1   │  │  Worker 2   │  │
│  │              │  │             │  │             │  │
│  │  local queue │  │ local queue │  │ local queue │  │
│  │  [t1, t2]   │  │ [t3]        │  │ []          │  │
│  │              │  │             │  │   ↑ steal   │  │
│  │  thread 0    │  │  thread 1   │  │  thread 2   │  │
│  └──────────────┘  └─────────────┘  └─────────────┘  │
│                                                        │
│  Each worker:                                          │
│    1. Pop from local queue                             │
│    2. If empty → check global queue                    │
│    3. If empty → steal from random worker              │
│    4. If nothing → park (sleep)                        │
└────────────────────────────────────────────────────────┘
```

## The worker loop

```rust
fn worker_loop(worker_id: usize, workers: &[Worker], global: &GlobalQueue) {
    loop {
        // 1. Try local queue first (no contention)
        if let Some(task) = self.local_queue.pop() {
            poll(task);
            continue;
        }

        // 2. Try global queue (shared, needs lock)
        if let Some(task) = global.pop() {
            poll(task);
            continue;
        }

        // 3. Try stealing from a random peer
        let victim = random_worker(workers, worker_id);
        if let Some(task) = victim.local_queue.steal() {
            poll(task);
            continue;
        }

        // 4. Nothing to do — park
        thread::park();
    }
}
```

## The Chase-Lev deque

The data structure that makes work-stealing efficient:

```
Owner (the worker thread):
  push to back:   O(1), no synchronization
  pop from back:  O(1), no synchronization (LIFO — most recent first)

Stealers (other workers):
  steal from front: O(1), uses atomic CAS (compare-and-swap)

┌──────────────────────────────────┐
│  front (stealers)                │
│  ↓                               │
│  [task_A] [task_B] [task_C]      │
│                              ↑   │
│                    back (owner)   │
└──────────────────────────────────┘

Owner pushes/pops from back (fast, no lock).
Stealers steal from front (atomic, minimal contention).
```

In Rust, use the `crossbeam-deque` crate:

```rust
use crossbeam_deque::{Worker, Stealer};

let worker = Worker::new_fifo();
let stealer = worker.stealer();  // can be cloned to other threads

worker.push(task);         // owner pushes
worker.pop();              // owner pops
stealer.steal();           // other thread steals
stealer.steal_batch(&other_worker);  // steal half
```

## Spawn: where does the task go?

```
spawn(future):
  Am I on a worker thread?
    Yes → push to my local queue (fast path)
    No  → push to global queue (cross-thread spawn)
```

Tokio uses thread-local storage to detect if `spawn` is called from a worker thread. If so, the task goes directly into the local queue (no lock, no contention).

## Parking and unparking

When a worker has nothing to do, it should sleep (not busy-poll):

```rust
// Worker with nothing to do:
thread::park();  // sleep, 0% CPU

// When a new task is spawned or a waker fires:
worker_thread.unpark();  // wake up!
```

The tricky part: deciding WHICH worker to unpark. Options:
- **Unpark one random idle worker** (tokio's approach)
- **Unpark all idle workers** (simple but thundering herd)

## Exercises

### Exercise 1: Two-worker runtime

Build a work-stealing runtime with 2 worker threads:

1. Each worker has a `VecDeque` local queue
2. A shared global queue for overflow
3. Workers pop local → pop global → steal from peer → park
4. Spawn 100 tasks that record `std::thread::current().id()`
5. Verify both threads ran tasks

### Exercise 2: Steal half

Implement "steal half" — when stealing, take half the victim's local queue:

```rust
fn steal_batch(victim: &LocalQueue, thief: &mut LocalQueue) {
    let count = victim.len() / 2;
    for _ in 0..count {
        if let Some(task) = victim.steal_front() {
            thief.push_back(task);
        }
    }
}
```

Test: push 50 tasks to Worker 0, none to Worker 1. Run. Assert Worker 1 processed some.

### Exercise 3: Benchmark

Compare throughput:
- Single-threaded executor (Lesson 10): spawn 10K tasks, measure time
- Work-stealing with 4 workers: same 10K tasks

Tasks should be trivial (increment atomic counter). Print tasks/second for each.

### Exercise 4: crossbeam-deque

Replace your `VecDeque` local queue with `crossbeam_deque::Worker`. Compare performance. The crossbeam deque uses lock-free atomics instead of a Mutex — it should be faster under contention.

Add to Cargo.toml: `crossbeam-deque = "0.8"`

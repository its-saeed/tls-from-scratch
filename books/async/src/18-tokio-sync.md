# Lesson 18: tokio::sync Internals — Mutex, RwLock, Semaphore, Notify

> **Prerequisites**: Lessons 13-17. You know how channels work — now see the lower-level primitives.

## Real-life analogy: bathroom keys at a gas station

```
┌─────────────────────────────────────────────────────────────────┐
│  Gas Station Bathroom Access                                    │
│                                                                 │
│  Mutex (one key):                                               │
│    🔑 One key on the counter. Take it, use bathroom, return.   │
│    If key is gone → WAIT (don't block the road, sit in car).   │
│                                                                 │
│  Semaphore (multiple keys):                                     │
│    🔑🔑🔑 Three keys → three stalls. Fourth person waits.      │
│    Each key = one permit.                                       │
│                                                                 │
│  RwLock (museum analogy):                                       │
│    Many visitors can READ the exhibit simultaneously.           │
│    But the curator needs EXCLUSIVE access to rearrange.         │
│    Readers share, writers exclude everyone.                     │
│                                                                 │
│  Notify (doorbell):                                             │
│    Ring the bell → someone waiting inside wakes up.             │
│    No data transferred, just "hey, something happened."         │
└─────────────────────────────────────────────────────────────────┘
```

## Why not std::sync in async code?

```
std::sync::Mutex::lock()              tokio::sync::Mutex::lock()
┌──────────────────────┐              ┌──────────────────────┐
│ Thread blocks (OS)   │              │ Task yields (Pending) │
│ Worker thread frozen │              │ Worker thread FREE    │
│ Other tasks starve   │              │ Runs other tasks      │
│ Deadlock risk!       │              │ Wakes when lock free  │
└──────────────────────┘              └──────────────────────┘

Rule of thumb:
  - Lock NOT held across .await → std::sync::Mutex is fine (faster)
  - Lock held across .await     → MUST use tokio::sync::Mutex
```

## Mutex vs RwLock

| Aspect | `Mutex<T>` | `RwLock<T>` |
|--------|-----------|-------------|
| Readers | One at a time | Many concurrent |
| Writers | Exclusive | Exclusive |
| Overhead | Lower | Higher (tracking readers) |
| Use when | Writes are frequent | Reads dominate |

```rust
use tokio::sync::RwLock;

let lock = RwLock::new(HashMap::new());

// Many tasks can read concurrently
let guard = lock.read().await;

// Only one task can write
let mut guard = lock.write().await;
guard.insert("key", "value");
```

## Semaphore — the universal primitive

```
Semaphore(3):       ┌───┬───┬───┐
  Available permits │ ● │ ● │ ● │
                    └───┴───┴───┘

Task A acquires 1:  ┌───┬───┬───┐
                    │   │ ● │ ● │  (2 left)
                    └───┴───┴───┘

Task B acquires 2:  ┌───┬───┬───┐
                    │   │   │   │  (0 left)
                    └───┴───┴───┘

Task C acquires 1:  WAITS... (no permits)
Task A drops:       Task C wakes up, gets permit
```

Semaphore is the building block for:
- **Rate limiting** — permits = max concurrent operations
- **Connection pooling** — permits = max connections
- **Bounded channels** — tokio uses it internally

```rust
let sem = Arc::new(Semaphore::new(10));
let permit = sem.acquire().await?; // blocks if 10 already held
do_work().await;
drop(permit); // returns permit, wakes a waiter
```

## Notify — async signaling

`Notify` is the simplest primitive: no data, just "wake up."

```
Notifier                    Waiter
   │                          │
   │                          ├── notified().await  (Pending)
   │                          │       ...sleeping...
   ├── notify_one() ──────────┤
   │                          ├── wakes up!
```

### Building a simple async mutex with Notify

```rust
use tokio::sync::Notify;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

struct SimpleMutex<T> {
    locked: AtomicBool,
    notify: Notify,
    data: UnsafeCell<T>,
}

// The pattern: spin on atomic + sleep on Notify
// Real tokio::sync::Mutex uses a wait queue, not spinning
```

## OwnedPermit and OwnedMutexGuard

When you need to move a guard into a spawned task:

```rust
let mutex = Arc::new(tokio::sync::Mutex::new(0));
let owned_guard = mutex.clone().lock_owned().await; // 'static lifetime
tokio::spawn(async move {
    // owned_guard is Send + 'static — works in spawned tasks
    drop(owned_guard);
});
```

## Exercises

### Exercise 1: Shared counter with tokio::sync::Mutex

Spawn 100 tasks that each increment a `tokio::sync::Mutex<u64>` counter. Verify the final count is 100.

### Exercise 2: Rate limiter with Semaphore

Create a `Semaphore::new(5)`. Spawn 20 tasks that each acquire a permit, do `tokio::time::sleep(100ms)`, then release. Verify at most 5 run concurrently at any time.

### Exercise 3: Producer-consumer with Notify

Build a queue using `std::sync::Mutex<VecDeque<T>>` + `Notify`. The producer pushes items and calls `notify_one()`. The consumer loops on `notified().await` + try-pop.

### Exercise 4: Build a simple async Mutex using Notify

Implement a `SimpleMutex<T>` that uses `AtomicBool` for state and `Notify` for waking. It won't be production-quality but demonstrates the concept.

# Lesson 17: tokio::sync Internals — Mutex, Semaphore, Notify

## What you'll learn

- Why `std::sync::Mutex` can be problematic in async code
- How `tokio::sync::Mutex` uses async-aware locking
- The `Semaphore` as the foundational primitive in Tokio's sync
- Using `Notify` for async signaling without data

## Key concepts

### std::sync::Mutex vs tokio::sync::Mutex

| Aspect | `std::sync::Mutex` | `tokio::sync::Mutex` |
|--------|---------------------|----------------------|
| Blocking | Blocks OS thread | Yields to executor |
| Hold across `.await` | Dangerous (blocks worker) | Safe |
| Performance | Faster for short critical sections | Higher overhead |
| Rule of thumb | Use when lock is never held across `.await` | Use when it must be |

### Semaphore

`tokio::sync::Semaphore` is the building block for many patterns:
- Rate limiting (acquire permit before work)
- Connection pooling (permits = max connections)
- Bounded channels use it internally

```rust
let sem = Arc::new(Semaphore::new(10));
let permit = sem.acquire().await?;
// do work
drop(permit); // release
```

### Notify

`Notify` is a lightweight async signal — no data, just "wake up":

```rust
let notify = Arc::new(Notify::new());
// Waiter
notify.notified().await;
// Notifier
notify.notify_one();
```

### OwnedSemaphorePermit

`acquire_owned()` returns a permit that is `'static`, useful when you need to move it into a spawned task.

## Exercises

1. Create a shared counter protected by `tokio::sync::Mutex`, increment from 100 concurrent tasks
2. Implement a rate limiter using `Semaphore` — allow at most 5 concurrent HTTP requests
3. Build a producer-consumer pattern using only `Notify` and a `VecDeque` behind a std `Mutex`
4. Demonstrate the deadlock risk of holding `std::sync::Mutex` across an `.await` point
5. Use `Semaphore::acquire_many()` to implement weighted resource access

# Lesson 21: Graceful Shutdown — CancellationToken, Signal Handling, Drain Pattern

> **Prerequisites**: Lessons 15 (select!), 18 (tokio::sync).

## Real-life analogy: closing a restaurant

```
┌────────────────────────────────────────────────────────────────┐
│  Closing Time Protocol                                         │
│                                                                │
│  Phase 1: STOP ACCEPTING                                       │
│    Manager locks the front door.                               │
│    No new customers allowed.                                   │
│    → Stop calling listener.accept()                            │
│                                                                │
│  Phase 2: SIGNAL WORKERS                                       │
│    Manager tells all waiters: "We're closing."                 │
│    → CancellationToken::cancel()                               │
│                                                                │
│  Phase 3: DRAIN                                                │
│    Let current diners finish their meals.                      │
│    Waiters complete current tables, don't start new ones.      │
│    → Wait for in-flight tasks to complete                      │
│                                                                │
│  Phase 4: HARD DEADLINE                                        │
│    "Kitchen closes in 5 minutes, eat or leave."                │
│    → tokio::time::timeout on the drain                         │
│                                                                │
│  Phase 5: CLEANUP                                              │
│    Turn off lights, lock up, save state.                       │
│    → Flush logs, close DB connections                          │
└────────────────────────────────────────────────────────────────┘
```

## Architecture

```
                    ┌──────────────┐
  Ctrl+C ──────────►  Signal       │
  SIGTERM ─────────►  Handler      │
                    └──────┬───────┘
                           │ cancel()
                    ┌──────▼───────┐
                    │ Cancellation  │
                    │ Token (root)  │
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
        child_token   child_token  child_token
              │            │            │
          ┌───▼───┐   ┌───▼───┐   ┌───▼───┐
          │Task A │   │Task B │   │Task C │
          │select!│   │select!│   │select!│
          │cancel │   │cancel │   │cancel │
          │or work│   │or work│   │or work│
          └───────┘   └───────┘   └───────┘
```

## Signal handling

```rust
use tokio::signal;

// Simple: wait for Ctrl+C
signal::ctrl_c().await?;

// In a server: use select! to race shutdown against work
tokio::select! {
    _ = signal::ctrl_c() => {
        println!("shutdown signal received");
    }
    _ = server.run() => {
        println!("server exited on its own");
    }
}
```

## CancellationToken

From `tokio_util::sync` (but we can build the pattern with `tokio::sync::Notify`):

```rust
// Using Notify as a poor-man's CancellationToken
let shutdown = Arc::new(Notify::new());

// In signal handler:
shutdown.notify_waiters();   // wake ALL waiters

// In each worker task:
tokio::select! {
    _ = shutdown.notified() => {
        println!("shutting down");
        return;
    }
    result = do_work() => {
        // process result
    }
}
```

## The drain pattern

```rust
let shutdown = Arc::new(Notify::new());
let in_flight = Arc::new(AtomicUsize::new(0));
let all_done = Arc::new(Notify::new());

// Worker tasks:
in_flight.fetch_add(1, Ordering::SeqCst);
// ... do work ...
if in_flight.fetch_sub(1, Ordering::SeqCst) == 1 {
    all_done.notify_one();  // last task done
}

// Shutdown sequence:
shutdown.notify_waiters();             // Phase 2: signal
tokio::time::timeout(                  // Phase 4: hard deadline
    Duration::from_secs(30),
    all_done.notified()                // Phase 3: wait for drain
).await;
```

## DropGuard pattern

Ensure cancellation even if the shutdown logic panics:

```rust
// Wrapping in a struct whose Drop triggers cleanup
struct ShutdownGuard {
    notify: Arc<Notify>,
}
impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        self.notify.notify_waiters();
    }
}
```

## Exercises

### Exercise 1: Ctrl+C echo server

Build a TCP echo server that stops accepting on Ctrl+C, finishes active connections, then exits.

### Exercise 2: Notify-based shutdown hierarchy

Create a parent `Notify` and 5 worker tasks. Each worker does a loop of work with `select!` checking for shutdown. Cancel all workers, verify they all exit.

### Exercise 3: Drain with in-flight counter

Track in-flight requests with `AtomicUsize`. After signaling shutdown, wait until the counter reaches zero (use `Notify`). Add a 5-second hard timeout.

### Exercise 4: Graceful shutdown with state persistence

On shutdown, serialize current application state (e.g., a counter value) to a file. On restart, load it back. This simulates real-world graceful shutdown where you save progress.

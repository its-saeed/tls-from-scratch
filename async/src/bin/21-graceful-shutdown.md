# Lesson 20: Graceful Shutdown — CancellationToken, Signal Handling, Drain Pattern

## What you'll learn

- How to catch OS signals (SIGINT, SIGTERM) in Tokio
- Using `CancellationToken` to propagate shutdown across tasks
- The drain pattern: stop accepting new work, finish in-flight work
- Combining shutdown with timeouts for hard deadlines

## Key concepts

### Signal handling

```rust
use tokio::signal;

tokio::select! {
    _ = signal::ctrl_c() => {
        println!("shutdown signal received");
    }
    _ = server.run() => {}
}
```

### CancellationToken

From `tokio_util::sync`:

```rust
let token = CancellationToken::new();
let child = token.child_token(); // hierarchy

// In worker tasks:
tokio::select! {
    _ = child.cancelled() => return,
    result = do_work() => { /* process */ }
}

// Trigger shutdown:
token.cancel(); // cancels all children too
```

### The drain pattern

1. **Stop accepting** — break the accept loop
2. **Signal workers** — cancel the token
3. **Wait for completion** — `join` on task handles or wait on a `WaitGroup`
4. **Timeout** — if tasks don't finish, force exit

```rust
token.cancel();
if tokio::time::timeout(Duration::from_secs(30), drain_tasks()).await.is_err() {
    eprintln!("forced shutdown after timeout");
}
```

### DropGuard

`token.drop_guard()` returns a guard that cancels the token when dropped, useful for RAII-style cleanup.

## Exercises

1. Build a TCP server that shuts down gracefully on Ctrl+C, finishing active connections
2. Create a `CancellationToken` hierarchy: parent cancels all child workers
3. Implement the drain pattern with a `tokio::sync::Semaphore` to track in-flight requests
4. Add a hard timeout: if drain takes longer than 10 seconds, force exit
5. Use `DropGuard` to ensure cleanup even if shutdown logic panics

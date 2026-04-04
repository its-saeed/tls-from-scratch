# Lesson 24: Cancellation Safety — Dropped Futures, Data Loss, Safe Patterns

## What you'll learn

- What "cancellation safe" means for async functions
- How `select!` can cause data loss with unsafe futures
- Identifying cancellation-unsafe operations
- Patterns to make code cancellation-safe

## Key concepts

### The danger

When `tokio::select!` picks one branch, the other futures are **dropped**. If a dropped future had partially completed work (e.g., read some bytes into a buffer), that work is lost.

```rust
// UNSAFE: if sleep wins, partial read data is lost
tokio::select! {
    result = reader.read(&mut buf) => { /* ... */ }
    _ = tokio::time::sleep(timeout) => { /* timeout */ }
}
```

### Cancellation-safe vs unsafe

| Safe | Unsafe |
|------|--------|
| `tokio::sync::mpsc::Receiver::recv()` | `tokio::io::AsyncReadExt::read_exact()` |
| `TcpListener::accept()` | `tokio::io::AsyncReadExt::read()` with reused buffer |
| `tokio::time::sleep()` | Futures that do partial work before first `.await` |

A future is cancellation-safe if dropping it after any `.await` point loses no data.

### Safe patterns

**Pattern 1: Use cancellation-safe alternatives**
```rust
// Use recv() in select — it's cancellation-safe
tokio::select! {
    msg = rx.recv() => { /* no data loss */ }
    _ = token.cancelled() => { return; }
}
```

**Pattern 2: Move work into a spawned task**
```rust
let handle = tokio::spawn(async { read_exact(&mut buf).await });
tokio::select! {
    result = handle => { /* ... */ }
    _ = shutdown => { /* handle still runs to completion */ }
}
```

**Pattern 3: Pin and reuse the future**
```rust
let read_fut = pin!(reader.read(&mut buf));
// Reuse across select iterations instead of recreating
```

## Exercises

1. Demonstrate data loss: use `select!` with `read_exact` and a timer, show bytes vanish
2. Fix the above using a spawned task
3. Write a cancellation-safe message reader that accumulates bytes across `select!` iterations
4. Audit `tokio::sync` docs — list which methods are cancellation-safe and which are not
5. Implement a `CancellationSafeReader` wrapper that buffers partial reads

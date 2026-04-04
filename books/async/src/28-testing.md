# Lesson 28: Testing Async Code — tokio::test, Time Mocking, Deterministic Testing

## What you'll learn

- Using `#[tokio::test]` for async unit tests
- Mocking time with `tokio::time::pause()` and `advance()`
- Deterministic testing with `current_thread` runtime
- Testing patterns for channels, tasks, and I/O

## Key concepts

### #[tokio::test]

```rust
#[tokio::test]
async fn test_something() {
    let result = my_async_fn().await;
    assert_eq!(result, 42);
}
```

By default, uses `current_thread` runtime. For multi-thread: `#[tokio::test(flavor = "multi_thread")]`.

### Time mocking

`pause()` freezes time; `advance()` moves it forward instantly:

```rust
#[tokio::test]
async fn test_timeout() {
    tokio::time::pause();

    let start = Instant::now();
    tokio::time::sleep(Duration::from_secs(3600)).await;

    // Completes instantly — time is mocked
    assert!(start.elapsed() >= Duration::from_secs(3600));
}
```

Auto-advance: when all tasks are waiting on time, the runtime jumps to the next timer automatically.

### Deterministic testing

`current_thread` runtime is deterministic — tasks run in a predictable order. Useful for reproducing race conditions.

### Testing patterns

| Pattern | Approach |
|---------|----------|
| Test a spawned task | Use `JoinHandle` to await result |
| Test channels | Create channel, send, recv, assert |
| Test shutdown | Create `CancellationToken`, cancel, verify cleanup |
| Test I/O | Use `tokio::io::duplex()` for in-memory streams |
| Test timeouts | Pause time, advance past deadline |

### tokio::io::duplex

```rust
let (client, server) = tokio::io::duplex(1024);
// Use client and server as AsyncRead + AsyncWrite
// No real TCP needed
```

## Exercises

1. Write a `#[tokio::test]` that verifies an async function returns the correct value
2. Use `time::pause()` and `advance()` to test a retry function with exponential backoff (no real waiting)
3. Test a producer-consumer pipeline using `mpsc` channels
4. Use `tokio::io::duplex()` to test a protocol parser without real sockets
5. Test graceful shutdown: cancel a token, verify all tasks exit cleanly

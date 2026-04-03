# Lesson 15: Tokio Architecture — Runtime Builder, current_thread vs multi_thread, Driver

## What you'll learn

- How Tokio's runtime is structured internally
- The difference between `current_thread` and `multi_thread` runtimes
- How the driver layer (I/O, time, signal) integrates with the scheduler
- Configuring the runtime via `Builder`

## Key concepts

### Runtime structure

Tokio's runtime has three main components:
1. **Scheduler** — decides which tasks run and on which threads
2. **Driver** — reacts to external events (I/O readiness, timer expiry, signals)
3. **Resource drivers** — I/O driver (wraps mio), time driver, signal driver

### current_thread vs multi_thread

| Aspect | `current_thread` | `multi_thread` |
|--------|-------------------|----------------|
| Threads | 1 | N (default: CPU cores) |
| Work stealing | No | Yes |
| `Send` requirement | Not required for `spawn_local` | Required for `spawn` |
| Best for | Tests, simple apps, WASM | Production servers |

### Runtime Builder

```rust
let rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(4)
    .enable_all()        // enables I/O + time drivers
    .build()?;
```

### Driver stack

The driver is layered: signal driver wraps time driver wraps I/O driver. Each `park()` call propagates down the stack, letting all drivers process events.

### Block-on entry point

`Runtime::block_on()` parks the current thread on the driver, polling the provided future and processing driver events between polls.

## Exercises

1. Build a `current_thread` runtime manually and run a simple TCP echo server on it
2. Build a `multi_thread` runtime with 2 worker threads; print `std::thread::current().id()` from spawned tasks to observe thread distribution
3. Create a runtime with only `enable_io()` (no time driver) and observe what happens when you call `tokio::time::sleep`
4. Use `runtime::Handle::current()` to spawn work from outside the runtime
5. Compare throughput of `current_thread` vs `multi_thread` for a simple echo server under load

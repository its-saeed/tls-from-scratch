# Lesson 24: Bridging Sync and Async — block_on, spawn_blocking, Handle

## What you'll learn

- Calling async code from sync code (`block_on`, `Handle::block_on`)
- Calling sync/blocking code from async code (`spawn_blocking`)
- The blocking thread pool and its configuration
- Common pitfalls and anti-patterns

## Key concepts

### Async from sync: block_on

```rust
let rt = tokio::runtime::Runtime::new()?;
let result = rt.block_on(async {
    fetch_data().await
});
```

`block_on` parks the current thread until the future completes. Never call it from inside an async context (deadlock).

### Sync from async: spawn_blocking

```rust
let hash = tokio::task::spawn_blocking(move || {
    // CPU-heavy or blocking I/O — runs on dedicated thread pool
    compute_bcrypt_hash(&password)
}).await?;
```

The blocking pool has up to 512 threads by default. Tasks here do not block the async worker threads.

### Handle for deferred async access

```rust
let handle = tokio::runtime::Handle::current();

std::thread::spawn(move || {
    // From a plain OS thread, run async code:
    handle.block_on(async {
        client.get(url).send().await
    });
});
```

### Anti-patterns

| Anti-pattern | Problem | Fix |
|-------------|---------|-----|
| `block_on` inside async | Deadlock | Use `.await` |
| Blocking in async task | Starves workers | `spawn_blocking` |
| `spawn_blocking` for I/O | Wastes pool threads | Use async I/O |
| Nested runtimes | Panic | Use `Handle::current()` |

## Exercises

1. Call an async HTTP client from a synchronous `main` using `block_on`
2. Use `spawn_blocking` to offload a CPU-heavy Fibonacci computation
3. Pass a `Handle` to a std thread and use it to run async DNS resolution
4. Demonstrate the deadlock when calling `block_on` inside an async task
5. Configure the blocking thread pool size with `max_blocking_threads` and observe behavior under load

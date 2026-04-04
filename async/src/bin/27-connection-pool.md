# Lesson 27: Connection Pooling — Reuse, Health Checks, Idle Timeout

## What you'll learn

- Why connection pooling matters (TCP handshake, TLS, auth overhead)
- Building a pool with `Semaphore` + `VecDeque`
- Health checking idle connections
- Idle timeout and eviction strategies

## Key concepts

### Why pool?

Each new TCP connection costs: DNS lookup, TCP handshake (1 RTT), TLS handshake (1-2 RTT), and often authentication. Reusing connections amortizes this cost.

### Pool architecture

```
checkout() -> acquire Semaphore permit
           -> pop from idle queue (or create new)
           -> health check
           -> return PooledConnection

drop(PooledConnection) -> health check
                       -> push to idle queue
                       -> release Semaphore permit
```

### Core components

```rust
struct Pool {
    idle: Mutex<VecDeque<Connection>>,
    semaphore: Semaphore,        // limits total connections
    max_idle: usize,
    idle_timeout: Duration,
}
```

### Health checks

- **On checkout** — ping before returning to caller
- **On return** — verify connection is still usable
- **Background** — periodic sweep of idle connections

### Idle timeout

Connections sitting idle too long may be closed by the server or a firewall. Evict them:

```rust
// Background task
loop {
    tokio::time::sleep(Duration::from_secs(30)).await;
    pool.evict_idle_older_than(idle_timeout);
}
```

### Real-world pools

- `deadpool` — generic async pool
- `bb8` — based on r2d2's design
- `sqlx` — built-in pool for database connections

## Exercises

1. Build a simple TCP connection pool using `Semaphore` and `VecDeque`
2. Add a health check that sends a ping before returning a connection
3. Implement idle timeout eviction with a background reaper task
4. Add metrics: total connections, idle connections, wait time
5. Stress test: 100 concurrent tasks sharing a pool of 10 connections

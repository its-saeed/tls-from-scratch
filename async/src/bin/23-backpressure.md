# Lesson 22: Backpressure — Bounded Channels, Semaphore, Flow Control

## What you'll learn

- What backpressure is and why unbounded queues are dangerous
- Using bounded channels to propagate backpressure
- Semaphore-based flow control for non-channel patterns
- Detecting and responding to overload

## Key concepts

### The problem

Without backpressure, a fast producer overwhelms a slow consumer. Memory grows unbounded, latency spikes, and eventually the system crashes (OOM).

### Bounded channels

```rust
let (tx, rx) = tokio::sync::mpsc::channel(100); // buffer of 100

// Producer blocks (awaits) when buffer is full
tx.send(item).await?;
```

The `send().await` suspends when the buffer is full, naturally slowing the producer.

### Semaphore-based flow control

When you don't use channels (e.g., spawned tasks):

```rust
let sem = Arc::new(Semaphore::new(100));
loop {
    let permit = sem.acquire().await?; // blocks if 100 tasks in-flight
    tokio::spawn(async move {
        do_work().await;
        drop(permit); // releases slot
    });
}
```

### Strategies for overload

| Strategy | Behavior |
|----------|----------|
| Block (await) | Producer waits — backpressure |
| Drop newest | `try_send` fails, drop the item |
| Drop oldest | Evict from buffer head |
| Return error | HTTP 503, GRPC RESOURCE_EXHAUSTED |

### TCP flow control as analogy

TCP's receive window is backpressure at the OS level. Tokio's bounded channels are the application-level equivalent.

## Exercises

1. Create a producer that generates items 10x faster than the consumer; compare bounded vs unbounded channel memory usage
2. Implement a "drop oldest" policy using a `VecDeque` behind a `Mutex`
3. Build a web server that returns 503 when a `Semaphore` is exhausted
4. Chain two bounded channels (producer -> transformer -> consumer) and observe backpressure propagation
5. Measure latency distribution with and without backpressure under overload

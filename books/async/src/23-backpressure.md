# Lesson 23: Backpressure -- Bounded Channels, Semaphore, poll_ready

> **Prerequisites**: Lessons 13 (channels), 18 (tokio::sync), 21 (graceful shutdown).

## Real-life analogy: water pressure in pipes

```
UNBOUNDED (no backpressure):

  Fire hydrant          Tiny garden hose
  =============>>>>>>>>>=============>  BURST!
  (fast producer)       (slow consumer)

  The producer pushes water faster than the pipe can carry.
  Pressure builds. The pipe bursts. Your system OOMs.


BOUNDED (with backpressure):

  Fire hydrant      Pressure valve      Garden hose
  =============>>>> |  BLOCKS  | >>>>=============>
  (fast producer)   (capacity=100)     (slow consumer)

  When the pipe is full, the valve closes.
  The producer STOPS until the consumer drains some water.
  Nobody bursts. The system stays alive.
```

## Architecture: bounded channel flow

```
          Producer                 Consumer
            |                        |
            v                        |
   tx.send(item).await               |
            |                        |
   +--------v--------+              |
   |  Bounded Buffer  |  capacity=4  |
   | [X] [X] [X] [X] |  FULL!       |
   +---------+--------+              |
             |                       |
   Producer SUSPENDS here    rx.recv().await
   until consumer takes one          |
             |                       v
   +--------v--------+       processes item
   | [X] [X] [X] [_] |  one slot free
   +---------+--------+
             |
   Producer WAKES UP, sends next item
```

## Bounded channels

```rust
let (tx, rx) = tokio::sync::mpsc::channel(100); // buffer of 100

// Producer blocks (awaits) when buffer is full
tx.send(item).await?;

// try_send for non-blocking "drop if full" strategy
match tx.try_send(item) {
    Ok(()) => { /* sent */ }
    Err(TrySendError::Full(item)) => { /* channel full, drop or retry */ }
    Err(TrySendError::Closed(item)) => { /* receiver gone */ }
}
```

The `send().await` suspends when the buffer is full, naturally slowing the producer.

## Semaphore-based flow control

When you do not use channels (e.g., spawned tasks hitting an API):

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

## Strategies for overload

| Strategy       | Mechanism             | When to use                    |
|----------------|-----------------------|--------------------------------|
| Block (await)  | `send().await`        | Default -- propagate pressure  |
| Drop newest    | `try_send` + discard  | Metrics, telemetry             |
| Drop oldest    | `VecDeque` pop front  | Real-time video frames         |
| Return error   | HTTP 503              | Load shedding at the edge      |

## TCP analogy

TCP's receive window is backpressure at the OS level. When the receiver's buffer fills, the sender's `write()` blocks. Tokio's bounded channels are the application-level equivalent.

## Exercises

### Exercise 1: Bounded vs unbounded memory

Create a producer that generates items 10x faster than the consumer. Compare memory usage between a bounded channel (capacity 10) and an unbounded channel after 100,000 items.

### Exercise 2: Semaphore rate limiter

Build a rate limiter using `Semaphore` that allows at most 5 concurrent HTTP-style requests. Log when a request waits for a permit vs gets one immediately.

### Exercise 3: Drop-oldest policy

Implement a "drop oldest" buffer using `VecDeque` behind a `Mutex`. When the buffer is full, pop the front before pushing to the back.

### Exercise 4: Chained backpressure

Chain two bounded channels: producer -> transformer -> consumer. Slow down the consumer and observe backpressure propagating all the way to the producer.

# Lesson 15: Select Internals

> **Prerequisites**: Lesson 7 (Combinators), Lesson 13 (Channels). You built a basic `MySelect` in Lesson 7. Now we go deeper into cancellation semantics.

## Real-life analogy: race to the airport

You need to get to the airport. You have two options running in parallel:

```
┌────────────────┐     ┌────────────────┐
│ Option A: Taxi │     │ Option B: Bus  │
│ ETA: unknown   │     │ ETA: 20 min    │
└───────┬────────┘     └───────┬────────┘
        │                      │
        │    ... waiting ...   │
        │                      │
        │                      ▼
        │               Bus arrives!
        │               You get on.
        │
        ▼
  Cancel taxi!  ← this is the DROP
  (taxi driver goes home)
```

**Select = race multiple futures, take the winner, cancel the losers.**

The "cancel" part is where it gets tricky. What if the taxi was already en route? What if it had already picked you up but you hadn't noticed? Cancellation has consequences.

## How select works

```rust
async fn select<A, B>(fut_a: A, fut_b: B) -> Either<A::Output, B::Output>
where A: Future, B: Future
{
    // Internally, each poll():
    //   1. Poll fut_a → Ready? return Left(value), DROP fut_b
    //   2. Poll fut_b → Ready? return Right(value), DROP fut_a
    //   3. Both Pending → Pending
}
```

```
poll #1:
  poll A → Pending
  poll B → Pending
  → return Pending

poll #2:
  poll A → Pending
  poll B → Ready(value)
  → drop A (cancelled!)
  → return Right(value)
```

## The cancellation problem

When a future is dropped mid-execution, any work it did between its last `Pending` and the drop is **lost**:

```
Cancellation-SAFE (no data loss):
  recv() → Pending (no message consumed)
         → dropped (nothing lost)

Cancellation-UNSAFE (data loss):
  read_exact(buf, 100 bytes)
         → read 50 bytes, Pending (50 bytes consumed from socket)
         → dropped (those 50 bytes are GONE, next read misses them)
```

```
┌──────────────────────────────────────────────────────┐
│  Cancellation Safety Rules                           │
│                                                      │
│  SAFE to use in select:                              │
│    ✓ channel.recv()    — no data consumed until Ready│
│    ✓ sleep()           — no side effects             │
│    ✓ listener.accept() — no connection consumed      │
│                                                      │
│  UNSAFE in select:                                   │
│    ✗ read_exact()      — partial reads lost          │
│    ✗ read_line()       — partial line lost           │
│    ✗ collect from stream — partial results lost      │
│                                                      │
│  Rule: if dropping after Pending loses data          │
│        that can't be recovered, it's UNSAFE.         │
└──────────────────────────────────────────────────────┘
```

This is covered in depth in Lesson 24 (Cancellation Safety).

## Polling bias

Naive select always polls A first:

```rust
// BIASED: A always gets polled first
if let Ready(v) = fut_a.poll(cx) { return Left(v); }
if let Ready(v) = fut_b.poll(cx) { return Right(v); }
```

If A is always ready, B is **starved** — never gets a chance. Solutions:
- **Random order**: pick which to poll first randomly each time
- **Round-robin**: alternate A-first and B-first
- **tokio::select!** has a `biased;` option to explicitly choose

## Fuse: poll-after-completion guard

The `Future` contract says: don't poll after `Ready`. But in a select loop, you might accidentally poll a completed future:

```rust
loop {
    select! {
        msg = channel.recv() => handle(msg),
        _ = timer.tick() => println!("tick"),
    }
    // After timer fires, the next iteration polls timer again
    // But it already returned Ready — UB!
}
```

`Fuse` wraps a future to return `Pending` forever after completing:

```rust
struct Fuse<F> {
    future: Option<F>,  // None after completion
}

impl<F: Future> Future for Fuse<F> {
    fn poll(...) -> Poll<...> {
        match &mut self.future {
            Some(f) => match f.poll(cx) {
                Ready(v) => { self.future = None; Ready(v) }
                Pending => Pending
            }
            None => Pending,  // already completed, safe to poll
        }
    }
}
```

## Exercises

### Exercise 1: Binary select

Implement `select(fut_a, fut_b) -> Either<A, B>`:
- Poll both futures
- Return whichever is Ready first
- Drop the loser

Test: `select(sleep(100ms), sleep(500ms))` should return Left after ~100ms.

### Exercise 2: Cancellation demo

Create a `CountingFuture` that increments an `Arc<AtomicU32>` each time it's polled. Select it against an immediately-ready future. Assert the counter shows it was polled exactly once before being dropped.

Add a `Drop` impl that prints "cancelled!" to make the drop visible.

### Exercise 3: Select loop with channel

```rust
let (tx, mut rx) = mpsc::channel();
let mut interval = Interval::new(Duration::from_millis(200));

loop {
    select! {
        msg = rx.recv() => match msg {
            Some(m) => println!("got: {m}"),
            None => break,  // channel closed
        },
        _ = interval.tick() => println!("tick"),
    }
}
```

Implement this pattern. Spawn a task that sends 5 messages with delays, then drops the sender. The select loop should print ticks between messages and exit when the channel closes.

### Exercise 4: Fuse

Implement the `Fuse` wrapper. Test: fuse a future that returns Ready(42). Poll it three times. First poll returns Ready(42), second and third return Pending (not UB).

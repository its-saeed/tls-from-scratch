# Lesson 6: Combinators

## Building complex futures from simple ones

Combinators are futures that wrap other futures to add behavior:

- **join**: poll two futures concurrently, return when BOTH complete
- **select**: poll two futures concurrently, return when EITHER completes (cancel the other)
- **map**: transform the output of a future
- **then**: chain futures sequentially

## How join works

```rust
struct Join<A, B> {
    a: Option<Pin<Box<A>>>,  // None when done
    b: Option<Pin<Box<B>>>,  // None when done
    result_a: Option<A::Output>,
    result_b: Option<B::Output>,
}
```

Each `poll`:
1. If `a` is not done → poll it. If Ready, store result, set to None.
2. If `b` is not done → poll it. If Ready, store result, set to None.
3. If both done → return `Ready((result_a, result_b))`
4. Otherwise → return `Pending`

Both futures make progress concurrently (interleaved on one thread, not parallel).

## How select works

```rust
struct Select<A, B> {
    a: Pin<Box<A>>,
    b: Pin<Box<B>>,
}
```

Each `poll`:
1. Poll `a`. If Ready → drop `b`, return `a`'s result.
2. Poll `b`. If Ready → drop `a`, return `b`'s result.
3. Both Pending → return `Pending`.

The loser gets **dropped** (cancelled). This has implications — Lesson 23 covers cancellation safety.

## Exercises

### Exercise 1: MyJoin
Implement a `Join` future that polls two futures and returns a tuple when both complete. Test with two `CountdownFuture`s with different counts.

### Exercise 2: MySelect
Implement a `Select` future that returns the first result. Verify the other future is dropped (add a `Drop` impl that prints).

### Exercise 3: join_all
Implement `join_all(Vec<impl Future>)` that polls a dynamic number of futures. Return `Vec<Output>` when all complete. This is `FuturesUnordered` in miniature.

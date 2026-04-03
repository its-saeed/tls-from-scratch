# Lesson 25: Streams — Async Iteration, StreamExt, Backpressure

## What you'll learn

- The `Stream` trait as the async equivalent of `Iterator`
- Useful combinators from `StreamExt` and `TryStreamExt`
- Creating streams from channels, iterators, and async generators
- Backpressure considerations with streams

## Key concepts

### The Stream trait

```rust
pub trait Stream {
    type Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<Option<Self::Item>>;
}
```

Like `Iterator`, but `poll_next` can return `Pending`.

### StreamExt combinators

```rust
use tokio_stream::StreamExt;

let mut stream = tokio_stream::iter(vec![1, 2, 3])
    .map(|x| x * 2)
    .filter(|x| *x > 2)
    .take(5);

while let Some(item) = stream.next().await {
    println!("{item}");
}
```

Key combinators: `map`, `filter`, `take`, `merge`, `chain`, `throttle`, `chunks`, `timeout`.

### Creating streams

- `tokio_stream::iter()` — from an iterator
- `ReceiverStream::new(rx)` — from an `mpsc::Receiver`
- `async_stream::stream!` — from an async block with `yield`
- `BroadcastStream` — from a `broadcast::Receiver`

### Backpressure in streams

Streams are pull-based: the consumer calls `next().await`, so the producer only runs when demanded. This provides natural backpressure. Combine with `buffer_unordered(n)` to control concurrency:

```rust
stream
    .map(|url| async move { fetch(url).await })
    .buffer_unordered(10)  // at most 10 concurrent fetches
```

## Exercises

1. Convert an `mpsc::Receiver` into a stream and process items with `StreamExt::map`
2. Use `buffer_unordered` to fetch 100 URLs with max 10 concurrent requests
3. Implement a custom `Stream` that yields Fibonacci numbers with a delay
4. Use `stream.chunks(10)` to batch database inserts
5. Merge two streams and process items in arrival order

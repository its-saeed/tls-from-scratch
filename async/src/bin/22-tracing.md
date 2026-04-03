# Lesson 21: Tracing & Debugging — tokio-console, Tracing Integration, Task Dumps

## What you'll learn

- The `tracing` crate ecosystem and how it fits async Rust
- Structured logging with spans and events
- Using `tokio-console` to inspect live task state
- Task dumps and diagnosing stuck futures

## Key concepts

### tracing vs log

`tracing` is span-aware and async-friendly. Unlike `log`, it tracks structured context across `.await` points.

```rust
use tracing::{info, instrument, span, Level};

#[instrument]
async fn handle_request(id: u64) {
    info!(id, "processing request");
    do_work().await;
    info!("done");
}
```

### Spans in async code

`#[instrument]` automatically creates a span for the function. The span is entered/exited around `.await` points so logs inside nested calls carry the parent context.

### Subscribers and layers

```rust
use tracing_subscriber::{fmt, EnvFilter, prelude::*};

tracing_subscriber::registry()
    .with(fmt::layer())
    .with(EnvFilter::from_default_env())
    .init();
```

Layers compose: formatting, filtering, JSON output, OpenTelemetry export.

### tokio-console

A diagnostic tool that shows live task state:
1. Add `console-subscriber` to your app
2. Run your app with `RUSTFLAGS="--cfg tokio_unstable"`
3. Run `tokio-console` in another terminal
4. See tasks, their polls, waker counts, and durations

### Task dumps

With `tokio_unstable`, `Handle::dump()` captures a snapshot of all task backtraces, useful for finding deadlocks.

## Exercises

1. Add `tracing` and `tracing-subscriber` to a TCP server; log connection events with structured fields
2. Use `#[instrument(skip(stream))]` to avoid logging non-Debug types
3. Set up `tokio-console` and observe task states in a chat server
4. Create a custom tracing layer that counts events per level
5. Simulate a stuck task and use `Handle::dump()` to find it

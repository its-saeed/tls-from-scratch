# Lesson 22: Tracing & Debugging Async Code

> **Prerequisites**: Lesson 16 (Tokio architecture), Lesson 20 (task-locals).

## Real-life analogy: security cameras in a building

```
┌────────────────────────────────────────────────────────────────┐
│  Building Security System                                      │
│                                                                │
│  println!/log = writing on paper:                              │
│    "Something happened at some time."                          │
│    No context, no trail, hard to correlate.                    │
│                                                                │
│  tracing = security cameras with zones:                        │
│    Camera in Lobby (span: "lobby")                             │
│      └── sees Person A enter at 9:01 (event)                  │
│    Camera in Office (span: "office_204")                       │
│      └── sees Person A sit down at 9:03 (event)               │
│      └── sees Person A make a call at 9:05 (event)            │
│                                                                │
│    You can follow Person A across cameras (spans).             │
│    Each camera records structured data (fields).               │
│    Zoom in on one zone or see the whole building.              │
│                                                                │
│  Spans follow a task across .await points,                     │
│  just like cameras track a person across rooms.                │
└────────────────────────────────────────────────────────────────┘
```

## tracing vs log

```
log crate:                           tracing crate:
┌───────────────────────┐            ┌───────────────────────────┐
│ info!("got request")  │            │ info!(id=42, "got req")   │
│                       │            │                           │
│ Plain text messages.  │            │ Structured fields.        │
│ No context tracking.  │            │ Span context across .await│
│ Thread-local at best. │            │ Composable layers.        │
│                       │            │ Works with tokio-console. │
└───────────────────────┘            └───────────────────────────┘
```

## Spans and events

```
Span: "handle_request" (id=42)
  │
  ├── Event: INFO "received request" { method: "GET", path: "/api" }
  │
  ├── Span: "db_query" (table="users")
  │     └── Event: DEBUG "query executed" { rows: 5, ms: 12 }
  │
  └── Event: INFO "response sent" { status: 200, ms: 15 }

Spans are like folders. Events are like log lines inside folders.
Spans carry context that events inherit.
```

## The #[instrument] macro

```rust
use tracing::{info, instrument};

#[instrument(skip(stream), fields(addr = %addr))]
async fn handle_connection(stream: TcpStream, addr: SocketAddr) {
    info!("new connection");           // automatically tagged with addr
    let data = read_data(&stream).await;
    info!(bytes = data.len(), "read complete");
}
```

`#[instrument]` automatically:
- Creates a span named after the function
- Records function arguments as span fields
- Enters/exits the span around `.await` points

## Subscribers and layers

```
┌──────────────────────────────────────────────────────────────┐
│  tracing-subscriber stack                                     │
│                                                               │
│  ┌─────────────────────┐                                      │
│  │  EnvFilter layer    │ ← RUST_LOG=info,my_crate=debug       │
│  └──────────┬──────────┘                                      │
│  ┌──────────▼──────────┐                                      │
│  │  fmt::Layer         │ ← pretty-prints to stderr            │
│  └──────────┬──────────┘                                      │
│  ┌──────────▼──────────┐                                      │
│  │  (optional: JSON)   │ ← machine-readable output            │
│  └──────────┬──────────┘                                      │
│  ┌──────────▼──────────┐                                      │
│  │  Registry           │ ← collects spans + events            │
│  └─────────────────────┘                                      │
└──────────────────────────────────────────────────────────────┘
```

```rust
use tracing_subscriber::{fmt, EnvFilter, prelude::*};

tracing_subscriber::registry()
    .with(EnvFilter::from_default_env())
    .with(fmt::layer())
    .init();
```

## tokio-console

A live diagnostic tool for async applications:

```
1. Add console-subscriber to your app
2. Build with: RUSTFLAGS="--cfg tokio_unstable" cargo build
3. Run your app
4. In another terminal: tokio-console

You see:
  - All tasks: name, state (idle/running/waiting), polls, durations
  - Waker counts: how often each task was woken
  - Task details: where it was spawned, what it's waiting on
```

## Common debugging patterns

| Problem | Tool |
|---------|------|
| "Where is time spent?" | `#[instrument]` + span timing |
| "Why is this task stuck?" | tokio-console task list |
| "What's the call chain?" | Nested spans in log output |
| "Which request caused this?" | Span fields (request_id) |
| "What are all tasks doing?" | `Handle::dump()` (tokio_unstable) |

## Exercises

### Exercise 1: Add structured tracing to a server

Add `tracing` and `tracing-subscriber` to a TCP echo server. Log connection events with structured fields (addr, bytes_read, duration). Use `#[instrument]`.

### Exercise 2: Custom event counter layer

Build a simple tracing layer (implement the `Layer` trait) that counts events per level (INFO, WARN, ERROR). Print the counts on shutdown.

### Exercise 3: Request ID propagation with spans

Create a span with a `request_id` field for each incoming request. Log from nested functions — verify the request_id appears in all log lines automatically.

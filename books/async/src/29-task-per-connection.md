# Pattern 1: Task-per-Connection

## Real-life analogy: the hotel concierge desk

```
Guest arrives → front desk assigns a personal concierge
  Concierge 1: handles Guest A (room service, wake-up call, taxi)
  Concierge 2: handles Guest B (restaurant booking, laundry)
  Concierge 3: handles Guest C (tour arrangements)

Each concierge handles ONE guest's entire stay.
When the guest checks out, the concierge is free.

This is task-per-connection:
  connection arrives → spawn a task → task handles everything → task ends
```

## The pattern

The simplest and most common async architecture. For every incoming connection, spawn a dedicated task:

```rust
loop {
    let (stream, addr) = listener.accept().await?;
    tokio::spawn(async move {
        handle_client(stream).await;
    });
}
```

```
┌──────────────────────────────────────────────────┐
│  Task-per-Connection                             │
│                                                  │
│  Listener                                        │
│    │                                             │
│    ├── accept() → spawn(handle(client_1))        │
│    ├── accept() → spawn(handle(client_2))        │
│    ├── accept() → spawn(handle(client_3))        │
│    └── ...                                       │
│                                                  │
│  Each task:                                      │
│    read request → process → write response       │
│    → loop or disconnect                          │
│                                                  │
│  Tasks are independent. One slow client          │
│  doesn't affect others.                          │
└──────────────────────────────────────────────────┘
```

## When to use

- **Web servers** — each HTTP request gets a task
- **Database servers** — each client connection gets a task
- **Chat servers** — each user gets a task
- **Proxies** — each proxied connection gets a task
- **Game servers** — each player gets a task

Basically: anything that accepts connections and handles them independently.

## When NOT to use

- When connections need to share heavy state (use actors instead)
- When you need to limit concurrency precisely (add a semaphore)
- When tasks need to coordinate tightly (use channels between tasks)

## The concurrency limit problem

Spawning unlimited tasks can exhaust memory:

```
10,000 connections → 10,000 tasks → fine
100,000 connections → 100,000 tasks → maybe fine
1,000,000 connections → 1,000,000 tasks → might OOM
```

Solution: limit concurrent connections with a semaphore:

```rust
let semaphore = Arc::new(Semaphore::new(10_000)); // max 10K concurrent

loop {
    let permit = semaphore.clone().acquire_owned().await.unwrap();
    let (stream, _) = listener.accept().await?;
    tokio::spawn(async move {
        handle_client(stream).await;
        drop(permit); // release the slot
    });
}
```

## Shared state between tasks

Tasks often need shared state (user list, config, counters). Three approaches:

```
Option A: Arc<Mutex<T>>
  Simple. Lock contention if many tasks write.
  Good for: counters, small config objects.

Option B: Arc<RwLock<T>>
  Many readers, few writers.
  Good for: shared config that rarely changes.

Option C: Dedicated state task (Actor pattern → next chapter)
  One task owns the state, others send messages.
  Good for: complex state, no lock contention.
```

## Code exercise: TCP Chat Server

Build a chat server where each client gets a task:

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│ Client A │     │ Client B │     │ Client C │
│  (task)  │     │  (task)  │     │  (task)  │
└────┬─────┘     └────┬─────┘     └────┬─────┘
     │                │                │
     └───────┬────────┴────────┬───────┘
             │                 │
     ┌───────▼─────────────────▼───────┐
     │  Shared state:                  │
     │  HashMap<ClientId, Sender>       │
     │  (behind Arc<Mutex>)            │
     └─────────────────────────────────┘
```

**Requirements**:
1. Accept TCP connections, spawn a task per client
2. Each task reads lines from its client
3. Broadcast messages to all other clients
4. Handle disconnect (remove from shared state)
5. Limit to 100 concurrent connections with a semaphore

**Starter code**:

```rust
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    let semaphore = Arc::new(Semaphore::new(100));
    // TODO: shared state for connected clients

    loop {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let (stream, addr) = listener.accept().await.unwrap();
        // TODO: clone shared state

        tokio::spawn(async move {
            println!("{addr} connected");
            // TODO: handle client (read lines, broadcast, disconnect)
            drop(permit);
        });
    }
}
```

**Test with**: `nc 127.0.0.1 8080` in multiple terminals.

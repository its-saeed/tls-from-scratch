# Pattern 6: Event Bus / Pub-Sub

## Real-life analogy: the radio station

```
┌──────────────┐
│ Radio Station│  broadcasts on 101.5 FM
│ (publisher)  │
└──────┬───────┘
       │ broadcast signal
  ┌────┼──────────────────┐
  │    │                  │
  ▼    ▼                  ▼
┌────┐ ┌────┐       ┌────┐
│Car │ │Home│       │Gym │  ← anyone tuned to 101.5 hears it
│ 🚗 │ │ 🏠 │       │ 🏋️ │
└────┘ └────┘       └────┘

Publisher sends once.
All subscribers receive.
New subscribers can join anytime.
Publisher doesn't know (or care) who's listening.
```

## The pattern

A publisher emits events. Multiple subscribers receive them. Publisher and subscribers are decoupled — they don't know about each other.

```
┌──────────────────────────────────────────────────────────┐
│  Event Bus                                               │
│                                                          │
│  ┌───────────┐     ┌─────────────┐                       │
│  │ Publisher  │────►│  broadcast  │                       │
│  │ (metrics)  │     │  channel    │                       │
│  └───────────┘     └──────┬──────┘                       │
│                           │                              │
│               ┌───────────┼───────────┐                  │
│               │           │           │                  │
│               ▼           ▼           ▼                  │
│         ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│         │Dashboard │ │Logger    │ │Alerter   │          │
│         │subscriber│ │subscriber│ │subscriber│          │
│         │shows live│ │writes to │ │sends     │          │
│         │graphs    │ │disk      │ │Slack msg │          │
│         └──────────┘ └──────────┘ └──────────┘          │
│                                                          │
│  Publisher doesn't know who listens.                     │
│  Subscribers don't know who publishes.                   │
│  New subscribers can join anytime.                       │
└──────────────────────────────────────────────────────────┘
```

## In Rust: tokio::sync::broadcast

```rust
use tokio::sync::broadcast;

let (tx, _) = broadcast::channel(100); // buffer 100 events

// Subscriber 1
let mut rx1 = tx.subscribe();
tokio::spawn(async move {
    while let Ok(event) = rx1.recv().await {
        println!("[dashboard] {event}");
    }
});

// Subscriber 2
let mut rx2 = tx.subscribe();
tokio::spawn(async move {
    while let Ok(event) = rx2.recv().await {
        println!("[logger] {event}");
    }
});

// Publisher
tx.send("user_login".to_string()).unwrap();
tx.send("page_view".to_string()).unwrap();
// Both subscribers receive both events
```

## broadcast vs mpsc

```
mpsc (multi-producer, single consumer):
  10 senders → 1 receiver
  Each message consumed by ONE receiver
  Use for: work queues, actor inboxes

broadcast (single producer, multi consumer):
  1 sender → N receivers
  Each message received by ALL receivers
  Use for: events, notifications, pub-sub

watch (single value, multi reader):
  1 writer → N readers
  Readers always see the LATEST value (not a queue)
  Use for: config changes, "current state" sharing
```

## When to use

- **Event-driven systems** — user actions, system events, state changes
- **Real-time updates** — live dashboards, notification feeds
- **Microservice communication** — services emit events, others react
- **Logging/monitoring** — multiple consumers of the same event stream
- **UI frameworks** — component A changes → components B, C, D update

## When NOT to use

- **Reliable delivery** — broadcast drops messages if a subscriber is slow (use mpsc for guaranteed delivery)
- **Point-to-point** — if only one consumer should handle each message, use mpsc
- **Large payloads** — broadcast clones the message for each subscriber (expensive for big data)

## Filtering events

Subscribers often only care about certain event types:

```rust
#[derive(Clone, Debug)]
enum Event {
    UserLogin { user: String },
    PageView { path: String },
    Error { message: String },
}

// Subscriber that only cares about errors:
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        if let Event::Error { message } = event {
            send_slack_alert(&message).await;
        }
    }
});
```

## Code exercise: Real-time Dashboard

Build a system where services emit metrics and a dashboard displays them live:

```
┌───────────┐     ┌───────────┐
│ Web Server│     │ DB Service│
│ emit:     │     │ emit:     │
│ req_count │     │ query_ms  │
│ latency   │     │ conn_count│
└─────┬─────┘     └─────┬─────┘
      │                 │
      └────────┬────────┘
               ▼
         ┌───────────┐
         │ broadcast  │
         │ channel    │
         └─────┬─────┘
               │
     ┌─────────┼─────────┐
     │         │         │
     ▼         ▼         ▼
┌─────────┐ ┌────────┐ ┌────────┐
│Dashboard│ │Logger  │ │Alerter │
│(print   │ │(write  │ │(if     │
│ every   │ │ to file│ │ error  │
│ 1 sec)  │ │ all)   │ │ rate > │
│         │ │        │ │ 10/min)│
└─────────┘ └────────┘ └────────┘
```

**Requirements**:
1. Define a `Metric` enum: `RequestCount`, `Latency(ms)`, `QueryTime(ms)`, `ErrorCount`
2. Two publisher tasks emit random metrics every 100ms
3. Dashboard subscriber: prints a summary every second
4. Logger subscriber: writes every metric to stdout
5. Alerter subscriber: prints ALERT if error count exceeds threshold

**Starter code**:

```rust
use tokio::sync::broadcast;
use std::time::Duration;

#[derive(Clone, Debug)]
enum Metric {
    Request { path: String, latency_ms: u64 },
    Error { message: String },
    DbQuery { query: String, duration_ms: u64 },
}

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel::<Metric>(256);

    // Publisher: web server metrics
    let tx1 = tx.clone();
    tokio::spawn(async move {
        loop {
            tx1.send(Metric::Request {
                path: "/api/data".into(),
                latency_ms: rand::random::<u64>() % 200,
            }).ok();
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    // TODO: more publishers, dashboard subscriber, logger, alerter
}
```

**Expected output**:
```
[logger] Request { path: "/api/data", latency_ms: 42 }
[logger] DbQuery { query: "SELECT *", duration_ms: 12 }
[logger] Error { message: "timeout" }
[ALERT] Error rate: 3/min — threshold exceeded!
[dashboard] === 1s summary ===
  Requests: 10, avg latency: 87ms
  DB queries: 5, avg: 15ms
  Errors: 1
```

# Pattern 3: Pipeline / Stream Processing

## Real-life analogy: the car factory assembly line

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│ Station 1│───►│ Station 2│───►│ Station 3│───►│ Station 4│
│ Weld     │    │ Paint    │    │ Install  │    │ Quality  │
│ frame    │    │ body     │    │ engine   │    │ check    │
└──────────┘    └──────────┘    └──────────┘    └──────────┘

Car 1: [quality check]
Car 2:           [install engine]
Car 3:                     [paint]
Car 4:                               [weld frame]

All stations work simultaneously on different cars.
If painting is slow, welding pauses (backpressure).
```

## The pattern

A chain of tasks connected by channels. Each stage processes items and passes them to the next:

```
producer → channel → transform → channel → filter → channel → sink
```

```
┌──────────────────────────────────────────────────────────┐
│  Pipeline                                                │
│                                                          │
│  ┌────────┐  ch  ┌──────────┐  ch  ┌────────┐  ch  ┌──┐│
│  │Producer│ ───► │Transform │ ───► │ Filter │ ───► │Out││
│  │        │      │          │      │        │      │  ││
│  │read    │      │parse     │      │errors  │      │  ││
│  │lines   │      │JSON      │      │only    │      │  ││
│  └────────┘      └──────────┘      └────────┘      └──┘│
│                                                          │
│  Each stage is a separate task.                          │
│  Bounded channels provide backpressure.                  │
│  If the filter is slow, transform pauses.                │
└──────────────────────────────────────────────────────────┘
```

```rust
let (tx1, rx1) = mpsc::channel(100);  // producer → parser
let (tx2, rx2) = mpsc::channel(100);  // parser → filter
let (tx3, rx3) = mpsc::channel(100);  // filter → output

// Stage 1: produce
tokio::spawn(async move {
    for line in read_lines("access.log").await {
        tx1.send(line).await.unwrap();
    }
});

// Stage 2: parse
tokio::spawn(async move {
    while let Some(line) = rx1.recv().await {
        if let Ok(record) = parse_json(&line) {
            tx2.send(record).await.unwrap();
        }
    }
});

// Stage 3: filter
tokio::spawn(async move {
    while let Some(record) = rx2.recv().await {
        if record.status >= 500 {
            tx3.send(record).await.unwrap();
        }
    }
});

// Stage 4: output
tokio::spawn(async move {
    while let Some(error) = rx3.recv().await {
        println!("ERROR: {} {}", error.path, error.status);
    }
});
```

## Why bounded channels matter

```
Unbounded channels (BAD):
  Producer: 1,000,000 items/sec
  Consumer: 100 items/sec
  → channel grows to 999,900 items → OOM

Bounded channels (GOOD):
  channel(100)
  Producer: 1,000,000 items/sec
  Consumer: 100 items/sec
  → channel fills to 100 → producer.send().await BLOCKS
  → producer slows down to match consumer
  → memory stays constant
  → this is BACKPRESSURE
```

## When to use

- **Log/event processing** — parse, filter, aggregate, alert
- **ETL pipelines** — extract, transform, load
- **Video/audio processing** — decode → transform → encode
- **Network packet processing** — capture → parse → analyze → store

## When NOT to use

- **Request/response** — a pipeline flows one direction; use task-per-connection for req/res
- **Simple transformations** — if it's one step, just do it inline (no pipeline needed)
- **When order doesn't matter** — use fan-out/fan-in instead (next pattern)

## Code exercise: Log Analyzer

Build a pipeline that processes web server access logs:

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│ Read     │───►│ Parse    │───►│ Filter   │───►│ Aggregate│
│ lines    │    │ fields   │    │ errors   │    │ count by │
│ from file│    │ (split)  │    │ (5xx)    │    │ endpoint │
└──────────┘    └──────────┘    └──────────┘    └──────────┘
```

**Input** (access.log):
```
GET /api/users 200 12ms
POST /api/login 401 5ms
GET /api/data 500 1502ms
GET /api/users 200 8ms
GET /api/data 503 30000ms
POST /api/upload 500 250ms
```

**Output**:
```
Error summary:
  /api/data:   2 errors (500, 503)
  /api/upload: 1 error  (500)
  Total: 3 errors out of 6 requests
```

**Requirements**:
1. Four pipeline stages, each a separate task
2. Bounded channels (capacity 100) between stages
3. Producer reads lines from a file (or generates them)
4. Parser extracts: method, path, status, latency
5. Filter passes only 5xx status codes
6. Aggregator counts errors by endpoint, prints summary when done

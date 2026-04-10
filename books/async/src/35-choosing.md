# Choosing a Pattern

## Decision table

```
I need to...                                    → Use
─────────────────────────────────────────────────────────────────
Handle many independent clients                 → Task-per-Connection
Manage complex state without locks              → Actor Model
Process data through multiple stages            → Pipeline
Do the same thing to many items in parallel     → Fan-out / Fan-in
Keep services running despite crashes           → Supervisor Tree
Broadcast events to multiple consumers          → Event Bus / Pub-Sub
```

## Choosing by symptom

```
Problem                              Solution
─────────────────────────────────────────────────────────────────
"I have lock contention"             → Actor (eliminate shared state)
"My pipeline stage is a bottleneck"  → Fan-out (parallelize that stage)
"Tasks crash and the system dies"    → Supervisor (auto-restart)
"I need to notify many components"   → Event Bus (decouple with broadcast)
"Each client needs isolated state"   → Task-per-Connection + Actor
"I process a stream of data"        → Pipeline with bounded channels
```

## Combining patterns

Real systems use multiple patterns together. Here's a typical web application:

```
┌──────────────────────────────────────────────────────────────┐
│  Web Application                                             │
│                                                              │
│  TCP Listener (task-per-connection)                          │
│    └── spawn(handle_request) for each HTTP request           │
│                                                              │
│  Request Handler                                             │
│    ├── Reads from Database Actor (actor model)               │
│    ├── Writes to Cache Actor (actor model)                   │
│    └── Emits metrics to Event Bus (pub-sub)                  │
│                                                              │
│  Background Jobs                                             │
│    ├── Job Queue → Workers (fan-out/fan-in)                  │
│    └── Supervised by a restart manager (supervisor tree)     │
│                                                              │
│  Log Pipeline (pipeline)                                     │
│    └── access log → parse → filter → ship to logging service │
│                                                              │
│  Metrics Dashboard (event bus subscriber)                    │
│    └── Receives metrics, displays live graphs                │
└──────────────────────────────────────────────────────────────┘
```

## Pattern comparison

```
Pattern              Concurrency    State         Communication  Failure
─────────────────────────────────────────────────────────────────────────
Task-per-Connection  per client     per task      shared/channels dies alone
Actor                per entity     per actor     messages        isolated
Pipeline             per stage      per stage     channels        stage stops
Fan-out/Fan-in       per item       none shared   JoinSet         retry item
Supervisor           per worker     per worker    restart signal  auto-restart
Event Bus            per subscriber none shared   broadcast       drops msgs
```

## Anti-patterns

### Don't spawn without joining

```rust
// BAD: fire and forget — leaked task, no error handling
tokio::spawn(async { do_work().await });

// GOOD: track the handle
let handle = tokio::spawn(async { do_work().await });
handle.await??; // propagate errors
```

### Don't use actors for everything

If you have a simple counter, `Arc<AtomicU64>` is better than an actor. Actors add overhead (channel, task, serialization). Use them when state is complex or when you'd hold a Mutex across `.await`.

### Don't use unbounded channels in production

```rust
// BAD: unbounded — OOM if consumer is slow
let (tx, rx) = mpsc::unbounded_channel();

// GOOD: bounded — backpressure if consumer is slow
let (tx, rx) = mpsc::channel(100);
```

### Don't block the executor

```rust
// BAD: blocks the worker thread
tokio::spawn(async {
    std::thread::sleep(Duration::from_secs(5)); // BLOCKS!
});

// GOOD: use async sleep or spawn_blocking
tokio::spawn(async {
    tokio::time::sleep(Duration::from_secs(5)).await; // yields
});

// GOOD: for CPU-heavy or blocking I/O
tokio::task::spawn_blocking(|| {
    std::fs::read("big-file.dat") // OK to block here
});
```

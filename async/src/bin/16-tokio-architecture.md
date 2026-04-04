# Lesson 16: Tokio Architecture

> **Prerequisites**: Courses 1-2. You've built your own runtime — now see how the production one is designed.

## Real-life analogy: a factory

```
┌──────────────────────────────────────────────────────────┐
│  Factory (Tokio Runtime)                                 │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Factory Floor (Scheduler)                         │  │
│  │                                                    │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐          │  │
│  │  │ Worker 1 │ │ Worker 2 │ │ Worker 3 │          │  │
│  │  │ (thread) │ │ (thread) │ │ (thread) │          │  │
│  │  └──────────┘ └──────────┘ └──────────┘          │  │
│  │                                                    │  │
│  │  Work-stealing: idle workers take from busy ones   │  │
│  └────────────────────────┬───────────────────────────┘  │
│                           │                              │
│  ┌────────────────────────▼───────────────────────────┐  │
│  │  Utility Room (Drivers)                            │  │
│  │                                                    │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐          │  │
│  │  │ I/O      │ │ Timer    │ │ Signal   │          │  │
│  │  │ Driver   │ │ Driver   │ │ Driver   │          │  │
│  │  │ (mio)    │ │ (wheel)  │ │ (unix)   │          │  │
│  │  └──────────┘ └──────────┘ └──────────┘          │  │
│  │                                                    │  │
│  │  Handles external events → wakes tasks             │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  Front Office: Runtime::block_on(), spawn(), Handle      │
└──────────────────────────────────────────────────────────┘
```

The factory has:
- **Workers** (threads) on the factory floor processing tasks
- A **utility room** (drivers) that monitors external events (power, deliveries, alarms)
- A **front office** (API) where you submit work orders

## Tokio's internal structure

```
tokio::runtime::Runtime
  │
  ├── Scheduler
  │     ├── current_thread::Scheduler (single-threaded)
  │     └── multi_thread::Scheduler (work-stealing)
  │           ├── Worker 0 (thread + local queue)
  │           ├── Worker 1
  │           └── Worker N
  │
  ├── Driver (layered)
  │     ├── Signal Driver (outermost)
  │     │     └── Time Driver
  │     │           └── I/O Driver (innermost, wraps mio::Poll)
  │     │
  │     │  Each park() call propagates down:
  │     │    signal.park() → time.park() → io.park() → mio.poll()
  │     │
  │     │  On return, each layer checks its events:
  │     │    mio events → wake I/O tasks
  │     │    expired timers → wake timer tasks
  │     │    signals → wake signal listeners
  │
  └── Handle (cloneable, Send + Sync)
        ├── spawn() — submit a task from anywhere
        ├── block_on() — enter the runtime on current thread
        └── spawn_blocking() — run sync code on a dedicated thread pool
```

## current_thread vs multi_thread

```
current_thread:                      multi_thread:
┌──────────────────┐                 ┌──────────────────┐
│  One thread      │                 │  N threads       │
│                  │                 │                  │
│  ┌────────────┐  │                 │  ┌────┐ ┌────┐  │
│  │  Scheduler │  │                 │  │ W0 │ │ W1 │  │
│  │  + Driver  │  │                 │  └────┘ └────┘  │
│  │  (same     │  │                 │  ┌────┐ ┌────┐  │
│  │   thread)  │  │                 │  │ W2 │ │ W3 │  │
│  └────────────┘  │                 │  └────┘ └────┘  │
│                  │                 │                  │
│  Pros:           │                 │  Pros:           │
│  - No Send req   │                 │  - Uses all CPUs │
│  - No sync cost  │                 │  - Work stealing │
│  - Deterministic │                 │  - Production    │
│                  │                 │                  │
│  Cons:           │                 │  Cons:           │
│  - One CPU core  │                 │  - Send required │
│  - Can't use     │                 │  - More complex  │
│    spawn()       │                 │  - Non-determ.   │
│    (only         │                 │                  │
│    spawn_local)  │                 │                  │
└──────────────────┘                 └──────────────────┘
```

### When to use each

- **`current_thread`**: tests, WASM, simple CLI tools, apps with `!Send` types
- **`multi_thread`**: web servers, database proxies, anything with high concurrency

## The Runtime Builder

```rust
// Multi-threaded (default for #[tokio::main])
let rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(4)        // default: num_cpus
    .max_blocking_threads(512) // for spawn_blocking
    .enable_io()              // I/O driver (mio)
    .enable_time()            // timer driver
    .thread_name("my-worker")
    .on_thread_start(|| println!("worker started"))
    .build()?;

// Single-threaded
let rt = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()?;
```

### What enable_io() and enable_time() do

```
enable_io():   creates mio::Poll, starts I/O driver
               without it: TcpStream, UdpSocket, etc. panic

enable_time(): creates timer wheel, starts time driver
               without it: tokio::time::sleep panics

enable_all():  enables both I/O and time
```

## The Driver stack

The driver is layered — each layer wraps the one below:

```
park() call flow:

  SignalDriver::park()
    │
    ├── check for pending signals
    │
    └── TimeDriver::park()
          │
          ├── compute timeout from next timer deadline
          │
          └── IoDriver::park(timeout)
                │
                └── mio::Poll::poll(timeout)
                      │
                      └── OS: kqueue / epoll (blocks)

return flow:

  mio returns events
    │
    └── IoDriver: wake I/O tasks
          │
          └── TimeDriver: fire expired timers, wake timer tasks
                │
                └── SignalDriver: dispatch signals
```

This is why `enable_io()` and `enable_time()` matter — without them, those driver layers don't exist.

## Handle: spawning from anywhere

```rust
let rt = Runtime::new()?;
let handle = rt.handle().clone();  // Handle is Send + Sync + Clone

// From another thread:
handle.spawn(async { /* runs on the runtime */ });

// From inside async code:
let handle = tokio::runtime::Handle::current();
handle.spawn(async { /* also works */ });
```

## Tracing through tokio source

A fun exercise: trace `tokio::spawn(my_future)` through the source code.

```
tokio::spawn(future)
  → context::spawn(future)        // get the current runtime context
  → scheduler.spawn(task)          // submit to the scheduler
  → worker.schedule(task)          // push to local queue (or global)
  → if worker idle: worker.unpark() // wake a sleeping worker
```

## Exercises

### Exercise 1: current_thread echo server

Build a TCP echo server on `current_thread`. Use `spawn_local` for tasks. Verify it works but only uses one CPU core.

### Exercise 2: multi_thread thread distribution

Spawn 100 tasks on a `multi_thread` runtime with 4 workers. Each task records `std::thread::current().id()`. Print how many tasks ran on each thread — should be roughly balanced.

### Exercise 3: Missing driver

Create a runtime with only `enable_io()` (no time). Try `tokio::time::sleep(1s).await`. What error do you get?

Then create with only `enable_time()` (no I/O). Try `TcpListener::bind`. What happens?

### Exercise 4: Handle from another thread

```rust
let rt = Runtime::new()?;
let handle = rt.handle().clone();

std::thread::spawn(move || {
    handle.spawn(async {
        println!("running on tokio from a std thread!");
    });
});
```

### Exercise 5: Throughput comparison

Build a simple echo server. Benchmark with `nc` or a load tester:
- `current_thread` with 1000 concurrent connections
- `multi_thread` (4 workers) with 1000 concurrent connections

Measure requests/second. How much does multi_thread help?

# Async Rust & Tokio Internals

A hands-on course that builds async Rust from the ground up — from raw futures and wakers to a multi-threaded runtime, then into real tokio internals and production patterns.

## Prerequisites

- Rust fundamentals (ownership, traits, generics)
- TCP networking basics
- Completed [tls/](../tls/) or equivalent (helpful, not required)

## Courses

### Course 1: Async Fundamentals (no runtime, no tokio)

| # | Topic | Code | Notes |
|---|-------|------|-------|
| 0 | [Why Async?](src/bin/0-why-async.md) | [0-why-async.rs](src/bin/0-why-async.rs) | Threads vs async, C10K problem, benchmarking |
| 1 | [Futures by Hand](src/bin/1-futures.md) | [1-futures.rs](src/bin/1-futures.rs) | `Future` trait, `Poll`, `Pending`/`Ready` |
| 2 | [State Machines](src/bin/2-state-machines.md) | [2-state-machines.rs](src/bin/2-state-machines.rs) | What `async fn` compiles to |
| 3 | [Wakers](src/bin/3-wakers.md) | [3-wakers.rs](src/bin/3-wakers.rs) | `RawWaker`, vtable, waking mechanism |
| 4 | [Tasks](src/bin/4-tasks.md) | [4-tasks.rs](src/bin/4-tasks.rs) | Task struct, waker-queue connection, JoinHandle, `'static` + `Send` |
| 5 | [A Minimal Executor](src/bin/5-executor.md) | [5-executor.rs](src/bin/5-executor.rs) | `block_on`, task queue, `spawn`, DelayFuture |
| 6 | [Pinning](src/bin/6-pinning.md) | [6-pinning.rs](src/bin/6-pinning.rs) | `Pin`, self-referential structs, `Unpin` |
| 7 | [Combinators](src/bin/7-combinators.md) | [7-combinators.rs](src/bin/7-combinators.rs) | `join`, `select` — built by hand |
| 8 | [Async I/O Foundations](src/bin/8-async-io.md) | [8-async-io.rs](src/bin/8-async-io.rs) | kqueue/epoll, non-blocking sockets |

**Project 1**: [TCP Echo Server on your executor](src/bin/p1-echo-server.md) — [p1-echo-server.rs](src/bin/p1-echo-server.rs)

### Course 2: Build a Mini Tokio

| # | Topic | Code | Notes |
|---|-------|------|-------|
| 9 | [Event Loop + Reactor](src/bin/9-reactor.md) | [9-reactor.rs](src/bin/9-reactor.rs) | mio-based reactor, fd → waker mapping |
| 10 | [Task Scheduling](src/bin/10-task-scheduling.md) | [10-task-scheduling.rs](src/bin/10-task-scheduling.rs) | Run queue, round-robin, fairness |
| 11 | [AsyncRead / AsyncWrite](src/bin/11-async-read-write.md) | [11-async-read-write.rs](src/bin/11-async-read-write.rs) | Wrap non-blocking sockets in async traits |
| 12 | [Timers](src/bin/12-timers.md) | [12-timers.rs](src/bin/12-timers.rs) | Timer heap, `sleep()`, deadlines |
| 13 | [Channels](src/bin/13-channels.md) | [13-channels.rs](src/bin/13-channels.rs) | Async oneshot and mpsc |
| 14 | [Work-Stealing Scheduler](src/bin/14-work-stealing.md) | [14-work-stealing.rs](src/bin/14-work-stealing.rs) | Multi-threaded runtime |
| 15 | [Select Internals](src/bin/15-select.md) | [15-select.rs](src/bin/15-select.rs) | Race futures, cancellation, drop |

**Project 2**: [Multi-threaded Chat Server](src/bin/p2-chat-server.md) — [p2-chat-server.rs](src/bin/p2-chat-server.rs)

### Course 3: Tokio Deep Dive

| # | Topic | Code | Notes |
|---|-------|------|-------|
| 16 | [Tokio Architecture](src/bin/16-tokio-architecture.md) | [16-tokio-architecture.rs](src/bin/16-tokio-architecture.rs) | Runtime builder, drivers, scheduler |
| 17 | [Tokio I/O Driver](src/bin/17-tokio-io-driver.md) | [17-tokio-io-driver.rs](src/bin/17-tokio-io-driver.rs) | mio integration, Registration, readiness |
| 18 | [tokio::sync Internals](src/bin/18-tokio-sync.md) | [18-tokio-sync.rs](src/bin/18-tokio-sync.rs) | Mutex, Semaphore, Notify |
| 19 | [tokio::net](src/bin/19-tokio-net.md) | [19-tokio-net.rs](src/bin/19-tokio-net.rs) | TcpListener/TcpStream internals |
| 20 | [Task-Local Storage](src/bin/20-task-locals.md) | [20-task-locals.rs](src/bin/20-task-locals.rs) | Task-locals vs thread-locals |
| 21 | [Graceful Shutdown](src/bin/21-graceful-shutdown.md) | [21-graceful-shutdown.rs](src/bin/21-graceful-shutdown.rs) | CancellationToken, drain pattern |
| 22 | [Tracing & Debugging](src/bin/22-tracing.md) | [22-tracing.rs](src/bin/22-tracing.rs) | tokio-console, task dumps |

**Project 3**: [HTTP Load Tester](src/bin/p3-load-tester.md) — [p3-load-tester.rs](src/bin/p3-load-tester.rs)

### Course 4: Advanced Patterns

| # | Topic | Code | Notes |
|---|-------|------|-------|
| 23 | [Backpressure](src/bin/23-backpressure.md) | [23-backpressure.rs](src/bin/23-backpressure.rs) | Bounded channels, flow control |
| 24 | [Cancellation Safety](src/bin/24-cancellation.md) | [24-cancellation.rs](src/bin/24-cancellation.rs) | Dropped futures, data loss risks |
| 25 | [Sync ↔ Async Bridge](src/bin/25-sync-async-bridge.md) | [25-sync-async-bridge.rs](src/bin/25-sync-async-bridge.rs) | `block_on`, `spawn_blocking` |
| 26 | [Streams](src/bin/26-streams.md) | [26-streams.rs](src/bin/26-streams.rs) | Async iteration, `StreamExt` |
| 27 | [Connection Pooling](src/bin/27-connection-pool.md) | [27-connection-pool.rs](src/bin/27-connection-pool.rs) | Reuse, health checks, idle timeout |
| 28 | [Testing Async Code](src/bin/28-testing.md) | [28-testing.rs](src/bin/28-testing.rs) | Time mocking, deterministic testing |

**Project 4**: [Async Job Queue](src/bin/p4-job-queue.md) — [p4-job-queue.rs](src/bin/p4-job-queue.rs)

## How it all connects

```
Course 1: Async Fundamentals
  Futures → State Machines → Wakers → Tasks → Executor → Pinning → Combinators → I/O
                                                │
                                                ▼
                                     Project 1: Echo Server
                                     (your runtime from scratch)
                                                │
Course 2: Mini Tokio                            ▼
  Reactor → Scheduling → AsyncRead → Timers → Channels → Work-Stealing → Select
                                                │
                                                ▼
                                     Project 2: Chat Server
                                     (multi-threaded, your runtime)
                                                │
Course 3: Tokio Deep Dive                      ▼
  Architecture → I/O Driver → Sync → Net → Task-Locals → Shutdown → Tracing
                                                │
                                                ▼
                                     Project 3: HTTP Load Tester
                                     (real tokio internals)
                                                │
Course 4: Advanced Patterns                    ▼
  Backpressure → Cancellation → Bridging → Streams → Pooling → Testing
                                                │
                                                ▼
                                     Project 4: Job Queue
                                     (production patterns)
```

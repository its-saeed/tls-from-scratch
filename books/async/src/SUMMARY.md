# Summary

[Introduction](./introduction.md)

# Course 1: Async Fundamentals

- [Why Async?](./00-why-async.md)
- [Futures by Hand](./01-futures.md)
- [State Machines](./02-state-machines.md)
- [Wakers & Waking](./03-wakers.md)
- [Tasks](./04-tasks.md)
- [A Minimal Executor](./05-executor.md)
- [Pinning](./06-pinning.md)
- [Combinators](./07-combinators.md)
- [Async I/O Foundations](./08-async-io.md)
- [Project 1: TCP Echo Server](./p1-echo-server.md)

# Course 2: Build a Mini Tokio

- [Event Loop + Reactor](./09-reactor.md)
- [Task Scheduling](./10-task-scheduling.md)
- [AsyncRead / AsyncWrite](./11-async-read-write.md)
- [Timers](./12-timers.md)
- [Channels](./13-channels.md)
- [Work-Stealing Scheduler](./14-work-stealing.md)
- [Select Internals](./15-select.md)
- [Project 2: Chat Server](./p2-chat-server.md)

# Course 3: Tokio Deep Dive

- [Tokio Architecture](./16-tokio-architecture.md)
- [Tokio I/O Driver](./17-tokio-io-driver.md)
- [tokio::sync Internals](./18-tokio-sync.md)
- [tokio::net](./19-tokio-net.md)
- [Task-Local Storage](./20-task-locals.md)
- [Graceful Shutdown](./21-graceful-shutdown.md)
- [Tracing & Debugging](./22-tracing.md)
- [Project 3: HTTP Load Tester](./p3-load-tester.md)

# Course 4: Advanced Patterns

- [Backpressure](./23-backpressure.md)
- [Cancellation Safety](./24-cancellation.md)
- [Sync / Async Bridge](./25-sync-async-bridge.md)
- [Streams](./26-streams.md)
- [Connection Pooling](./27-connection-pool.md)
- [Testing Async Code](./28-testing.md)
- [Project 4: Async Job Queue](./p4-job-queue.md)

# Course 5: Architecture Patterns

- [Task-per-Connection](./29-task-per-connection.md)
- [Actor Model](./30-actor-model.md)
- [Pipeline / Stream Processing](./31-pipeline.md)
- [Fan-out / Fan-in](./32-fan-out-fan-in.md)
- [Supervisor Tree](./33-supervisor.md)
- [Event Bus / Pub-Sub](./34-event-bus.md)
- [Choosing a Pattern](./35-choosing.md)
- [Capstone: Distributed Task Queue](./p5-task-queue.md)

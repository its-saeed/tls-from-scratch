# Lesson 9: Event Loop + Reactor

> **Prerequisites**: Lesson 5 (Executor), Lesson 8 (Async I/O). You need to understand both the executor's poll loop and how kqueue/epoll works before combining them.

## Real-life analogy: the hotel front desk

A hotel front desk manages hundreds of rooms:

```
┌─────────────────────────────────────────────────────┐
│  Front Desk (Reactor)                               │
│                                                     │
│  Room Registry:                                     │
│    Room 101 → Guest A's wake-up call at 7am        │
│    Room 205 → Guest B wants fresh towels            │
│    Room 312 → Guest C ordered room service          │
│                                                     │
│  Loop:                                              │
│    1. Wait for any event (phone rings, bell rings)  │
│    2. Look up which room it's for                   │
│    3. Notify the right guest                        │
│    4. Back to waiting                               │
│                                                     │
│  The desk doesn't DO the work (cooking, cleaning).  │
│  It just routes notifications to the right person.  │
└─────────────────────────────────────────────────────┘
```

The reactor works the same way:
- **Rooms** = file descriptors (sockets)
- **Guests** = tasks (futures waiting for I/O)
- **Registry** = HashMap<Token, Waker>
- **Phone ringing** = kqueue/epoll event
- **Notifying the guest** = `waker.wake()`

The reactor doesn't read or write data. It just tells futures "your socket is ready — try now."

## What is a Reactor?

The reactor is the component that connects the OS event system to your async runtime:

```
┌───────────────────────────────────────────────────────────┐
│                        Reactor                            │
│                                                           │
│   ┌──────────┐         ┌─────────────────────────┐       │
│   │ mio::Poll│         │ wakers: HashMap          │       │
│   │          │         │   Token(0) → Waker_A     │       │
│   │ wraps    │         │   Token(1) → Waker_B     │       │
│   │ kqueue / │         │   Token(2) → Waker_C     │       │
│   │ epoll    │         │                           │       │
│   └─────┬────┘         └──────────┬──────────────┘       │
│         │                         │                       │
│         │  poll() returns         │  look up waker        │
│         │  [Token(1), Token(2)]   │  for each token       │
│         │                         │                       │
│         └────────────┬────────────┘                       │
│                      │                                    │
│                      ▼                                    │
│              waker_b.wake()                               │
│              waker_c.wake()                               │
│              → tasks re-queued in executor                │
│                                                           │
└───────────────────────────────────────────────────────────┘
```

## The Reactor struct

```rust
struct Reactor {
    /// The OS event system (wraps kqueue on macOS, epoll on Linux)
    poll: mio::Poll,

    /// Maps tokens to wakers. When an event fires for Token(N),
    /// we look up the waker here and call wake().
    wakers: HashMap<mio::Token, Waker>,

    /// Counter for assigning unique tokens to new sockets
    next_token: usize,
}
```

Three core methods:

### register — "watch this socket"

```rust
fn register(&mut self, source: &mut impl Source, interest: Interest) -> mio::Token {
    let token = mio::Token(self.next_token);
    self.next_token += 1;
    self.poll.registry().register(source, token, interest).unwrap();
    token
}
```

Called when a future first needs to wait for I/O. The future says "tell me when socket X is readable."

### set_waker — "here's how to notify me"

```rust
fn set_waker(&mut self, token: mio::Token, waker: Waker) {
    self.wakers.insert(token, waker);
}
```

Called during `poll()` when a future returns `Pending`. The future stores its waker so the reactor can notify it later.

### wait — "block until something happens"

```rust
fn wait(&mut self) {
    let mut events = mio::Events::with_capacity(64);
    self.poll.poll(&mut events, None).unwrap();  // blocks!

    for event in events.iter() {
        if let Some(waker) = self.wakers.get(&event.token()) {
            waker.wake_by_ref();  // re-queue the task
        }
    }
}
```

This is where the thread sleeps. `poll()` blocks until the OS says a socket is ready. Then we wake the corresponding tasks.

## How Reactor + Executor work together

```
Executor                         Reactor                        OS
   │                                │                            │
   ├── poll task_A ──►              │                            │
   │   task_A tries read()          │                            │
   │   → WouldBlock                 │                            │
   │   task_A calls:                │                            │
   │     reactor.set_waker(tok, wk) │                            │
   │   ◄── Pending ────────────────│                            │
   │                                │                            │
   ├── queue empty                  │                            │
   ├── reactor.wait() ────────────►│                            │
   │                                ├── poll.poll() ────────────►│
   │                                │   (thread sleeps)          │
   │                                │                     data arrives!
   │                                │   ◄── Token(0) ready ─────┤
   │                                │                            │
   │                                ├── waker.wake() ──►        │
   │   ◄── task_A re-queued ────────┤                            │
   │                                │                            │
   ├── poll task_A ──►              │                            │
   │   task_A tries read()          │                            │
   │   → got data!                  │                            │
   │   ◄── Ready(data) ───────────│                            │
```

The key insight: **the executor never busy-polls**. When there's nothing to do, it calls `reactor.wait()` which blocks until the OS has an event. Zero CPU usage while waiting.

## Where does the Reactor live?

The reactor needs to be accessible from:
1. **Futures** — to call `register()` and `set_waker()` during `poll()`
2. **The executor** — to call `wait()` when the queue is empty

Common patterns:

```
Option A: Thread-local (single-threaded runtime)
  thread_local! { static REACTOR: RefCell<Reactor> = ... }

Option B: Arc<Mutex<Reactor>> (shared, but lock contention)

Option C: Global static with OnceLock (initialized once)
  static REACTOR: OnceLock<Mutex<Reactor>> = OnceLock::new();
```

Tokio uses a more sophisticated approach — the reactor runs on a dedicated thread and communicates via channels. For our mini-runtime, thread-local is simplest.

## Readiness vs Completion

The reactor uses the **readiness model**:
- "Socket is readable" = you CAN try `read()` and it probably won't block
- It does NOT mean "data has been read into your buffer"
- You still need to call `read()` yourself — it might still return `WouldBlock`

This is different from the **completion model** (used by io_uring on Linux):
- "Read is complete" = data is already in your buffer
- No need to call `read()` again

```
Readiness (kqueue/epoll/mio):
  1. Register: "tell me when fd is readable"
  2. Event fires: "fd is readable"
  3. You call read(fd) → get data (usually)

Completion (io_uring):
  1. Submit: "read fd into this buffer"
  2. Event fires: "read is done, data is in your buffer"
  3. Just use the buffer
```

Tokio uses readiness (mio). This matters because your futures must handle `WouldBlock` even after being woken — the event was a hint, not a guarantee.

## Exercises

### Exercise 1: Single-socket reactor

Build a minimal Reactor that watches one `TcpListener`:

1. Create `mio::Poll` and `mio::Events`
2. Create a `mio::net::TcpListener`, register it for `READABLE`
3. Loop: `poll.poll()`, accept connections, print their address
4. No executor, no wakers — just the raw event loop

Test: run the program, connect with `nc 127.0.0.1 8080` from another terminal.

### Exercise 2: Multi-socket reactor

Extend Exercise 1 to handle multiple connections:

1. Accept connections, register each for `READABLE`
2. Map each `Token` to a `TcpStream` (use a `HashMap<Token, TcpStream>`)
3. When a stream is readable, read data and echo it back
4. Handle disconnection (read returns 0)

Test: connect 3 clients, type in each — all should echo independently.

### Exercise 3: Waker integration

Wire the reactor into a waker:

1. Create a reactor with a `HashMap<Token, Waker>`
2. Create a `ReadableFuture` that:
   - First poll: registers the socket with the reactor, stores waker, returns `Pending`
   - Later polls: tries `read()` — if data, returns `Ready(data)`, if `WouldBlock`, returns `Pending`
3. Use `block_on` from Lesson 5 to run the future
4. The reactor's `wait()` should be called when the executor has nothing to do

This is the bridge between Course 1 (individual components) and Course 2 (integrated runtime).

### Exercise 4: Deregistration and cleanup

Add proper cleanup:

1. When a connection closes, deregister its token from `mio::Poll`
2. Remove the waker from the HashMap
3. Verify no leaked entries — print the waker map size periodically
4. Handle the case where a waker fires for a token that was already deregistered (it should be a no-op)

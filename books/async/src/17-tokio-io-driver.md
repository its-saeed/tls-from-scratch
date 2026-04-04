# Lesson 17: Tokio's I/O Driver

> **Prerequisites**: Lesson 9 (Reactor), Lesson 11 (AsyncRead/AsyncWrite), Lesson 16 (Tokio Architecture).

## Real-life analogy: the switchboard operator

```
Old telephone system:
┌──────────────────────────────────────────────────┐
│  Switchboard (I/O Driver)                        │
│                                                  │
│  Plug board:                                     │
│    Jack 1 → Room 101 (Alice's phone)             │
│    Jack 2 → Room 205 (Bob's phone)               │
│    Jack 3 → (empty)                              │
│                                                  │
│  Operator loop:                                  │
│    1. Watch all jacks for incoming signals        │
│    2. Jack 2 lights up → "Bob has a call!"       │
│    3. Ring Bob's room (wake his task)            │
│                                                  │
│  Registration:                                   │
│    New guest checks in → operator plugs a jack   │
│    Guest checks out → operator unplugs           │
└──────────────────────────────────────────────────┘
```

Tokio's I/O driver is the switchboard:
- **Jacks** = file descriptors registered with mio
- **Plugging in** = `Registration::new()` → `mio::Poll::register()`
- **Light up** = readiness event from kqueue/epoll
- **Ring the room** = `waker.wake()`

## How tokio wraps mio

Your Lesson 9 reactor and tokio's I/O driver do the same thing — but tokio adds a layer of abstraction:

```
Your reactor (Lesson 9):              Tokio's I/O driver:
  mio::Poll                             mio::Poll
  HashMap<Token, Waker>                 Slab<ScheduledIo>
  register/deregister manually          Registration handles lifecycle
  you call wait()                       driver calls park()
```

### The Registration type

In tokio, every I/O resource (TcpStream, UdpSocket, etc.) holds a `Registration`:

```rust
// Simplified from tokio source
struct Registration {
    handle: Handle,     // reference to the I/O driver
    token: usize,       // slab index for event dispatch
}
```

When you create a `tokio::net::TcpStream`, it calls `Registration::new()`:
1. Registers the fd with `mio::Poll`
2. Allocates a slot in the driver's slab
3. Returns a `Registration` that derefs to wake/interest methods

### The readiness flow

```
Application:  stream.read(&mut buf).await

Tokio TcpStream::read():
  │
  ├── poll_read_ready()              // check if driver says readable
  │     │
  │     ├── already ready? → try read()
  │     │
  │     └── not ready? → register waker with driver
  │                      return Pending
  │
  ├── (later) I/O driver: mio::Poll returns event
  │     │
  │     └── driver looks up ScheduledIo by token
  │         calls waker.wake()
  │
  └── re-polled: poll_read_ready() → ready!
      try read() → Ok(n) → Ready(n)
```

## Interest and Ready

```rust
// Interest: what events you want
Interest::READABLE    // want to know when data is available
Interest::WRITABLE    // want to know when write buffer has space
Interest::READABLE | Interest::WRITABLE  // both

// Ready: what actually happened
if ready.is_readable() { /* data available */ }
if ready.is_writable() { /* can write */ }
if ready.is_read_closed() { /* peer closed their write half */ }
if ready.is_write_closed() { /* peer closed their read half */ }
```

## Spurious wakeups

The driver might wake you when there's nothing to do:

```
Driver says: "fd 5 is readable!"
You call read(buf):  → WouldBlock  (nothing actually available)
```

This happens because:
- Edge-triggered events can be delivered before data fully arrives
- Multiple events can coalesce
- The OS might report readiness optimistically

Your I/O code MUST handle `WouldBlock` by returning `Pending` and re-registering — never assume the operation will succeed just because you were woken.

## Exercises

### Exercise 1: mio echo server

Build a raw mio echo server (no tokio). Register a listener, accept connections, register each for READABLE, echo data. This is what tokio does internally.

### Exercise 2: Tokio echo server

Convert the mio echo server to tokio. Observe how much code disappears — tokio handles registration, wakers, and the event loop for you.

### Exercise 3: Readiness exploration

```rust
use tokio::net::TcpStream;
use tokio::io::Interest;

let stream = TcpStream::connect("127.0.0.1:8080").await?;
let ready = stream.ready(Interest::READABLE | Interest::WRITABLE).await?;
println!("readable: {}, writable: {}", ready.is_readable(), ready.is_writable());
```

Connect to a server, check readiness. Write data, check again. Close the peer, check for `is_read_closed()`.

### Exercise 4: Spurious wakeup handling

Write a stream wrapper that logs every `WouldBlock`. Connect to a server, start reading. How many `WouldBlock`s do you see? This shows why the retry loop is essential.

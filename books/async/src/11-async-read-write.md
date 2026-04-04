# Lesson 11: AsyncRead / AsyncWrite

> **Prerequisites**: Lesson 9 (Reactor), Lesson 10 (Task Scheduling). You need a working reactor that maps tokens to wakers and a scheduler that polls tasks.

## Real-life analogy: the drive-through window

At a drive-through:

```
Blocking:
  You:     "One burger please"
  Window:  (silence for 5 minutes while they cook)
  You:     (frozen, can't do anything, car is blocked)
  Window:  "Here's your burger"

Non-blocking:
  You:     "One burger please"
  Window:  "Not ready, come back later" (WouldBlock)
  You:     Drive away, do other errands
  You:     Come back: "Ready yet?"
  Window:  "Not ready" (WouldBlock again)
  You:     Come back: "Ready yet?"
  Window:  "Here's your burger!"

Async (what we're building):
  You:     "One burger please. Call me when it's ready."
  Window:  (stores your number)    → reactor.set_waker(token, waker)
  You:     Go do other errands     → return Poll::Pending
           ...
  Window:  (rings your phone)      → waker.wake()
  You:     Drive back, pick it up  → read() → Ready(burger)
```

AsyncRead/AsyncWrite wraps the "non-blocking + callback" pattern into a clean `.await`-able API.

## The pattern: try → WouldBlock → register → Pending

Every async I/O operation follows the same pattern:

```
fn poll_read(cx, buf) → Poll<usize>:
  ┌──────────────────────────────┐
  │ Try read(buf)                │
  │                              │
  │ Got data (n bytes)?          │
  │   → return Poll::Ready(n)   │
  │                              │
  │ Got WouldBlock?              │
  │   → reactor.set_waker(token, cx.waker())
  │   → return Poll::Pending    │
  │                              │
  │ Got error?                   │
  │   → return Poll::Ready(Err) │
  └──────────────────────────────┘
```

This is the **readiness pattern**:
1. **Try** the operation (non-blocking socket, so it returns immediately)
2. If **success** → return the data
3. If **WouldBlock** → register with the reactor, yield to the executor
4. When the reactor fires → executor re-polls → back to step 1

## The AsyncTcpStream

```rust
struct AsyncTcpStream {
    /// The underlying non-blocking socket
    inner: mio::net::TcpStream,
    /// Token registered with the reactor (for event matching)
    token: Token,
}
```

### poll_read

```rust
impl AsyncTcpStream {
    fn poll_read(
        &mut self,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match self.inner.read(buf) {
            Ok(n) => Poll::Ready(Ok(n)),          // got data
            Err(e) if e.kind() == WouldBlock => {
                reactor().set_waker(self.token, cx.waker().clone());
                Poll::Pending                      // wait for reactor
            }
            Err(e) => Poll::Ready(Err(e)),        // real error
        }
    }
}
```

### The read future

To make this `.await`-able, wrap it in a future:

```rust
struct ReadFuture<'a> {
    stream: &'a mut AsyncTcpStream,
    buf: &'a mut [u8],
}

impl<'a> Future for ReadFuture<'a> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.stream.poll_read(cx, self.buf)
    }
}

// Usage:
let n = stream.read(&mut buf).await?;
```

### poll_write

Same pattern, but for writing:

```rust
fn poll_write(&mut self, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
    match self.inner.write(buf) {
        Ok(n) => Poll::Ready(Ok(n)),
        Err(e) if e.kind() == WouldBlock => {
            reactor().set_waker(self.token, cx.waker().clone());
            Poll::Pending
        }
        Err(e) => Poll::Ready(Err(e)),
    }
}
```

## Visualizing the data flow

```
Application code:
  let n = stream.read(&mut buf).await;

Calls:
  ReadFuture::poll()
    │
    ├── stream.inner.read(buf)
    │     │
    │     ├── Ok(n) ──────────────────► Poll::Ready(Ok(n))
    │     │                              executor receives data
    │     │
    │     └── Err(WouldBlock) ────────► reactor.set_waker(token, waker)
    │                                   Poll::Pending
    │                                    │
    │         ... executor does other    │
    │             tasks ...              │
    │                                    │
    │         OS: data arrives on socket │
    │         reactor: event for Token(N)│
    │         reactor: waker.wake()      │
    │                                    │
    └── (re-polled by executor) ────────┘
        stream.inner.read(buf) → Ok(n)
        Poll::Ready(Ok(n))
```

## write_all: a higher-level helper

A single `write()` may not write all bytes (partial write). `write_all` loops until everything is written:

```rust
async fn write_all(stream: &mut AsyncTcpStream, mut buf: &[u8]) -> io::Result<()> {
    while !buf.is_empty() {
        let n = stream.write(buf).await?;
        buf = &buf[n..];
    }
    Ok(())
}
```

Each `.await` might yield if the socket's write buffer is full. The reactor wakes us when there's space.

## read_exact: fill the buffer completely

```rust
async fn read_exact(stream: &mut AsyncTcpStream, buf: &mut [u8]) -> io::Result<()> {
    let mut filled = 0;
    while filled < buf.len() {
        let n = stream.read(&mut buf[filled..]).await?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "connection closed"));
        }
        filled += n;
    }
    Ok(())
}
```

## The AsyncTcpListener

```rust
struct AsyncTcpListener {
    inner: mio::net::TcpListener,
    token: Token,
}

impl AsyncTcpListener {
    fn poll_accept(&mut self, cx: &mut Context) -> Poll<io::Result<(AsyncTcpStream, SocketAddr)>> {
        match self.inner.accept() {
            Ok((stream, addr)) => {
                let token = reactor().register(&mut stream, Interest::READABLE);
                Poll::Ready(Ok((AsyncTcpStream { inner: stream, token }, addr)))
            }
            Err(e) if e.kind() == WouldBlock => {
                reactor().set_waker(self.token, cx.waker().clone());
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}
```

Same pattern: try accept → got connection? Ready. WouldBlock? Register and Pending.

## Edge-triggered vs level-triggered

mio uses **edge-triggered** events by default: you get notified once when the state changes (not readable → readable). After handling the event, you must drain all available data, otherwise you might miss events.

```
Level-triggered:                    Edge-triggered:
  "socket IS readable"               "socket BECAME readable"
  (keeps firing until you read)       (fires once on transition)

  Safe: you can read one byte         Must read ALL available data
  and you'll be notified again.       or you might not be notified again.
```

In practice: after `waker.wake()`, your `poll_read` should loop reading until `WouldBlock`, then return Pending. This ensures you don't miss data.

## Cancellation safety

What happens if a `ReadFuture` is dropped mid-await?

```rust
let read_future = stream.read(&mut buf);
// Drop it before completion (e.g., select! picked the other branch)
drop(read_future);
```

With our design, this is **safe**: no data is lost because we haven't actually read anything yet. The read only happens inside `poll()`. If we never poll, no bytes are consumed.

But `read_exact` is **NOT cancellation-safe**: if you've read 50 of 100 bytes and the future is dropped, those 50 bytes are gone. This is Lesson 24's topic.

## Exercises

### Exercise 1: AsyncTcpStream

Implement `AsyncTcpStream` with `poll_read` and `poll_write`. Create `read()` and `write()` methods that return futures. Test with a simple echo server on your runtime from Lesson 5.

### Exercise 2: read_exact and write_all

Build `read_exact` and `write_all` as async functions. Test: send a 10KB message in small chunks, verify read_exact receives all of it.

### Exercise 3: Async echo server

Combine AsyncTcpListener + AsyncTcpStream + your executor. Accept connections, spawn a task per connection, echo data back. Test with multiple `nc` clients.

### Exercise 4: Throughput test

Implement `async fn copy(src, dst)` that pipes bytes from one stream to another. Measure throughput: connect two streams, send 1MB, time it. Compare with blocking `std::io::copy`.

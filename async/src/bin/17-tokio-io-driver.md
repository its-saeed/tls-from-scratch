# Lesson 16: Tokio's I/O Driver — mio Integration, Registration, Interest, Readiness

## What you'll learn

- How Tokio wraps mio's event loop as its I/O driver
- The registration lifecycle for I/O resources
- How `Interest` and `Ready` translate between mio and Tokio
- The readiness model vs completion model distinction

## Key concepts

### mio under the hood

Tokio's I/O driver owns a `mio::Poll` instance. When a `TcpStream` or other I/O resource is created, it registers with mio via `epoll` (Linux), `kqueue` (macOS), or `IOCP` (Windows).

### Registration flow

1. Create I/O resource (e.g., `mio::net::TcpStream`)
2. Call `Registration::new()` which registers with the mio `Poll`
3. Tokio assigns a token (slab index) for event dispatch
4. On readiness, the driver wakes the associated task's `Waker`

### Interest and Ready

```rust
// Interest declares what events you care about
let interest = Interest::READABLE | Interest::WRITABLE;

// Ready is what the driver reports back
if ready.is_readable() {
    // proceed with read
}
```

### Readiness vs Completion

- **Readiness** (epoll/kqueue): OS says "socket is ready", you do the I/O
- **Completion** (io_uring/IOCP): OS does the I/O, notifies when done
- Tokio currently uses the readiness model via mio

### Handling spurious wakeups

A readiness notification can be spurious. Tokio I/O operations must handle `WouldBlock` by re-registering interest and returning `Poll::Pending`.

## Exercises

1. Use `mio` directly to register a `TcpListener` and poll for events in a loop
2. Read Tokio's `PollEvented` source to trace how a `TcpStream::read` becomes a mio poll
3. Write a raw mio echo server, then convert it to Tokio — observe how much boilerplate disappears
4. Experiment with `Interest::READABLE` vs `Interest::WRITABLE` — what happens if you only register one?
5. Instrument the I/O driver with `tracing` to log every readiness event

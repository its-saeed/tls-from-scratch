# Lesson 10: AsyncRead / AsyncWrite

Wrap non-blocking sockets in async read/write traits so that calling
`.read()` or `.write()` returns a future instead of blocking. This connects
the reactor (Lesson 8) to user-facing I/O.

## What you'll build

- `AsyncTcpStream` -- a wrapper around `mio::net::TcpStream` that registers
  with your reactor and implements async read/write
- `poll_read()` / `poll_write()` methods that attempt the syscall, and if it
  returns `WouldBlock`, register interest and return `Poll::Pending`
- A small `AsyncRead` / `AsyncWrite` trait (or use the signatures from
  `futures::io`)
- Helper functions: `read_exact()`, `write_all()`, `copy()` built on top
- An `AsyncTcpListener` with an `accept()` that returns a future

## Key concepts

- **Non-blocking I/O + WouldBlock** -- the pattern: try the syscall, if EAGAIN
  register for readiness and yield
- **Buffer management** -- caller provides the buffer; the future borrows it
  across await points (lifetime considerations)
- **Re-registration** -- after a partial read you may need to re-arm edge-
  triggered interest
- **Vectored I/O** -- `readv` / `writev` for scatter-gather, optional but
  efficient
- **Cancellation safety** -- what happens if you drop a read future mid-flight?

## Exercises

1. **Async echo server** -- implement `AsyncTcpStream` and write an echo server
   that accepts connections and echoes bytes back, all on your runtime.

2. **read_exact future** -- build a `ReadExact` future that keeps reading until
   the buffer is full or EOF. Test with a client that sends data in small
   chunks.

3. **Async copy** -- implement `async fn copy(src, dst)` that streams bytes
   from one `AsyncTcpStream` to another. Measure throughput against a blocking
   version.

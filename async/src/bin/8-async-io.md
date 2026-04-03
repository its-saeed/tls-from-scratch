# Lesson 7: Async I/O Foundations

## How the OS tells you a socket is ready

When you call `read()` on a socket, it blocks until data arrives. In async, you can't block — you need the OS to notify you.

### kqueue (macOS) / epoll (Linux)

These are OS APIs for event notification:

1. **Register**: "Tell me when file descriptor 5 is readable"
2. **Wait**: Block until ANY registered fd has an event
3. **Process**: Handle the ready fds
4. **Repeat**

```c
// Pseudocode
kqueue = kqueue_create();
kqueue_register(kqueue, socket_fd, READABLE);

loop {
    events = kqueue_wait(kqueue);  // blocks until something is ready
    for event in events {
        // event.fd is ready — read from it without blocking
    }
}
```

One `kqueue_wait()` call watches thousands of sockets simultaneously.

### Non-blocking sockets

A socket set to non-blocking returns immediately from `read()`:
- Data available → returns the data
- No data → returns `WouldBlock` error (instead of blocking)

Async I/O = non-blocking sockets + kqueue/epoll to know when to retry.

## The mio crate

`mio` (Metal I/O) is a cross-platform wrapper around kqueue/epoll. Tokio is built on mio.

```rust
let mut poll = mio::Poll::new()?;
let mut events = mio::Events::with_capacity(1024);
poll.registry().register(&mut socket, Token(0), Interest::READABLE)?;

loop {
    poll.poll(&mut events, None)?; // wait for events
    for event in events.iter() {
        // event.token() tells you which socket is ready
    }
}
```

## Exercises

### Exercise 1: Raw kqueue
Use raw `libc::kqueue`, `libc::kevent` syscalls (macOS) to watch a TCP socket for readability. No mio, no tokio — just syscalls. Accept a connection, wait for data, read it, print it.

### Exercise 2: Non-blocking read
Set a socket to non-blocking (`set_nonblocking(true)`). Try to read — get `WouldBlock`. Register with kqueue. Wait for event. Read again — get data.

### Exercise 3: mio event loop
Rewrite Exercise 1 using `mio`. Register multiple sockets. Handle events for each. This is the foundation of a reactor.

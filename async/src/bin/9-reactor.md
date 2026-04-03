# Lesson 8: Event Loop + Reactor

Build a reactor using `mio` that watches file descriptors and wakes tasks when
I/O readiness events arrive. The reactor is the core loop that bridges the OS
kernel's event notification system (epoll on Linux, kqueue on macOS) with your
Rust futures.

## What you'll build

A `Reactor` struct that:
- Owns a `mio::Poll` instance and a slab of registered I/O sources
- Runs an event loop that calls `poll()` and dispatches readiness events
- Stores `Waker`s keyed by token so it can wake the right future
- Exposes `register()` / `deregister()` for adding and removing sockets
- Integrates with the executor from Lessons 5-7 via a shared handle

## Key concepts

- **mio::Poll** -- thin, portable wrapper over epoll / kqueue / IOCP
- **Token** -- integer handle that maps an event back to its I/O source
- **Interest** -- bitmask specifying READABLE, WRITABLE, or both
- **Readiness vs completion** -- reactor tells you *when* to try I/O, not that
  I/O is done (this is the readiness model)
- **Thread safety** -- the reactor typically lives on one thread; wakers can be
  sent across threads
- **Slab allocation** -- O(1) insert/remove for mapping tokens to wakers

## Exercises

1. **Single-fd reactor** -- create a `TcpListener`, register it with your
   reactor, and print a message every time a new connection arrives. Do not use
   tokio; use only `mio` and `std`.

2. **Multi-fd reactor** -- extend Exercise 1 to accept multiple connections and
   echo data back. Maintain a slab of connections and handle both READABLE and
   WRITABLE events.

3. **Waker integration** -- wire your reactor into the executor from Lesson 7.
   Register a socket, return a `Future` that resolves when the socket is
   readable, and `poll` it from your executor.

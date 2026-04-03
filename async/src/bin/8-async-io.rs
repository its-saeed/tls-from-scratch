// Lesson 7: Async I/O Foundations
// Use raw kqueue (macOS) or epoll (Linux) to watch sockets.

fn main() {
    // TODO:
    // 1. Create a TCP listener (non-blocking)
    // 2. Register it with kqueue/epoll (or use mio for portability)
    // 3. Wait for events
    // 4. Accept connection, register new socket
    // 5. Wait for readable event on connection
    // 6. Read data, print it
    //
    // No async/await — just raw event-driven I/O.
    // This is what tokio's reactor does under the hood.
    todo!()
}

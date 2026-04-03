# Lesson 18: tokio::net — TcpListener/TcpStream Wrapping mio + Reactor

## What you'll learn

- How `tokio::net::TcpStream` wraps `mio::net::TcpStream`
- The accept loop pattern with `TcpListener`
- How `AsyncRead`/`AsyncWrite` are implemented on top of readiness
- UDP and Unix socket support in Tokio

## Key concepts

### TcpListener accept loop

```rust
let listener = TcpListener::bind("0.0.0.0:8080").await?;
loop {
    let (stream, addr) = listener.accept().await?;
    tokio::spawn(handle_connection(stream, addr));
}
```

Internally, `accept()` calls `poll_accept()`, which checks mio readiness. On `WouldBlock`, it registers interest and returns `Pending`.

### TcpStream internals

`TcpStream` wraps a `PollEvented<mio::net::TcpStream>`. Every read/write:
1. Checks readiness via the I/O driver
2. Attempts the syscall
3. On `WouldBlock`, clears readiness and returns `Pending`
4. The driver re-wakes the task when readiness returns

### split() and into_split()

- `stream.split()` — borrows, returns `ReadHalf` + `WriteHalf` (not `Send`)
- `stream.into_split()` — owned, returns `OwnedReadHalf` + `OwnedWriteHalf` (`Send`)

Use `into_split()` when read/write halves go to different tasks.

### Other socket types

- `UdpSocket` — connectionless, `send_to`/`recv_from`
- `UnixStream` / `UnixListener` — Unix domain sockets (Linux/macOS)

## Exercises

1. Write a TCP echo server that splits each connection into a reader task and writer task using `into_split()`
2. Implement a simple UDP ping-pong between two sockets
3. Use `TcpStream::connect_with_config` to set `TCP_NODELAY` and observe latency differences
4. Build a Unix domain socket echo server and client
5. Benchmark `split()` vs `into_split()` — measure any overhead difference

// Lesson 11: AsyncRead / AsyncWrite
//
// Wrap non-blocking sockets in async read/write so they can be .await'd.
// Run with: cargo run -p async-lessons --bin 11-async-read-write -- <command>
//
// Commands:
//   poll-read           Demo poll_read with WouldBlock handling
//   echo-server         Async echo server using your AsyncTcpStream
//   all                 Run all demos

use clap::{Parser, Subcommand};
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::future::Future;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context as TaskContext, Waker};

// ============================================================
// AsyncTcpStream: non-blocking socket wrapped for async
// ============================================================

/// An async wrapper around a non-blocking TCP stream.
/// Registered with the reactor; poll_read/poll_write return
/// Pending when the socket would block.
struct AsyncTcpStream {
    inner: TcpStream,
    token: Token,
}

impl AsyncTcpStream {
    /// Try to read into buf. Returns Ready(Ok(n)) if data available,
    /// Pending if WouldBlock (registers waker with reactor).
    ///
    /// TODO: Implement this.
    ///   1. Call self.inner.read(buf)
    ///   2. Ok(n) → Poll::Ready(Ok(n))
    ///   3. Err(WouldBlock) → store waker, return Poll::Pending
    ///   4. Err(other) → Poll::Ready(Err(other))
    ///
    /// For storing the waker, you'll need access to the reactor.
    /// Use a thread-local or pass it as a parameter.
    fn poll_read(
        &mut self,
        _cx: &mut TaskContext<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        todo!("Implement poll_read")
    }

    /// Try to write buf. Returns Ready(Ok(n)) if bytes written,
    /// Pending if WouldBlock.
    ///
    /// TODO: Implement this. Same pattern as poll_read.
    fn poll_write(
        &mut self,
        _cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        todo!("Implement poll_write")
    }
}

// ============================================================
// Read/Write futures (make poll_read/poll_write .await-able)
// ============================================================

/// A future that reads from an AsyncTcpStream once.
///
/// TODO: Implement Future for ReadFuture.
///   poll() should call self.stream.poll_read(cx, self.buf)
struct ReadFuture<'a> {
    stream: &'a mut AsyncTcpStream,
    buf: &'a mut [u8],
}

// TODO: impl<'a> Future for ReadFuture<'a> { ... }

/// A future that writes to an AsyncTcpStream once.
struct WriteFuture<'a> {
    stream: &'a mut AsyncTcpStream,
    buf: &'a [u8],
}

// TODO: impl<'a> Future for WriteFuture<'a> { ... }

// ============================================================
// Higher-level helpers
// ============================================================

/// Write all bytes, looping on partial writes.
///
/// TODO: Implement this.
///   while !buf.is_empty() {
///       let n = write(stream, buf).await?;
///       buf = &buf[n..];
///   }
async fn write_all(_stream: &mut AsyncTcpStream, _buf: &[u8]) -> io::Result<()> {
    todo!("Implement write_all")
}

/// Read exactly buf.len() bytes, looping on partial reads.
///
/// TODO: Implement this.
///   while filled < buf.len() {
///       let n = read(stream, &mut buf[filled..]).await?;
///       if n == 0 { return Err(UnexpectedEof); }
///       filled += n;
///   }
async fn read_exact(_stream: &mut AsyncTcpStream, _buf: &mut [u8]) -> io::Result<()> {
    todo!("Implement read_exact")
}

// ============================================================
// Demo: raw poll_read without executor (shows the pattern)
// ============================================================

fn demo_poll_read() {
    println!("=== poll_read pattern (without executor) ===");
    println!("Demonstrating the try → WouldBlock → register → retry pattern.");
    println!();
    println!("This demo uses raw mio to show what AsyncTcpStream does internally.");
    println!("Connect with: echo hello | nc 127.0.0.1 8090");
    println!();

    let addr: SocketAddr = "127.0.0.1:8090".parse().unwrap();
    let mut listener = TcpListener::bind(addr).unwrap();
    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(32);

    poll.registry()
        .register(&mut listener, Token(0), Interest::READABLE)
        .unwrap();

    println!("  [1] Listening on {addr}, waiting for connection...");
    poll.poll(&mut events, None).unwrap();

    let (mut stream, peer) = listener.accept().unwrap();
    println!("  [2] Accepted connection from {peer}");

    // Register stream for readable events
    poll.registry()
        .register(&mut stream, Token(1), Interest::READABLE)
        .unwrap();

    // Try non-blocking read — might get WouldBlock
    let mut buf = [0u8; 1024];
    match stream.read(&mut buf) {
        Ok(n) => {
            println!("  [3] read() returned {n} bytes immediately: {:?}",
                String::from_utf8_lossy(&buf[..n]));
        }
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
            println!("  [3] read() returned WouldBlock — no data yet");
            println!("      In async: we'd store waker and return Pending here");
            println!("      Waiting for reactor event...");

            poll.poll(&mut events, None).unwrap();

            match stream.read(&mut buf) {
                Ok(n) => {
                    println!("  [4] After event, read() returned {n} bytes: {:?}",
                        String::from_utf8_lossy(&buf[..n]));
                }
                Err(e) => println!("  [4] read error: {e}"),
            }
        }
        Err(e) => println!("  [3] read error: {e}"),
    }

    println!();
    println!("Takeaway: this is exactly what AsyncTcpStream::poll_read does.");
    println!("  try read → WouldBlock → register waker → Pending → event → retry → Ready");
}

fn demo_echo_server() {
    println!("=== Async Echo Server ===");
    println!("TODO: implement AsyncTcpStream + ReadFuture + WriteFuture,");
    println!("then build an echo server using your executor from Lesson 5/10.");
    println!();
    println!("The goal:");
    println!("  async fn handle(mut stream: AsyncTcpStream) {{");
    println!("      let mut buf = [0u8; 1024];");
    println!("      loop {{");
    println!("          let n = stream.read(&mut buf).await;");
    println!("          if n == 0 {{ return; }}");
    println!("          stream.write_all(&buf[..n]).await;");
    println!("      }}");
    println!("  }}");
}

// ============================================================
// CLI
// ============================================================

#[derive(Parser)]
#[command(name = "async-read-write", about = "Lesson 11: AsyncRead / AsyncWrite")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Demo poll_read pattern with raw mio
    PollRead,
    /// Async echo server (TODO: implement)
    EchoServer,
    /// Run all demos
    All,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::PollRead => demo_poll_read(),
        Command::EchoServer => demo_echo_server(),
        Command::All => {
            println!("Running poll-read demo (connect with: echo hello | nc 127.0.0.1 8090):");
            println!();
            demo_poll_read();
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_blocking_would_block() {
        // Create a listener, connect to it, try to read immediately — should WouldBlock
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let _client = std::net::TcpStream::connect(addr).unwrap();
        let (stream, _) = listener.accept().unwrap();
        stream.set_nonblocking(true).unwrap();

        let mut stream = stream;
        let mut buf = [0u8; 64];
        let result = stream.read(&mut buf);
        assert!(
            result.is_err() && result.unwrap_err().kind() == io::ErrorKind::WouldBlock,
            "Non-blocking read with no data should return WouldBlock"
        );
    }

    #[test]
    fn non_blocking_read_after_write() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let mut client = std::net::TcpStream::connect(addr).unwrap();
        let (stream, _) = listener.accept().unwrap();
        stream.set_nonblocking(true).unwrap();

        // Write from client
        client.write_all(b"hello").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Now read should succeed
        let mut stream = stream;
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"hello");
    }
}

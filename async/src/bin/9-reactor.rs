// Lesson 9: Event Loop + Reactor
//
// Build a reactor that bridges OS events (kqueue/epoll) to wakers.
// Run with: cargo run -p async-lessons --bin 9-reactor -- <command>
//
// Commands:
//   single-socket      Watch one TcpListener, print connections
//   multi-socket       Accept + echo multiple connections
//   waker-integration  Wire reactor into a future + block_on
//   all                Run all demos (single-socket only in all mode)

use clap::{Parser, Subcommand};
use mio::net::TcpListener;
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::task::Waker;

// ============================================================
// The Reactor
// ============================================================

/// The Reactor bridges OS I/O events to async task wakers.
///
/// It owns a mio::Poll (kqueue/epoll wrapper) and a map of Token → Waker.
/// When an I/O event fires, it wakes the corresponding task.
struct Reactor {
    /// The OS event notification handle
    poll: Poll,
    /// Maps I/O tokens to the waker that should be notified
    wakers: HashMap<Token, Waker>,
    /// Counter for assigning unique tokens
    next_token: usize,
}

impl Reactor {
    fn new() -> io::Result<Self> {
        Ok(Self {
            poll: Poll::new()?,
            wakers: HashMap::new(),
            next_token: 0,
        })
    }

    /// Register an I/O source with the reactor.
    /// Returns a Token that identifies this source in future events.
    ///
    /// TODO: Implement this.
    ///   1. Create a Token from self.next_token, increment counter
    ///   2. Call self.poll.registry().register(source, token, interest)
    ///   3. Return the token
    fn register(
        &mut self,
        source: &mut impl mio::event::Source,
        interest: Interest,
    ) -> io::Result<Token> {
        todo!("Implement register")
    }

    /// Store or update the waker for a token.
    /// Called by futures in their poll() when they return Pending.
    fn set_waker(&mut self, token: Token, waker: Waker) {
        self.wakers.insert(token, waker);
    }

    /// Remove an I/O source from the reactor.
    fn deregister(&mut self, source: &mut impl mio::event::Source, token: Token) -> io::Result<()> {
        self.poll.registry().deregister(source)?;
        self.wakers.remove(&token);
        Ok(())
    }

    /// Block until at least one I/O event fires, then wake the corresponding tasks.
    ///
    /// TODO: Implement this.
    ///   1. Create mio::Events buffer
    ///   2. Call self.poll.poll(&mut events, timeout)
    ///   3. For each event, look up waker by event.token()
    ///   4. Call waker.wake_by_ref() for each
    ///   5. Return the number of events processed
    fn wait(&mut self, timeout: Option<std::time::Duration>) -> io::Result<usize> {
        todo!("Implement wait")
    }
}

// ============================================================
// CLI
// ============================================================

#[derive(Parser)]
#[command(name = "reactor", about = "Lesson 9: Event Loop + Reactor")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Watch one TcpListener, print when clients connect
    SingleSocket,
    /// Accept multiple connections, echo data back
    MultiSocket,
    /// Wire the reactor to a future via wakers
    WakerIntegration,
    /// Run single-socket demo
    All,
}

fn demo_single_socket() {
    println!("=== Single-Socket Reactor ===");
    println!("Watching one TcpListener for connections.");
    println!("Connect with: nc 127.0.0.1 8080");
    println!("Press Ctrl+C to stop.");
    println!();

    // This demo is pre-built to show the raw event loop pattern.
    // No executor, no wakers — just mio directly.

    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let mut listener = TcpListener::bind(addr).unwrap();
    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(32);

    let listener_token = Token(0);
    poll.registry()
        .register(&mut listener, listener_token, Interest::READABLE)
        .unwrap();

    println!("  [reactor] Listening on {addr}, registered as Token(0)");
    println!("  [reactor] Entering event loop...");
    println!();

    loop {
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
            match event.token() {
                token if token == listener_token => {
                    match listener.accept() {
                        Ok((stream, addr)) => {
                            println!("  [event] Token(0) READABLE → accepted connection from {addr}");
                            drop(stream); // we don't handle the connection in this demo
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            println!("  [event] Token(0) READABLE → but accept() would block (spurious wake)");
                        }
                        Err(e) => {
                            println!("  [event] Token(0) accept error: {e}");
                        }
                    }
                }
                token => {
                    println!("  [event] Unexpected token: {:?}", token);
                }
            }
        }
    }
}

fn demo_multi_socket() {
    println!("=== Multi-Socket Reactor ===");
    println!("Accepting multiple connections and echoing data.");
    println!("Connect with: nc 127.0.0.1 8081 (multiple terminals)");
    println!("Press Ctrl+C to stop.");
    println!();

    // TODO: Implement this.
    //
    // 1. Create Poll, register TcpListener on Token(0)
    // 2. Maintain a HashMap<Token, mio::net::TcpStream> for connections
    // 3. When listener is readable: accept, register new stream for READABLE
    // 4. When a stream is readable: read data, write it back (echo)
    // 5. If read returns 0 bytes: deregister and remove the stream
    //
    // Key insight: one event loop handles the listener AND all streams.
    // Each socket gets its own Token so you know which one to handle.
    //
    // Expected output:
    //   [reactor] Listening on 127.0.0.1:8081
    //   [event] Token(0) → new connection from 127.0.0.1:54321 → Token(1)
    //   [event] Token(1) READABLE → echoed 6 bytes: "hello\n"
    //   [event] Token(0) → new connection from 127.0.0.1:54322 → Token(2)
    //   [event] Token(2) READABLE → echoed 4 bytes: "hi\n"
    //   [event] Token(1) READABLE → 0 bytes, client disconnected
    todo!("Implement multi-socket echo reactor")
}

fn demo_waker_integration() {
    println!("=== Waker Integration ===");
    println!("Connecting the reactor to an executor via wakers.");
    println!();

    // TODO: Implement this.
    //
    // This is the key exercise that bridges Course 1 and Course 2.
    //
    // 1. Create a Reactor (shared via thread_local! or Arc<Mutex>)
    // 2. Create a ReadableFuture that:
    //    - On first poll: registers a socket with the reactor,
    //      stores the waker via reactor.set_waker(), returns Pending
    //    - On later polls: tries read() on the socket
    //      → data available: returns Ready(data)
    //      → WouldBlock: returns Pending (waker already set)
    // 3. Use block_on (from Lesson 5) to run the future
    // 4. In the block_on loop, when Pending, call reactor.wait()
    //    instead of thread::park()
    //
    // The flow:
    //   block_on polls ReadableFuture → Pending
    //   block_on calls reactor.wait() → blocks on mio::poll()
    //   client sends data → OS event → reactor wakes the task
    //   block_on polls ReadableFuture again → Ready(data)
    //
    // Test: start the demo, in another terminal: echo hello | nc 127.0.0.1 8082
    println!("TODO: Implement waker integration.");
    println!("See the exercises in 9-reactor.md for the step-by-step guide.");
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::SingleSocket => demo_single_socket(),
        Command::MultiSocket => demo_multi_socket(),
        Command::WakerIntegration => demo_waker_integration(),
        Command::All => {
            println!("Running single-socket demo (Ctrl+C to stop):");
            println!();
            demo_single_socket();
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
    fn reactor_new() {
        let reactor = Reactor::new().unwrap();
        assert_eq!(reactor.next_token, 0);
        assert!(reactor.wakers.is_empty());
    }

    #[test]
    fn reactor_register_increments_token() {
        let mut reactor = Reactor::new().unwrap();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let mut listener = TcpListener::bind(addr).unwrap();

        let token = reactor.register(&mut listener, Interest::READABLE).unwrap();
        assert_eq!(token, Token(0));
        assert_eq!(reactor.next_token, 1);
    }

    #[test]
    fn reactor_set_and_remove_waker() {
        let mut reactor = Reactor::new().unwrap();
        let waker = Waker::noop();
        let token = Token(42);

        reactor.set_waker(token, waker.clone());
        assert!(reactor.wakers.contains_key(&token));

        reactor.wakers.remove(&token);
        assert!(!reactor.wakers.contains_key(&token));
    }
}

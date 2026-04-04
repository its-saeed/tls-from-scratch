// Lesson 17: Tokio's I/O Driver
//
// Compare raw mio with tokio's I/O abstractions.
// Run with: cargo run -p async-lessons --bin 17-tokio-io-driver -- <command>
//
// Commands:
//   mio-echo         Raw mio echo server (no tokio)
//   tokio-echo       Same echo server using tokio
//   readiness        Check readiness state of a connection
//   all              Run tokio-echo (mio-echo is blocking)

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tokio-io-driver", about = "Lesson 17: Tokio's I/O Driver")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Raw mio echo server
    MioEcho,
    /// Tokio echo server
    TokioEcho,
    /// Explore readiness states
    Readiness,
    /// Run tokio-echo
    All,
}

fn demo_mio_echo() {
    println!("=== Raw mio Echo Server ===");
    println!("Connect with: echo hello | nc 127.0.0.1 8091");
    println!();

    use mio::net::TcpListener;
    use mio::{Events, Interest, Poll, Token};
    use std::collections::HashMap;
    use std::io::{Read, Write};

    let addr = "127.0.0.1:8091".parse().unwrap();
    let mut listener = TcpListener::bind(addr).unwrap();
    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(128);
    let mut connections: HashMap<Token, mio::net::TcpStream> = HashMap::new();
    let mut next_token = 1usize;

    poll.registry().register(&mut listener, Token(0), Interest::READABLE).unwrap();
    println!("  Listening on {addr} (mio, no tokio)");

    loop {
        poll.poll(&mut events, None).unwrap();
        for event in events.iter() {
            match event.token() {
                Token(0) => {
                    loop {
                        match listener.accept() {
                            Ok((mut stream, addr)) => {
                                let token = Token(next_token);
                                next_token += 1;
                                poll.registry().register(&mut stream, token, Interest::READABLE).unwrap();
                                println!("  [mio] accepted {addr} → Token({})", token.0);
                                connections.insert(token, stream);
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                            Err(e) => { eprintln!("  accept error: {e}"); break; }
                        }
                    }
                }
                token => {
                    let mut buf = [0u8; 1024];
                    if let Some(stream) = connections.get_mut(&token) {
                        match stream.read(&mut buf) {
                            Ok(0) => {
                                println!("  [mio] Token({}) disconnected", token.0);
                                connections.remove(&token);
                            }
                            Ok(n) => {
                                println!("  [mio] Token({}) echoed {} bytes", token.0, n);
                                let _ = stream.write_all(&buf[..n]);
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                            Err(e) => {
                                eprintln!("  [mio] Token({}) error: {e}", token.0);
                                connections.remove(&token);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn demo_tokio_echo() {
    println!("=== Tokio Echo Server ===");
    println!("Same echo server, but tokio handles mio registration + wakers.");
    println!("Connect with: echo hello | nc 127.0.0.1 8092");
    println!();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:8092").await.unwrap();
        println!("  Listening on 127.0.0.1:8092 (tokio)");

        loop {
            let (mut stream, addr) = listener.accept().await.unwrap();
            println!("  [tokio] accepted {addr}");

            tokio::task::spawn_local(async move {
                let mut buf = [0u8; 1024];
                loop {
                    let n = match tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await {
                        Ok(0) => { println!("  [tokio] {addr} disconnected"); return; }
                        Ok(n) => n,
                        Err(e) => { eprintln!("  [tokio] {addr} error: {e}"); return; }
                    };
                    println!("  [tokio] {addr} echoed {n} bytes");
                    if tokio::io::AsyncWriteExt::write_all(&mut stream, &buf[..n]).await.is_err() {
                        return;
                    }
                }
            });
        }
    });
}

fn demo_readiness() {
    println!("=== Readiness Exploration ===");
    println!("Checking I/O readiness states on a TcpStream.");
    println!();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        // Create a listener and connect to it
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client = tokio::net::TcpStream::connect(addr).await.unwrap();
        let (server_stream, _) = listener.accept().await.unwrap();

        // Check readiness
        let ready = client.ready(tokio::io::Interest::READABLE | tokio::io::Interest::WRITABLE).await.unwrap();
        println!("  Client readiness:");
        println!("    readable: {}", ready.is_readable());
        println!("    writable: {}", ready.is_writable());

        // Write from server, check client readiness again
        let mut server_stream = server_stream;
        tokio::io::AsyncWriteExt::write_all(&mut server_stream, b"hello").await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let ready = client.ready(tokio::io::Interest::READABLE).await.unwrap();
        println!("  After server writes:");
        println!("    readable: {} (should be true)", ready.is_readable());

        println!();
        println!("Takeaway: readiness tells you WHEN to try I/O, not that it will succeed.");
    });
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::MioEcho => demo_mio_echo(),
        Command::TokioEcho => demo_tokio_echo(),
        Command::Readiness => demo_readiness(),
        Command::All => demo_readiness(), // non-blocking demo
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn tokio_echo_roundtrip() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 64];
            let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await.unwrap();
            tokio::io::AsyncWriteExt::write_all(&mut stream, &buf[..n]).await.unwrap();
        });

        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        tokio::io::AsyncWriteExt::write_all(&mut client, b"hello").await.unwrap();
        let mut buf = [0u8; 64];
        let n = tokio::io::AsyncReadExt::read(&mut client, &mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"hello");
    }
}

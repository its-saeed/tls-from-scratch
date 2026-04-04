// Lesson 19: tokio::net — TcpListener, TcpStream, and the Reactor
//
// Run with: cargo run -p async-lessons --bin 19-tokio-net -- <command>
//
// Commands:
//   echo-server     Start a TCP echo server on 127.0.0.1:8080
//   split-demo      Demo split() vs into_split() on a TcpStream
//   udp-demo        Send/receive a UDP datagram
//   all             Run non-blocking demos

use clap::{Parser, Subcommand};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};

#[derive(Parser)]
#[command(name = "tokio-net", about = "Lesson 19: tokio::net networking")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start a TCP echo server (Ctrl+C to stop)
    EchoServer,
    /// Demo into_split: separate reader/writer tasks
    SplitDemo,
    /// Demo UDP send/receive between two sockets
    UdpDemo,
    /// Run non-server demos
    All,
}

async fn demo_echo_server() {
    println!("=== TCP Echo Server ===");
    println!("Listening on 127.0.0.1:8080. Connect with: nc 127.0.0.1 8080");
    println!("Press Ctrl+C to stop.\n");

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    let conn_count = Arc::new(AtomicUsize::new(0));

    loop {
        let (stream, addr) = listener.accept().await.unwrap();
        let n = conn_count.fetch_add(1, Ordering::SeqCst) + 1;
        println!("  Connection #{n} from {addr}");

        tokio::spawn(async move {
            if let Err(e) = handle_echo(stream).await {
                println!("  Connection from {addr} error: {e}");
            }
            println!("  {addr} disconnected");
        });
    }
}

async fn handle_echo(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 1024];
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }
        stream.write_all(&buf[..n]).await?;
    }
}

async fn demo_split() {
    println!("=== into_split() Demo ===");
    println!("Splitting a TCP connection into separate reader/writer tasks.\n");

    // Start a tiny server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    println!("  Server bound to {addr}");

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).await.unwrap();
        stream.write_all(&buf[..n]).await.unwrap();
    });

    // Client: connect, split, use separate halves
    let stream = TcpStream::connect(addr).await.unwrap();
    let (mut reader, mut writer) = stream.into_split();

    // Writer task
    let write_handle = tokio::spawn(async move {
        writer.write_all(b"hello from split").await.unwrap();
        writer.shutdown().await.unwrap();
        println!("  [writer task] sent 'hello from split'");
    });

    // Reader task
    let read_handle = tokio::spawn(async move {
        let mut buf = vec![0u8; 64];
        let n = reader.read(&mut buf).await.unwrap();
        let msg = String::from_utf8_lossy(&buf[..n]);
        println!("  [reader task] got back: '{msg}'");
    });

    write_handle.await.unwrap();
    read_handle.await.unwrap();
    server.await.unwrap();

    println!();
    println!("Takeaway: into_split() gives owned halves that are Send + 'static.");
    println!("You can move each half into a separate spawned task.");
}

async fn demo_udp() {
    println!("=== UDP Demo ===");
    println!("Two sockets send datagrams to each other.\n");

    let socket_a = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let socket_b = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let addr_a = socket_a.local_addr().unwrap();
    let addr_b = socket_b.local_addr().unwrap();

    println!("  Socket A: {addr_a}");
    println!("  Socket B: {addr_b}");

    // A sends ping to B
    let start = std::time::Instant::now();
    socket_a.send_to(b"ping", addr_b).await.unwrap();
    println!("  A → B: 'ping'");

    // B receives and replies
    let mut buf = [0u8; 64];
    let (n, from) = socket_b.recv_from(&mut buf).await.unwrap();
    println!("  B received '{}' from {from}", String::from_utf8_lossy(&buf[..n]));

    socket_b.send_to(b"pong", from).await.unwrap();
    println!("  B → A: 'pong'");

    // A receives reply
    let (n, _) = socket_a.recv_from(&mut buf).await.unwrap();
    let rtt = start.elapsed();
    println!("  A received '{}'", String::from_utf8_lossy(&buf[..n]));
    println!("  Round-trip time: {:?}", rtt);
    println!();
    println!("Takeaway: UDP is connectionless — no accept/connect handshake.");
    println!("send_to/recv_from work with any address. Fast but unreliable.");
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::EchoServer => demo_echo_server().await,
        Command::SplitDemo => demo_split().await,
        Command::UdpDemo => demo_udp().await,
        Command::All => {
            demo_split().await;
            println!("\n");
            demo_udp().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tcp_echo_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 64];
            let n = s.read(&mut buf).await.unwrap();
            s.write_all(&buf[..n]).await.unwrap();
        });

        let mut client = TcpStream::connect(addr).await.unwrap();
        client.write_all(b"test").await.unwrap();
        client.shutdown().await.unwrap();

        let mut buf = vec![];
        client.read_to_end(&mut buf).await.unwrap();
        assert_eq!(buf, b"test");
    }

    #[tokio::test]
    async fn udp_send_recv() {
        let a = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let b = UdpSocket::bind("127.0.0.1:0").await.unwrap();

        a.send_to(b"hello", b.local_addr().unwrap()).await.unwrap();
        let mut buf = [0u8; 64];
        let (n, _) = b.recv_from(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"hello");
    }
}

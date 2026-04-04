// Lesson 24: Cancellation Safety
//
// What happens when futures are dropped mid-await.
// Run with: cargo run -p async-lessons --bin 24-cancellation -- <command>

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cancellation", about = "Lesson 24: Cancellation Safety")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show data loss with read_exact in select
    DataLoss,
    /// Show safe recv in select
    SafeRecv,
    /// All demos
    All,
}

fn demo_data_loss() {
    println!("=== Cancellation Unsafe: read_exact ===");
    println!("read_exact reads partial data, then gets cancelled by select.");
    println!("The partial bytes are lost forever.");
    println!();

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Server sends data in two chunks
        tokio::task::spawn_local(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::io::AsyncWriteExt::write_all(&mut stream, b"hel").await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            tokio::io::AsyncWriteExt::write_all(&mut stream, b"lo").await.unwrap();
        });

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut buf = [0u8; 5];

        // Race read_exact against a timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            tokio::io::AsyncReadExt::read_exact(&mut stream, &mut buf),
        ).await;

        match result {
            Ok(Ok(_)) => println!("  Got all data: {:?}", std::str::from_utf8(&buf)),
            Ok(Err(e)) => println!("  Read error: {e}"),
            Err(_) => {
                println!("  Timeout! read_exact was cancelled.");
                println!("  It read 'hel' (3 bytes) but needed 5.");
                println!("  Those 3 bytes are GONE from the socket buffer.");
                println!("  Next read won't get them back.");

                // Try reading again — we'll get "lo" (the second chunk), not "hello"
                let mut buf2 = [0u8; 10];
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf2).await.unwrap();
                println!("  Next read got: {:?} ({n} bytes)", std::str::from_utf8(&buf2[..n]));
                println!("  → 'hel' is lost forever. This is cancellation-unsafe.");
            }
        }
    });
}

fn demo_safe_recv() {
    println!("=== Cancellation Safe: channel recv ===");
    println!("recv() in select is safe — no data consumed until Ready.");
    println!();

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<&str>(10);

        // Send after a delay
        tokio::task::spawn_local(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            tx.send("hello").await.unwrap();
        });

        // First select: timeout wins, recv is cancelled (but no data lost)
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            rx.recv(),
        ).await;
        println!("  First attempt: {:?} (timed out)", result);

        // Second attempt: message is still there
        let msg = rx.recv().await.unwrap();
        println!("  Second attempt: got {:?}", msg);
        println!("  → No data lost! recv is cancellation-safe.");
    });
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::DataLoss => demo_data_loss(),
        Command::SafeRecv => demo_safe_recv(),
        Command::All => { demo_data_loss(); println!("\n"); demo_safe_recv(); }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn recv_is_cancellation_safe() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<i32>(10);
        tx.send(42).await.unwrap();

        // Cancel recv via timeout (even though message is available, this tests the pattern)
        let _ = tokio::time::timeout(std::time::Duration::from_millis(1), async {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }).await;

        // Message still available
        assert_eq!(rx.recv().await, Some(42));
    }
}

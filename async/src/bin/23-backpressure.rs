// Lesson 23: Backpressure
//
// Bounded channels, Semaphore, and flow control.
// Run with: cargo run -p async-lessons --bin 23-backpressure -- <command>

use clap::{Parser, Subcommand};
use std::time::Instant;

#[derive(Parser)]
#[command(name = "backpressure", about = "Lesson 23: Backpressure")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Bounded channel: producer blocks when buffer is full
    BoundedChannel,
    /// Semaphore rate limiting
    Semaphore,
    /// All demos
    All,
}

fn demo_bounded_channel() {
    println!("=== Bounded Channel Backpressure ===");
    println!("Producer sends 20 items into a channel(5). Slow consumer.");
    println!();

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<usize>(5);
        let start = Instant::now();

        tokio::task::spawn_local(async move {
            for i in 0..20 {
                tx.send(i).await.unwrap();
                println!("  [producer] sent {i} at {:?}", start.elapsed());
            }
        });

        tokio::task::spawn_local(async move {
            while let Some(i) = rx.recv().await {
                println!("  [consumer] got {i} at {:?}", start.elapsed());
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });

        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    });

    println!();
    println!("Takeaway: producer is throttled by the channel capacity.");
    println!("Without backpressure, all 20 items would queue instantly (OOM risk).");
}

fn demo_semaphore() {
    println!("=== Semaphore Rate Limiting ===");
    println!("10 tasks, but only 3 can run concurrently.");
    println!();

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(3));
        let start = Instant::now();
        let mut handles = vec![];

        for i in 0..10 {
            let sem = sem.clone();
            handles.push(tokio::task::spawn_local(async move {
                let _permit = sem.acquire().await.unwrap();
                println!("  [task {i}] started at {:?}", start.elapsed());
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                println!("  [task {i}] done at {:?}", start.elapsed());
            }));
        }

        for h in handles { h.await.unwrap(); }
    });

    println!();
    println!("Takeaway: Semaphore limits concurrent work. Essential for rate limiting.");
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::BoundedChannel => demo_bounded_channel(),
        Command::Semaphore => demo_semaphore(),
        Command::All => { demo_bounded_channel(); println!("\n"); demo_semaphore(); }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn bounded_channel_blocks_producer() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<i32>(2);
        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();
        // Third send would block — channel is full (receiver not consuming)
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            tx.send(3),
        ).await;
        assert!(result.is_err(), "Should timeout because channel is full");
    }
}

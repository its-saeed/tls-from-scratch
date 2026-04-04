// Lesson 25: Bridging Sync and Async
// Run with: cargo run -p async-lessons --bin 25-sync-async-bridge -- <command>

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sync-async-bridge", about = "Lesson 25: Sync/Async Bridge")]
struct Cli { #[command(subcommand)] command: Command }

#[derive(Subcommand)]
enum Command {
    /// Call async from sync using block_on
    BlockOn,
    /// Call sync (CPU-heavy) from async using spawn_blocking
    SpawnBlocking,
    /// Use Handle to spawn from a std thread
    Handle,
    All,
}

fn demo_block_on() {
    println!("=== block_on: async from sync ===");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        42
    });
    println!("  Got {result} from async world");
    println!("  Takeaway: block_on bridges sync → async. Blocks the current thread.");
}

fn demo_spawn_blocking() {
    println!("=== spawn_blocking: sync from async ===");
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let result = tokio::task::spawn_blocking(|| {
            // Simulate CPU-heavy work
            let mut sum = 0u64;
            for i in 0..1_000_000 { sum += i; }
            println!("  [blocking thread] computed sum = {sum}");
            sum
        }).await.unwrap();
        println!("  [async] got {result} from blocking thread");
    });
    println!("  Takeaway: spawn_blocking runs sync code on a separate thread pool.");
    println!("  Never do CPU-heavy work directly in async — it blocks the executor.");
}

fn demo_handle() {
    println!("=== Handle: spawn from std thread ===");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let handle = rt.handle().clone();

    let join = std::thread::spawn(move || {
        handle.block_on(async {
            println!("  [std thread] running async code via handle");
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            println!("  [std thread] done");
        });
    });
    join.join().unwrap();
    println!("  Takeaway: Handle lets any thread submit work to the runtime.");
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::BlockOn => demo_block_on(),
        Command::SpawnBlocking => demo_spawn_blocking(),
        Command::Handle => demo_handle(),
        Command::All => { demo_block_on(); println!(); demo_spawn_blocking(); println!(); demo_handle(); }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn spawn_blocking_returns_result() {
        let val = tokio::task::spawn_blocking(|| 42).await.unwrap();
        assert_eq!(val, 42);
    }
}

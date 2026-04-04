// Lesson 26: Streams
// Run with: cargo run -p async-lessons --bin 26-streams -- <command>

use clap::{Parser, Subcommand};
use tokio_stream::StreamExt;

#[derive(Parser)]
#[command(name = "streams", about = "Lesson 26: Streams")]
struct Cli { #[command(subcommand)] command: Command }

#[derive(Subcommand)]
enum Command {
    /// Create and consume a stream
    Basic,
    /// Transform streams with map/filter
    Transform,
    /// Stream from a channel
    Channel,
    All,
}

fn demo_basic() {
    println!("=== Basic Stream ===");
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let mut stream = tokio_stream::iter(vec![1, 2, 3, 4, 5]);
        while let Some(val) = stream.next().await {
            println!("  got: {val}");
        }
    });
    println!("  Takeaway: Stream is like Iterator, but each .next() is async.");
}

fn demo_transform() {
    println!("=== Stream Transformations ===");
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let stream = tokio_stream::iter(1..=10);
        let mut evens_doubled = stream
            .filter(|x| x % 2 == 0)
            .map(|x| x * 2);

        let mut results = vec![];
        while let Some(val) = evens_doubled.next().await {
            results.push(val);
        }
        println!("  filter(even) + map(*2): {:?}", results);
        // [4, 8, 12, 16, 20]
    });
    println!("  Takeaway: StreamExt gives you map, filter, take, etc. — like Iterator.");
}

fn demo_channel() {
    println!("=== Stream from Channel ===");
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let (tx, rx) = tokio::sync::mpsc::channel::<String>(10);
        let mut stream = tokio_stream::wrappers::ReceiverStream::new(rx);

        tokio::task::spawn_local(async move {
            for msg in ["hello", "world", "done"] {
                tx.send(msg.to_string()).await.unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });

        while let Some(msg) = stream.next().await {
            println!("  stream got: {msg}");
        }
    });
    println!("  Takeaway: channels become streams via ReceiverStream wrapper.");
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Basic => demo_basic(),
        Command::Transform => demo_transform(),
        Command::Channel => demo_channel(),
        Command::All => { demo_basic(); println!(); demo_transform(); println!(); demo_channel(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stream_collect() {
        let stream = tokio_stream::iter(vec![1, 2, 3]);
        let vals: Vec<_> = stream.collect().await;
        assert_eq!(vals, vec![1, 2, 3]);
    }
}

// Lesson 18: tokio::sync Internals
//
// Mutex, RwLock, Semaphore, Notify — how they differ from std.
// Run with: cargo run -p async-lessons --bin 18-tokio-sync -- <command>
//
// Commands:
//   mutex          Demo tokio::sync::Mutex across concurrent tasks
//   semaphore      Demo Semaphore for concurrency limiting
//   notify         Demo Notify for async signaling
//   rwlock         Demo RwLock with concurrent readers
//   all            Run all demos

use clap::{Parser, Subcommand};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, Notify, RwLock};

#[derive(Parser)]
#[command(name = "tokio-sync", about = "Lesson 18: tokio::sync primitives")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Increment a shared counter from 100 concurrent tasks using tokio::sync::Mutex
    Mutex,
    /// Limit concurrency to 5 using Semaphore, spawn 20 tasks
    Semaphore,
    /// Producer-consumer signaling with Notify
    Notify,
    /// Concurrent readers with RwLock
    Rwlock,
    /// Run all demos
    All,
}

async fn demo_mutex() {
    println!("=== tokio::sync::Mutex Demo ===");
    println!("100 tasks increment a shared counter.\n");

    let counter = Arc::new(Mutex::new(0u64));
    let mut handles = vec![];

    let start = Instant::now();
    for _ in 0..100 {
        let counter = counter.clone();
        handles.push(tokio::spawn(async move {
            let mut guard = counter.lock().await;
            *guard += 1;
            // Lock is held across this yield point — safe with tokio::sync::Mutex
            tokio::task::yield_now().await;
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let final_val = *counter.lock().await;
    println!("  Final counter: {} (expected 100)", final_val);
    println!("  Elapsed: {:?}", start.elapsed());
    println!();
    println!("Takeaway: tokio::sync::Mutex is safe to hold across .await points.");
    println!("The lock yields to the executor instead of blocking the OS thread.");
}

async fn demo_semaphore() {
    println!("=== Semaphore Demo ===");
    println!("20 tasks, but only 5 can run concurrently.\n");

    let sem = Arc::new(tokio::sync::Semaphore::new(5));
    let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let max_active = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let mut handles = vec![];
    let start = Instant::now();

    for i in 0..20 {
        let sem = sem.clone();
        let active = active.clone();
        let max_active = max_active.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let cur = active.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            max_active.fetch_max(cur, std::sync::atomic::Ordering::SeqCst);

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            active.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);

            if i < 3 {
                println!("  Task {i} completed (active at start: {cur})");
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let peak = max_active.load(std::sync::atomic::Ordering::SeqCst);
    println!("  ...");
    println!("  Peak concurrent: {} (limit was 5)", peak);
    println!("  Elapsed: {:?}", start.elapsed());
    println!();
    println!("Takeaway: Semaphore limits concurrency without spawning fewer tasks.");
    println!("Permits are returned automatically when the guard is dropped.");
}

async fn demo_notify() {
    println!("=== Notify Demo ===");
    println!("Producer pushes items, consumer waits on Notify.\n");

    let queue: Arc<std::sync::Mutex<std::collections::VecDeque<String>>> =
        Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new()));
    let notify = Arc::new(Notify::new());

    let q = queue.clone();
    let n = notify.clone();

    // Consumer
    let consumer = tokio::spawn(async move {
        let mut received = vec![];
        loop {
            n.notified().await;
            while let Some(item) = q.lock().unwrap().pop_front() {
                if item == "DONE" {
                    println!("  Consumer received {} items: {:?}", received.len(), received);
                    return received;
                }
                received.push(item);
            }
        }
    });

    // Producer
    for i in 0..5 {
        queue.lock().unwrap().push_back(format!("msg-{i}"));
        notify.notify_one();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    queue.lock().unwrap().push_back("DONE".into());
    notify.notify_one();

    let items = consumer.await.unwrap();
    assert_eq!(items.len(), 5);
    println!();
    println!("Takeaway: Notify is the simplest async signal — no data, just 'wake up'.");
    println!("Combine with a std::sync::Mutex<VecDeque> for a basic async queue.");
}

async fn demo_rwlock() {
    println!("=== RwLock Demo ===");
    println!("Multiple readers run concurrently, writer gets exclusive access.\n");

    let data = Arc::new(RwLock::new(vec![1, 2, 3]));
    let mut handles = vec![];

    // Spawn 10 readers
    for i in 0..10 {
        let data = data.clone();
        handles.push(tokio::spawn(async move {
            let guard = data.read().await;
            let sum: i32 = guard.iter().sum();
            if i < 3 {
                println!("  Reader {i}: sum = {sum}");
            }
            sum
        }));
    }

    // Spawn 1 writer
    let data_w = data.clone();
    handles.push(tokio::spawn(async move {
        let mut guard = data_w.write().await;
        guard.push(4);
        println!("  Writer: pushed 4, vec is now {:?}", *guard);
        0
    }));

    for h in handles {
        h.await.unwrap();
    }

    let final_data = data.read().await;
    println!("  Final: {:?}", *final_data);
    println!();
    println!("Takeaway: RwLock allows many concurrent readers OR one writer.");
    println!("Use when reads vastly outnumber writes (e.g., config, caches).");
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Mutex => demo_mutex().await,
        Command::Semaphore => demo_semaphore().await,
        Command::Notify => demo_notify().await,
        Command::Rwlock => demo_rwlock().await,
        Command::All => {
            demo_mutex().await;
            println!("\n");
            demo_semaphore().await;
            println!("\n");
            demo_notify().await;
            println!("\n");
            demo_rwlock().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mutex_counter_is_correct() {
        let counter = Arc::new(Mutex::new(0u64));
        let mut handles = vec![];
        for _ in 0..50 {
            let c = counter.clone();
            handles.push(tokio::spawn(async move {
                let mut g = c.lock().await;
                *g += 1;
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(*counter.lock().await, 50);
    }

    #[tokio::test]
    async fn semaphore_limits_concurrency() {
        let sem = Arc::new(tokio::sync::Semaphore::new(3));
        let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let max_active = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let mut handles = vec![];
        for _ in 0..10 {
            let sem = sem.clone();
            let active = active.clone();
            let max_active = max_active.clone();
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let cur = active.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                max_active.fetch_max(cur, std::sync::atomic::Ordering::SeqCst);
                tokio::task::yield_now().await;
                active.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert!(max_active.load(std::sync::atomic::Ordering::SeqCst) <= 3);
    }
}

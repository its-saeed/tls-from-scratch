// Lesson 27: Connection Pooling
// Run with: cargo run -p async-lessons --bin 27-connection-pool -- <command>

use clap::{Parser, Subcommand};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(name = "connection-pool", about = "Lesson 27: Connection Pooling")]
struct Cli { #[command(subcommand)] command: Command }

#[derive(Subcommand)]
enum Command {
    /// Demo a simple connection pool
    Pool,
    All,
}

/// A simple connection pool.
struct Pool {
    connections: Mutex<VecDeque<String>>, // simulated connections
    semaphore: tokio::sync::Semaphore,   // limits total connections
    max_size: usize,
}

impl Pool {
    fn new(max_size: usize) -> Arc<Self> {
        Arc::new(Self {
            connections: Mutex::new(VecDeque::new()),
            semaphore: tokio::sync::Semaphore::new(max_size),
            max_size,
        })
    }

    async fn get(&self) -> PooledConnection<'_> {
        let _permit = self.semaphore.acquire().await.unwrap();
        let mut conns = self.connections.lock().await;
        let conn = conns.pop_front().unwrap_or_else(|| {
            let id = self.max_size - self.semaphore.available_permits();
            format!("conn-{id}")
        });
        println!("  [pool] checked out: {conn}");
        PooledConnection { pool: self, conn: Some(conn) }
    }
}

struct PooledConnection<'a> {
    pool: &'a Pool,
    conn: Option<String>,
}

impl<'a> PooledConnection<'a> {
    fn name(&self) -> &str { self.conn.as_ref().unwrap() }
}

impl<'a> Drop for PooledConnection<'a> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            println!("  [pool] returned: {conn}");
            // Return to pool (blocking lock in drop — simplified)
            let pool = self.pool;
            let _ = pool.connections.try_lock().map(|mut conns| {
                conns.push_back(conn);
            });
            pool.semaphore.add_permits(1);
        }
    }
}

fn demo_pool() {
    println!("=== Connection Pool ===");
    println!("Pool(3): 5 tasks compete for 3 connections.");
    println!();

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let pool = Pool::new(3);

        let mut handles = vec![];
        for i in 0..5 {
            let pool = pool.clone();
            handles.push(tokio::task::spawn_local(async move {
                let conn = pool.get().await;
                println!("  [task {i}] using {}", conn.name());
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                println!("  [task {i}] done with {}", conn.name());
                drop(conn); // returns to pool
            }));
        }

        for h in handles { h.await.unwrap(); }
        println!();
        println!("  Available: {}", pool.semaphore.available_permits());
    });

    println!();
    println!("Takeaway: pool reuses connections. Semaphore limits concurrency.");
    println!("Drop returns the connection automatically.");
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Pool => demo_pool(),
        Command::All => demo_pool(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pool_limits_connections() {
        let pool = Pool::new(2);
        let _c1 = pool.get().await;
        let _c2 = pool.get().await;
        // Third would block — semaphore exhausted
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            pool.get(),
        ).await;
        assert!(result.is_err(), "Should timeout — pool exhausted");
    }
}

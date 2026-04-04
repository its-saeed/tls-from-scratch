// Lesson 28: Testing Async Code
// Run with: cargo run -p async-lessons --bin 28-testing -- <command>
// Tests: cargo test -p async-lessons --bin 28-testing

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "testing", about = "Lesson 28: Testing Async Code")]
struct Cli { #[command(subcommand)] command: Command }

#[derive(Subcommand)]
enum Command {
    /// Demo time pausing/advancing
    TimeMock,
    /// Demo deterministic task ordering
    Deterministic,
    All,
}

fn demo_time_mock() {
    println!("=== Time Mocking ===");
    println!("tokio::time::pause() freezes the clock. advance() jumps forward.");
    println!("A 1-hour sleep completes in microseconds.");
    println!();
    println!("Run the tests to see it: cargo test -p async-lessons --bin 28-testing");
    println!();
    println!("Takeaway: time mocking makes timer tests fast and deterministic.");
    println!("No more sleeping in tests!");
}

fn demo_deterministic() {
    println!("=== Deterministic Testing ===");
    println!("current_thread runtime + start_paused = deterministic ordering.");
    println!();
    println!("Tips:");
    println!("  1. Use #[tokio::test] (current_thread by default)");
    println!("  2. Use start_paused = true for timer tests");
    println!("  3. Use channels instead of shared state for test communication");
    println!("  4. Use tokio::time::advance() to control time precisely");
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::TimeMock => demo_time_mock(),
        Command::Deterministic => demo_deterministic(),
        Command::All => { demo_time_mock(); println!(); demo_deterministic(); }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    #[tokio::test(start_paused = true)]
    async fn time_pause_makes_sleep_instant() {
        // With start_paused, time is frozen. sleep() completes instantly.
        let start = tokio::time::Instant::now();
        tokio::time::sleep(Duration::from_secs(3600)).await; // 1 hour!
        let elapsed = start.elapsed();
        // In wall-clock time, this was instant.
        // In tokio time, 1 hour passed.
        assert_eq!(elapsed, Duration::from_secs(3600));
    }

    #[tokio::test(start_paused = true)]
    async fn advance_controls_time() {
        let start = tokio::time::Instant::now();

        // Spawn a task that sleeps 500ms
        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(500)).await;
            "done"
        });

        // Advance time by 200ms — task is NOT done yet
        tokio::time::advance(Duration::from_millis(200)).await;
        assert!(!handle.is_finished());

        // Advance another 400ms — task should be done
        tokio::time::advance(Duration::from_millis(400)).await;
        tokio::task::yield_now().await; // let the task run
        let result = handle.await.unwrap();
        assert_eq!(result, "done");
        assert!(start.elapsed() >= Duration::from_millis(600));
    }

    #[tokio::test(start_paused = true)]
    async fn timeout_test_without_waiting() {
        // Test a timeout without actually waiting
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            async {
                tokio::time::sleep(Duration::from_secs(10)).await;
                "too slow"
            },
        ).await;

        assert!(result.is_err(), "Should timeout");
        // This test ran in microseconds, not 5 seconds!
    }

    #[tokio::test]
    async fn channel_based_test() {
        // Use channels for deterministic test communication
        let (tx, mut rx) = tokio::sync::mpsc::channel::<i32>(10);

        tokio::spawn(async move {
            for i in 0..5 {
                tx.send(i).await.unwrap();
            }
        });

        let mut results = vec![];
        while let Some(val) = rx.recv().await {
            results.push(val);
        }
        assert_eq!(results, vec![0, 1, 2, 3, 4]);
    }
}

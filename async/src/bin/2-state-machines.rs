// Lesson 2: State Machines
//
// Manually desugar an async function into its state machine form.
// Run with: cargo run -p async-lessons --bin 2-state-machines -- <command>
//
// Commands:
//   yield-once         Demo the YieldOnce future
//   hand-rolled        Run the hand-written AddSlowly state machine
//   async-version      Run the same logic using async/await
//   sizes              Print sizes of various futures
//   all                Run all demos

use clap::{Parser, Subcommand};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

// ============================================================
// YieldOnce: a future that yields once then returns a value
// ============================================================

/// A future that returns Pending on first poll, Ready(value) on second.
/// This simulates a single async operation completing.
struct YieldOnce<T> {
    value: Option<T>,
    yielded: bool,
}

impl<T> YieldOnce<T> {
    fn new(value: T) -> Self {
        Self { value: Some(value), yielded: false }
    }
}

impl<T: Unpin> Future for YieldOnce<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        // TODO:
        // 1. If not yet yielded: set yielded=true, wake, return Pending
        // 2. If already yielded: take the value from Option, return Ready(value)
        //
        // This simulates: first poll → "not ready yet", second poll → "here's the data"
        todo!("Implement YieldOnce::poll")
    }
}

// ============================================================
// AddSlowly: hand-written state machine (what the compiler generates)
// ============================================================

// This is what we're desugaring:
//
// async fn add_slowly(a: u32, b: u32) -> u32 {
//     let x = yield_once(a).await;  // await #1
//     let y = yield_once(b).await;  // await #2
//     x + y
// }

/// The state machine enum — each variant represents a point between awaits.
///
/// TODO: Add the missing variants:
///   - YieldingA: waiting for first yield_once. Holds: b, yield_future (YieldOnce<u32>)
///   - YieldingB: waiting for second yield_once. Holds: x (result of first), yield_future
///   - Done: finished
enum AddSlowly {
    Start { a: u32, b: u32 },
    // TODO: add YieldingA, YieldingB, Done variants
}

impl Future for AddSlowly {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        // TODO: implement state transitions
        //
        // The pattern for each state:
        //   1. Match the current state
        //   2. Poll the sub-future for that state
        //   3. If sub-future returns Pending → return Pending (stay in same state)
        //   4. If sub-future returns Ready(value) → transition to next state
        //   5. Loop back to match (the next state might also be immediately ready)
        //
        // Use `unsafe { self.get_unchecked_mut() }` to get &mut Self from Pin
        // (safe because AddSlowly is Unpin — all fields are Unpin)
        //
        // State transitions:
        //   Start { a, b } → create YieldOnce(a), move to YieldingA { b, future }
        //   YieldingA → poll future → Ready(x) → create YieldOnce(b), move to YieldingB { x, future }
        //   YieldingB → poll future → Ready(y) → move to Done, return Ready(x + y)
        //   Done → panic!("polled after completion")
        todo!("Implement AddSlowly::poll")
    }
}

// ============================================================
// Async version (for comparison — identical logic, cleaner code)
// ============================================================

/// The async version — compiler generates a state machine just like AddSlowly.
async fn add_slowly_async(a: u32, b: u32) -> u32 {
    let x = YieldOnce::new(a).await;
    let y = YieldOnce::new(b).await;
    x + y
}

// ============================================================
// Future size measurement
// ============================================================

async fn no_awaits() -> u32 { 42 }

async fn one_await() -> u32 {
    YieldOnce::new(42).await
}

async fn two_awaits(a: u32, b: u32) -> u32 {
    let x = YieldOnce::new(a).await;
    let y = YieldOnce::new(b).await;
    x + y
}

async fn holds_big_data() -> u32 {
    let buf = [0u8; 1024];
    YieldOnce::new(0u32).await;
    buf[0] as u32
}

// ============================================================
// Poll helper (reused from Lesson 1)
// ============================================================

fn poll_to_completion<F: Future>(label: &str, mut future: F) -> F::Output {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);
    let mut pinned = unsafe { Pin::new_unchecked(&mut future) };
    let mut poll_count = 0;
    loop {
        poll_count += 1;
        match pinned.as_mut().poll(&mut cx) {
            Poll::Pending => {
                println!("  [poll #{poll_count}] {label}: Pending");
            }
            Poll::Ready(output) => {
                println!("  [poll #{poll_count}] {label}: Ready!");
                return output;
            }
        }
    }
}

// ============================================================
// CLI
// ============================================================

#[derive(Parser)]
#[command(name = "state-machines", about = "Lesson 2: State machines")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Demo the YieldOnce future
    YieldOnce,
    /// Run the hand-written AddSlowly state machine
    HandRolled,
    /// Run the same logic using async/await
    AsyncVersion,
    /// Print sizes of various futures
    Sizes,
    /// Run all demos
    All,
}

fn demo_yield_once() {
    println!("=== YieldOnce<u32> ===");
    println!("A future that yields once (Pending), then returns the value (Ready).");
    println!();
    let future = YieldOnce::new(42u32);
    let val = poll_to_completion("yield_once(42)", future);
    println!("Result: {val}");
    println!();
    println!("Takeaway: first poll returns Pending (simulates waiting for I/O),");
    println!("second poll returns Ready(42). This is the building block for AddSlowly.");
}

fn demo_hand_rolled() {
    println!("=== AddSlowly (hand-written state machine) ===");
    println!("Desugared from: async fn add_slowly(3, 7) -> u32");
    println!("States: Start → YieldingA → YieldingB → Done");
    println!();
    let future = AddSlowly::Start { a: 3, b: 7 };
    let result = poll_to_completion("add_slowly(3, 7)", future);
    println!("Result: {result}");
    assert_eq!(result, 10);
    println!();
    println!("Takeaway: 4 polls for 2 awaits — each YieldOnce takes 2 polls (Pending + Ready).");
    println!("The compiler generates this exact pattern for every async fn.");
}

fn demo_async_version() {
    println!("=== add_slowly_async (compiler-generated state machine) ===");
    println!("Same logic as hand-rolled, but using async/await.");
    println!();
    let future = add_slowly_async(3, 7);
    let result = poll_to_completion("add_slowly_async(3, 7)", future);
    println!("Result: {result}");
    assert_eq!(result, 10);
    println!();
    println!("Takeaway: identical behavior to the hand-rolled version.");
    println!("async/await is syntactic sugar — the compiler writes the state machine for you.");
}

fn demo_sizes() {
    println!("=== Future sizes ===");
    println!("Each async fn becomes an enum. Size = largest variant.");
    println!();

    let f1 = no_awaits();
    let f2 = one_await();
    let f3 = two_awaits(1, 2);
    let f4 = holds_big_data();
    let f5 = AddSlowly::Start { a: 0, b: 0 };

    println!("  no_awaits():       {} bytes", std::mem::size_of_val(&f1));
    println!("  one_await():       {} bytes", std::mem::size_of_val(&f2));
    println!("  two_awaits(1,2):   {} bytes", std::mem::size_of_val(&f3));
    println!("  holds_big_data():  {} bytes  (holds [u8; 1024] across await)", std::mem::size_of_val(&f4));
    println!("  AddSlowly (hand):  {} bytes", std::mem::size_of_val(&f5));
    println!();
    println!("  Compare: a thread stack = 8,388,608 bytes (8 MB)");
    println!();
    println!("Takeaway: futures are tiny. This is why async scales to millions of tasks.");

    // Drop without polling to avoid warnings
    drop(f1); drop(f2); drop(f3); drop(f4);
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::YieldOnce => demo_yield_once(),
        Command::HandRolled => demo_hand_rolled(),
        Command::AsyncVersion => demo_async_version(),
        Command::Sizes => demo_sizes(),
        Command::All => {
            demo_yield_once();
            println!("\n");
            demo_hand_rolled();
            println!("\n");
            demo_async_version();
            println!("\n");
            demo_sizes();
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(&waker);
        Pin::new(future).poll(&mut cx)
    }

    #[test]
    fn yield_once_pending_then_ready() {
        let mut f = YieldOnce::new(42u32);
        assert!(poll_once(&mut f).is_pending());
        assert_eq!(poll_once(&mut f), Poll::Ready(42));
    }

    #[test]
    fn add_slowly_hand_rolled() {
        let future = AddSlowly::Start { a: 5, b: 3 };
        let result = poll_to_completion("test", future);
        assert_eq!(result, 8);
    }

    #[test]
    fn add_slowly_async_version() {
        let future = add_slowly_async(5, 3);
        let result = poll_to_completion("test", future);
        assert_eq!(result, 8);
    }

    #[test]
    fn add_slowly_both_match() {
        let hand = poll_to_completion("hand", AddSlowly::Start { a: 10, b: 20 });
        let async_ver = poll_to_completion("async", add_slowly_async(10, 20));
        assert_eq!(hand, async_ver);
        assert_eq!(hand, 30);
    }
}

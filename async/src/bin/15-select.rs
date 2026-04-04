// Lesson 15: Select Internals
//
// Race futures, handle cancellation, understand drop semantics.
// Run with: cargo run -p async-lessons --bin 15-select -- <command>
//
// Commands:
//   binary           Select between two futures, take the winner
//   cancellation     Show what happens when the loser is dropped
//   fuse             Demo Fuse wrapper for poll-after-Ready safety
//   all              Run all demos

use clap::{Parser, Subcommand};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

// ============================================================
// Either: return type for select
// ============================================================

#[derive(Debug)]
enum Either<L, R> {
    Left(L),
    Right(R),
}

// ============================================================
// Select: race two futures
// ============================================================

/// Race two futures. Returns whichever completes first, drops the other.
///
/// TODO: Implement Future for Select.
///   poll():
///     1. Poll A → Ready? return Left(value) (B is dropped automatically)
///     2. Poll B → Ready? return Right(value) (A is dropped automatically)
///     3. Both Pending → Pending
struct Select<A, B> {
    a: Option<Pin<Box<A>>>,
    b: Option<Pin<Box<B>>>,
}

impl<A: Future, B: Future> Select<A, B> {
    fn new(a: A, b: B) -> Self {
        Self {
            a: Some(Box::pin(a)),
            b: Some(Box::pin(b)),
        }
    }
}

impl<A: Future, B: Future> Future for Select<A, B> {
    type Output = Either<A::Output, B::Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // TODO: implement
        // 1. If a is Some, poll it
        // 2. If Ready → take a, drop b (set to None), return Left
        // 3. If b is Some, poll it
        // 4. If Ready → take b, drop a, return Right
        // 5. Both Pending → Pending
        todo!("Implement Select::poll")
    }
}

fn select<A: Future, B: Future>(a: A, b: B) -> Select<A, B> {
    Select::new(a, b)
}

// ============================================================
// Fuse: safe to poll after completion
// ============================================================

/// Wraps a future so it returns Pending forever after completing.
///
/// TODO: Implement Future for Fuse.
///   If inner is Some: poll it. If Ready → set to None, return Ready.
///   If inner is None: return Pending (already completed).
struct Fuse<F> {
    inner: Option<Pin<Box<F>>>,
}

impl<F: Future> Fuse<F> {
    fn new(future: F) -> Self {
        Self { inner: Some(Box::pin(future)) }
    }
}

impl<F: Future> Future for Fuse<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        todo!("Implement Fuse::poll")
    }
}

// ============================================================
// Helper futures
// ============================================================

/// A future that tracks poll count and prints on Drop.
struct TrackedFuture {
    label: String,
    polls: Arc<AtomicU32>,
    yields_before_ready: u32,
    dropped: Arc<AtomicBool>,
}

impl TrackedFuture {
    fn new(label: &str, yields: u32, polls: Arc<AtomicU32>, dropped: Arc<AtomicBool>) -> Self {
        Self {
            label: label.to_string(),
            polls,
            yields_before_ready: yields,
            dropped,
        }
    }
}

impl Future for TrackedFuture {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<String> {
        let count = self.polls.fetch_add(1, Ordering::SeqCst) + 1;
        if self.yields_before_ready == 0 {
            Poll::Ready(format!("{} (polled {} times)", self.label, count))
        } else {
            self.yields_before_ready -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

impl Drop for TrackedFuture {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
        println!("  [DROP] {} cancelled (polled {} times)",
            self.label, self.polls.load(Ordering::SeqCst));
    }
}

/// Simple sleep (busy-poll version from Lesson 12)
struct Sleep { deadline: Instant }
impl Sleep {
    fn new(dur: Duration) -> Self { Self { deadline: Instant::now() + dur } }
}
impl Future for Sleep {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if Instant::now() >= self.deadline {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

// ============================================================
// Poll helper
// ============================================================

fn poll_to_completion<F: Future>(label: &str, mut future: F) -> F::Output {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);
    let mut pinned = unsafe { Pin::new_unchecked(&mut future) };
    loop {
        match pinned.as_mut().poll(&mut cx) {
            Poll::Pending => {}
            Poll::Ready(output) => {
                println!("  [{label}] Ready!");
                return output;
            }
        }
    }
}

// ============================================================
// CLI
// ============================================================

#[derive(Parser)]
#[command(name = "select", about = "Lesson 15: Select Internals")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Select between two futures
    Binary,
    /// Show cancellation when loser is dropped
    Cancellation,
    /// Demo Fuse wrapper
    Fuse,
    /// Run all demos
    All,
}

fn demo_binary() {
    println!("=== Binary Select ===");
    println!("Racing a fast future (1 yield) vs slow future (10 yields).");
    println!();

    let fast_polls = Arc::new(AtomicU32::new(0));
    let slow_polls = Arc::new(AtomicU32::new(0));
    let fast_dropped = Arc::new(AtomicBool::new(false));
    let slow_dropped = Arc::new(AtomicBool::new(false));

    let result = poll_to_completion("select", select(
        TrackedFuture::new("fast", 1, fast_polls.clone(), fast_dropped.clone()),
        TrackedFuture::new("slow", 10, slow_polls.clone(), slow_dropped.clone()),
    ));

    println!("  Result: {:?}", result);
    println!("  Fast polled: {} times", fast_polls.load(Ordering::SeqCst));
    println!("  Slow polled: {} times", slow_polls.load(Ordering::SeqCst));
    println!("  Fast dropped: {}", fast_dropped.load(Ordering::SeqCst));
    println!("  Slow dropped: {}", slow_dropped.load(Ordering::SeqCst));
    println!();
    println!("Takeaway: fast won, slow was cancelled (dropped).");
}

fn demo_cancellation() {
    println!("=== Cancellation Demo ===");
    println!("Select: a counting future vs an immediately-ready future.");
    println!("The counter should be polled exactly once before being cancelled.");
    println!();

    let counter_polls = Arc::new(AtomicU32::new(0));
    let counter_dropped = Arc::new(AtomicBool::new(false));

    let result = poll_to_completion("select", select(
        TrackedFuture::new("counter", 100, counter_polls.clone(), counter_dropped.clone()),
        TrackedFuture::new("instant", 0, Arc::new(AtomicU32::new(0)), Arc::new(AtomicBool::new(false))),
    ));

    println!("  Result: {:?}", result);
    println!("  Counter polled: {} times", counter_polls.load(Ordering::SeqCst));
    println!("  Counter dropped: {}", counter_dropped.load(Ordering::SeqCst));
    println!();
    println!("Takeaway: the losing future is polled at most once per select poll,");
    println!("then dropped when the winner completes.");
}

fn demo_fuse() {
    println!("=== Fuse Demo ===");
    println!("Fuse wraps a future: after Ready, returns Pending forever.");
    println!();

    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);

    let mut fused = Fuse::new(async { 42u32 });
    let mut pinned = unsafe { Pin::new_unchecked(&mut fused) };

    for i in 1..=3 {
        match pinned.as_mut().poll(&mut cx) {
            Poll::Ready(v) => println!("  [poll #{i}] Ready({v})"),
            Poll::Pending => println!("  [poll #{i}] Pending (already completed, safe)"),
        }
    }

    println!();
    println!("Takeaway: without Fuse, polling after Ready is undefined behavior.");
    println!("Fuse makes it safe — returns Pending forever after completion.");
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Binary => demo_binary(),
        Command::Cancellation => demo_cancellation(),
        Command::Fuse => demo_fuse(),
        Command::All => {
            demo_binary();
            println!("\n");
            demo_cancellation();
            println!("\n");
            demo_fuse();
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_returns_faster() {
        let result = poll_to_completion("test", select(
            TrackedFuture::new("fast", 0, Arc::new(AtomicU32::new(0)), Arc::new(AtomicBool::new(false))),
            TrackedFuture::new("slow", 5, Arc::new(AtomicU32::new(0)), Arc::new(AtomicBool::new(false))),
        ));
        assert!(matches!(result, Either::Left(_)));
    }

    #[test]
    fn select_drops_loser() {
        let dropped = Arc::new(AtomicBool::new(false));
        let _ = poll_to_completion("test", select(
            TrackedFuture::new("winner", 0, Arc::new(AtomicU32::new(0)), Arc::new(AtomicBool::new(false))),
            TrackedFuture::new("loser", 100, Arc::new(AtomicU32::new(0)), dropped.clone()),
        ));
        assert!(dropped.load(Ordering::SeqCst), "Loser should be dropped");
    }
}

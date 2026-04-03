// Lesson 1: Futures by Hand
//
// Implement the Future trait manually — no async/await.
// Run with: cargo run -p async-lessons --bin 1-futures -- <command>
//
// Commands:
//   countdown <n>     Poll a CountdownFuture from n to 0
//   ready             Poll a ReadyFuture that returns immediately
//   delay <n> <msg>   Poll a DelayFuture that waits n polls then returns msg
//   all               Run all demos

use clap::{Parser, Subcommand};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

// ============================================================
// Future #1: CountdownFuture
// ============================================================

/// A future that counts down from `count` to 0.
/// Each poll decrements the counter and returns Pending.
/// When count reaches 0, returns Ready(()).
struct CountdownFuture {
    count: u32,
}

impl Future for CountdownFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        // TODO:
        // 1. Get a mutable reference to self (Pin::get_mut is safe for Unpin types)
        // 2. If count > 0: decrement count, call cx.waker().wake_by_ref(), return Pending
        // 3. If count == 0: return Ready(())
        //
        // Why wake_by_ref()? It tells the executor "I have more work, poll me again."
        // Without it, the executor would never re-poll and the future hangs.
        match self.count {
            0 => Poll::Ready(()),
            _ => {
                self.count -= 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

// ============================================================
// Future #2: ReadyFuture
// ============================================================

/// A future that immediately returns a value on first poll.
/// This is what std::future::ready() does internally.
struct ReadyFuture<T>(Option<T>);

impl<T: Unpin> Future for ReadyFuture<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<T> {
        // TODO:
        // 1. Take the value out of the Option (self.get_mut().0.take())
        // 2. If Some(value): return Ready(value)
        // 3. If None: panic — this means poll was called after Ready (contract violation)
        match self.0.take() {
            Some(v) => Poll::Ready(v),
            None => panic!("Poll called after ready"),
        }
    }
}

// ============================================================
// Poll helper — manually drives a future to completion
// ============================================================

/// Polls a future in a loop until it returns Ready.
/// Prints each poll attempt so you can see the state machine in action.
///
/// This is already implemented for you — it's what an executor does,
/// simplified to a single future on the current thread.
fn poll_to_completion<F: Future>(label: &str, mut future: F) -> F::Output {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);

    // SAFETY: we never move `future` after pinning it here.
    let mut pinned = unsafe { Pin::new_unchecked(&mut future) };

    let mut poll_count = 0;
    loop {
        poll_count += 1;
        match pinned.as_mut().poll(&mut cx) {
            Poll::Pending => {
                println!("  [poll #{poll_count}] {label}: Pending (not ready yet)");
            }
            Poll::Ready(output) => {
                println!("  [poll #{poll_count}] {label}: Ready! ✓");
                return output;
            }
        }
    }
}

// ============================================================
// CLI
// ============================================================

#[derive(Parser)]
#[command(name = "futures", about = "Lesson 1: Futures by hand")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Poll a CountdownFuture from n to 0
    Countdown { n: u32 },
    /// Poll a ReadyFuture that returns immediately
    Ready,
    /// Run all demos
    All,
}

fn demo_countdown(n: u32) {
    println!("=== CountdownFuture (count={n}) ===");
    println!("Polling until countdown reaches 0...");
    println!();
    let future = CountdownFuture { count: n };
    poll_to_completion("countdown", future);
    println!();
    println!("Takeaway: the future returned Pending {n} times, then Ready.");
    println!("Each Pending → waker.wake() → executor polls again.");
}

fn demo_ready() {
    println!("=== ReadyFuture ===");
    println!("Polling a future that's immediately ready...");
    println!();
    let future = ReadyFuture(Some(42));
    let value = poll_to_completion("ready", future);
    println!("Got value: {value}");
    println!();
    println!("Takeaway: ReadyFuture returns Ready on first poll. No Pending.");
    println!("This is what std::future::ready() does.");
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Countdown { n } => demo_countdown(n),
        Command::Ready => demo_ready(),
        Command::All => {
            demo_countdown(5);
            println!();
            demo_ready();
        }
    }
}

// ============================================================
// Tests — run with: cargo test -p async-lessons --bin 1-futures
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
    fn countdown_returns_pending_then_ready() {
        let mut future = CountdownFuture { count: 3 };

        assert!(poll_once(&mut future).is_pending(), "count=3 → Pending");
        assert!(poll_once(&mut future).is_pending(), "count=2 → Pending");
        assert!(poll_once(&mut future).is_pending(), "count=1 → Pending");
        assert!(poll_once(&mut future).is_ready(), "count=0 → Ready");
    }

    #[test]
    fn countdown_zero_is_immediately_ready() {
        let mut future = CountdownFuture { count: 0 };
        assert!(poll_once(&mut future).is_ready());
    }

    #[test]
    fn ready_future_returns_value() {
        let mut future = ReadyFuture(Some(42));
        match poll_once(&mut future) {
            Poll::Ready(val) => assert_eq!(val, 42),
            Poll::Pending => panic!("ReadyFuture should never return Pending"),
        }
    }

    #[test]
    #[should_panic]
    fn ready_future_panics_on_second_poll() {
        let mut future = ReadyFuture(Some(42));
        let _ = poll_once(&mut future); // first poll: Ready(42)
        let _ = poll_once(&mut future); // second poll: should panic
    }

}

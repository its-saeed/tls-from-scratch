# Pattern 2: Actor Model

## Real-life analogy: office departments

```
┌──────────────┐  memo  ┌──────────────┐  memo  ┌──────────────┐
│  Sales Dept  │ ──────►│  Accounting  │ ──────►│  Shipping    │
│              │        │              │        │              │
│  Inbox: 📬   │        │  Inbox: 📬   │        │  Inbox: 📬   │
│  State: leads│        │  State: books│        │  State: orders│
│  Staff: 1    │        │  Staff: 1    │        │  Staff: 1    │
└──────────────┘        └──────────────┘        └──────────────┘

Each department:
  - Has its own inbox (channel)
  - Processes memos one at a time (no multitasking within a dept)
  - Has private state (no other dept can touch it)
  - Communicates only via memos (messages)

Nobody walks into Accounting and grabs the books.
They send a memo and wait for a reply.
```

## The pattern

An **actor** is a task that:
1. Owns its state exclusively (no shared memory)
2. Receives messages through a channel (its "inbox")
3. Processes messages one at a time (sequential, no locks needed)
4. Can send messages to other actors

```rust
struct BankAccount {
    balance: u64,
    inbox: mpsc::Receiver<AccountMessage>,
}

enum AccountMessage {
    Deposit { amount: u64 },
    Withdraw { amount: u64, reply: oneshot::Sender<Result<(), String>> },
    GetBalance { reply: oneshot::Sender<u64> },
}

impl BankAccount {
    async fn run(mut self) {
        while let Some(msg) = self.inbox.recv().await {
            match msg {
                AccountMessage::Deposit { amount } => {
                    self.balance += amount;
                }
                AccountMessage::Withdraw { amount, reply } => {
                    if self.balance >= amount {
                        self.balance -= amount;
                        let _ = reply.send(Ok(()));
                    } else {
                        let _ = reply.send(Err("insufficient funds".into()));
                    }
                }
                AccountMessage::GetBalance { reply } => {
                    let _ = reply.send(self.balance);
                }
            }
        }
    }
}
```

```
┌────────────────────────────────────────────────────────┐
│  Actor Model                                           │
│                                                        │
│  ┌─────────────┐  msg   ┌─────────────┐               │
│  │  Client     │ ──────►│  Actor      │               │
│  │  (any task) │        │             │               │
│  │             │◄───────│  - inbox    │               │
│  │  sends msg  │ reply  │  - state    │               │
│  │  + oneshot  │        │  - run loop │               │
│  └─────────────┘        └─────────────┘               │
│                                                        │
│  No locks. No shared state. No data races.             │
│  The actor processes one message at a time.            │
│  State is private — only the actor touches it.         │
└────────────────────────────────────────────────────────┘
```

## Actor vs Shared State

```
Shared state (Arc<Mutex<T>>):         Actor:
  ┌──────────┐ ┌──────────┐           ┌──────────┐
  │ Task A   │ │ Task B   │           │ Task A   │──msg──┐
  │ lock()   │ │ lock()   │           │          │       │
  │ modify   │ │ BLOCKED  │           └──────────┘       ▼
  │ unlock() │ │ ...      │           ┌──────────┐  ┌─────────┐
  └──────────┘ └──────────┘           │ Task B   │──►  Actor  │
                                      │          │  │ (no lock│
  Lock contention.                    └──────────┘  │  needed)│
  Deadlock risk.                                    └─────────┘
  Complex error handling.              No contention.
                                       Sequential processing.
                                       Simple reasoning.
```

## When to use

- **Stateful services** — user sessions, game entities, connection managers
- **When state is complex** — a mutex would be held too long or across `.await`
- **Isolation** — each actor can fail independently without corrupting shared state
- **Erlang/Elixir-style systems** — the actor model is their core abstraction

## When NOT to use

- **Simple shared counters** — `AtomicU64` or `Arc<Mutex<u64>>` is simpler
- **Read-heavy workloads** — actors serialize all access; `RwLock` allows concurrent reads
- **Fire-and-forget operations** — if you don't need a reply, a plain `spawn` is simpler

## The request-reply pattern

To get data back from an actor, send a `oneshot::Sender` with the message:

```rust
// Client side:
let (reply_tx, reply_rx) = oneshot::channel();
actor_tx.send(AccountMessage::GetBalance { reply: reply_tx }).await?;
let balance = reply_rx.await?;  // waits for the actor to respond
```

## Code exercise: Bank System

Build a bank with account actors:

```
┌──────────┐     ┌────────────────┐
│  Client  │────►│ Account "alice"│  (actor)
│  (task)  │     │ balance: 1000  │
└──────────┘     └────────────────┘
     │
     │           ┌────────────────┐
     └──────────►│ Account "bob"  │  (actor)
                 │ balance: 500   │
                 └────────────────┘
```

**Requirements**:
1. Each bank account is an actor (a task with a channel inbox)
2. Support: `Deposit`, `Withdraw`, `GetBalance`, `Transfer(to_account, amount)`
3. Transfer is atomic: withdraw from A, deposit to B. If withdraw fails, B is unchanged.
4. Multiple clients can interact with accounts concurrently — no locks.

**Starter code**:

```rust
use tokio::sync::{mpsc, oneshot};

enum AccountMsg {
    Deposit { amount: u64 },
    Withdraw { amount: u64, reply: oneshot::Sender<Result<(), String>> },
    Balance { reply: oneshot::Sender<u64> },
}

#[derive(Clone)]
struct AccountHandle {
    tx: mpsc::Sender<AccountMsg>,
}

impl AccountHandle {
    async fn deposit(&self, amount: u64) {
        self.tx.send(AccountMsg::Deposit { amount }).await.unwrap();
    }

    async fn balance(&self) -> u64 {
        let (tx, rx) = oneshot::channel();
        self.tx.send(AccountMsg::Balance { reply: tx }).await.unwrap();
        rx.await.unwrap()
    }

    // TODO: withdraw, transfer
}

fn spawn_account(name: &str, initial_balance: u64) -> AccountHandle {
    let (tx, mut rx) = mpsc::channel(32);
    let name = name.to_string();

    tokio::spawn(async move {
        let mut balance = initial_balance;
        while let Some(msg) = rx.recv().await {
            match msg {
                AccountMsg::Deposit { amount } => balance += amount,
                // TODO: handle other messages
                _ => todo!(),
            }
        }
        println!("{name} actor shutting down");
    });

    AccountHandle { tx }
}
```

**Test**: create Alice (1000) and Bob (500). Transfer 200 from Alice to Bob. Check balances: Alice=800, Bob=700.

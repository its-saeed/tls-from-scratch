# Project: Session Keys (Delegated Signing)

> **Prerequisites**: Lesson 3 (Ed25519 signatures), Lesson 8 (Certificate Generation), Lesson 14 (tokio-rustls), Project 2 (Signed Commits). You'll build the signing pattern DEXes and smart-wallet dApps use to let a user sign *once* and trade many times.

## What is this?

Imagine using a DEX. Every trade pops a wallet confirmation. Ten trades, ten signatures. Painful.

Real dApps solve this with **session keys**: the user signs *one* delegation with their long-term key, authorizing a short-lived "session key" to act on their behalf under strict limits (expiry, max amount, allowed symbols). Trades are then signed by the session key — no wallet popup.

You're building the mini version of this: a user, an exchange server, and the full verify pipeline that makes it safe.

```
┌──────────────────────────────────────────────────────────┐
│  The problem:                                            │
│                                                          │
│  User has a high-value key (cold wallet, hardware        │
│  wallet, master identity). Every action needs a          │
│  signature — but signing each one:                       │
│                                                          │
│    1. Painful UX (10 trades = 10 prompts)                │
│    2. Exposes the master key to the app every time       │
│    3. Makes high-frequency use impossible                │
│                                                          │
│  The solution: delegated signing                         │
│                                                          │
│    Master key signs once → authorizes a session key      │
│    Session key signs every action → bounded by scope     │
│    Session key compromise ≠ master key compromise        │
│                                                          │
│  Used by:                                                │
│    • DEX protocols (dYdX, Hyperliquid, Vertex)           │
│    • Smart wallets (ERC-4337 session keys)               │
│    • Passkeys (WebAuthn) in a related form               │
│    • SSH certificates (same pattern, older name)         │
└──────────────────────────────────────────────────────────┘
```

## What you're building

Two binaries — `p10-dex-client` and `p10-dex-server` — plus a shared `dex` module with the wire format and verify rules.

```sh
# --- Setup (one-time) ---
cargo run -p tls --bin p10-dex-client -- master-keygen --out master.key
cargo run -p tls --bin p10-dex-client -- session-keygen --out session.key

# --- User authorizes a 1-hour trading session ---
cargo run -p tls --bin p10-dex-client -- delegate \
    --master-key master.key \
    --session-pub session.key.pub \
    --ttl-secs 3600 \
    --max-qty 10 \
    --symbols BTC-USD,ETH-USD \
    --sides buy,sell \
    --out delegation.bin
# Signed delegation: session fp a1b2..., expires 2026-04-17T15:00:00Z

# --- Exchange is running ---
cargo run -p tls --bin p10-dex-server -- --bind 127.0.0.1:8443
# Listening on https://127.0.0.1:8443

# --- Trade without further master-key prompts ---
cargo run -p tls --bin p10-dex-client -- connect \
    --server 127.0.0.1:8443 \
    --session-key session.key \
    --delegation delegation.bin
# Connected. Delegation accepted (expires in 3599s).
# > buy BTC-USD 1 @ 50000
# ← ACK order_id=7f3a..
# > sell ETH-USD 0.5 @ 3000
# ← ACK order_id=e102..
# > buy DOGE-USD 1 @ 0.1
# ← REJECT symbol not in delegation scope

# --- Revoke if the laptop is lost ---
cargo run -p tls --bin p10-dex-client -- revoke \
    --master-key master.key \
    --session-pub session.key.pub \
    --out revoke.bin
cargo run -p tls --bin p10-dex-client -- submit-revoke \
    --server 127.0.0.1:8443 \
    --revoke revoke.bin
# Revocation accepted. Session a1b2.. is now rejected.
```

## Architecture

```
        ┌─────────────┐
        │  Master key │   long-term, high-value
        │  (Ed25519)  │   rarely touched
        └──────┬──────┘
               │ signs once
               ▼
    ┌────────────────────────┐
    │      Delegation         │
    │  session_pubkey         │   ← authorized key
    │  not_before / not_after │   ← time window
    │  max_qty                │   ← size cap
    │  allowed_symbols        │   ← scope
    │  allowed_sides          │   ← scope
    │  nonce                  │   ← for revocation
    │  master_signature       │   ← the "authorization"
    └──────┬──────────────────┘
           │  attached to every order
           ▼
      ┌─────────────┐
      │ Session key │   short-lived, lives in app memory
      │  (Ed25519)  │   signs every order
      └──────┬──────┘
             │
             ▼
       ┌──────────┐       TLS 1.3 (rustls)       ┌───────────┐
       │  Client  │ ◄──────────────────────────► │  Server   │
       └──────────┘   length-prefixed bincode    └───────────┘
                       Signed orders + delegation
```

Two layers of crypto, doing different jobs:

| Layer       | Purpose                         | Key used                       |
|-------------|---------------------------------|--------------------------------|
| Transport   | encryption, server identity     | rustls cert (from Lesson 8)    |
| Application | user identity, action authority | master Ed25519 → session Ed25519 |

The TLS layer stops network attackers. The signing layer stops a malicious exchange server from forging orders.

## Try it with existing tools first

```sh
# === SSH certificates: the same pattern, 30 years older ===
# A user CA signs a short-lived certificate for a session key.
# The server verifies the cert chain, not a pre-loaded public key.

ssh-keygen -f ca-key -C "my CA"
ssh-keygen -f session-key -C "alice session"
ssh-keygen -s ca-key -I "alice-session-2026-04-17" \
    -V +1h -n alice session-key.pub
# session-key-cert.pub is a delegation: CA signs session pubkey + validity

ssh-keygen -L -f session-key-cert.pub
# Shows: principals (alice), validity (+1h), CA fingerprint

# You're building the same shape, for trading actions instead of logins.
```

```sh
# === Ethereum EIP-4337 session keys ===
# Smart contract wallets (ERC-4337) allow session-key plugins:
# master EOA signs a UserOperation authorizing a session key
# with scope (target contracts, selectors, spend limit).

# Real implementations: Kernel, Safe modules, Biconomy.
# Same idea — different wire format and on-chain verification.
```

```sh
# === JWT with short expiry — a weaker cousin ===
# A central auth server (not the user) signs a token.
# Client sends it until it expires. No scope, usually.
# Session keys are user-signed, scoped, and revocable per-session.
```

## The threat model (what each check blocks)

Before writing code, get clear on what a session-key scheme is *supposed* to prevent:

```
Attacker                          Defense
────────                          ───────
Network eavesdropper              rustls TLS 1.3 (Lesson 14)
Forged delegation                 master signature over delegation
Expired delegation reuse          not_before / not_after check
Session key stolen                scope limits + revocation list
Order tampered in flight          session signature over order
Replay of captured order          client_order_id + replay cache
Off-scope action (wrong symbol)   server-side scope validation
Over-size order                   max_qty check against delegation
Delegation for wrong master       delegation includes master_pubkey
```

Every one of these maps to a step in the verify pipeline below.

## Implementation guide

### Step 0: Project setup

```sh
touch tls/src/bin/p10-dex-client.rs
touch tls/src/bin/p10-dex-server.rs
touch tls/src/dex.rs
```

Register the module in `tls/src/lib.rs`:

```rust
pub mod common;
pub mod dex;
```

Add to `tls/Cargo.toml` (most are already there from prior lessons):

```toml
bincode = "1.3"
# already present: serde, tokio, tokio-rustls, rustls, rcgen,
#                  ed25519-dalek, anyhow, clap, hex
```

### Step 1: Design the shared types

Both client and server have to agree on byte-for-byte wire format. Put the types — and **only the types** — in `tls/src/dex.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum Side { Buy, Sell }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Delegation {
    pub master_pubkey:    [u8; 32],
    pub session_pubkey:   [u8; 32],
    pub not_before:       u64,
    pub not_after:        u64,
    pub max_qty:          u64,
    pub allowed_symbols:  Vec<String>,
    pub allowed_sides:    Vec<Side>,
    pub nonce:            [u8; 16],
    pub master_signature: [u8; 64],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Order {
    pub symbol:          String,
    pub side:            Side,
    pub qty:             u64,
    pub limit_price:     u64,
    pub client_order_id: [u8; 16],
    pub timestamp:       u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignedOrder {
    pub order:             Order,
    pub session_signature: [u8; 64],
    pub delegation:        Delegation,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientMsg { Auth(Delegation), Submit(SignedOrder), Revoke(Revocation) }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMsg { Accepted(String), Rejected(String) }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Revocation {
    pub master_pubkey:    [u8; 32],
    pub session_pubkey:   [u8; 32],
    pub issued_at:        u64,
    pub master_signature: [u8; 64],
}
```

### Step 2: Canonical signing bytes

Signatures must cover a fixed byte representation — never re-serialize the struct and hope bincode is deterministic about it. Define a single function each side agrees on:

```rust
impl Delegation {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"dex/delegation/v1");     // domain tag
        b.extend_from_slice(&self.master_pubkey);
        b.extend_from_slice(&self.session_pubkey);
        b.extend_from_slice(&self.not_before.to_be_bytes());
        b.extend_from_slice(&self.not_after.to_be_bytes());
        b.extend_from_slice(&self.max_qty.to_be_bytes());
        for s in &self.allowed_symbols {
            b.extend_from_slice(&(s.len() as u32).to_be_bytes());
            b.extend_from_slice(s.as_bytes());
        }
        for side in &self.allowed_sides {
            b.push(match side { Side::Buy => 0, Side::Sell => 1 });
        }
        b.extend_from_slice(&self.nonce);
        b
    }
}
```

Do the equivalent for `Order::signing_bytes()` and `Revocation::signing_bytes()`. The domain tag (`"dex/delegation/v1"`) prevents a signature over an order from being reused as a signature over a delegation.

### Step 3: The verify pipeline

This is the core lesson. One function, seven ordered checks, each mapping to a threat.

```rust
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use std::collections::HashSet;

pub struct ServerState {
    pub trusted_master_keys: HashSet<[u8; 32]>,   // onboarded users
    pub revoked_sessions:    HashSet<[u8; 32]>,   // revoked session pubkeys
    pub seen_order_ids:      HashSet<[u8; 16]>,   // replay cache
}

pub fn verify_signed_order(
    state: &ServerState,
    signed: &SignedOrder,
    now: u64,
) -> Result<(), String> {
    let d = &signed.delegation;

    // 1. Master must be a user we know about.
    if !state.trusted_master_keys.contains(&d.master_pubkey) {
        return Err("unknown master key".into());
    }

    // 2. Master's signature on the delegation must verify.
    let master = VerifyingKey::from_bytes(&d.master_pubkey)
        .map_err(|_| "bad master pubkey")?;
    let sig = Signature::from_bytes(&d.master_signature);
    master.verify(&d.signing_bytes(), &sig)
        .map_err(|_| "bad master signature on delegation")?;

    // 3. Session hasn't been revoked.
    if state.revoked_sessions.contains(&d.session_pubkey) {
        return Err("session revoked".into());
    }

    // 4. Delegation is time-valid.
    if now < d.not_before { return Err("delegation not yet valid".into()); }
    if now > d.not_after  { return Err("delegation expired".into()); }

    // 5. Session's signature on the order must verify.
    let session = VerifyingKey::from_bytes(&d.session_pubkey)
        .map_err(|_| "bad session pubkey")?;
    let sig = Signature::from_bytes(&signed.session_signature);
    session.verify(&signed.order.signing_bytes(), &sig)
        .map_err(|_| "bad session signature on order")?;

    // 6. Order is within delegation scope.
    if !d.allowed_symbols.contains(&signed.order.symbol) {
        return Err("symbol not in scope".into());
    }
    if !d.allowed_sides.contains(&signed.order.side) {
        return Err("side not in scope".into());
    }
    if signed.order.qty > d.max_qty {
        return Err("qty exceeds delegation limit".into());
    }

    // 7. Order is not a replay.
    if state.seen_order_ids.contains(&signed.order.client_order_id) {
        return Err("replayed order".into());
    }

    Ok(())
}
```

Study that order. Step 2 *must* happen before trusting any other field of the delegation — otherwise an attacker could forge their own scope values. Step 5 *must* happen before trusting any order field — same reason.

### Step 4: Client — generate and delegate

```rust
// master-keygen / session-keygen: same as Lesson 3
// (write secret bytes to `master.key`, public bytes to `master.key.pub`)

fn cmd_delegate(
    master_path: &str, session_pub_path: &str,
    ttl_secs: u64, max_qty: u64,
    symbols: Vec<String>, sides: Vec<Side>,
    out: &str,
) {
    let master_sk = SigningKey::from_bytes(&load32(master_path));
    let session_pk: [u8; 32] = load32(session_pub_path);
    let now = unix_now();

    let mut d = Delegation {
        master_pubkey:    master_sk.verifying_key().to_bytes(),
        session_pubkey:   session_pk,
        not_before:       now,
        not_after:        now + ttl_secs,
        max_qty,
        allowed_symbols:  symbols,
        allowed_sides:    sides,
        nonce:            rand::random(),
        master_signature: [0u8; 64],
    };
    let sig = master_sk.sign(&d.signing_bytes());
    d.master_signature = sig.to_bytes();

    std::fs::write(out, bincode::serialize(&d).unwrap()).unwrap();
    println!("Signed delegation -> {out}");
    println!("  session fp: {}", hex::encode(&session_pk[..8]));
    println!("  expires in: {ttl_secs}s");
}
```

### Step 5: Client — signed orders over TLS

The client opens a TLS connection (self-signed cert, server-only auth — like `p6`), sends one `ClientMsg::Auth(delegation)` up front, then loops reading orders from stdin and sending `ClientMsg::Submit(signed)`.

```rust
async fn submit_order(
    stream: &mut TlsStream<TcpStream>,
    session_sk: &SigningKey,
    delegation: &Delegation,
    order: Order,
) -> anyhow::Result<()> {
    let sig = session_sk.sign(&order.signing_bytes()).to_bytes();
    let signed = SignedOrder {
        order,
        session_signature: sig,
        delegation: delegation.clone(),
    };
    send_msg(stream, &ClientMsg::Submit(signed)).await?;
    let resp: ServerMsg = recv_msg(stream).await?;
    println!("  ← {resp:?}");
    Ok(())
}
```

Frame format: 4-byte big-endian length, then bincode bytes. Keep `send_msg` / `recv_msg` in `dex.rs` so both sides match.

### Step 6: Server — accept connections, run the pipeline

Reuse the `p6` pattern: `rcgen::generate_simple_self_signed(...)`, `tokio_rustls::TlsAcceptor`, one task per connection.

```rust
async fn handle_conn(
    state: Arc<Mutex<ServerState>>,
    stream: &mut TlsStream<TcpStream>,
) -> anyhow::Result<()> {
    loop {
        let msg: ClientMsg = recv_msg(stream).await?;
        let now = unix_now();
        let mut s = state.lock().await;

        let reply = match msg {
            ClientMsg::Submit(signed) => {
                match verify_signed_order(&s, &signed, now) {
                    Ok(()) => {
                        s.seen_order_ids.insert(signed.order.client_order_id);
                        ServerMsg::Accepted(format!(
                            "order {}",
                            hex::encode(&signed.order.client_order_id[..4])
                        ))
                    }
                    Err(e) => ServerMsg::Rejected(e),
                }
            }
            ClientMsg::Revoke(r)  => apply_revocation(&mut s, &r, now),
            ClientMsg::Auth(d)    => try_register_session(&mut s, &d, now),
        };
        send_msg(stream, &reply).await?;
    }
}
```

Log every verify step (`[1/7] master verified`, `[6/7] scope OK`, etc.) so a student running this sees the pipeline execute.

### Step 7: Revocation

A revocation is a tiny delegation-like message — master signs the session pubkey they're cancelling:

```rust
pub fn apply_revocation(s: &mut ServerState, r: &Revocation, _now: u64) -> ServerMsg {
    if !s.trusted_master_keys.contains(&r.master_pubkey) {
        return ServerMsg::Rejected("unknown master".into());
    }
    let master = VerifyingKey::from_bytes(&r.master_pubkey).unwrap();
    let sig = Signature::from_bytes(&r.master_signature);
    if master.verify(&r.signing_bytes(), &sig).is_err() {
        return ServerMsg::Rejected("bad revocation signature".into());
    }
    s.revoked_sessions.insert(r.session_pubkey);
    ServerMsg::Accepted(format!("revoked {}", hex::encode(&r.session_pubkey[..8])))
}
```

Why this is only a half-solution: the revocation only lives in *this* server's memory. A real exchange persists it; a real on-chain system needs a revocation oracle all verifiers consult. Make the student feel that gap — it's the hardest part.

### Step 8: Test it

```sh
# Terminal 1: server
cargo run -p tls --bin p10-dex-server -- --bind 127.0.0.1:8443
# [init] listening on https://127.0.0.1:8443

# Terminal 2: client
cargo run -p tls --bin p10-dex-client -- master-keygen --out master.key
cargo run -p tls --bin p10-dex-client -- session-keygen --out session.key
cargo run -p tls --bin p10-dex-client -- delegate \
    --master-key master.key --session-pub session.key.pub \
    --ttl-secs 60 --max-qty 5 \
    --symbols BTC-USD --sides buy \
    --out delegation.bin

# Pre-register the master pubkey on the server (easiest: read master.key.pub from a
# --users file at startup). Real systems would have account onboarding here.

cargo run -p tls --bin p10-dex-client -- connect \
    --server 127.0.0.1:8443 \
    --session-key session.key --delegation delegation.bin

> buy BTC-USD 1 @ 50000
  ← Accepted("order 7f3a")
> buy BTC-USD 99 @ 50000
  ← Rejected("qty exceeds delegation limit")
> sell BTC-USD 1 @ 60000
  ← Rejected("side not in scope")
> buy ETH-USD 1 @ 3000
  ← Rejected("symbol not in scope")

# Wait 60 seconds:
> buy BTC-USD 1 @ 50000
  ← Rejected("delegation expired")
```

Every rejection corresponds to a specific step of the pipeline. That's the whole point.

## What curl's `-k` equivalent is here

Nothing. That's the point. The TLS layer alone would let you build a system where a compromised exchange server forges orders — so you *also* sign them at the application layer. A curl-style "skip verification" flag has no meaning in the session-key layer; skipping any of the seven checks breaks the scheme.

## Exercises

### Exercise 1: Basic flow

Implement steps 1-6. Verify the four rejection cases above all fire for the right reasons.

### Exercise 2: Domain tags matter

Remove the `b"dex/delegation/v1"` / `b"dex/order/v1"` prefixes. Show how a signed delegation for scope `max_qty=1000` could, with the right alignment, be interpreted as a signed *order* with enormous qty. (You will likely need to craft byte lengths carefully — that's the lesson.) Put the prefixes back.

### Exercise 3: Out-of-order checks

Move check 5 (session signature) *before* check 2 (master signature). Explain what goes wrong: a client can put any `session_pubkey` + matching `session_signature` they like into a fake delegation, and if the master sig check is gated on something that fails first, the error message can leak whether the session key is valid. Restore the order.

### Exercise 4: Replay after restart

Kill the server, reconnect, resubmit an old accepted order. What happens? Fix it: persist `seen_order_ids` to disk, or include `not_before` in the order and reject anything older than the server's startup time.

### Exercise 5: Revocation lag

Build a second verifier (another `p10-dex-server` instance in front of a load balancer). Revoke a session on one; submit to the other. Observe the window. Propose fixes: shared store (Redis), pub/sub, shorter TTLs, or a revocation Merkle root included in each delegation.

### Exercise 6: Account abstraction gap

Read ERC-4337 session keys. Compare: what does the on-chain contract enforce that your server doesn't, and vice versa? Write down one capability your design has that 4337 doesn't (hint: your verifier is stateful; theirs has to be nearly stateless).

### Exercise 7: mTLS bind (advanced)

Add client certificates (Lesson 11). Require the TLS client cert's public key to match the master pubkey on the delegation. What class of attack does this close? What does it cost in UX?

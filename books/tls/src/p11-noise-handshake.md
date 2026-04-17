# Project: Noise Handshake (a mini WireGuard)

> **Prerequisites**: Lesson 2 (ChaCha20-Poly1305), Lesson 4 (X25519), Lesson 5 (HKDF), Lessons 9-10 (your mini-TLS). Build the handshake pattern WireGuard and Signal use — *without* TLS.

## What is this?

TLS is not the only way to build a secure channel. The [Noise Protocol Framework](http://noiseprotocol.org) is a *cookbook* for building handshakes from primitives you already know: X25519, ChaCha20-Poly1305, HKDF. It's what WhatsApp, Signal, WireGuard, and Tailscale use under the hood.

You're going to build a minimal Noise-like handshake (specifically, the `XX` pattern) and run a VPN-style tunnel over it. The repo is called `simple-vpn` for a reason — this is the project that earns the name.

```
┌──────────────────────────────────────────────────────────┐
│  Why Noise instead of TLS?                               │
│                                                          │
│  TLS:                                                    │
│    • Designed for "unknown server → browser" scenarios    │
│    • Certificates, CAs, SNI, ALPN, session tickets       │
│    • Protocol is huge: ~200 pages of RFC                 │
│    • Dozens of cipher suites, modes, versions            │
│                                                          │
│  Noise:                                                  │
│    • Designed for peer-to-peer with pre-known identities │
│    • No certificates — just raw public keys              │
│    • ~30 lines of state for a handshake                  │
│    • You pick ONE cipher, ONE curve, ONE hash            │
│    • Used by: WireGuard, Signal, WhatsApp, Lightning,    │
│      Nym, I2P, Tailscale's control plane                 │
└──────────────────────────────────────────────────────────┘
```

## What you're building

A two-binary VPN-lite: a responder binds a port, an initiator dials it, they do a 3-message Noise XX handshake, then anything typed on one side comes out the other — encrypted, authenticated, with forward secrecy.

```sh
# Terminal 1 — responder (knows its own static key):
cargo run -p tls --bin p11-noise -- responder \
  --port 9000 --static-key server.key
# Listening on 0.0.0.0:9000
# [+] new connection from 127.0.0.1:51234
# [+] handshake complete, peer static = fe2a..
# client: hello
# >

# Terminal 2 — initiator (knows its own static key):
cargo run -p tls --bin p11-noise -- initiator \
  --host 127.0.0.1:9000 --static-key client.key
# [+] handshake complete, peer static = 8c1f..
# > hello
# server: got it
```

No TLS. No certificates. No CAs. Just X25519 + ChaCha20-Poly1305 + HKDF, composed into a handshake.

## Try it with existing tools first

```sh
# === WireGuard: Noise IK pattern, baked into the kernel ===

# Generate keypairs:
wg genkey | tee server.sk | wg pubkey > server.pk
wg genkey | tee client.sk | wg pubkey > client.pk

# One-line "what does a Noise handshake look like":
#   cat server.sk client.pk   ← responder's static + initiator's known-public
# WireGuard uses these to derive session keys with no messages back and forth
# (the IK pattern, 1-round-trip).

# === Signal's X3DH: Noise-like but for async messaging ===
# See https://signal.org/docs/specifications/x3dh/
# Same primitives, different choreography for offline delivery.
```

## How Noise works (the pattern)

A Noise pattern is a short program, read from top to bottom. Each line is either `->` (initiator sends to responder) or `<-` (the other direction). The tokens describe what gets mixed into the shared key.

Noise **XX** (mutual authentication, 3 messages, no pre-shared keys):

```
XX:
  -> e                          initiator sends ephemeral pubkey
  <- e, ee, s, es               responder: ephemeral, then its static (encrypted)
  -> s, se                      initiator: its static (encrypted)
```

Tokens read as:

- `e` — send/receive an **ephemeral** X25519 public key (32 bytes, plaintext first time, encrypted if we have a key)
- `s` — send/receive a **static** X25519 public key (long-term identity)
- `ee` — DH between both ephemerals
- `es` — DH between initiator's ephemeral + responder's static
- `se` — DH between initiator's static + responder's ephemeral

After each DH, you HKDF-extract the result into the running chaining key `ck`. After each message, `ck` seeds the AEAD key for the *next* message. At the end, `ck` gives you two keys (`k1` for initiator→responder, `k2` for the reverse).

That's the whole thing. The rest is bookkeeping.

## Architecture

```
Initiator                                Responder
                                          (has static s_r)

                  msg1: e_i (32B)
  ─────────────────────────────────────────►

                                    DH(e_i, e_r) → ck
                                    encrypt(s_r) under ck
                                    DH(e_i, s_r)  → ck

                  msg2: e_r + Enc(s_r) + tag
  ◄─────────────────────────────────────────

  DH(e_i, e_r)   → ck
  decrypt(s_r)
  DH(e_i, s_r)    → ck
  encrypt(s_i) under ck
  DH(s_i, e_r)    → ck

                  msg3: Enc(s_i) + tag
  ─────────────────────────────────────────►

                                    decrypt(s_i)
                                    DH(s_i, e_r)  → ck

     both sides split ck into {k_send, k_recv}, then data flows.
```

Three messages. Mutual authentication (each side proved possession of its static key through a DH). Forward secrecy (ephemeral keys are discarded). Identity hiding (the initiator's static key only leaves after we have a key to encrypt it under).

## Implementation guide

### Step 0: Project setup

```sh
touch tls/src/bin/p11-noise.rs
touch tls/src/noise.rs
```

Register in `tls/src/lib.rs`:

```rust
pub mod noise;
```

All dependencies are already in `Cargo.toml` from earlier lessons: `x25519-dalek`, `chacha20poly1305`, `hkdf`, `sha2`, `clap`, `tokio`.

### Step 1: The Noise state machine

Four pieces of state, mutated by every token:

```rust
pub struct HandshakeState {
    pub ck: [u8; 32],   // chaining key — feeds the next HKDF
    pub h:  [u8; 32],   // handshake hash — MixHash over everything sent/received
    pub k:  Option<[u8; 32]>, // current AEAD key (None until the first DH)
    pub n:  u64,        // nonce counter for k
}
```

Two primitive operations do all the work:

```rust
use hkdf::Hkdf;
use sha2::{Digest, Sha256};

impl HandshakeState {
    // MixHash: absorb arbitrary bytes into the handshake transcript hash.
    fn mix_hash(&mut self, data: &[u8]) {
        let mut h = Sha256::new();
        h.update(&self.h);
        h.update(data);
        self.h = h.finalize().into();
    }

    // MixKey: fold a DH result into ck, derive a fresh AEAD key.
    fn mix_key(&mut self, dh: &[u8]) {
        let (prk, hk) = Hkdf::<Sha256>::extract(Some(&self.ck), dh);
        let mut ck = [0u8; 32];
        let mut k  = [0u8; 32];
        hk.expand(b"noise/ck", &mut ck).unwrap();
        hk.expand(b"noise/k",  &mut k).unwrap();
        self.ck = ck;
        self.k  = Some(k);
        self.n  = 0;
    }
}
```

### Step 2: Encrypted tokens (`encrypt_and_hash` / `decrypt_and_hash`)

Every time the pattern writes a public key or a payload *after* a `MixKey` has happened, it's encrypted under the current `k` with the handshake hash `h` as AAD:

```rust
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce, aead::{Aead, Payload}};

impl HandshakeState {
    fn encrypt_and_hash(&mut self, plaintext: &[u8]) -> Vec<u8> {
        let ct = match self.k {
            Some(key) => {
                let cipher = ChaCha20Poly1305::new((&key).into());
                let nonce = nonce_bytes(self.n);
                let ct = cipher.encrypt(
                    Nonce::from_slice(&nonce),
                    Payload { msg: plaintext, aad: &self.h },
                ).unwrap();
                self.n += 1;
                ct
            }
            None => plaintext.to_vec(),
        };
        self.mix_hash(&ct);
        ct
    }

    fn decrypt_and_hash(&mut self, ciphertext: &[u8]) -> Vec<u8> {
        let pt = match self.k {
            Some(key) => {
                let cipher = ChaCha20Poly1305::new((&key).into());
                let nonce = nonce_bytes(self.n);
                let pt = cipher.decrypt(
                    Nonce::from_slice(&nonce),
                    Payload { msg: ciphertext, aad: &self.h },
                ).expect("noise decrypt failed");
                self.n += 1;
                pt
            }
            None => ciphertext.to_vec(),
        };
        self.mix_hash(ciphertext);
        pt
    }
}

fn nonce_bytes(n: u64) -> [u8; 12] {
    let mut b = [0u8; 12];
    b[4..].copy_from_slice(&n.to_be_bytes());
    b
}
```

### Step 3: The three handshake messages

Each message is a direct transliteration of the pattern:

```rust
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};

// msg1: -> e
pub fn write_msg1(state: &mut HandshakeState, e_i: &EphemeralSecret) -> Vec<u8> {
    let e_pub = PublicKey::from(e_i);
    state.mix_hash(e_pub.as_bytes());
    e_pub.as_bytes().to_vec()
}

// msg2: <- e, ee, s, es
pub fn write_msg2(
    state: &mut HandshakeState,
    e_r: &EphemeralSecret,
    s_r: &StaticSecret,
    e_i_pub: &PublicKey,
) -> Vec<u8> {
    let mut out = Vec::new();

    // e
    let e_pub = PublicKey::from(e_r);
    state.mix_hash(e_pub.as_bytes());
    out.extend_from_slice(e_pub.as_bytes());

    // ee
    let dh = e_r.diffie_hellman(e_i_pub);
    state.mix_key(dh.as_bytes());

    // s  (encrypted under current k)
    let s_pub = PublicKey::from(s_r);
    out.extend_from_slice(&state.encrypt_and_hash(s_pub.as_bytes()));

    // es
    let dh = s_r.diffie_hellman(e_i_pub);
    state.mix_key(dh.as_bytes());

    out
}

// msg3: -> s, se
// (write it yourself — mirrors msg2 with roles swapped)
```

Reading each message is the mirror: take `e` off the wire, `mix_hash`; do the DH, `mix_key`; `decrypt_and_hash` the encrypted static key.

### Step 4: Split into data-phase keys

After message 3, both sides have the same `ck`. Derive two AEAD keys:

```rust
pub fn split(state: &HandshakeState) -> ([u8; 32], [u8; 32]) {
    let (_, hk) = Hkdf::<Sha256>::extract(Some(&state.ck), &[]);
    let mut k_send = [0u8; 32];
    let mut k_recv = [0u8; 32];
    hk.expand(b"noise/send", &mut k_send).unwrap();
    hk.expand(b"noise/recv", &mut k_recv).unwrap();
    (k_send, k_recv)
}
```

The initiator uses `k_send` for outgoing; the responder uses `k_recv` for incoming. (Swap for the other direction.)

### Step 5: Data phase

Identical to Lesson 9-10: AEAD-encrypt each message with a counter nonce, length-prefix the frame. You've already built this.

### Step 6: Wire it up

`p11-noise.rs` has two subcommands, `initiator` and `responder`. Each loads its static key, opens a TCP socket, does the handshake, calls `split`, and enters a tokio line-oriented chat loop using the data-phase keys.

### Step 7: Verify the properties

The whole point of Noise is that you get specific cryptographic properties *for free* by choosing the right pattern. Prove them:

1. **Mutual authentication** — flip the responder's static key to a random one mid-test. Handshake fails (`decrypt failed` on msg3's DH).
2. **Forward secrecy** — capture a full transcript with `tcpdump`. Then leak one side's static key. Show you *still* can't decrypt the transcript, because the session keys depended on the ephemerals, which were discarded.
3. **Identity hiding** — `tcpdump` the handshake. Note that the initiator's static public key never appears in plaintext (it's sent in msg3, encrypted). This is why WireGuard has no "client connected from IP X.X.X.X with key Y" banner visible on the wire.

## What WireGuard does differently

WireGuard uses the **IK** pattern, not XX. In IK, the initiator already knows the responder's static public key (think: you configured it in `[Peer]`). That lets the handshake collapse to *one* round-trip, at the cost of the initiator having to know the responder's key up front. The code structure is identical — only the message-by-message tokens change. See `noiseprotocol.org` for the full pattern table.

## Exercises

### Exercise 1: Full XX handshake

Implement everything through Step 5. Demonstrate a two-terminal chat survives over the channel.

### Exercise 2: Switch to IK

Re-derive your handshake for the IK pattern. The initiator takes the responder's static public key as a CLI flag. Observe: one fewer message, and you leak the responder's identity (the initiator had to know it).

### Exercise 3: Forward secrecy, demonstrated

Run a session, log the traffic with `tcpdump -w cap.pcap`. Then delete the client's static key — but keep the public key. Try to re-derive session keys from what's left. Fail. Now rerun the handshake and *keep the ephemeral secrets*; verify you *could* decrypt the capture. That's forward secrecy: it's a property of ephemerals being discarded, not of the crypto itself.

### Exercise 4: Transport-level rekey

After sending 1GB or 10 minutes, rotate the data-phase keys (`k_send = HKDF(k_send, "rekey")`). Explain why this matters even when your cipher's nonce space is effectively unlimited. (Answer: compromised-now, decrypted-everything-before defence in depth.)

### Exercise 5: Kyber hybrid

Replace X25519 with X25519 + ML-KEM-768 hybrid in the DHs. The pattern structure is unchanged — the DH output is just concatenated with the Kyber shared secret before `mix_key`. This is how TLS 1.3 is going to get post-quantum security, and Noise gets it for less code.

### Exercise 6: Run it as a real VPN

Wrap the data phase in a `tun` device (Linux) or `utun` (macOS). Now all IP packets go through your tunnel. Congratulations — that's a VPN. WireGuard is roughly 4000 lines of kernel code doing what you just did in 400 lines of Rust.

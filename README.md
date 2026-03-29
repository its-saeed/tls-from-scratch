# TLS from Scratch

A hands-on course that builds up TLS from its cryptographic primitives. Each lesson introduces one concept, explains the theory, and includes a working Rust implementation.

## Prerequisites

- TCP/UDP networking basics
- Rust fundamentals

## Lessons

### Phase 1: Cryptographic Building Blocks

| # | Topic | Code | Notes |
|---|-------|------|-------|
| 1 | [Hashing (SHA-256)](src/bin/1-hash.md) | [1-hash.rs](src/bin/1-hash.rs) | Fixed-size fingerprints, integrity, one-way functions |
| 2 | [Symmetric Encryption (ChaCha20-Poly1305)](src/bin/2-encrypt.md) | [2-encrypt.rs](src/bin/2-encrypt.rs) | AEAD, nonces, tamper detection |
| 3 | [Asymmetric Crypto & Signatures (Ed25519)](src/bin/3-sign.md) | [3-sign.rs](src/bin/3-sign.rs) | Key pairs, sign/verify, digital identity |
| 4 | [Key Exchange (X25519)](src/bin/4-keyexchange.md) | [4-keyexchange.rs](src/bin/4-keyexchange.rs) | Diffie-Hellman, forward secrecy |

### Phase 2: Putting Primitives Together

| # | Topic | Code | Notes |
|---|-------|------|-------|
| 5 | [Key Derivation (HKDF)](src/bin/5-kdf.md) | [5-kdf.rs](src/bin/5-kdf.rs) | HMAC, extract-and-expand, multiple keys from one secret |
| 6 | [Certificates & Trust (X.509)](src/bin/6-certs.md) | [6-certs.rs](src/bin/6-certs.rs) | Chain of trust, self-signed certs, preventing MITM |

### Phase 3: Build a Mini-TLS

| # | Topic | Code | Notes |
|---|-------|------|-------|
| 7 | [Encrypted Echo Server](src/bin/7-echo-server.md) | [server](src/bin/7-echo-server.rs) / [client](src/bin/7-echo-client.rs) | Combines lessons 2+4+5 into a working encrypted channel |
| 8 | [Authenticated Echo Server](src/bin/8-echo-server.md) | [genkey](src/bin/8-genkey.rs) / [server](src/bin/8-echo-server.rs) / [client](src/bin/8-echo-client.rs) | Signs the handshake to prevent MITM (adds lessons 3+6) |
| 9 | [Mutual TLS (mTLS)](src/bin/9-mtls.md) | [genkeys](src/bin/9-mtls-genkeys.rs) / [server](src/bin/9-mtls-server.rs) / [client](src/bin/9-mtls-client.rs) | Both sides authenticate each other |
| 10 | [Replay Attack Defense](src/bin/10-replay.md) | [server](src/bin/10-replay-server.rs) / [client](src/bin/10-replay-client.rs) | Counter nonces prevent replay and reordering |

### Phase 4: Real TLS

| # | Topic | Code | Notes |
|---|-------|------|-------|
| 11 | [Real TLS (tokio-rustls)](src/bin/11-real-tls.md) | [server](src/bin/11-real-tls-server.rs) / [client](src/bin/11-real-tls-client.rs) | Production TLS — see how your hand-built protocol maps to the real thing |
| 12 | [HTTPS Client](src/bin/12-https-client.md) | [12-https-client.rs](src/bin/12-https-client.rs) | Connect to real websites over TLS, the full circle |

## How it all connects

```
Lesson 1: Hashing ─────────────────────────────────┐
Lesson 2: Symmetric Encryption ──────────┐          │
Lesson 3: Signatures ──────────┐         │          │
Lesson 4: Key Exchange ────┐   │         │          │
                           │   │         │          │
                           ▼   │         ▼          ▼
Lesson 5: Key Derivation ──┤   │   (HKDF uses HMAC, which uses hashing)
                           │   │         │
                           ▼   ▼         ▼
Lesson 7: Encrypted channel (DH + HKDF + ChaCha20)
Lesson 6: Certificates ───►│
                           ▼
Lesson 8: Authenticated channel (+ signatures + certs)
Lesson 9: Mutual authentication (both sides prove identity)
Lesson 10: Replay defense (counter nonces)
                           │
                           ▼
Lesson 11: Real TLS (tokio-rustls does it all)
Lesson 12: HTTPS client (connect to the real internet)
```

## Running

```sh
# Run any lesson
cargo run --bin 1-hash -- --file-path Cargo.toml
cargo run --bin 2-encrypt
cargo run --bin 3-sign
cargo run --bin 4-keyexchange
cargo run --bin 5-kdf
cargo run --bin 6-certs

# Echo server — no auth (two terminals)
cargo run --bin 7-echo-server
cargo run --bin 7-echo-client

# Authenticated echo server (generate key first, then two terminals)
cargo run --bin 8-genkey
cargo run --bin 8-echo-server
cargo run --bin 8-echo-client

# Mutual TLS (generate both keys first, then two terminals)
cargo run --bin 9-mtls-genkeys
cargo run --bin 9-mtls-server
cargo run --bin 9-mtls-client

# Replay defense (two terminals)
cargo run --bin 10-replay-server
cargo run --bin 10-replay-client

# Real TLS echo server (two terminals)
cargo run --bin 11-real-tls-server
cargo run --bin 11-real-tls-client

# HTTPS client (connects to example.com)
cargo run --bin 12-https-client
```

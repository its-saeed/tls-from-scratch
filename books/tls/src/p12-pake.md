# Project: Password-Authenticated Key Exchange (PAKE)

> **Prerequisites**: Lesson 4 (X25519), Lesson 5 (HKDF), Lesson 6 (password KDFs), P3 (Password Vault). Bootstrap a shared secret from a low-entropy password — without ever sending the password, and without trusting the server to keep it.

## What is this?

Every project so far has an uncomfortable assumption baked in: **the keys already exist**. Someone generated them, someone distributed them, both sides know them before the first handshake. In real life, humans don't exchange 32-byte X25519 public keys — they remember passwords. Short, guessable, re-used passwords.

PAKE (Password-Authenticated Key Exchange) is the cryptographic trick that lets two parties who share only a *password* derive a strong session key such that:

- The password is **never sent** — not in plaintext, not hashed, not encrypted.
- A passive observer learns nothing about the password.
- An active MITM gets **exactly one online guess per connection attempt** — no offline brute-force possible.
- The server doesn't need to store the password either. In augmented PAKE (OPAQUE), the server stores a verifier; a full server database leak doesn't let the attacker log in as any user — they still have to brute-force each password.

```
┌──────────────────────────────────────────────────────────┐
│  What's wrong with "just hash the password"              │
│                                                          │
│  Classic login:                                          │
│    client sends username + password (over TLS)           │
│    server hashes, compares with stored hash              │
│                                                          │
│  Problems:                                               │
│    1. Server SEES the password. If server is compromised │
│       or malicious, all passwords leak instantly.        │
│    2. TLS breach (compromised CA, bad cert pin) reveals  │
│       the password in transit.                           │
│    3. Password-hash leaks from DB → offline brute force. │
│                                                          │
│  With PAKE:                                              │
│    1. Server NEVER sees the password — only a DH-like    │
│       transcript that's useless without it.              │
│    2. A MITM without the password learns nothing they    │
│       can brute-force offline.                           │
│    3. Server stores a verifier (not the password).       │
│       DB leak still forces a brute-force-per-user.       │
└──────────────────────────────────────────────────────────┘
```

Used by: 1Password's secret-sharing, WiFi WPA3 (SAE), iCloud Keychain, Apple FaceTime, Magic Wormhole (file transfer).

## What you're building

You'll implement **CPace**, a modern PAKE that's roughly 50 lines of math on top of X25519. Then you wrap your mini-TLS from Lessons 9-10 with it, so two parties share a password, type `hunter2` on both ends, and end up with a secure authenticated channel — no certs, no PKI.

```sh
# Terminal 1 — responder:
cargo run -p tls --bin p12-pake -- responder \
  --port 9500 --password hunter2
# Listening on 0.0.0.0:9500
# [+] connection from 127.0.0.1:54321
# [+] PAKE succeeded, session id = 7f3a...
# peer: hello

# Terminal 2 — initiator:
cargo run -p tls --bin p12-pake -- initiator \
  --host 127.0.0.1:9500 --password hunter2
# [+] PAKE succeeded, session id = 7f3a...
# > hello

# Terminal 3 — wrong password:
cargo run -p tls --bin p12-pake -- initiator \
  --host 127.0.0.1:9500 --password hunter3
# Handshake failed: authentication tag mismatch
```

The wrong-password side sees an AEAD decryption failure. Not a "wrong password" error — the server literally cannot construct the same session key, so its reply decrypts to garbage. This is exactly the property we want: a remote attacker gets one guess per connection attempt.

## Try it with existing tools first

```sh
# === Magic Wormhole: PAKE for humans ===
# brew install magic-wormhole  (or pip install magic-wormhole)

# Sender:
wormhole send ~/somefile.txt
# "Wormhole code is: 7-crossover-clockwork"

# Receiver:
wormhole receive
# "Enter receive wormhole code: 7-crossover-clockwork"
# Files transfer.

# Under the hood: SPAKE2 (a PAKE) derives a session key from
# the 3-word code. The code is LOW entropy (~24 bits) and yet
# secure, because only one online guess is possible per session.
```

```sh
# === WiFi WPA3 ===
# Every WPA3-certified router since 2020 replaced PSK with SAE,
# which is a PAKE (Dragonfly). This is why WPA3 networks are
# resistant to offline dictionary attacks against the handshake,
# and WPA2 networks are not.
```

## How CPace works

One page of math. Here's everything:

```
Both sides know: password pw, optional session id sid

1. Compute G = MapToGroup(SHA-256(pw || sid))
   → a group element (point) on Curve25519 derived from the password

2. Alice:  picks random scalar a; sends A = a·G
   Bob:    picks random scalar b; sends B = b·G

3. Both compute K = SHA-256(A || B || a·B)  [Alice]
                K = SHA-256(A || B || b·A)  [Bob]

4. a·B == b·A == ab·G, so both derive the same K.

5. Anyone not knowing pw cannot reconstruct G, so even seeing
   A and B, they cannot derive K. This is the PAKE property.
```

Compare with a plain DH:

```
Normal DH:
  Alice sends A = a·BASE_POINT          (attacker sees it fine)
  Bob   sends B = b·BASE_POINT          (attacker sees it fine)
  Shared = a·B = b·A = ab·BASE_POINT    (attacker recovers it if they
                                         solve discrete log)

  → No authentication. Attacker-in-middle just runs DH with each side
    separately and decrypts everything.

CPace:
  Same structure, but BASE_POINT is replaced by G = MapToGroup(pw).
  Without pw, the attacker can't even start a DH with the right generator.
  → Authentication baked in — the password IS the generator.
```

## Architecture

```
Alice                                          Bob
(knows password pw)                            (knows password pw)

                               sid (16B)                  shared a salt / session id
  ◄───────────────────────────────────────────────────►

  G = MapToGroup(SHA-256(pw || sid))           G = same

  a = random scalar                            b = random scalar
  A = a · G                                    B = b · G

                       A (32B)
  ─────────────────────────────────────────────►

                       B (32B)
  ◄─────────────────────────────────────────────

  K = SHA-256(A || B || a·B)                   K = SHA-256(A || B || b·A)

  (a·B == b·A == ab·G)

  Derive session AEAD keys from K with HKDF.
  Data phase = your Lesson 9-10 channel.
```

Three messages. No certificates. Shared secret derived from a human password.

## Implementation guide

### Step 0: Project setup

```sh
touch tls/src/bin/p12-pake.rs
touch tls/src/pake.rs
```

Register in `tls/src/lib.rs`:

```rust
pub mod pake;
```

Everything you need is already in `Cargo.toml`: `x25519-dalek`, `sha2`, `hkdf`, `rand_core`, `argon2` (for Step 3's key stretching), `tokio`, `chacha20poly1305`.

### Step 1: MapToGroup — the password-derived generator

The password can't be used directly as an X25519 point — random 32-byte values aren't guaranteed to be valid curve points. Use the Elligator 2 map, which is exactly designed for this:

```rust
use curve25519_dalek::montgomery::MontgomeryPoint;
use sha2::{Digest, Sha256};

/// Deterministically map a password + session id to a Curve25519 point.
/// This point becomes the "base" for the PAKE's Diffie-Hellman.
pub fn map_to_group(password: &[u8], sid: &[u8]) -> MontgomeryPoint {
    let mut h = Sha256::new();
    h.update(b"cpace/g/v1");
    h.update(password);
    h.update(sid);
    let digest: [u8; 32] = h.finalize().into();
    // Elligator 2: every 32-byte input maps to a valid point.
    MontgomeryPoint(digest).mul_base_clamped([1; 32]).double() // simplified
}
```

(In a real implementation, use the proper Elligator2 routine from `curve25519-dalek`'s low-level API. The naive approach above is for exposition — exercise 2 asks you to replace it.)

### Step 2: The exchange

```rust
use curve25519_dalek::scalar::Scalar;
use rand_core::{OsRng, RngCore};

pub struct PakeState {
    pub scalar:    Scalar,            // a (initiator) or b (responder)
    pub local:     MontgomeryPoint,   // A or B
}

pub fn start(password: &[u8], sid: &[u8]) -> PakeState {
    let g = map_to_group(password, sid);

    // Pick a random scalar. MUST be fresh per handshake.
    let mut scalar_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut scalar_bytes);
    let scalar = Scalar::from_bytes_mod_order(scalar_bytes);

    let local = g * scalar;
    PakeState { scalar, local }
}

pub fn finish(
    password: &[u8],
    sid: &[u8],
    my: &PakeState,
    peer_point: &MontgomeryPoint,
    my_is_initiator: bool,
) -> [u8; 32] {
    let shared = peer_point * my.scalar;

    let mut h = Sha256::new();
    h.update(b"cpace/k/v1");
    h.update(sid);
    // Consistent ordering so both sides hash the same thing:
    let (a, b) = if my_is_initiator {
        (&my.local, peer_point)
    } else {
        (peer_point, &my.local)
    };
    h.update(a.as_bytes());
    h.update(b.as_bytes());
    h.update(shared.as_bytes());
    h.finalize().into()
}
```

### Step 3: Stretch the password first

A password has ~20 bits of entropy. Feed it through Argon2 before it hits MapToGroup, so each guess costs the attacker real CPU:

```rust
use argon2::Argon2;

pub fn stretch(password: &[u8], sid: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    Argon2::default().hash_password_into(password, sid, &mut out).unwrap();
    out
}
```

Call `stretch` once at the top of the handshake; use the output as the `password` argument to `map_to_group`. This is the "password KDF" from Lesson 6, reused here.

### Step 4: Session-id agreement

CPace needs both sides to use the *same* session id to derive the same `G`. Simplest: initiator picks a random 16-byte `sid` and sends it first. A better scheme (binding to both parties) is an exercise.

```rust
// Initiator:
let mut sid = [0u8; 16];
OsRng.fill_bytes(&mut sid);
send(&mut stream, &sid).await?;
let state = start(&stretched, &sid);
send(&mut stream, state.local.as_bytes()).await?;

let peer_bytes: [u8; 32] = recv(&mut stream).await?;
let peer = MontgomeryPoint(peer_bytes);
let k = finish(&stretched, &sid, &state, &peer, true);
```

### Step 5: Turn K into session keys

```rust
use hkdf::Hkdf;

pub fn derive_session_keys(k: &[u8; 32], sid: &[u8]) -> ([u8; 32], [u8; 32]) {
    let (_, hk) = Hkdf::<Sha256>::extract(Some(sid), k);
    let mut k_i2r = [0u8; 32];  // initiator → responder
    let mut k_r2i = [0u8; 32];  // responder → initiator
    hk.expand(b"cpace/i2r", &mut k_i2r).unwrap();
    hk.expand(b"cpace/r2i", &mut k_r2i).unwrap();
    (k_i2r, k_r2i)
}
```

From here, the data phase is identical to Lesson 9-10: counter-nonce ChaCha20-Poly1305 over length-prefixed frames.

### Step 6: Verify you got it right

Confirmation messages are the sharpest test. Right after the key exchange, each side sends an encrypted tag derived from `K`:

```rust
// Initiator sends first:
let tag_i = hmac(k_i2r, b"confirm-initiator");
send_encrypted(&mut stream, &tag_i).await?;

// Responder computes what it *expects* to see and compares:
let expected = hmac(k_i2r, b"confirm-initiator");
assert_eq!(expected, recv_encrypted(&mut stream).await?);
```

If the passwords differ, the two sides derived different `K`s, so the AEAD decryption of the tag fails outright. This is where a wrong password gets caught — *actively*, by the server, on the first frame.

### Step 7: Test it

```sh
# Happy path:
p12-pake responder --port 9500 --password hunter2
p12-pake initiator --host 127.0.0.1:9500 --password hunter2
# Both log: "PAKE succeeded, session id = ..."

# Wrong password — fails loudly:
p12-pake initiator --host 127.0.0.1:9500 --password wrong
# Handshake failed: authentication tag mismatch

# Packet capture of a successful run:
tcpdump -i lo -A port 9500
# You see: 16 bytes (sid), 32 bytes (A), 32 bytes (B), then AEAD ciphertexts.
# Nothing resembles the password. No dictionary attack is possible
# from the transcript alone.
```

## Why the server doesn't need to store the password

The scheme above is a **symmetric PAKE** — both sides are equal, both hold the same password. If the server is compromised, the attacker gets the password and can impersonate any user, *but also* the user can impersonate the server (because both sides are symmetric).

**Augmented PAKE** (OPAQUE, SRP-6a) breaks the symmetry: the server stores a *verifier* derived from the password such that the server can authenticate the user without being able to impersonate them. This is what 1Password, iCloud Keychain, and modern secure messengers use. The protocol is more involved (roughly: an oblivious PRF evaluation plus a signed envelope); it's a natural next step after you have this one working.

## Exercises

### Exercise 1: Working CPace

Implement steps 1-6. Prove: (a) right password succeeds, (b) wrong password fails on the confirmation tag, (c) the wire transcript contains no brute-forceable data.

### Exercise 2: Proper Elligator 2

The MapToGroup in Step 1 is a simplification. Replace it with the real Elligator 2 routine (`MontgomeryPoint::from_slice` + the curve25519-dalek `Elligator2` hash-to-curve). Verify the output is distributed uniformly by sampling 10,000 random passwords and confirming the outputs cover the curve.

### Exercise 3: Online-guess rate limiting

A remote attacker still gets *one* guess per connection. If the server allows unlimited connections, they can brute-force online. Add per-IP rate limiting (e.g. 3 attempts / minute). This is the piece every real PAKE deployment includes — the crypto alone isn't enough.

### Exercise 4: Session id binding

Instead of the initiator unilaterally choosing `sid`, do a mutual exchange: each side contributes 16 bytes of entropy, `sid = SHA-256(si || sr)`. Why does this matter? Hint: it prevents an attacker from replaying a PAKE transcript against the same server with a different user.

### Exercise 5: OPAQUE sketch

Read the [OPAQUE RFC draft](https://datatracker.ietf.org/doc/draft-irtf-cfrg-opaque/). In 100 words, describe what the server stores per-user and why a full server-DB dump still requires offline brute-force-per-user to log in. This is the property that makes OPAQUE the gold standard for password-based authentication in 2026.

### Exercise 6: Attach it to Lesson 10

Replace the Ed25519-signed handshake in the Lesson 10 authenticated echo server with a PAKE. Now there are no long-term keys to distribute — just a shared password. Consider: what use cases does this enable that a pre-shared-key setup does not?

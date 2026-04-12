# Lesson 2: Symmetric Encryption (ChaCha20-Poly1305)

> **Alice's Bookstore — Chapter 2**
>
> Alice's bookstore is growing. Customers now buy books by sending their credit card numbers through her website. One afternoon, Bob shows her something alarming at the coffee shop:
>
> *"I'm on the same Wi-Fi as you. Watch this."*
>
> He opens a packet sniffer and shows her the raw network traffic. There it is — a customer's credit card number, in plaintext.
>
> *"Eve doesn't even need to be a skilled hacker. Anyone on this Wi-Fi can see everything your customers send."*
>
> *"How do I hide it?"*
>
> *"Encryption. You scramble the data so only you and the customer can read it. Everyone else sees random noise."*
>
> *"But we'd need to agree on a secret key first... how?"*
>
> *"One problem at a time. First, let's learn how encryption works. We'll solve the key-sharing problem in Lesson 4."*

## Real-life analogy: the lockbox with a shared key

Alice and Bob each have a copy of the same key for a lockbox:

```
Alice's side:                        Bob's side:
  ┌──────────────────┐               ┌──────────────────┐
  │ "meet at 3pm"    │               │                  │
  │    + key 🔑      │               │  ciphertext      │
  │    → lock 🔒     │──── send ────►│    + key 🔑      │
  │    → ciphertext  │               │    → unlock 🔓   │
  └──────────────────┘               │    → "meet at 3pm│
                                     └──────────────────┘

Eve intercepts:
  "x7#kQ!9pL@" ← meaningless without the key
```

**Symmetric** = same key locks and unlocks. Fast. Simple. The question is: how do Alice and Bob get the same key? (That's Lesson 4.)

## What symmetric encryption does

```
encrypt(key, nonce, plaintext) → ciphertext
decrypt(key, nonce, ciphertext) → plaintext

Key:        32 bytes (256 bits) — the shared secret
Nonce:      12 bytes — unique per message (explained below)
Plaintext:  your data (any size)
Ciphertext: same size as plaintext + 16 bytes (auth tag)
```

This is what TLS uses for **all** bulk data after the handshake. Fast — gigabytes per second.

## Try it yourself

```sh
# Encrypt a file with OpenSSL using ChaCha20:
echo "secret message" > plain.txt
openssl enc -chacha20 -in plain.txt -out encrypted.bin \
  -K $(openssl rand -hex 32) -iv $(openssl rand -hex 16)

# The encrypted file is unreadable:
xxd encrypted.bin | head -3

# See AES hardware acceleration on your CPU:
# macOS:
sysctl -a | grep -i aes
# hw.optional.aes: 1  ← your CPU has AES-NI

# Linux:
grep -o aes /proc/cpuinfo | head -1
```

## The old problem: encryption without authentication

Old ciphers (AES-CBC) only gave you **confidentiality**:

```
Old encryption (AES-CBC):
  plaintext: "transfer $100 to Bob"
  encrypt → ciphertext: 0x7a3f8b2e1c...

  Attacker can't READ it                    ✓
  But attacker CAN flip bits:               ✗
    0x7a3f8b2e1c... → 0x7a3f8b2e9c...
    Decrypts to: "transfer $900 to Bob"

  Nobody detects the tampering!
  This is the "padding oracle" family of attacks.
```

## AEAD: Authenticated Encryption with Associated Data

Modern encryption combines **encryption + integrity** in one operation:

```
┌──────────────────────────────────────────────────────┐
│  AEAD Output                                         │
│                                                      │
│  ┌─────────────────────────┬──────────┐              │
│  │  Ciphertext             │ Auth Tag │              │
│  │  (same length as        │ (16 bytes│              │
│  │   plaintext, encrypted) │  MAC)    │              │
│  └─────────────────────────┴──────────┘              │
│                                                      │
│  On decrypt:                                         │
│    1. Verify the tag — was anything modified?         │
│    2. If tag invalid → ERROR (reject immediately)    │
│    3. If tag valid → decrypt → return plaintext      │
│                                                      │
│  Attacker flips one bit of ciphertext?               │
│    → Tag doesn't match → decryption FAILS            │
│    → No corrupted data ever reaches your code        │
└──────────────────────────────────────────────────────┘
```

## ChaCha20-Poly1305

TLS 1.3 supports exactly two AEAD ciphers:

```
                    AES-256-GCM          ChaCha20-Poly1305
─────────────────────────────────────────────────────────────
Encryption          AES (block cipher)   ChaCha20 (stream cipher)
Authentication      GHASH                Poly1305
Key / Nonce / Tag   256b / 96b / 128b   256b / 96b / 128b
Hardware accel      AES-NI (Intel/AMD)   None needed
Software speed      Slow without AES-NI  Fast everywhere
Used by             Most servers          Mobile, IoT, WireGuard
```

### How ChaCha20 works (simplified)

ChaCha20 is a **stream cipher** — it generates a pseudorandom keystream XORed with plaintext:

```
ChaCha20(key, nonce, counter) → keystream

plaintext:  "hello world!"
keystream:  0x7a3f8b2e1c...   (deterministic from key+nonce)
ciphertext: plaintext XOR keystream

To decrypt: ciphertext XOR keystream = plaintext
            (XOR undoes itself)
```

### How Poly1305 works (simplified)

Poly1305 computes a 16-byte **tag** over the ciphertext:

```
Poly1305(key, ciphertext) → tag (16 bytes)

Change one bit of ciphertext → completely different tag.
Receiver recomputes tag and compares — mismatch = tamper detected.
```

## Nonces: the most dangerous footgun

Every encryption call takes a **nonce** (number used once) — 12 bytes.

**The absolute rule: never reuse a nonce with the same key.**

### Why nonce reuse is catastrophic

Same key + same nonce → same keystream:

```
Message 1: "hello world!" XOR keystream = ciphertext_1
Message 2: "secret msg!!" XOR keystream = ciphertext_2
                               ↑ SAME keystream!

Attacker computes:
  ciphertext_1 XOR ciphertext_2
  = plaintext_1 XOR plaintext_2       ← keystreams cancel out!

From plaintext XOR, frequency analysis recovers both messages.
```

```sh
# Demonstrate XOR cancellation:
python3 -c "
a = b'hello world!'
b = b'secret msg!!'
xor = bytes(x ^ y for x, y in zip(a, b))
print(f'plaintext XOR: {xor.hex()}')
print('This is what the attacker gets from nonce reuse.')
"
```

### How TLS avoids nonce reuse

TLS uses a **counter**: message 0 → nonce 0, message 1 → nonce 1, etc. Simple, bulletproof.

### Real-world nonce disasters

- **2016 TLS**: nonce reuse across servers when session tickets were shared. Plaintext recovered.
- **PS3 (2010)**: Sony used the same nonce for every ECDSA signature. Private key recovered.
- **WPA2 KRACK (2017)**: Wi-Fi nonce reuse allowed decryption of packets.

## Associated Data: authenticated but not encrypted

AEAD can authenticate **plaintext metadata** alongside encrypted data:

```
┌──────────────────────────────────────────────────┐
│  Associated data (plaintext, tamper-proof):       │
│    "message-id: 42, type: chat"                  │
│                                                  │
│  Encrypted payload:                              │
│    "meet at 3pm" → 0x7a3f8b...                  │
│                                                  │
│  Auth tag covers BOTH:                           │
│    Modify the header? Tag fails.                 │
│    Modify the ciphertext? Tag fails.             │
└──────────────────────────────────────────────────┘
```

TLS uses this for record headers — the header is plaintext but authenticated.

## Benchmark on your machine

```sh
# Compare AES-GCM vs ChaCha20 on your hardware:
openssl speed -evp chacha20-poly1305
openssl speed -evp aes-256-gcm
# AES-GCM is usually faster with AES-NI hardware
# ChaCha20 wins on devices without AES-NI
```

## Exercises

### Exercise 1: Encrypt, decrypt, tamper

Encrypt a message with ChaCha20-Poly1305. Decrypt it. Flip one byte of ciphertext, try to decrypt — show the error.

### Exercise 2: Nonce reuse attack

Encrypt two messages with the **same** key and nonce. XOR the ciphertexts (skip the tag). Compare with XORing the plaintexts — they match. This proves nonce reuse leaks `plain1 XOR plain2`.

### Exercise 3: Counter nonce

Encrypt 10 messages with counter nonces (0, 1, 2, ...). Decrypt each. Try decrypting message 5 with nonce 3 — should fail.

```rust
fn counter_nonce(n: u64) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[4..12].copy_from_slice(&n.to_be_bytes());
    nonce
}
```

### Exercise 4: Associated data

Encrypt with AAD. Decrypt with correct AAD — works. Decrypt with modified AAD — fails. The metadata is plaintext but tamper-proof.

### Exercise 5: Large file encryption

Encrypt a 1MB file in 4KB chunks. Each chunk uses nonce = chunk index. Decrypt all chunks, reassemble, verify SHA-256 matches the original. This is how TLS encrypts a data stream.

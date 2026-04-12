# Lesson 1: Hashing (SHA-256)

> **Alice's Bookstore — Chapter 1**
>
> Alice sells digital books as PDF downloads. A customer named Carol emails her:
>
> *"The book I downloaded is 400 pages, but my friend got 402 pages from the same link. Did someone tamper with it? How do I know I got the real file?"*
>
> Bob suggests: *"Publish a fingerprint of each file on your website. Customers download the book, compute the fingerprint themselves, and compare. If they match — the file is intact. If not — something changed it."*
>
> *"A fingerprint... of a file?"*
>
> *"It's called a hash."*

## Real-life analogy: the fingerprint

Every person has a unique fingerprint. You can't reconstruct a person from their fingerprint, but you can verify "is this the same person?" by comparing prints.

```
Person      → Fingerprint
"hello"     → 2cf24dba5fb0a30e...
Cargo.toml  → 1d3901bae4c11bd5...
Linux ISO   → e3b0c44298fc1c14...

Properties:
  ✓ Same person → same fingerprint (deterministic)
  ✗ Fingerprint → person (one-way, can't reverse)
  ✓ Twins look alike but have different prints (collision resistant)
  ✓ Tiny scar → completely different print (avalanche effect)
```

A hash function is a digital fingerprint machine.

## What is a hash function?

A hash function takes **any** input — a single byte, a password, an entire movie — and produces a **fixed-size** output called a **digest** or **hash**.

SHA-256 always outputs 256 bits (32 bytes), no matter the input:

```
Input                          SHA-256 Output
──────────────────────────────────────────────────────────────────
"hello"                        2cf24dba5fb0a30e26e83b2ac5b9e29e...
"hello "  (added a space)      98ea6e4f216f2fb4b69fff9b3a44842c...
""  (empty string)             e3b0c44298fc1c149afbf4c8996fb924...
(4.7 GB Linux ISO)             a1b2c3d4... (still just 32 bytes)
```

## The four properties

### 1. Deterministic

Same input → same hash. Always. On any machine, any OS, any time.

```sh
# Try it — these will give the same hash on every computer in the world:
echo -n "hello" | shasum -a 256
# 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
```

### 2. One-way (preimage resistant)

Given a hash, you **cannot** compute the original input. The only option is brute force — try every possible input until one matches.

```
Forward (easy):   "hello" → sha256 → 2cf24dba...     ✓ instant
Reverse (hard):   2cf24dba... → ??? → "hello"         ✗ impossible

Why? SHA-256 is a series of bit operations (AND, OR, XOR, rotate, add)
that are easy to compute forward but destroy information.
It's like mixing paint — easy to mix red + blue → purple,
impossible to unmix purple → red + blue.
```

### 3. Avalanche effect

Change **one bit** of input → completely different hash. The outputs are unrelated.

```sh
# One character difference:
echo -n "hello" | shasum -a 256
# 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824

echo -n "hellp" | shasum -a 256
# 1f40fc92da241694750979ee6cf582f2d5d7d28e18335de05abc54d0560e0f53

# Completely different! Not a single hex digit in common.
```

### 4. Collision resistant

It's practically impossible to find two different inputs with the same hash. SHA-256 has 2^256 possible outputs — more than atoms in the observable universe (~10^80).

```
                     How big is 2^256?
    ┌──────────────────────────────────────────────┐
    │  Atoms in the universe:     ~10^80           │
    │  2^256:                     ~10^77           │
    │                                              │
    │  If you hashed 1 billion inputs per second   │
    │  for the entire age of the universe           │
    │  (13.8 billion years), you'd have tried       │
    │  ~4 × 10^26 inputs.                          │
    │                                              │
    │  That's 0.000...0% of the hash space.        │
    │  Collision? Not in this universe.             │
    └──────────────────────────────────────────────┘
```

## Try it yourself

### Hash a string

```sh
# macOS / Linux:
echo -n "hello" | shasum -a 256
# The -n is important! Without it, echo adds a newline.

# Verify: with newline gives a DIFFERENT hash
echo "hello" | shasum -a 256
# Different! The newline character changed the input.
```

### Hash a file

```sh
# Hash a file:
shasum -a 256 /etc/hosts

# On Linux, you can also use:
sha256sum /etc/hosts
```

### Compare two files

```sh
# Create two files, one byte different:
echo -n "hello world" > file1.txt
echo -n "hello worle" > file2.txt

shasum -a 256 file1.txt file2.txt
# Completely different hashes — one byte changed everything.
```

### Hash algorithms comparison

```sh
# Different hash functions, different output sizes:
echo -n "hello" | shasum -a 1       # SHA-1:   20 bytes — BROKEN, don't use
echo -n "hello" | shasum -a 256     # SHA-256: 32 bytes — standard
echo -n "hello" | shasum -a 512     # SHA-512: 64 bytes — extra security
echo -n "hello" | md5               # MD5:     16 bytes — BROKEN, don't use
```

**Never use MD5 or SHA-1 for security** — collisions have been found.

## How SHA-256 works (simplified)

You don't need to implement SHA-256, but understanding the structure helps:

```
Input: "hello" (5 bytes)
          │
          ▼
┌─────────────────────────────┐
│  Step 1: Padding            │  Add bits so length = multiple of 512 bits
│  Step 2: Split into blocks  │  One or more 512-bit blocks
│  Step 3: Initialize state   │  8 variables from √primes: h0..h7
│  Step 4: 64 rounds of mixing│  Rotate, shift, XOR, add constants
│  Step 5: Output h0..h7      │  Concatenate → 256-bit hash
└─────────────────────────────┘

Each round destroys information about the input.
After 64 rounds, recovering the input is infeasible.
```

## Real-world uses

### File integrity (Linux downloads)

```sh
# Every Linux distro publishes SHA-256 hashes:
wget https://releases.ubuntu.com/22.04/ubuntu-22.04-desktop-amd64.iso
wget https://releases.ubuntu.com/22.04/SHA256SUMS

# Verify:
shasum -a 256 -c SHA256SUMS
# ubuntu-22.04-desktop-amd64.iso: OK
```

### Git

Every commit, tree, and blob is identified by its hash:

```sh
# See the hash of a commit:
git log --oneline -1

# See the hash git computes for a file:
git hash-object Cargo.toml
```

### Password storage

```
WRONG — plaintext:
  Database: { user: "alice", password: "hunter2" }
  Attacker leaks DB → all passwords exposed

WRONG — plain SHA-256:
  Database: { user: "alice", hash: "f52fbd..." }
  Attacker uses rainbow table → cracked instantly

BETTER — SHA-256 + salt:
  Database: { user: "alice", salt: "x7k2", hash: "a3f1..." }
  Rainbow tables fail, but GPU brute force is fast

RIGHT — Argon2 (Lesson 6):
  Database: { user: "alice", hash: "$argon2id$..." }
  Intentionally slow → brute force impractical
```

### Blockchain (Bitcoin)

Find an input where `SHA-256(SHA-256(input))` starts with N zero bits:

```sh
# Simulate mining:
python3 -c "
import hashlib
for i in range(1000000):
    h = hashlib.sha256(f'block-nonce-{i}'.encode()).hexdigest()
    if h.startswith('0000'):
        print(f'Found! nonce={i} hash={h}')
        break
"
```

### How TLS uses hashing

```
┌────────────────────────────────────────────────────────┐
│  Hashing in TLS                                        │
│                                                        │
│  1. HMAC (Lesson 5)                                    │
│     Hash + secret key → message authentication         │
│                                                        │
│  2. HKDF (Lesson 5)                                    │
│     Derive encryption keys from shared secret          │
│                                                        │
│  3. Handshake transcript (Lesson 13)                   │
│     Hash ALL handshake messages → detect tampering     │
│                                                        │
│  4. Certificate fingerprint                            │
│     Identify certs by hash → certificate pinning       │
│                                                        │
│  5. Finished message                                   │
│     HMAC of transcript → proves handshake integrity    │
└────────────────────────────────────────────────────────┘
```

## Exercises

### Exercise 1: File hasher

Build a CLI tool in Rust that takes a file path and prints its SHA-256 hash. Use the `sha2` crate.

```sh
cargo run -p tls --bin 1-hash -- --file-path Cargo.toml
# Verify:
shasum -a 256 Cargo.toml
```

### Exercise 2: Avalanche effect

Hash these two strings and compare:
```
"The quick brown fox jumps over the lazy dog"
"The quick brown fox jumps over the lazy dog."
```

One period. How many hex digits differ? (All of them.)

### Exercise 3: Hash chain

Compute `SHA-256(SHA-256(SHA-256("hello")))` — hash the hash of the hash.

```sh
# Verify with shell:
echo -n "hello" | shasum -a 256 | awk '{print $1}' | \
  xxd -r -p | shasum -a 256 | awk '{print $1}' | \
  xxd -r -p | shasum -a 256
```

### Exercise 4: Commitment scheme

Build a commit-reveal protocol:
1. Ask for a secret prediction
2. Hash it, print the hash (commitment)
3. Ask the user to reveal the prediction
4. Hash the revealed text, compare
5. Match → prediction was honest

### Exercise 5: Hash speed benchmark

```rust
let start = Instant::now();
for i in 0..1_000_000 {
    let mut hasher = Sha256::new();
    hasher.update(i.to_be_bytes());
    hasher.finalize();
}
println!("{:.0} hashes/sec", 1_000_000.0 / start.elapsed().as_secs_f64());
```

Compare with `openssl speed sha256`. This shows why SHA-256 is too fast for passwords (Lesson 6).

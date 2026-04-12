# Lesson 6: Password-Based KDFs (PBKDF2/Argon2)

> **Prerequisites**: Lesson 5 (HKDF). You understand key derivation — now learn why passwords need a different approach.

## The core problem: passwords are terrible secrets

In Lesson 5, we used HKDF to derive encryption keys from a DH shared secret. That shared secret was 32 random bytes — impossible to guess.

But what if the secret is a **password**? Passwords are:

```
A random 256-bit key:
  7f3a9b2c...  (32 bytes of pure randomness)
  Possible values: 2^256 ≈ 10^77
  Time to brute-force: longer than the universe exists

A typical password:
  "password123"
  Possible values: maybe 10 billion common passwords
  Time to brute-force with HKDF: seconds
```

**Feel the problem yourself** — run this:

```sh
python3 -c "
import hashlib, time

# Simulate an attacker trying passwords with fast hashing
passwords = ['password', '123456', 'qwerty', 'letmein', 'monkey',
             'dragon', 'master', 'abc123', 'password123', 'target']

target_hash = hashlib.sha256(b'password123').hexdigest()

start = time.time()
for pw in passwords:
    h = hashlib.sha256(pw.encode()).hexdigest()
    if h == target_hash:
        elapsed = time.time() - start
        print(f'CRACKED! Password is: {pw}')
        print(f'Time: {elapsed*1000:.3f}ms')
        print(f'Tried {passwords.index(pw)+1} passwords')
        break

print()
print('Now imagine trying 10 billion passwords instead of 10.')
print('At 100 million hashes/sec (one GPU), that takes 100 seconds.')
print('Your password is cracked in under 2 minutes.')
"
```

That's the problem. HKDF and SHA-256 are **too fast**. An attacker with a GPU can try billions of passwords per second.

## The solution: make it slow on purpose

```
The idea is simple:

  Fast hash (SHA-256):
    password → key              instantly
    Attacker: 1,000,000,000 attempts/second

  Slow hash (PBKDF2, 100K iterations):
    password → hash → hash → ... (100K times) → key
    Attacker: 10 attempts/second

  Memory-hard hash (Argon2):
    password → fill 64MB of RAM with data → key
    Attacker: needs 64MB PER attempt
    GPU with 1000 cores but only 8GB RAM?
    → only 125 parallel attempts, not 1000
```

## Real-life analogy: the vault door

```
Regular door lock (HKDF):
  Insert key → door opens instantly
  If someone has a lockpick (GPU), they try 1 billion keys/second
  ┌──┐
  │🚪│ → click → open
  └──┘

Bank vault door (PBKDF2):
  Insert key → wait 1 second → door opens
  Lockpick still works, but only 1 attempt/second
  ┌──┐
  │🚪│ → click → ⏰ 1 second → open
  └──┘

Vault door + weight requirement (Argon2):
  Insert key → must also carry 64kg weight → wait → door opens
  Lockpick works, but you need one 64kg weight per attempt
  Can't try 1000 doors in parallel unless you have 64,000 kg of weights
  ┌──┐
  │🚪│ → click → 🏋️ 64kg → ⏰ 1 second → open
  └──┘
```

The delay and memory cost don't bother legitimate users (one login = one derivation). They devastate attackers who need billions of attempts.

## See the difference yourself

```sh
# FAST: SHA-256 — how many per second?
python3 -c "
import hashlib, time
start = time.time()
for i in range(1_000_000):
    hashlib.sha256(b'password123').digest()
elapsed = time.time() - start
rate = 1_000_000 / elapsed
print(f'SHA-256:   {rate:,.0f} hashes/second')
print(f'  10 billion passwords cracked in: {10_000_000_000/rate:.0f} seconds')
"

# SLOW: PBKDF2 100K iterations
python3 -c "
import hashlib, os, time
salt = os.urandom(16)
start = time.time()
for i in range(10):
    hashlib.pbkdf2_hmac('sha256', b'password123', salt, 100_000)
elapsed = time.time() - start
rate = 10 / elapsed
print(f'PBKDF2:    {rate:,.1f} hashes/second')
print(f'  10 billion passwords cracked in: {10_000_000_000/rate/3600/24/365:.0f} years')
"
```

Run both. The difference is dramatic — millions per second vs a handful per second.

## What does a password KDF actually do?

All password KDFs do the same thing conceptually:

```
Input:  "password123" (weak, guessable)
Output: 7f3a9b2c4d... (32 bytes, looks random, usable as encryption key)
```

Same as HKDF — but **intentionally slow**. The slowness IS the security feature.

Think of it this way:
- HKDF is a machine that instantly turns a secret into a key
- A password KDF is the same machine, but with a **speed limiter bolted on**
- The speed limiter wastes CPU time (PBKDF2) or RAM (Argon2) on purpose
- A legitimate user calls it once (1 second delay — barely noticeable)
- An attacker calls it billions of times (1 second × 1 billion = 31 years)

## The three password KDFs

### PBKDF2 (Password-Based Key Derivation Function 2)

The oldest, simplest. Runs HMAC in a loop:

```
PBKDF2(password, salt, iterations) → key

Internally:
  round 1: HMAC(password, salt || 1)     → h1
  round 2: HMAC(password, h1)            → h2
  round 3: HMAC(password, h2)            → h3
  ...
  round 100000: HMAC(password, h99999)   → h100000
  key = h1 XOR h2 XOR h3 XOR ... XOR h100000
```

Each iteration is cheap individually, but 100,000 of them add up. The problem: GPUs run HMAC very fast in parallel. PBKDF2 is CPU-bound but not memory-bound.

### bcrypt

Designed specifically for password hashing. Uses a modified Blowfish cipher internally. Harder to parallelize on GPUs than PBKDF2, but fixed output size (not a general-purpose KDF).

### Argon2 (the modern choice)

Winner of the 2015 Password Hashing Competition. Three parameters:

```
Argon2(password, salt, time_cost, memory_cost, parallelism) → key

  time_cost:    number of iterations (like PBKDF2)
  memory_cost:  MB of RAM required per computation
  parallelism:  number of threads to use

Example:
  Argon2id("password123", salt, t=3, m=65536, p=4)
  → requires 64MB RAM and 4 threads for ~1 second
```

The memory requirement is what makes Argon2 special. GPUs have limited per-core memory — requiring 64MB per attempt makes GPU attacks 1000x slower.

```
┌──────────────────────────────────────────────────────┐
│  Password KDF Comparison                             │
│                                                      │
│  Algorithm   CPU Cost    Memory Cost    GPU Resistant │
│  ─────────────────────────────────────────────────── │
│  PBKDF2      High        None           No           │
│  bcrypt      High        4KB            Somewhat     │
│  Argon2id    High        Configurable   Yes          │
│                                                      │
│  For new projects: always use Argon2id.              │
│  PBKDF2 is acceptable if Argon2 isn't available.     │
│  bcrypt for password hashing (not key derivation).   │
└──────────────────────────────────────────────────────┘
```

## Salt: preventing rainbow tables

A **salt** is random data added to the password before hashing. Without salt, two users with the same password get the same hash — an attacker can precompute a table of "password → hash" pairs (rainbow table).

```
Without salt:
  hash("password123") → 0xabc...
  hash("password123") → 0xabc...  ← same! rainbow table works

With salt (unique per user):
  hash("password123" + salt_alice) → 0x123...
  hash("password123" + salt_bob)   → 0x789...  ← different!
  Rainbow table is useless — would need a separate table per salt.
```

Salt should be:
- **Random**: 16+ bytes from a CSPRNG
- **Unique**: different for every user / every encryption
- **Stored alongside the hash**: it's not secret, just unique

## Try it yourself

```sh
# Generate a random salt
openssl rand -hex 16

# PBKDF2 with OpenSSL
echo -n "mypassword" | openssl kdf -keylen 32 -kdfopt digest:SHA256 \
  -kdfopt pass:mypassword -kdfopt salt:$(openssl rand -hex 16) \
  -kdfopt iter:100000 PBKDF2 | xxd

# Or use Python to see PBKDF2 in action:
python3 -c "
import hashlib, os, time

password = b'password123'
salt = os.urandom(16)

start = time.time()
key = hashlib.pbkdf2_hmac('sha256', password, salt, 100_000)
elapsed = time.time() - start

print(f'Salt:     {salt.hex()}')
print(f'Key:      {key.hex()}')
print(f'Time:     {elapsed:.3f}s')
print(f'Rate:     {1/elapsed:.0f} attempts/second')
print()
print(f'With 10B passwords to try: {10_000_000_000 * elapsed / 3600:.0f} hours')
"
```

```sh
# Compare with fast hashing (SHA-256):
python3 -c "
import hashlib, time

password = b'password123'
start = time.time()
for _ in range(1_000_000):
    hashlib.sha256(password).digest()
elapsed = time.time() - start

print(f'SHA-256: {1_000_000/elapsed:.0f} hashes/second')
print('That is why you never use plain SHA-256 for passwords.')
"
```

## PBKDF2 in Rust

```rust
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

let password = b"my secret password";
let salt = b"random-salt-here"; // in real code: use rand to generate
let iterations = 100_000;
let mut key = [0u8; 32];

pbkdf2_hmac::<Sha256>(password, salt, iterations, &mut key);
println!("Derived key: {}", hex::encode(key));
```

## Argon2 in Rust

```rust
use argon2::Argon2;

let password = b"my secret password";
let salt = b"random-salt-here";
let mut key = [0u8; 32];

let argon2 = Argon2::default(); // Argon2id, t=3, m=64MB, p=4
argon2.hash_password_into(password, salt, &mut key).unwrap();
println!("Derived key: {}", hex::encode(key));
```

## When to use which

```
Deriving a key from a DH shared secret → HKDF (Lesson 5)
  Fast, the input is already strong.

Deriving a key from a user's password → Argon2id
  Slow on purpose, the input is weak.

Storing a password hash in a database → Argon2id or bcrypt
  You don't need the key, just a hash to verify against.

API key / token derivation → HKDF
  The input (master secret) is already random.
```

## Exercises

### Exercise 1: PBKDF2 key derivation

Add `pbkdf2` and `sha2` to your dependencies. Derive a 32-byte key from a password with 100,000 iterations. Print the key as hex. Verify that the same password + salt always gives the same key.

### Exercise 2: Timing comparison

Time how long PBKDF2 takes with 1,000 vs 10,000 vs 100,000 vs 1,000,000 iterations. Plot the results. How many passwords/second can an attacker try at each level?

### Exercise 3: Argon2 key derivation

Add `argon2` to your dependencies. Derive a key with default parameters. Then try:
- `t_cost = 1, m_cost = 16384` (16MB) — fast
- `t_cost = 3, m_cost = 65536` (64MB) — default
- `t_cost = 10, m_cost = 262144` (256MB) — paranoid

Measure time and observe memory usage with `ps -o rss -p <pid>`.

### Exercise 4: Salt uniqueness

Derive keys from the same password with two different salts. Verify the keys are completely different. Then derive from the same password + same salt twice — keys should match. This proves salt prevents precomputation.

### Exercise 5: Encrypt a file with a password

Combine this lesson with Lesson 2:
1. Ask for a password
2. Generate a random salt
3. Derive a key with Argon2
4. Encrypt a file with ChaCha20-Poly1305 using that key
5. Store `salt || nonce || ciphertext` to disk
6. Decrypt: read salt, ask for password, derive key, decrypt

This is the foundation of the Password Manager Vault project.

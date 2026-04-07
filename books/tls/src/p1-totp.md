# Project: TOTP Authenticator

> **Prerequisites**: Lesson 1 (Hashing), Lesson 5 (HMAC). This project uses HMAC-SHA1 to generate time-based codes.

## What is TOTP?

TOTP is the 6-digit code on your phone that changes every 30 seconds. It's the most common form of two-factor authentication (2FA):

```
┌──────────────────────────────────────────────────────┐
│  Login flow with TOTP                                │
│                                                      │
│  1. You enter username + password                    │
│  2. Server says: "Enter your 2FA code"               │
│  3. You open Google Authenticator                    │
│  4. It shows: 847 293  (changes in 14 seconds)       │
│  5. You type 847293                                  │
│  6. Server verifies → access granted                 │
│                                                      │
│  If attacker steals your password:                   │
│    They still can't log in without the code.         │
│    The code changes every 30 seconds.                │
│    It's derived from a secret only you and the       │
│    server share.                                     │
└──────────────────────────────────────────────────────┘
```

## How TOTP works

The algorithm is simple — just 5 steps:

```
Shared secret (base32)          Current time
"JBSWY3DPEHPK3PXP"             1714000000 (Unix timestamp)
        │                              │
        ▼                              ▼
   Decode base32                floor(time / 30)
   → raw bytes                  → 57133333 (time step)
        │                              │
        └──────────┬───────────────────┘
                   │
                   ▼
        HMAC-SHA1(secret, time_step as u64 big-endian)
                   │
                   ▼
           20-byte HMAC output
                   │
                   ▼
        Dynamic truncation (extract 4 bytes)
                   │
                   ▼
            31-bit integer
                   │
                   ▼
          integer mod 1,000,000
                   │
                   ▼
           6-digit code: 847293
```

### Step 1: The shared secret

When you scan a QR code to set up 2FA, you're receiving a **shared secret** — typically 20 bytes encoded in base32:

```
Base32: JBSWY3DPEHPK3PXP
Bytes:  [0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x21, 0xde, 0xad, 0xbe, 0xef]
```

Both your phone and the server store this secret. It never travels over the network again.

### Step 2: Time step

Divide the current Unix timestamp by 30 (the time window):

```
T = floor(unix_timestamp / 30)

Example:
  timestamp = 1714000000
  T = floor(1714000000 / 30) = 57133333
```

This means the code changes every 30 seconds. Everyone in the same 30-second window computes the same T.

### Step 3: HMAC-SHA1

Compute `HMAC-SHA1(secret, T)` where T is an 8-byte big-endian integer:

```
T = 57133333
T as u64 big-endian = [0x00, 0x00, 0x00, 0x00, 0x03, 0x68, 0xA1, 0x55]

HMAC-SHA1(secret, T_bytes) → 20 bytes
```

### Step 4: Dynamic truncation

Take the last nibble (4 bits) of the HMAC as an offset, then extract 4 bytes starting at that offset:

```rust
let offset = (hmac_result[19] & 0x0F) as usize;
let code = u32::from_be_bytes([
    hmac_result[offset] & 0x7F,  // mask high bit (ensure positive)
    hmac_result[offset + 1],
    hmac_result[offset + 2],
    hmac_result[offset + 3],
]);
```

### Step 5: Modulo

```
code = truncated_value % 1_000_000
// Gives a 6-digit number (zero-padded)
// Example: 847293
```

That's it. The entire TOTP algorithm is about 15 lines of code.

## Try it before coding

```sh
# Install oathtool (TOTP reference implementation):
# macOS:
brew install oath-toolkit
# Linux:
sudo apt install oathtool

# Generate a TOTP code:
oathtool --totp -b "JBSWY3DPEHPK3PXP"
# Output: a 6-digit code (changes every 30 seconds)

# Wait 30 seconds, run again — different code:
sleep 30 && oathtool --totp -b "JBSWY3DPEHPK3PXP"

# Show the code for a specific time (for testing):
oathtool --totp -b "JBSWY3DPEHPK3PXP" --now "2024-04-25 12:00:00 UTC"
```

```sh
# Same thing in Python:
pip3 install pyotp
python3 -c "
import pyotp, time
totp = pyotp.TOTP('JBSWY3DPEHPK3PXP')
code = totp.now()
remaining = 30 - (int(time.time()) % 30)
print(f'Code: {code} (expires in {remaining}s)')
print(f'Valid? {totp.verify(code)}')
"
```

```sh
# See the current time step:
python3 -c "
import time
now = int(time.time())
step = now // 30
remaining = 30 - (now % 30)
print(f'Unix time:  {now}')
print(f'Time step:  {step}')
print(f'Next code in: {remaining}s')
"
```

## The QR code (otpauth:// URI)

When a website shows a QR code for 2FA setup, it encodes a URI:

```
otpauth://totp/MyService:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=MyService&digits=6&period=30
```

```
otpauth://totp/              ← type (TOTP vs HOTP)
MyService:alice@example.com  ← label shown in the app
?secret=JBSWY3DPEHPK3PXP   ← the shared secret (base32)
&issuer=MyService            ← company name
&digits=6                    ← code length (6 or 8)
&period=30                   ← time step in seconds
```

```sh
# Generate a QR code from the URI:
# macOS: brew install qrencode
# Linux: sudo apt install qrencode

qrencode -o totp-qr.png \
  "otpauth://totp/MyApp:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=MyApp"
open totp-qr.png  # macOS
# Scan with Google Authenticator — it will start showing codes!
```

## The RFC (6238)

TOTP is defined in [RFC 6238](https://datatracker.ietf.org/doc/html/rfc6238):

```
Parameter    Default    Description
─────────────────────────────────────────
Algorithm    SHA-1      Hash for HMAC (SHA-1, SHA-256, SHA-512)
Digits       6          Length of code (6 or 8)
Period       30         Seconds per time step
T0           0          Unix epoch start
```

Most services use SHA-1 + 6 digits + 30 seconds.

## Implementation guide

We'll build this step by step. At each step, you can compile and test before moving on.

### Step 0: Project setup

Create the binary and add dependencies:

```sh
# If adding to the tls crate, create the file:
touch tls/src/bin/p1-totp.rs
```

Add to `tls/Cargo.toml`:

```toml
[dependencies]
hmac = "0.12"
sha1 = "0.10"
data-encoding = "2"  # for base32 decoding
clap = { version = "4", features = ["derive"] }
```

Start with a skeleton:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a TOTP code
    Generate { secret: String },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate { secret } => {
            println!("TODO: generate TOTP for {}", secret);
        }
    }
}
```

```sh
cargo run -p tls --bin p1-totp -- generate JBSWY3DPEHPK3PXP
# Should print the TODO message
```

### Step 1: Decode the base32 secret

The shared secret comes as a base32 string. Decode it to raw bytes:

```rust
fn decode_secret(secret_base32: &str) -> Vec<u8> {
    data_encoding::BASE32
        .decode(secret_base32.as_bytes())
        .expect("invalid base32 secret")
}
```

Test it:

```rust
fn main() {
    let secret = decode_secret("JBSWY3DPEHPK3PXP");
    println!("Secret bytes: {:?}", secret);
    println!("Length: {} bytes", secret.len());
    // Should be 10 bytes: [72, 101, 108, 108, 111, 33, 222, 173, 190, 175]
}
```

```sh
# Verify with Python:
python3 -c "import base64; print(list(base64.b32decode('JBSWY3DPEHPK3PXP')))"
# [72, 101, 108, 108, 111, 33, 222, 173, 190, 175]
```

### Step 2: Compute the time step

Get the current Unix timestamp and divide by 30:

```rust
fn current_time_step() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() / 30
}
```

Test it:

```rust
fn main() {
    let step = current_time_step();
    let remaining = 30 - (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() % 30);
    println!("Time step: {}", step);
    println!("Next code in: {}s", remaining);
}
```

```sh
# Compare with Python:
python3 -c "import time; print(int(time.time()) // 30)"
# Should match your Rust output
```

### Step 3: HMAC-SHA1

Compute the HMAC using the secret and time step (as big-endian u64):

```rust
use hmac::{Hmac, Mac};
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

fn hmac_sha1(secret: &[u8], time_step: u64) -> [u8; 20] {
    let mut mac = HmacSha1::new_from_slice(secret)
        .expect("HMAC accepts any key length");
    mac.update(&time_step.to_be_bytes());  // 8 bytes, big-endian
    let result = mac.finalize().into_bytes();

    let mut output = [0u8; 20];
    output.copy_from_slice(&result);
    output
}
```

Test it:

```rust
fn main() {
    let secret = decode_secret("JBSWY3DPEHPK3PXP");
    let step = current_time_step();
    let hmac = hmac_sha1(&secret, step);
    println!("HMAC: {}", hex::encode(hmac));  // add `hex` dep, or use {:02x} formatting
    println!("Length: {} bytes (always 20 for SHA-1)", hmac.len());
}
```

Why big-endian? The RFC specifies it. If you use little-endian, your codes won't match any other TOTP implementation.

### Step 4: Dynamic truncation

This is the clever part — extract a 4-byte chunk from the HMAC at a position determined by the last byte:

```rust
fn truncate(hmac_result: &[u8; 20]) -> u32 {
    // The last nibble (4 bits) determines the offset
    let offset = (hmac_result[19] & 0x0F) as usize;
    // offset is 0-15, and we read 4 bytes, so max index is 15+3=18 (within 20)

    // Extract 4 bytes at that offset, mask the high bit
    u32::from_be_bytes([
        hmac_result[offset] & 0x7F,  // & 0x7F clears the sign bit
        hmac_result[offset + 1],
        hmac_result[offset + 2],
        hmac_result[offset + 3],
    ])
}
```

Test it:

```rust
fn main() {
    let secret = decode_secret("JBSWY3DPEHPK3PXP");
    let step = current_time_step();
    let hmac = hmac_sha1(&secret, step);

    let offset = (hmac[19] & 0x0F) as usize;
    println!("Last byte: 0x{:02x}", hmac[19]);
    println!("Offset: {} (last nibble)", offset);

    let truncated = truncate(&hmac);
    println!("Truncated: {} (31-bit integer)", truncated);
}
```

Why `& 0x7F`? To ensure the result is positive (clear the sign bit). The RFC requires a 31-bit unsigned value.

### Step 5: Modulo → 6-digit code

```rust
fn generate_totp(secret_base32: &str, time_step: u64) -> u32 {
    let secret = decode_secret(secret_base32);
    let hmac = hmac_sha1(&secret, time_step);
    let truncated = truncate(&hmac);
    truncated % 1_000_000  // 6 digits
}
```

Test it against the reference:

```rust
fn main() {
    let code = generate_totp("JBSWY3DPEHPK3PXP", current_time_step());
    println!("Code: {:06}", code);  // zero-pad to 6 digits
}
```

```sh
# Compare:
cargo run -p tls --bin p1-totp -- generate JBSWY3DPEHPK3PXP
oathtool --totp -b "JBSWY3DPEHPK3PXP"
# MUST be identical!
```

If they don't match, check:
1. Is the base32 decoding correct? (Step 1)
2. Is the time step the same? (Step 2 — clocks might differ by a second across the boundary)
3. Is the HMAC input big-endian? (Step 3)

### Step 6: Wrap it in a nice CLI

```rust
fn totp_now(secret_base32: &str) -> u32 {
    generate_totp(secret_base32, current_time_step())
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate { secret } => {
            let code = totp_now(&secret);
            let remaining = 30 - (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() % 30);
            println!("{:06}  (expires in {}s)", code, remaining);
        }
    }
}
```

### Step 7: Validation

Accept current window + previous (for clock skew):

```rust
fn verify_totp(secret: &str, code: u32) -> bool {
    let current_step = current_time_step();
    // Check current and previous window
    for step in [current_step, current_step - 1] {
        if generate_totp(secret, step) == code {
            return true;
        }
    }
    false
}
```

Add to CLI:

```rust
Command::Verify { secret, code } => {
    if verify_totp(&secret, code) {
        println!("Valid!");
    } else {
        println!("Invalid!");
    }
}
```

Test:

```sh
# Get current code:
CODE=$(oathtool --totp -b "JBSWY3DPEHPK3PXP")
echo "Code: $CODE"

# Verify immediately:
cargo run -p tls --bin p1-totp -- verify JBSWY3DPEHPK3PXP $CODE
# Valid!

# Wait 60 seconds (two windows), verify again:
sleep 60
cargo run -p tls --bin p1-totp -- verify JBSWY3DPEHPK3PXP $CODE
# Invalid! (code has expired beyond the ±1 window)
```

You now have a working TOTP authenticator. The exercises below extend it further.

## Exercises

### Exercise 1: Generate and verify

Implement `generate_totp` and `totp_now`. Verify your output matches `oathtool`:

```sh
# Your program:
cargo run -p tls --bin p1-totp -- generate JBSWY3DPEHPK3PXP

# Reference:
oathtool --totp -b "JBSWY3DPEHPK3PXP"

# Both should show the same 6-digit code.
```

### Exercise 2: Live display with countdown

Build a CLI that shows the current code with a countdown timer:

```
$ cargo run -p tls --bin p1-totp -- watch JBSWY3DPEHPK3PXP

  Code: 847293  [████████░░░░░░] 14s remaining
```

Hint: use `\r` (carriage return) to overwrite the line. `print!("\r  Code: {:06}  [{}>{}] {}s remaining", code, "█".repeat(filled), "░".repeat(30-filled), remaining)`.

### Exercise 3: Validate a code

Implement `verify_totp` with a ±1 window. Test:

```sh
cargo run -p tls --bin p1-totp -- verify JBSWY3DPEHPK3PXP 847293
# Valid! (if the code is current)

cargo run -p tls --bin p1-totp -- verify JBSWY3DPEHPK3PXP 000000
# Invalid!
```

### Exercise 4: SHA-256 and SHA-512 variants

Extend to support different hash algorithms:

```sh
cargo run -p tls --bin p1-totp -- generate --algo sha256 JBSWY3DPEHPK3PXP
```

Compare: `oathtool --totp=sha256 -b "JBSWY3DPEHPK3PXP"`

### Exercise 5: Generate QR code

Generate an `otpauth://` URI and render as a QR code in the terminal (using the `qrcode` crate):

```sh
cargo run -p tls --bin p1-totp -- setup --issuer MyApp --account alice@example.com
# Displays QR code in terminal
# Scan with Google Authenticator
# Verify: codes from your CLI match codes in the app
```

This is the full setup flow — you've built a Google Authenticator clone.

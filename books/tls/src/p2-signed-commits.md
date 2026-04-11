# Project: Signed Git Commits

> **Prerequisites**: Lesson 3 (Ed25519 Signatures). This project applies signing/verification to a real workflow.

## What are digital signatures used for?

Digital signatures are everywhere — you interact with them daily without realizing:

```
┌──────────────────────────────────────────────────────────────┐
│  Where signatures are used in real life                      │
│                                                              │
│  Software updates:                                           │
│    Your phone checks: "Is this update really from Apple?"    │
│    → Apple signed it with their private key                  │
│    → Your phone verifies with Apple's public key             │
│    → Without this, malware could pretend to be an update     │
│                                                              │
│  Package managers (apt, cargo, npm):                         │
│    cargo install ripgrep                                     │
│    → crates.io signs the package metadata                    │
│    → cargo verifies before installing                        │
│    → Without this, a compromised mirror could serve malware  │
│                                                              │
│  Git commits:                                                │
│    git commit -S -m "release v2.0"                           │
│    → Your Ed25519 key signs the commit                       │
│    → GitHub shows a green "Verified" badge                   │
│    → Without this, anyone can forge a commit as "you"        │
│                                                              │
│  PDF / legal documents:                                      │
│    Sign a contract digitally                                 │
│    → Your key proves you agreed to it                        │
│    → The document can't be modified after signing            │
│                                                              │
│  HTTPS certificates (Lesson 7):                              │
│    Server proves its identity during TLS handshake           │
│    → CA signed the server's certificate                      │
│    → Browser verifies the chain                              │
└──────────────────────────────────────────────────────────────┘
```

## What you're building

A CLI tool that signs files with Ed25519 and produces detached signatures — the same concept behind `git commit -S`, `ssh-keygen -Y sign`, and package signing.

```sh
# Sign a file:
cargo run -p tls --bin p2-sign -- sign --key my.key document.txt
# Created document.txt.sig

# Verify it:
cargo run -p tls --bin p2-sign -- verify --pubkey my.pub document.txt document.txt.sig
# Signature valid ✓

# Tamper with the file:
echo "extra" >> document.txt
cargo run -p tls --bin p2-sign -- verify --pubkey my.pub document.txt document.txt.sig
# Signature INVALID ✗
```

## How git signing works

Git commits are just text objects. Anyone with write access can create a commit claiming to be "Linus Torvalds":

```sh
# This is trivially easy — no verification:
git -c user.name="Linus Torvalds" -c user.email="torvalds@linux-foundation.org" \
  commit --allow-empty -m "I am definitely Linus"

git log -1
# Author: Linus Torvalds <torvalds@linux-foundation.org>   ← fake!
```

Signed commits fix this:

```
┌──────────────────────────────────────────────────────┐
│  Normal git commit:                                  │
│    commit message + tree hash + author + timestamp   │
│    → stored as a git object                          │
│    → ANYONE can forge the author field               │
│                                                      │
│  Signed git commit (git commit -S):                  │
│    same data + Ed25519 signature                     │
│    → signature proves the author has the private key │
│    → GitHub shows "Verified" badge                   │
│    → forging would require stealing the private key  │
└──────────────────────────────────────────────────────┘
```

## Try it with existing tools first

Before building our own, let's see how the real tools work:

```sh
# === SSH signatures (the modern approach) ===

# Generate a key if you don't have one:
ssh-keygen -t ed25519 -f sign_key -N ""
# Creates sign_key (private) and sign_key.pub (public)

# Sign a file:
echo "important document" > doc.txt
ssh-keygen -Y sign -f sign_key -n file doc.txt
# Creates doc.txt.sig

# Look at the signature:
cat doc.txt.sig
# -----BEGIN SSH SIGNATURE-----
# U1NIU0lHAAAAAQA... (base64-encoded)
# -----END SSH SIGNATURE-----

# To verify, SSH needs an "allowed signers" file (like a trust store):
echo "user@example.com $(cat sign_key.pub)" > allowed_signers

# Verify:
ssh-keygen -Y verify -f allowed_signers -I user@example.com -n file -s doc.txt.sig < doc.txt
# Good "file" signature for user@example.com

# Tamper and verify again:
echo "tampered" >> doc.txt
ssh-keygen -Y verify -f allowed_signers -I user@example.com -n file -s doc.txt.sig < doc.txt
# Could not verify signature — FAILED!
```

```sh
# === Set up git commit signing ===

# Tell git to use SSH for signing:
git config --global gpg.format ssh
git config --global user.signingkey ~/.ssh/id_ed25519.pub

# Sign a commit:
git commit -S -m "this commit is signed"

# Verify:
git log --show-signature -1
# Good "git" signature for user@example.com

# On GitHub: push the commit → see the green "Verified" badge
# (You need to upload your public key to GitHub → Settings → SSH and GPG keys)
```

```sh
# === OpenSSL signatures ===

openssl genpkey -algorithm Ed25519 -out sign.key
openssl pkey -in sign.key -pubout -out sign.pub

echo "document content" > doc.txt
openssl pkeyutl -sign -inkey sign.key -in doc.txt -out doc.sig
openssl pkeyutl -verify -pubin -inkey sign.pub -in doc.txt -sigfile doc.sig
# Signature Verified Successfully

# The .sig file is raw bytes (64 bytes for Ed25519):
wc -c doc.sig
# 64 doc.sig

xxd doc.sig | head -3
# Raw signature bytes — not human-readable
```

## The signing flow

```
Sign:
  ┌────────────┐     ┌──────────────┐     ┌────────────┐
  │ file bytes │ ──► │ Ed25519 sign │ ──► │ .sig file  │
  │            │     │ (private key)│     │ (64 bytes) │
  └────────────┘     └──────────────┘     └────────────┘

  The signature is "detached" — it's a separate file.
  The original file is NOT modified.

Verify:
  ┌────────────┐
  │ file bytes │──┐
  └────────────┘  │   ┌──────────────┐
                  ├──►│Ed25519 verify│──► ✓ or ✗
  ┌────────────┐  │   │ (public key) │
  │ .sig file  │──┘   └──────────────┘
  └────────────┘

  You need: the file, the signature, AND the public key.
  If ANY of the three is wrong, verification fails.
```

## Implementation guide

### Step 0: Project setup

```sh
touch tls/src/bin/p2-sign.rs
```

Dependencies (already in `tls/Cargo.toml`):

```toml
ed25519-dalek = { version = "2", features = ["rand_core"] }
rand_core = { version = "0.6", features = ["getrandom"] }
clap = { version = "4", features = ["derive"] }
hex = "0.4"
```

Start with a CLI skeleton:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sign-tool", about = "Sign and verify files with Ed25519")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a new Ed25519 key pair
    Keygen { key_path: String },
    /// Sign a file
    Sign {
        #[arg(long)]
        key: String,
        file: String,
    },
    /// Verify a file's signature
    Verify {
        #[arg(long)]
        pubkey: String,
        file: String,
        signature: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Keygen { key_path } => todo!(),
        Command::Sign { key, file } => todo!(),
        Command::Verify { pubkey, file, signature } => todo!(),
    }
}
```

```sh
cargo run -p tls --bin p2-sign -- --help
# Should show the three subcommands
```

### Step 1: Key generation

Generate an Ed25519 key pair and save to disk:

```rust
use ed25519_dalek::SigningKey;

fn generate_keypair(key_path: &str) {
    let signing_key = SigningKey::generate(&mut rand_core::OsRng);
    let public_key = signing_key.verifying_key();

    // Save private key (raw 32 bytes)
    std::fs::write(key_path, signing_key.to_bytes()).unwrap();
    // Save public key (raw 32 bytes)
    std::fs::write(format!("{key_path}.pub"), public_key.to_bytes()).unwrap();

    println!("Private key saved to: {key_path}");
    println!("Public key saved to:  {key_path}.pub");
    println!("Public key (hex):     {}", hex::encode(public_key.to_bytes()));
}
```

Test it:

```sh
cargo run -p tls --bin p2-sign -- keygen my.key
# Private key saved to: my.key
# Public key saved to:  my.key.pub
# Public key (hex):     a1b2c3d4...

# Verify the files exist and have the right size:
ls -la my.key my.key.pub
# my.key     32 bytes (private key)
# my.key.pub 32 bytes (public key)
```

**Security note**: in a real tool, you'd encrypt the private key with a password (like SSH does). Here we store it raw for simplicity.

### Step 2: Signing a file

Read the file, sign its contents, write the signature to a `.sig` file:

```rust
use ed25519_dalek::{SigningKey, Signer};

fn sign_file(key_path: &str, file_path: &str) {
    // Load private key
    let key_bytes: [u8; 32] = std::fs::read(key_path)
        .expect("can't read key file")
        .try_into()
        .expect("key file must be exactly 32 bytes");
    let signing_key = SigningKey::from_bytes(&key_bytes);

    // Read the file to sign
    let file_data = std::fs::read(file_path)
        .expect("can't read file to sign");

    // Sign
    let signature = signing_key.sign(&file_data);

    // Write signature
    let sig_path = format!("{file_path}.sig");
    std::fs::write(&sig_path, signature.to_bytes()).unwrap();

    println!("Signed: {file_path}");
    println!("Signature: {sig_path} ({} bytes)", signature.to_bytes().len());
    println!("Signature (hex): {}", hex::encode(signature.to_bytes()));
}
```

Test it:

```sh
# Create a test file:
echo "Hello, this is an important document." > doc.txt

# Sign it:
cargo run -p tls --bin p2-sign -- sign --key my.key doc.txt
# Signed: doc.txt
# Signature: doc.txt.sig (64 bytes)

# Look at the signature:
xxd doc.txt.sig | head -3
# Raw bytes — 64 bytes of Ed25519 signature

# Compare with OpenSSL:
openssl genpkey -algorithm Ed25519 -out openssl.key
openssl pkeyutl -sign -inkey openssl.key -in doc.txt -out doc.openssl.sig
wc -c doc.txt.sig doc.openssl.sig
# Both are 64 bytes — same algorithm, same output size
```

### Step 3: Verification

Read the file, the signature, and the public key. Verify:

```rust
use ed25519_dalek::{VerifyingKey, Verifier, Signature};

fn verify_file(pubkey_path: &str, file_path: &str, sig_path: &str) {
    // Load public key
    let pub_bytes: [u8; 32] = std::fs::read(pubkey_path)
        .expect("can't read public key")
        .try_into()
        .expect("public key must be exactly 32 bytes");
    let verifying_key = VerifyingKey::from_bytes(&pub_bytes)
        .expect("invalid public key");

    // Read the file
    let file_data = std::fs::read(file_path)
        .expect("can't read file");

    // Read the signature
    let sig_bytes: [u8; 64] = std::fs::read(sig_path)
        .expect("can't read signature file")
        .try_into()
        .expect("signature must be exactly 64 bytes");
    let signature = Signature::from_bytes(&sig_bytes);

    // Verify
    match verifying_key.verify_strict(&file_data, &signature) {
        Ok(()) => {
            println!("✓ Signature valid");
            println!("  File: {file_path}");
            println!("  Signed by: {}", hex::encode(pub_bytes));
        }
        Err(e) => {
            println!("✗ Signature INVALID");
            println!("  File: {file_path}");
            println!("  Error: {e}");
            std::process::exit(1);
        }
    }
}
```

Test it — the moment of truth:

```sh
# Verify the good signature:
cargo run -p tls --bin p2-sign -- verify --pubkey my.key.pub doc.txt doc.txt.sig
# ✓ Signature valid

# Now tamper with the file:
echo " sneaky modification" >> doc.txt
cargo run -p tls --bin p2-sign -- verify --pubkey my.key.pub doc.txt doc.txt.sig
# ✗ Signature INVALID

# Restore and verify again:
echo "Hello, this is an important document." > doc.txt
cargo run -p tls --bin p2-sign -- verify --pubkey my.key.pub doc.txt doc.txt.sig
# ✓ Signature valid

# Try with the WRONG public key:
cargo run -p tls --bin p2-sign -- keygen other.key
cargo run -p tls --bin p2-sign -- verify --pubkey other.key.pub doc.txt doc.txt.sig
# ✗ Signature INVALID — wrong key, even though file is untouched
```

Three things must match: the **file**, the **signature**, and the **public key**. Change any one → verification fails.

### Step 4: Put it all together

Wire the functions into the CLI match arms:

```rust
fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Keygen { key_path } => generate_keypair(&key_path),
        Command::Sign { key, file } => sign_file(&key, &file),
        Command::Verify { pubkey, file, signature } => verify_file(&pubkey, &file, &signature),
    }
}
```

The complete tool in ~60 lines of logic. That's the beauty of Ed25519 — tiny keys, tiny signatures, simple API.

## Real-world scenario: software release

Let's walk through a complete release signing workflow:

```sh
# === You are the developer ===

# 1. Generate your release signing key (do this ONCE, keep it safe):
cargo run -p tls --bin p2-sign -- keygen release.key
# Public key: a1b2c3d4...  ← publish this on your website

# 2. Build your software:
cargo build --release
cp target/release/my-app ./my-app-v2.0

# 3. Sign the release:
cargo run -p tls --bin p2-sign -- sign --key release.key my-app-v2.0
# Creates my-app-v2.0.sig

# 4. Upload both to your download page:
#    my-app-v2.0     (the binary)
#    my-app-v2.0.sig (the signature)
#    release.key.pub (your public key — on your website)

# === A user downloads your software ===

# 5. User downloads the binary + signature + your public key
# 6. User verifies:
cargo run -p tls --bin p2-sign -- verify \
  --pubkey release.key.pub my-app-v2.0 my-app-v2.0.sig
# ✓ Signature valid — safe to install!

# === An attacker compromises the download mirror ===

# 7. Attacker replaces my-app-v2.0 with malware
# 8. User verifies:
cargo run -p tls --bin p2-sign -- verify \
  --pubkey release.key.pub my-app-v2.0 my-app-v2.0.sig
# ✗ Signature INVALID — don't install!
```

## Exercises

### Exercise 1: Sign and verify CLI

Build the complete CLI as described above. Test all three failure cases:
1. Tampered file → invalid
2. Wrong public key → invalid
3. Correct file + correct key → valid

### Exercise 2: Sign multiple files (manifest)

Sign an entire directory. Create a manifest that maps filenames to signatures:

```sh
cargo run -p tls --bin p2-sign -- sign-dir --key my.key ./release/
# Signed 3 files:
#   README.md   → README.md.sig
#   main.rs     → main.rs.sig
#   Cargo.toml  → Cargo.toml.sig
# Manifest written to: ./release/MANIFEST.sig

cargo run -p tls --bin p2-sign -- verify-dir --pubkey my.key.pub ./release/
# ✓ README.md   — valid
# ✓ main.rs     — valid
# ✗ Cargo.toml  — INVALID (was modified!)
```

### Exercise 3: Timestamped signatures

Include the current timestamp in the signed data: `sign(key, timestamp_bytes || file_data)`. The signature covers both the time and the content.

**The `.sig` file contains both the timestamp and the signature** — bundled together so they can't be tampered with independently:

```
┌──────────────────────────────────┐
│  .sig file layout (72 bytes):   │
│                                  │
│  bytes 0-7:   timestamp          │  u64 big-endian, Unix seconds
│  bytes 8-71:  Ed25519 signature  │  64 bytes
│                                  │
│  The signature covers:           │
│    timestamp_bytes || file_bytes │
│                                  │
│  Why not a separate timestamp    │
│  file? Because an attacker could │
│  swap the timestamp while keeping│
│  the signature — bundling them   │
│  means both are protected.       │
└──────────────────────────────────┘
```

Signing:

```rust
fn sign_with_timestamp(key: &SigningKey, file_data: &[u8]) -> Vec<u8> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Sign: timestamp || file_data
    let mut signed_data = Vec::new();
    signed_data.extend_from_slice(&timestamp.to_be_bytes());
    signed_data.extend_from_slice(file_data);
    let signature = key.sign(&signed_data);

    // .sig file: timestamp (8 bytes) + signature (64 bytes)
    let mut sig_file = Vec::new();
    sig_file.extend_from_slice(&timestamp.to_be_bytes());
    sig_file.extend_from_slice(&signature.to_bytes());
    sig_file  // 72 bytes total
}
```

Verification:

```rust
fn verify_with_timestamp(pubkey: &VerifyingKey, file_data: &[u8],
                          sig_file: &[u8], max_age_secs: u64) -> Result<(), String> {
    // Extract timestamp + signature from .sig file
    let timestamp = u64::from_be_bytes(sig_file[..8].try_into().unwrap());
    let signature = Signature::from_bytes(&sig_file[8..72].try_into().unwrap());

    // Reconstruct signed data
    let mut signed_data = Vec::new();
    signed_data.extend_from_slice(&timestamp.to_be_bytes());
    signed_data.extend_from_slice(file_data);

    // Verify signature
    pubkey.verify_strict(&signed_data, &signature)
        .map_err(|_| "signature invalid".to_string())?;

    // Check timestamp
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if timestamp > now { return Err("timestamp is in the future".into()); }
    if now - timestamp > max_age_secs { return Err(format!("signature expired ({} seconds old)", now - timestamp)); }

    Ok(())
}
```

Test:

```sh
cargo run -p tls --bin p2-sign -- sign --key my.key --timestamp doc.txt
# Signed at: 2026-04-11 10:30:00 UTC

# Verify immediately:
cargo run -p tls --bin p2-sign -- verify --pubkey my.key.pub --max-age 24h doc.txt doc.txt.sig
# ✓ Signature valid (signed 2 minutes ago)

# The .sig file is now 72 bytes (8 timestamp + 64 signature):
wc -c doc.txt.sig
# 72

# Wait 25 hours...
cargo run -p tls --bin p2-sign -- verify --pubkey my.key.pub --max-age 24h doc.txt doc.txt.sig
# ✗ Signature expired (signed 25 hours ago)
```

### Exercise 4: Cross-verify with ssh-keygen

This is advanced: make your `.sig` file format compatible with SSH signatures so `ssh-keygen -Y verify` can check your Rust-generated signatures. Read the [SSH signature format spec](https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig) — it wraps the raw Ed25519 signature in a structured envelope with namespace and hash algorithm fields.

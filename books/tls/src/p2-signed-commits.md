# Project: Signed Git Commits

> **Prerequisites**: Lesson 3 (Ed25519 Signatures). This project applies signing/verification to a real workflow.

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

```
┌──────────────────────────────────────────────────────┐
│  Normal git commit:                                  │
│    commit message + tree hash + author + timestamp   │
│    → stored as a git object                          │
│    → anyone can forge a commit claiming to be you    │
│                                                      │
│  Signed git commit (git commit -S):                  │
│    same data + Ed25519 signature                     │
│    → signature proves the commit author              │
│    → GitHub shows "Verified" badge                   │
└──────────────────────────────────────────────────────┘
```

```sh
# See a signed commit on your machine:
git log --show-signature -1 2>/dev/null || echo "No signed commits found"

# See signed commits on GitHub:
# Look for the green "Verified" badge on any commit

# Sign a commit yourself:
# git config --global user.signingkey ~/.ssh/id_ed25519.pub
# git config --global gpg.format ssh
# git commit -S -m "signed commit"
```

## The signing flow

```
Sign:
  ┌────────────┐     ┌──────────────┐     ┌────────────┐
  │ file bytes │ ──► │ Ed25519 sign │ ──► │ .sig file  │
  │            │     │ (private key)│     │ (64 bytes) │
  └────────────┘     └──────────────┘     └────────────┘

Verify:
  ┌────────────┐     ┌──────────────┐
  │ file bytes │ ──► │Ed25519 verify│ ──► ✓ or ✗
  │ + .sig     │     │ (public key) │
  └────────────┘     └──────────────┘
```

## Try it with existing tools

```sh
# SSH can sign and verify files (since OpenSSH 8.0):

# Generate a key if you don't have one:
ssh-keygen -t ed25519 -f sign_key -N ""

# Sign a file:
echo "important document" > doc.txt
ssh-keygen -Y sign -f sign_key -n file doc.txt
# Creates doc.txt.sig

# Create an allowed signers file:
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
# With OpenSSL:
openssl genpkey -algorithm Ed25519 -out sign.key
openssl pkey -in sign.key -pubout -out sign.pub

echo "document content" > doc.txt
openssl pkeyutl -sign -inkey sign.key -in doc.txt -out doc.sig
openssl pkeyutl -verify -pubin -inkey sign.pub -in doc.txt -sigfile doc.sig
# Signature Verified Successfully
```

## Implementation guide

### Step 1: Key generation

```rust
use ed25519_dalek::SigningKey;

fn generate_keypair(key_path: &str) {
    let signing_key = SigningKey::generate(&mut rand_core::OsRng);
    let public_key = signing_key.verifying_key();

    // Save private key (raw 32 bytes)
    std::fs::write(key_path, signing_key.to_bytes()).unwrap();
    // Save public key
    std::fs::write(format!("{key_path}.pub"), public_key.to_bytes()).unwrap();

    println!("Public key: {}", hex::encode(public_key.to_bytes()));
}
```

### Step 2: Signing

```rust
use ed25519_dalek::{SigningKey, Signer};

fn sign_file(key_path: &str, file_path: &str) {
    let key_bytes: [u8; 32] = std::fs::read(key_path).unwrap().try_into().unwrap();
    let signing_key = SigningKey::from_bytes(&key_bytes);

    let file_data = std::fs::read(file_path).unwrap();
    let signature = signing_key.sign(&file_data);

    let sig_path = format!("{file_path}.sig");
    std::fs::write(&sig_path, signature.to_bytes()).unwrap();
    println!("Signature written to {sig_path}");
}
```

### Step 3: Verification

```rust
use ed25519_dalek::{VerifyingKey, Verifier, Signature};

fn verify_file(pubkey_path: &str, file_path: &str, sig_path: &str) {
    let pub_bytes: [u8; 32] = std::fs::read(pubkey_path).unwrap().try_into().unwrap();
    let verifying_key = VerifyingKey::from_bytes(&pub_bytes).unwrap();

    let file_data = std::fs::read(file_path).unwrap();
    let sig_bytes: [u8; 64] = std::fs::read(sig_path).unwrap().try_into().unwrap();
    let signature = Signature::from_bytes(&sig_bytes);

    match verifying_key.verify_strict(&file_data, &signature) {
        Ok(()) => println!("Signature valid ✓"),
        Err(e) => println!("Signature INVALID ✗ ({e})"),
    }
}
```

## Exercises

### Exercise 1: Sign and verify CLI

Build the full CLI with clap:
```sh
cargo run -p tls --bin p2-sign -- keygen my.key
cargo run -p tls --bin p2-sign -- sign --key my.key document.txt
cargo run -p tls --bin p2-sign -- verify --pubkey my.key.pub document.txt document.txt.sig
```

### Exercise 2: Cross-verify with ssh-keygen

Sign a file with your Rust tool. Verify it with `ssh-keygen -Y verify`. This requires matching the SSH signature format — read the SSH signature spec to format the `.sig` file correctly.

### Exercise 3: Sign multiple files

Sign an entire directory of files. Output a manifest: `filename → signature`:

```
manifest.sig:
  README.md   a1b2c3d4...
  main.rs     e5f6a7b8...
  Cargo.toml  c9d0e1f2...
```

Verify all files at once. If any file was modified, report which one.

### Exercise 4: Timestamped signatures

Include the current timestamp in the signed data: `sign(key, timestamp || file_data)`. On verification, check that the timestamp is within a reasonable window (e.g., last 24 hours). This prevents using old signatures on new data.

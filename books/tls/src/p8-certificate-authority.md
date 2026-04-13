# Project: Certificate Authority

> **Prerequisites**: Lesson 7 (Certificates), Lesson 8 (Certificate Generation with rcgen).

## What is this?

A Certificate Authority (CA) issues certificates. Let's Encrypt is a public CA — it issues certs for the whole internet. But companies also run **private CAs** for internal services: microservices, databases, CI servers, dev environments.

You're building a private CA as a CLI tool:

```sh
# Initialize the CA (do once):
cargo run -p tls --bin p8-ca -- init --name "My Company CA"
# Created: ca.key (KEEP SECRET), ca.crt (distribute to clients)

# Issue a server certificate:
cargo run -p tls --bin p8-ca -- issue --domain api.internal.com --days 90
# Created: api.internal.com.key, api.internal.com.crt
# Signed by: My Company CA
# Expires: 2026-07-13

# Issue another:
cargo run -p tls --bin p8-ca -- issue --domain db.internal.com --days 90

# List all issued certs:
cargo run -p tls --bin p8-ca -- list
# api.internal.com   expires 2026-07-13  VALID
# db.internal.com    expires 2026-07-13  VALID

# Verify a cert:
openssl verify -CAfile ca.crt api.internal.com.crt
# api.internal.com.crt: OK

# Revoke a cert:
cargo run -p tls --bin p8-ca -- revoke api.internal.com
# Revoked.
```

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│  Your CA                                                   │
│                                                            │
│  Files:                                                    │
│    ca.key           ← CA private key (PROTECT THIS!)       │
│    ca.crt           ← CA certificate (distribute to all)   │
│    ca-db.json       ← database of issued certs             │
│    certs/           ← directory of issued certs + keys     │
│                                                            │
│  Commands:                                                 │
│    init             → create CA key pair + self-signed cert │
│    issue --domain   → generate key + cert, sign with CA    │
│    list             → show all issued certs + status        │
│    revoke --domain  → mark a cert as revoked               │
│    verify --cert    → check if a cert is valid + not revoked│
└────────────────────────────────────────────────────────────┘
```

## Implementation guide

### Step 0: Project setup

```sh
mkdir -p tls/src/bin
touch tls/src/bin/p8-ca.rs
```

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mini-ca", about = "Private Certificate Authority")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new CA
    Init {
        #[arg(long, default_value = "My CA")]
        name: String,
    },
    /// Issue a server certificate
    Issue {
        #[arg(long)]
        domain: String,
        #[arg(long, default_value = "90")]
        days: u32,
    },
    /// List all issued certificates
    List,
    /// Revoke a certificate
    Revoke {
        #[arg(long)]
        domain: String,
    },
}
```

### Step 1: Initialize the CA

Generate a self-signed CA certificate using rcgen (Lesson 8):

```rust
use rcgen::{CertificateParams, IsCa, BasicConstraints, KeyPair};

fn init_ca(name: &str) {
    let mut params = CertificateParams::new(vec![name.into()]).unwrap();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);

    let ca_key = KeyPair::generate().unwrap();
    let ca_cert = params.self_signed(&ca_key).unwrap();

    std::fs::write("ca.key", ca_key.serialize_pem()).unwrap();
    std::fs::write("ca.crt", ca_cert.pem()).unwrap();
    std::fs::write("ca-db.json", "[]").unwrap();
    std::fs::create_dir_all("certs").ok();

    println!("CA initialized: {name}");
    println!("  ca.key — KEEP THIS SECRET");
    println!("  ca.crt — distribute to all clients");
}
```

Test:

```sh
cargo run -p tls --bin p8-ca -- init --name "Acme Corp CA"
ls -la ca.key ca.crt ca-db.json
openssl x509 -in ca.crt -text -noout | head -15
# Issuer: CN = Acme Corp CA
# Subject: CN = Acme Corp CA  (same = self-signed)
# X509v3 Basic Constraints: critical
#     CA:TRUE
```

### Step 2: Issue a server certificate

Load the CA key + cert, generate a new server key + cert, sign it:

```rust
fn issue_cert(domain: &str, days: u32) {
    // Load CA
    let ca_key_pem = std::fs::read_to_string("ca.key").expect("Run 'init' first");
    let ca_cert_pem = std::fs::read_to_string("ca.crt").unwrap();
    let ca_key = KeyPair::from_pem(&ca_key_pem).unwrap();
    let ca_cert_params = CertificateParams::from_ca_cert_pem(&ca_cert_pem).unwrap();
    let ca_cert = ca_cert_params.self_signed(&ca_key).unwrap();

    // Generate server cert
    let mut params = CertificateParams::new(vec![domain.into()]).unwrap();
    params.is_ca = IsCa::NoCa;
    // Set validity (rcgen uses time crate)

    let server_key = KeyPair::generate().unwrap();
    let server_cert = params.signed_by(&server_key, &ca_cert, &ca_key).unwrap();

    // Save
    let key_path = format!("certs/{domain}.key");
    let cert_path = format!("certs/{domain}.crt");
    std::fs::write(&key_path, server_key.serialize_pem()).unwrap();
    std::fs::write(&cert_path, server_cert.pem()).unwrap();

    // Update database
    // ... append to ca-db.json ...

    println!("Issued: {domain}");
    println!("  Key:  {key_path}");
    println!("  Cert: {cert_path}");
}
```

Test:

```sh
cargo run -p tls --bin p8-ca -- issue --domain api.internal.com
openssl verify -CAfile ca.crt certs/api.internal.com.crt
# api.internal.com.crt: OK

openssl x509 -in certs/api.internal.com.crt -text -noout | head -15
# Issuer: CN = Acme Corp CA  ← signed by your CA
# Subject: CN = api.internal.com
```

### Step 3: Certificate database

Track all issued certs in a JSON file:

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct CertRecord {
    domain: String,
    issued: String,   // ISO date
    expires: String,  // ISO date
    serial: u64,
    revoked: bool,
}
```

### Step 4: List and revoke

```sh
cargo run -p tls --bin p8-ca -- list
# Domain              Issued      Expires     Status
# api.internal.com    2026-04-13  2026-07-12  VALID
# db.internal.com     2026-04-13  2026-07-12  VALID

cargo run -p tls --bin p8-ca -- revoke --domain api.internal.com
# Revoked: api.internal.com

cargo run -p tls --bin p8-ca -- list
# Domain              Issued      Expires     Status
# api.internal.com    2026-04-13  2026-07-12  REVOKED
# db.internal.com     2026-04-13  2026-07-12  VALID
```

### Step 5: Use with tokio-rustls

This is the payoff — use your CA-issued certs in a real TLS server:

```rust
// Server loads its cert:
let config = ServerConfig::builder()
    .with_no_client_auth()
    .with_single_cert(server_certs, server_key)?;

// Client trusts your CA:
let mut root_store = RootCertStore::empty();
let ca_cert = std::fs::read("ca.crt")?;
root_store.add(CertificateDer::from(ca_cert))?;
let config = ClientConfig::builder()
    .with_root_certificates(root_store)
    .with_no_client_auth();

// Handshake succeeds — your CA is trusted!
```

## Exercises

### Exercise 1: Basic CA

Implement init + issue. Verify with `openssl verify -CAfile ca.crt server.crt`.

### Exercise 2: Use with P6 (HTTPS Server)

Start the HTTPS server (P6) using your CA-issued cert instead of a self-signed one. Configure curl to trust your CA: `curl --cacert ca.crt https://localhost:8443/`. No more `-k`!

### Exercise 3: Intermediate CA

Create a 3-level chain: Root CA → Intermediate CA → Server cert. Verify with:
```sh
openssl verify -CAfile root.crt -untrusted intermediate.crt server.crt
```

### Exercise 4: Certificate renewal

Add a `renew` command that issues a new cert for an existing domain (new key pair, new expiry) and marks the old one as superseded in the database.

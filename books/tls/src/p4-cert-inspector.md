# Project: Certificate Inspector

> **Prerequisites**: Lesson 7 (Certificates), Lesson 14 (tokio-rustls). Connect to real websites and inspect their TLS certificates.

## Why inspect certificates?

Certificates are the backbone of internet trust. Being able to inspect them is a fundamental skill:

```
┌──────────────────────────────────────────────────────────────┐
│  When you need to inspect certificates                       │
│                                                              │
│  DevOps / SRE:                                               │
│    "Our HTTPS is broken" → is the cert expired?              │
│    "Users see a warning" → hostname mismatch? wrong chain?   │
│    "Cert renew failed"   → what's the current expiry?        │
│                                                              │
│  Security:                                                   │
│    "Is this site legit?" → who issued the cert? trusted CA?  │
│    "MITM detection"      → did the cert fingerprint change?  │
│    "CT monitoring"       → was a cert issued for my domain?  │
│                                                              │
│  Development:                                                │
│    "mTLS isn't working"  → is the client cert valid?         │
│    "Self-signed setup"   → are SANs configured correctly?    │
│    "Testing TLS code"    → what does the server actually send?│
└──────────────────────────────────────────────────────────────┘
```

## What you're building

A CLI tool that connects to any website, downloads its certificate chain, and shows everything — like a mini `openssl s_client` but with cleaner output.

```sh
cargo run -p tls --bin p3-cert-inspector -- google.com

  google.com:443
  ──────────────
  Protocol:   TLS 1.3
  Cipher:     TLS_AES_256_GCM_SHA384

  Certificate chain:
    [0] *.google.com
        Issuer:     GTS CA 1C3
        Valid:      2024-10-21 to 2025-01-13
        Expires in: 42 days
        Key:        ECDSA (P-256)
        SANs:       *.google.com, google.com, *.youtube.com, ...

    [1] GTS CA 1C3
        Issuer:     GTS Root R1
        Valid:      2020-08-13 to 2027-09-30
        Key:        RSA (2048 bits)

  Fingerprint (SHA-256): a1b2c3d4e5f6...
```

## The reference tools

Before building our own, see what the standard tools show:

```sh
# === openssl s_client — the classic ===

# Full connection info:
echo | openssl s_client -connect google.com:443 2>/dev/null | head -25
# CONNECTED(00000003)
# depth=2 ...
# depth=1 ...
# depth=0 ...
# ---
# Certificate chain
#  0 s:CN = *.google.com
#    i:CN = GTS CA 1C3
#  1 s:CN = GTS CA 1C3
#    i:CN = GTS Root R1

# Just the certificate details:
echo | openssl s_client -connect google.com:443 2>/dev/null | \
  openssl x509 -text -noout | head -30

# Just the dates:
echo | openssl s_client -connect google.com:443 2>/dev/null | \
  openssl x509 -noout -dates
# notBefore=Oct 21 08:22:04 2024 GMT
# notAfter=Jan 13 08:22:03 2025 GMT

# Just the SANs:
echo | openssl s_client -connect google.com:443 2>/dev/null | \
  openssl x509 -noout -ext subjectAltName

# Just the fingerprint:
echo | openssl s_client -connect google.com:443 2>/dev/null | \
  openssl x509 -noout -fingerprint -sha256
```

```sh
# === Test sites with intentional cert problems ===

# Expired cert:
echo | openssl s_client -connect expired.badssl.com:443 2>/dev/null | \
  openssl x509 -noout -dates
# notAfter is in the past!

# Wrong hostname:
echo | openssl s_client -connect wrong.host.badssl.com:443 2>/dev/null | \
  openssl x509 -noout -subject -ext subjectAltName
# Subject doesn't match the hostname

# Self-signed:
echo | openssl s_client -connect self-signed.badssl.com:443 2>/dev/null | head -5
# "verify error:num=18:self-signed certificate"
```

## Implementation guide

### Step 0: Project setup

```sh
touch tls/src/bin/p3-cert-inspector.rs
```

Add to `tls/Cargo.toml`:

```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net"] }
tokio-rustls = "0.26"
rustls = "0.23"
webpki-roots = "0.26"
x509-parser = "0.16"
clap = { version = "4", features = ["derive"] }
```

CLI skeleton:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cert-inspector", about = "Inspect TLS certificates of any website")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Inspect a site's certificate chain
    Inspect {
        /// Domain name (e.g., google.com)
        host: String,
        /// Port (default: 443)
        #[arg(long, default_value = "443")]
        port: u16,
    },
    /// Check certificate expiry for multiple domains
    CheckExpiry {
        /// Domain names
        hosts: Vec<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Inspect { host, port } => todo!(),
        Command::CheckExpiry { hosts } => todo!(),
    }
}
```

### Step 1: Connect over TLS

```rust
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use rustls::{ClientConfig, RootCertStore};

async fn tls_connect(host: &str, port: u16)
    -> Result<tokio_rustls::client::TlsStream<TcpStream>, Box<dyn std::error::Error>>
{
    // Load the system's trusted root CAs
    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));
    let tcp = TcpStream::connect(format!("{host}:{port}")).await?;
    let server_name = host.try_into()?;
    let tls = connector.connect(server_name, tcp).await?;
    Ok(tls)
}
```

Test it:

```rust
#[tokio::main]
async fn main() {
    let tls = tls_connect("google.com", 443).await.unwrap();
    println!("Connected to google.com over TLS!");

    // Extract negotiated parameters:
    let (_, conn) = tls.get_ref();
    println!("Protocol: {:?}", conn.protocol_version().unwrap());
    println!("Cipher:   {:?}", conn.negotiated_cipher_suite().unwrap());
}
```

```sh
cargo run -p tls --bin p3-cert-inspector -- inspect google.com
# Connected to google.com over TLS!
# Protocol: TLSv1_3
# Cipher: TLS13_AES_256_GCM_SHA384
```

### Step 2: Extract the certificate chain

After the TLS handshake, the peer's certificates are available:

```rust
let (_, conn) = tls.get_ref();
let certs = conn.peer_certificates()
    .expect("server didn't send certificates");

println!("Certificate chain: {} certificates", certs.len());
```

Each cert is DER-encoded bytes. Let's parse them.

### Step 3: Parse certificates with x509-parser

```rust
use x509_parser::prelude::*;

fn print_cert(index: usize, der: &[u8]) {
    let (_, cert) = X509Certificate::from_der(der)
        .expect("failed to parse certificate");

    println!("  [{}] {}", index, cert.subject());
    println!("      Issuer:  {}", cert.issuer());
    println!("      Valid:   {} to {}",
        cert.validity().not_before,
        cert.validity().not_after);

    // Check if it's a CA certificate
    if let Some(bc) = cert.basic_constraints().ok().flatten() {
        if bc.value.ca {
            println!("      Type:    CA certificate");
        }
    }

    // Self-signed?
    if cert.subject() == cert.issuer() {
        println!("      Note:    Self-signed (root CA or self-signed cert)");
    }
}
```

Test it:

```sh
cargo run -p tls --bin p3-cert-inspector -- inspect google.com
# Certificate chain: 3 certificates
#   [0] CN=*.google.com
#       Issuer:  CN=GTS CA 1C3
#       Valid:   2024-10-21 to 2025-01-13
#   [1] CN=GTS CA 1C3
#       Issuer:  CN=GTS Root R1
#       Valid:   2020-08-13 to 2027-09-30
#       Type:    CA certificate

# Compare with openssl:
echo | openssl s_client -connect google.com:443 2>/dev/null | grep -E "s:|i:"
```

### Step 4: Extract Subject Alternative Names

SANs tell you which domains the certificate covers:

```rust
fn print_sans(cert: &X509Certificate) {
    if let Ok(Some(san_ext)) = cert.subject_alternative_name() {
        let names: Vec<String> = san_ext.value.general_names.iter()
            .filter_map(|name| match name {
                x509_parser::extensions::GeneralName::DNSName(dns) => {
                    Some(dns.to_string())
                }
                x509_parser::extensions::GeneralName::IPAddress(ip) => {
                    Some(format!("IP:{:?}", ip))
                }
                _ => None,
            })
            .collect();

        if !names.is_empty() {
            println!("      SANs:    {}", names.join(", "));
        }
    }
}
```

```sh
cargo run -p tls --bin p3-cert-inspector -- inspect google.com
# ...
# SANs: *.google.com, google.com, *.youtube.com, youtube.com, ...
```

### Step 5: Compute days until expiry

```rust
fn days_until_expiry(cert: &X509Certificate) -> i64 {
    let not_after = cert.validity().not_after.timestamp();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    (not_after - now) / 86400
}
```

```rust
let days = days_until_expiry(&cert);
if days < 0 {
    println!("      ⚠ EXPIRED {} days ago!", -days);
} else if days < 30 {
    println!("      ⚠ Expires in {} days (renew soon!)", days);
} else {
    println!("      Expires in {} days", days);
}
```

### Step 6: Certificate fingerprint

The SHA-256 fingerprint uniquely identifies a certificate (used for pinning):

```rust
use sha2::{Sha256, Digest};

fn cert_fingerprint(der: &[u8]) -> String {
    let hash = Sha256::digest(der);
    hash.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(":")
}
```

```sh
# Compare with openssl:
echo | openssl s_client -connect google.com:443 2>/dev/null | \
  openssl x509 -noout -fingerprint -sha256
# SHA256 Fingerprint=A1:B2:C3:D4:...
```

### Step 7: Batch expiry checker

Check multiple domains at once:

```rust
async fn check_expiry(hosts: Vec<String>) {
    for host in &hosts {
        match tls_connect(host, 443).await {
            Ok(tls) => {
                let (_, conn) = tls.get_ref();
                if let Some(certs) = conn.peer_certificates() {
                    let (_, cert) = X509Certificate::from_der(&certs[0]).unwrap();
                    let days = days_until_expiry(&cert);
                    let status = if days < 0 { "EXPIRED" }
                        else if days < 30 { "⚠ RENEW SOON" }
                        else { "✓" };
                    println!("{:<30} {:>4} days  {}", host, days, status);
                }
            }
            Err(e) => {
                println!("{:<30} ERROR: {}", host, e);
            }
        }
    }
}
```

```sh
cargo run -p tls --bin p3-cert-inspector -- check-expiry \
  google.com github.com cloudflare.com expired.badssl.com

# google.com                       62 days  ✓
# github.com                      198 days  ✓
# cloudflare.com                  150 days  ✓
# expired.badssl.com              ERROR: certificate has expired
```

## Test targets

[badssl.com](https://badssl.com) provides certificates with every kind of problem — perfect for testing:

```sh
# Working:
cargo run -p tls --bin p3-cert-inspector -- inspect sha256.badssl.com
cargo run -p tls --bin p3-cert-inspector -- inspect tls-v1-2.badssl.com

# Broken (your tool should show useful errors):
cargo run -p tls --bin p3-cert-inspector -- inspect expired.badssl.com
cargo run -p tls --bin p3-cert-inspector -- inspect wrong.host.badssl.com
cargo run -p tls --bin p3-cert-inspector -- inspect self-signed.badssl.com
cargo run -p tls --bin p3-cert-inspector -- inspect untrusted-root.badssl.com
cargo run -p tls --bin p3-cert-inspector -- inspect revoked.badssl.com
```

**Tip**: for sites with invalid certs, you'll need to configure rustls to accept them (for inspection only). Create a custom `ServerCertVerifier` that accepts everything:

```rust
// DANGEROUS — for inspection only, never in production
struct NoVerify;
impl rustls::client::danger::ServerCertVerifier for NoVerify {
    fn verify_server_cert(&self, ...) -> Result<...> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    // ... implement other required methods
}
```

This lets you connect to expired/self-signed sites to inspect their certs.

## Exercises

### Exercise 1: Basic inspector

Connect, extract chain, print subject/issuer/validity/SANs for each cert. Compare your output with `openssl s_client`.

### Exercise 2: Batch expiry checker

```sh
cargo run -p tls --bin p3-cert-inspector -- check-expiry \
  google.com github.com example.com expired.badssl.com
```

Color-code output: green for >30 days, yellow for <30 days, red for expired.

### Exercise 3: Certificate pinning

Download a site's cert, compute SHA-256 fingerprint, save to a JSON file. On subsequent runs, compare the current fingerprint with the saved one. Alert if it changed (possible MITM or cert rotation).

```sh
# First run — saves the pin:
cargo run -p tls --bin p3-cert-inspector -- pin google.com
# Fingerprint: SHA-256:A1:B2:C3...
# Saved to pins.json

# Later run — checks the pin:
cargo run -p tls --bin p3-cert-inspector -- pin google.com
# Fingerprint: SHA-256:A1:B2:C3... ✓ matches saved pin

# After cert rotation:
cargo run -p tls --bin p3-cert-inspector -- pin google.com
# ⚠ FINGERPRINT CHANGED!
# Old: SHA-256:A1:B2:C3...
# New: SHA-256:D4:E5:F6...
```

### Exercise 4: JSON output

Add `--json` flag for machine-readable output:

```sh
cargo run -p tls --bin p3-cert-inspector -- inspect --json google.com
```

```json
{
  "host": "google.com",
  "port": 443,
  "protocol": "TLS 1.3",
  "cipher": "TLS_AES_256_GCM_SHA384",
  "chain": [
    {
      "subject": "CN=*.google.com",
      "issuer": "CN=GTS CA 1C3",
      "not_before": "2024-10-21",
      "not_after": "2025-01-13",
      "days_until_expiry": 62,
      "sans": ["*.google.com", "google.com", "*.youtube.com"],
      "fingerprint": "SHA-256:A1:B2:C3:D4..."
    }
  ]
}
```

This is useful for monitoring scripts that parse the output programmatically.

## Extension: turn your inspector into a scanner

Once the inspector works, it's one step from the tools security teams actually use ([testssl.sh](https://testssl.sh), [SSL Labs](https://www.ssllabs.com/ssltest/)). Every commercial scanner is built from the same primitives you already have: protocol/cipher introspection from rustls, expiry and SAN checks from x509-parser, plus a handful of security-header checks.

```
┌──────────────────────────────────────────────────────────┐
│  What a TLS scanner checks                               │
│                                                          │
│  ✓ Protocol version  — TLS 1.3? 1.2? Old 1.0? (bad!)     │
│  ✓ Cipher suite      — modern AEAD? or weak RC4? (bad!)  │
│  ✓ Certificate       — expired? wrong hostname?          │
│  ✓ Key exchange      — ephemeral DH (forward secrecy)?   │
│  ✓ HSTS header       — does the site force HTTPS?        │
│  ✓ Certificate chain — complete? trusted root?           │
└──────────────────────────────────────────────────────────┘
```

Try with existing tools first to see the shape of the data:

```sh
echo | openssl s_client -connect google.com:443 2>/dev/null | \
  grep -E "Protocol|Cipher|Server Temp Key"
curl -sI https://google.com | grep -i strict-transport

# Known-broken test sites (badssl.com):
echo | openssl s_client -connect expired.badssl.com:443 2>/dev/null | \
  openssl x509 -noout -dates
echo | openssl s_client -connect tls-v1-0.badssl.com:1010 2>/dev/null | grep Protocol
```

Extract the negotiated parameters after your TLS connect:

```rust
let tls = tls_connect(host, port).await?;
let (_, conn) = tls.get_ref();
let protocol = conn.protocol_version().unwrap();
let cipher   = conn.negotiated_cipher_suite().unwrap();
let certs    = conn.peer_certificates().unwrap();
```

After the handshake, reuse the same TLS stream to fetch HTTP headers:

```rust
let request = format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
tls.write_all(request.as_bytes()).await?;
let mut response = vec![0u8; 8192];
let n = tls.read(&mut response).await.unwrap_or(0);
let response = String::from_utf8_lossy(&response[..n]);

let has_hsts = response.lines()
    .any(|l| l.to_lowercase().starts_with("strict-transport-security"));
```

### Exercise 5: Full scanner output

Combine everything into one report:

```
TLS Scan: google.com:443
─────────────────────────
Protocol:     TLS 1.3 ✓
Cipher:       TLS_AES_256_GCM_SHA384 ✓
Key exchange: X25519 (forward secrecy ✓)

Certificate:
  Subject:    *.google.com
  Issuer:     GTS CA 1C3
  Expires in: 62 days ✓
  SANs:       *.google.com, google.com, *.youtube.com, ...

HTTP headers:
  HSTS:       max-age=31536000 ✓

Grade: A
```

Assign a grade with a simple rubric:

```rust
fn grade(protocol_ok: bool, cipher_ok: bool, cert_days: i64, has_hsts: bool) -> &'static str {
    if !protocol_ok || cert_days < 0 { return "F"; }
    if !cipher_ok                    { return "C"; }
    if cert_days < 30                { return "B"; }
    if !has_hsts                     { return "B+"; }
    "A"
}
```

Run it against `expired.badssl.com`, `self-signed.badssl.com`, `tls-v1-0.badssl.com:1010` — each should drop to a different grade for a different reason.

### Exercise 6: Comparison table

Scan multiple hosts and emit a side-by-side table. This is the shape most monitoring dashboards want:

```
Domain           Protocol  Cipher           Expires  HSTS  Grade
google.com       TLS 1.3   AES-256-GCM      62 days  ✓     A
example.com      TLS 1.3   AES-128-GCM      90 days  ✗     B+
old-site.com     TLS 1.2   AES-128-CBC      EXPIRED  ✗     F
```

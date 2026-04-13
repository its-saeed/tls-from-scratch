# Project: TLS Scanner

> **Prerequisites**: Lesson 13 (TLS Handshake Deep Dive), Lesson 14 (tokio-rustls). Probe a server's TLS configuration.

## What is this?

When something goes wrong with TLS ("site won't load", "certificate warning", "slow handshake"), the first thing you do is scan the server. Tools like [testssl.sh](https://testssl.sh) and [SSL Labs](https://www.ssllabs.com/ssltest/) probe a server's TLS setup and report issues.

You're building a simplified version:

```
┌──────────────────────────────────────────────────────────┐
│  What a TLS scanner checks                               │
│                                                          │
│  ✓ Protocol version — TLS 1.3? 1.2? Old 1.0? (bad!)     │
│  ✓ Cipher suite — modern AEAD? or weak RC4? (bad!)       │
│  ✓ Certificate — expired? wrong hostname? weak key?      │
│  ✓ Key exchange — ephemeral DH? (forward secrecy)        │
│  ✓ HSTS header — does the site force HTTPS?              │
│  ✓ Certificate chain — complete? trusted root?            │
└──────────────────────────────────────────────────────────┘
```

## What you're building

```sh
cargo run -p tls --bin p7-scanner -- scan google.com

  TLS Scan: google.com:443
  ─────────────────────────
  Protocol:     TLS 1.3 ✓
  Cipher:       TLS_AES_256_GCM_SHA384 ✓
  Key exchange: X25519 (forward secrecy ✓)

  Certificate:
    Subject:    *.google.com
    Issuer:     GTS CA 1C3
    Expires in: 62 days ✓
    Key type:   ECDSA P-256
    SANs:       *.google.com, google.com, *.youtube.com, ...

  HTTP headers:
    HSTS:       max-age=31536000 ✓

  Grade: A
```

## Try it with existing tools

```sh
# === openssl s_client (the manual way) ===

# Protocol + cipher:
echo | openssl s_client -connect google.com:443 2>/dev/null | \
  grep -E "Protocol|Cipher|Server Temp Key"

# Certificate dates:
echo | openssl s_client -connect google.com:443 2>/dev/null | \
  openssl x509 -noout -dates -subject -issuer

# SANs:
echo | openssl s_client -connect google.com:443 2>/dev/null | \
  openssl x509 -noout -ext subjectAltName

# HSTS header:
curl -sI https://google.com | grep -i strict-transport

# === testssl.sh (the automated way) ===
# brew install testssl
# testssl google.com
```

```sh
# Test sites with known problems (badssl.com):
echo | openssl s_client -connect expired.badssl.com:443 2>/dev/null | \
  openssl x509 -noout -dates
# notAfter is in the past!

echo | openssl s_client -connect wrong.host.badssl.com:443 2>/dev/null | \
  openssl x509 -noout -subject -ext subjectAltName
# hostname doesn't match SANs

echo | openssl s_client -connect tls-v1-0.badssl.com:1010 2>/dev/null | \
  grep Protocol
# TLS 1.0 — insecure!
```

## Implementation guide

### Step 0: Project setup

```sh
touch tls/src/bin/p7-scanner.rs
```

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tls-scanner", about = "Scan a server's TLS configuration")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan a single host
    Scan {
        host: String,
        #[arg(long, default_value = "443")]
        port: u16,
    },
    /// Check certificate expiry for multiple hosts
    Expiry {
        hosts: Vec<String>,
    },
}
```

### Step 1: Connect and extract TLS info

Reuse the TLS connection code from P4 (Certificate Inspector):

```rust
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use rustls::{ClientConfig, RootCertStore};

async fn tls_connect(host: &str, port: u16)
    -> Result<tokio_rustls::client::TlsStream<TcpStream>, Box<dyn std::error::Error>>
{
    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));
    let tcp = TcpStream::connect(format!("{host}:{port}")).await?;
    let server_name = host.try_into()?;
    Ok(connector.connect(server_name, tcp).await?)
}
```

After connecting, extract the negotiated parameters:

```rust
let tls = tls_connect(host, port).await?;
let (_, conn) = tls.get_ref();

let protocol = conn.protocol_version().unwrap();
let cipher = conn.negotiated_cipher_suite().unwrap();
let certs = conn.peer_certificates().unwrap();

println!("Protocol: {:?}", protocol);
println!("Cipher:   {:?}", cipher);
println!("Chain:    {} certificates", certs.len());
```

Test:

```sh
cargo run -p tls --bin p7-scanner -- scan google.com
# Protocol: TLSv1_3
# Cipher: TLS13_AES_256_GCM_SHA384
# Chain: 3 certificates
```

### Step 2: Parse the certificate

Reuse the x509 parsing from P4:

```rust
use x509_parser::prelude::*;

fn analyze_cert(der: &[u8], index: usize) {
    let (_, cert) = X509Certificate::from_der(der).unwrap();

    println!("  [{}] {}", index, cert.subject());
    println!("      Issuer:  {}", cert.issuer());

    // Expiry
    let not_after = cert.validity().not_after.timestamp();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let days = (not_after - now) / 86400;

    if days < 0 {
        println!("      Expires:  EXPIRED {} days ago ✗", -days);
    } else if days < 30 {
        println!("      Expires:  {} days (renew soon!) ⚠", days);
    } else {
        println!("      Expires:  {} days ✓", days);
    }

    // SANs
    if let Ok(Some(san)) = cert.subject_alternative_name() {
        let names: Vec<_> = san.value.general_names.iter()
            .filter_map(|n| match n {
                x509_parser::extensions::GeneralName::DNSName(d) => Some(d.to_string()),
                _ => None,
            })
            .collect();
        if !names.is_empty() {
            println!("      SANs:    {}", names[..names.len().min(5)].join(", "));
            if names.len() > 5 {
                println!("               ... and {} more", names.len() - 5);
            }
        }
    }
}
```

### Step 3: Check HTTP security headers

After the TLS handshake, send a minimal HTTP request and check the response headers:

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn check_http_headers(tls: &mut tokio_rustls::client::TlsStream<TcpStream>, host: &str) {
    let request = format!(
        "GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
    );
    tls.write_all(request.as_bytes()).await.unwrap();

    let mut response = vec![0u8; 8192];
    let n = tls.read(&mut response).await.unwrap_or(0);
    let response = String::from_utf8_lossy(&response[..n]);

    // Check HSTS
    if let Some(hsts) = response.lines().find(|l| l.to_lowercase().starts_with("strict-transport-security")) {
        println!("  HSTS:     {hsts} ✓");
    } else {
        println!("  HSTS:     not set ✗");
    }

    // Check other security headers
    for header in ["x-content-type-options", "x-frame-options", "content-security-policy"] {
        if response.lines().any(|l| l.to_lowercase().starts_with(header)) {
            println!("  {}: set ✓", header);
        }
    }
}
```

### Step 4: Grade the result

Assign a simple letter grade:

```rust
fn grade(protocol_ok: bool, cipher_ok: bool, cert_days: i64, has_hsts: bool) -> &'static str {
    if !protocol_ok || cert_days < 0 { return "F"; }
    if !cipher_ok { return "C"; }
    if cert_days < 30 { return "B"; }
    if !has_hsts { return "B+"; }
    "A"
}
```

### Step 5: Batch expiry checker

```rust
async fn check_expiry(hosts: Vec<String>) {
    println!("{:<30} {:>6}  {}", "Host", "Days", "Status");
    println!("{}", "─".repeat(50));

    for host in &hosts {
        match tls_connect(host, 443).await {
            Ok(tls) => {
                let (_, conn) = tls.get_ref();
                if let Some(certs) = conn.peer_certificates() {
                    let (_, cert) = X509Certificate::from_der(&certs[0]).unwrap();
                    let not_after = cert.validity().not_after.timestamp();
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
                    let days = (not_after - now) / 86400;
                    let status = if days < 0 { "EXPIRED ✗" }
                        else if days < 30 { "RENEW SOON ⚠" }
                        else { "✓" };
                    println!("{:<30} {:>6}  {}", host, days, status);
                }
            }
            Err(e) => println!("{:<30} {:>6}  ERROR: {}", host, "-", e),
        }
    }
}
```

### Step 6: Test it

```sh
# Scan good sites:
cargo run -p tls --bin p7-scanner -- scan google.com
cargo run -p tls --bin p7-scanner -- scan github.com
cargo run -p tls --bin p7-scanner -- scan cloudflare.com

# Scan problematic sites:
cargo run -p tls --bin p7-scanner -- scan expired.badssl.com
cargo run -p tls --bin p7-scanner -- scan self-signed.badssl.com

# Batch expiry check:
cargo run -p tls --bin p7-scanner -- expiry google.com github.com cloudflare.com
# Host                            Days  Status
# ──────────────────────────────────────────────
# google.com                        62  ✓
# github.com                       198  ✓
# cloudflare.com                   150  ✓
```

## Exercises

### Exercise 1: Basic scanner

Implement steps 1-4. Scan google.com and display protocol, cipher, certificate details, grade.

### Exercise 2: Batch expiry checker

Scan a list of domains, report days until cert expiry. Color-code: green >30, yellow <30, red expired.

### Exercise 3: JSON output

Add `--json` flag for machine-readable output:

```sh
cargo run -p tls --bin p7-scanner -- scan --json google.com
```

### Exercise 4: Compare sites

Scan multiple sites side-by-side in a table:

```
Domain           Protocol  Cipher           Expires  HSTS  Grade
google.com       TLS 1.3   AES-256-GCM      62 days  ✓     A
example.com      TLS 1.3   AES-128-GCM      90 days  ✗     B+
old-site.com     TLS 1.2   AES-128-CBC      EXPIRED  ✗     F
```

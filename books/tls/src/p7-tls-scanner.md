# Project: TLS Scanner

> **Prerequisites**: Lesson 13 (TLS Handshake Deep Dive), Lesson 14 (tokio-rustls). Probe a server's TLS configuration.

## What you're building

A tool that probes a server's TLS setup — like a mini [testssl.sh](https://testssl.sh) or [SSL Labs](https://www.ssllabs.com/ssltest/).

```sh
cargo run -p tls --bin p7-scanner -- google.com

  TLS Scan: google.com:443
  ─────────────────────────
  Protocol:    TLS 1.3 ✓
  Cipher:      TLS_AES_256_GCM_SHA384
  Key exchange: X25519 (253 bits)
  Certificate:
    Subject:   *.google.com
    Issuer:    GTS CA 1C3
    Expires:   62 days
    Key:       ECDSA P-256
    SANs:      *.google.com, google.com, youtube.com, ...
  HSTS:        max-age=31536000 ✓
  OCSP staple: yes ✓
```

## What to check

```
Check                  What it means                    How to test
──────────────────────────────────────────────────────────────────────
TLS version            1.3 is current, 1.2 ok,         Try connecting with each version
                       1.1/1.0 is bad
Cipher suite           AEAD ciphers only (GCM, ChaCha) List negotiated suite
Forward secrecy        Ephemeral DH (X25519, P-256)    Check "Server Temp Key"
Certificate validity   Not expired, not too long        Parse dates
Certificate chain      Complete, trusted root            Verify chain
HSTS header            Forces HTTPS                     Check HTTP response headers
```

## The reference tool

```sh
# What your scanner replaces:
echo | openssl s_client -connect google.com:443 2>/dev/null | \
  grep -E "Protocol|Cipher|Server Temp Key"

# Test specific TLS version:
openssl s_client -connect google.com:443 -tls1_2 2>/dev/null | grep "Protocol"
openssl s_client -connect google.com:443 -tls1_3 2>/dev/null | grep "Protocol"

# Check HSTS:
curl -sI https://google.com | grep -i strict-transport
```

## Implementation guide

### Step 1: Connect and extract TLS info

After the TLS handshake with `tokio-rustls`, extract the negotiated parameters:

```rust
let (_, conn) = tls_stream.get_ref();
let cipher = conn.negotiated_cipher_suite().unwrap();
let version = conn.protocol_version().unwrap();
let certs = conn.peer_certificates().unwrap();
```

### Step 2: Parse the certificate

Use `x509-parser` to extract subject, issuer, expiry, key type, SANs (from Lesson 7 and P3).

### Step 3: Check HSTS

Send a minimal HTTP request and check the response headers:

```rust
stream.write_all(b"GET / HTTP/1.1\r\nHost: google.com\r\nConnection: close\r\n\r\n").await?;
let mut response = String::new();
stream.read_to_string(&mut response).await?;
let has_hsts = response.contains("strict-transport-security");
```

## Test targets

```sh
# Good configurations:
cargo run -p tls --bin p7-scanner -- google.com github.com cloudflare.com

# Problematic configurations (badssl.com):
cargo run -p tls --bin p7-scanner -- tls-v1-0.badssl.com   # old TLS
cargo run -p tls --bin p7-scanner -- rc4.badssl.com         # weak cipher
cargo run -p tls --bin p7-scanner -- expired.badssl.com     # expired cert
cargo run -p tls --bin p7-scanner -- no-sct.badssl.com      # no CT
```

## Exercises

### Exercise 1: Basic scan
Connect, report protocol version, cipher suite, certificate details.

### Exercise 2: Version probing
Try TLS 1.3, 1.2, 1.1, 1.0 separately. Report which versions the server accepts. Flag 1.1 and 1.0 as insecure.

### Exercise 3: HSTS and headers
After TLS, send an HTTP request and check security headers: HSTS, X-Content-Type-Options, X-Frame-Options.

### Exercise 4: Batch scan
Scan a list of domains from a file, output a report table:
```
Domain         TLS    Cipher              Expires   HSTS
google.com     1.3    AES-256-GCM         62 days   ✓
example.com    1.3    AES-128-GCM         90 days   ✗
old-site.com   1.2    AES-128-CBC         EXPIRED   ✗
```

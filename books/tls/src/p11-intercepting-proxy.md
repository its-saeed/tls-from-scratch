# Project: HTTPS Intercepting Proxy

> **Prerequisites**: Lesson 8 (cert generation), Lesson 13 (handshake), Lesson 14 (tokio-rustls), P8 (CA). The ultimate TLS capstone.

## What you're building

A proxy that sits between a client and any HTTPS server, decrypting all traffic so you can inspect it — like a mini [mitmproxy](https://mitmproxy.org). This is the tool you used for debugging JSON-RPC requests.

```
Browser                 Your Proxy              Real Server
  │                        │                       │
  │── CONNECT google.com:443 ─►│                   │
  │                        │                       │
  │◄── 200 Connection     │                       │
  │    Established         │                       │
  │                        │                       │
  │── TLS handshake ─────►│                       │
  │   (proxy's fake cert   │── TLS handshake ────►│
  │    for google.com)     │   (real cert)         │
  │                        │                       │
  │── GET / (encrypted) ──►│── GET / (re-encrypted)►│
  │                        │                       │
  │   Proxy sees plaintext │                       │
  │   request + response!  │                       │
```

## How it works

The magic: **on-the-fly certificate generation**.

```
1. Client says: "I want to connect to google.com"  (HTTP CONNECT)
2. Proxy says: "OK" (200 Connection Established)
3. Client starts TLS handshake, sends ClientHello with SNI=google.com
4. Proxy reads SNI, generates a FAKE certificate for google.com
   → signed by YOUR CA (installed on the client machine)
5. Client verifies cert → chain leads to YOUR CA → trusted!
6. Proxy connects to REAL google.com with real TLS
7. Proxy decrypts client traffic, logs it, re-encrypts to real server
8. Both sides think they have a private connection — but the proxy sees everything
```

### Why the client trusts the fake cert

```
Normal:
  Browser trusts: DigiCert, Let's Encrypt, ... (100+ root CAs)
  google.com shows cert signed by Google Trust Services → trusted ✓

With intercepting proxy:
  Browser trusts: DigiCert, Let's Encrypt, ..., YOUR CA
  Proxy shows cert for google.com signed by YOUR CA → trusted ✓

  You must install YOUR CA in the browser/OS trust store.
  Without it, the browser shows a certificate warning.
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Intercepting Proxy                                         │
│                                                             │
│  ┌──────────────────┐                                       │
│  │  CA Key + Cert    │  (generated once, installed on client)│
│  └────────┬─────────┘                                       │
│           │                                                 │
│           ▼                                                 │
│  ┌──────────────────┐     ┌──────────────────┐              │
│  │  Cert Generator   │     │  TLS Client      │              │
│  │  (rcgen)          │     │  (to real server) │              │
│  │                   │     │                   │              │
│  │  SNI: google.com  │     │  connect to       │              │
│  │  → fake cert for  │     │  google.com:443   │              │
│  │    google.com     │     │  (real TLS)       │              │
│  └──────────────────┘     └──────────────────┘              │
│           │                         │                       │
│           ▼                         ▼                       │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Request/Response Logger                              │   │
│  │                                                       │   │
│  │  → GET /search?q=hello HTTP/1.1                       │   │
│  │  ← 200 OK (text/html, 15KB)                          │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Implementation guide

### Step 1: HTTP CONNECT proxy

Listen for `CONNECT host:port` requests. Respond with `200`. Then tunnel the connection.

```rust
// Read the CONNECT request
let mut buf = [0u8; 4096];
let n = client.read(&mut buf).await?;
let request = String::from_utf8_lossy(&buf[..n]);
// "CONNECT google.com:443 HTTP/1.1\r\n..."
let host = parse_connect_host(&request); // "google.com"

client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
```

### Step 2: Generate fake certificate

```rust
use rcgen::{CertificateParams, KeyPair, IsCa};

fn generate_cert_for_host(host: &str, ca_cert: &Certificate, ca_key: &KeyPair) -> (CertificateDer, PrivateKeyDer) {
    let mut params = CertificateParams::new(vec![host.into()])?;
    params.is_ca = IsCa::NoCa;
    let key = KeyPair::generate()?;
    let cert = params.signed_by(&key, ca_cert, ca_key)?;
    (cert.der().clone(), key.serialize_der())
}
```

### Step 3: TLS to client (with fake cert)

```rust
let server_config = ServerConfig::builder()
    .with_no_client_auth()
    .with_single_cert(vec![fake_cert], fake_key)?;
let mut client_tls = TlsAcceptor::from(Arc::new(server_config))
    .accept(client).await?;
```

### Step 4: TLS to real server

```rust
let mut real_tls = TlsConnector::from(Arc::new(client_config))
    .connect(host.try_into()?, real_tcp).await?;
```

### Step 5: Pipe and log

```rust
// Read from client, log, forward to server
// Read from server, log, forward to client
```

## Setting up the CA trust

```sh
# Generate CA:
cargo run -p tls --bin p8-ca -- init

# macOS: install CA cert
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain ca.crt

# Linux: install CA cert
sudo cp ca.crt /usr/local/share/ca-certificates/proxy-ca.crt
sudo update-ca-certificates

# For curl:
curl --cacert ca.crt https://google.com --proxy http://127.0.0.1:8080

# Remove when done:
# macOS: sudo security remove-trusted-cert -d ca.crt
# Linux: sudo rm /usr/local/share/ca-certificates/proxy-ca.crt && sudo update-ca-certificates
```

## Ethical note

This tool is for **debugging your own traffic** and **authorized security testing** only. Intercepting someone else's traffic without consent is illegal in most jurisdictions.

Legitimate uses:
- Debugging API calls during development
- Security testing your own services
- Corporate network monitoring (with employee consent)
- CTF challenges and security research

## Exercises

### Exercise 1: Basic CONNECT proxy
Handle CONNECT, tunnel bytes without decryption. Verify with `curl --proxy http://127.0.0.1:8080 https://example.com`.

### Exercise 2: TLS interception
Generate fake certs, decrypt traffic, log HTTP requests/responses. Verify with `curl --proxy ... --cacert ca.crt https://example.com`.

### Exercise 3: Certificate caching
Cache generated certificates so you don't regenerate for every connection to the same host. Use a `HashMap<String, (CertificateDer, PrivateKeyDer)>`.

### Exercise 4: Web UI
Add a web dashboard at `http://127.0.0.1:8081` that shows all intercepted requests in real-time (like mitmproxy's web interface).

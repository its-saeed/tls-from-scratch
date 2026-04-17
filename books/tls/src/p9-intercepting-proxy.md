# Project: HTTPS Intercepting Proxy

> **Prerequisites**: Lesson 8 (cert generation), Lesson 13 (handshake), Lesson 14 (tokio-rustls), P7 (CA). The ultimate TLS capstone — builds in three stages.

## What is this?

When you're debugging API calls, you need to see the actual HTTP requests and responses — but they're encrypted with TLS. Tools like [mitmproxy](https://mitmproxy.org), [Charles Proxy](https://www.charlesproxy.com), and [Fiddler](https://www.telerik.com/fiddler) solve this by intercepting HTTPS traffic.

This project uses **every TLS concept** you've learned, so we'll build it in three staged milestones. Get each one working before moving on — the jump from "plain proxy" to "MITM" is exactly where most people get stuck.

```
Stage 1: CONNECT tunnel              (30 min) — proxy sees nothing
Stage 2: SNI + dynamic cert          (60 min) — proxy decrypts client traffic
Stage 3: Full MITM + cert cache      (90 min) — proxy logs plaintext both ways
```

## Try it with existing tools first

Install mitmproxy — running it for 5 minutes makes the rest of this project click:

```sh
# macOS: brew install mitmproxy
# Linux: pip3 install mitmproxy

mitmproxy --listen-port 8080

# In another terminal:
curl -x http://127.0.0.1:8080 -k https://example.com
# mitmproxy shows: GET https://example.com/ → 200 (1256 bytes)

# Look at the CA it generated:
ls ~/.mitmproxy/
# mitmproxy-ca-cert.pem  ← CA cert. You install this to avoid warnings.
```

The core trick: **your browser trusts the proxy's CA, so the proxy can mint a cert for any domain the browser asks for.** Stage 2 is where you build that.

## The whole picture (before stages)

```
Browser                 Your Proxy              Real Server
  │                        │                       │
  │── CONNECT example.com:443 ─►                   │       Stage 1
  │                        │                       │       parses this
  │◄── 200 Connection     │                       │
  │    Established         │                       │
  │                        │                       │
  │── TLS handshake ─────►│                       │       Stage 2
  │   (proxy's fake cert   │── TLS handshake ────►│       generates
  │    for example.com)    │   (real cert)         │       a fake cert
  │                        │                       │       here
  │── GET / (encrypted) ──►│── GET / (re-encrypted)►│
  │                        │                       │       Stage 3
  │   Proxy sees plaintext │                       │       logs this
  │   request + response!  │                       │
```

Everything the client encrypts with the proxy's fake cert, the proxy can decrypt. The proxy then re-encrypts to the real server with a fresh TLS session. Two separate TLS connections, bridged by plaintext in the middle.

## Stage 1 — CONNECT tunnel (no interception)

Build a plain HTTP `CONNECT` proxy. No TLS on your side. Just tunnel bytes.

### Why this first

HTTPS-through-a-proxy starts with an HTTP request. Your client sends `CONNECT host:port HTTP/1.1`; you reply `200 Connection Established`; then you shovel bytes both ways until either side hangs up. No crypto involved *yet*. Prove you can do this, then add the crypto.

### The request

```
CONNECT example.com:443 HTTP/1.1
Host: example.com:443
User-Agent: curl/8.0
<blank line>
```

### Implementation

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional};
use tokio::net::{TcpListener, TcpStream};

async fn handle(mut client: TcpStream) -> anyhow::Result<()> {
    // 1. Read the CONNECT request line
    let mut buf = [0u8; 4096];
    let n = client.read(&mut buf).await?;
    let request = std::str::from_utf8(&buf[..n])?;
    let first = request.lines().next().unwrap_or("");
    let host_port = first.strip_prefix("CONNECT ")
        .and_then(|s| s.split_whitespace().next())
        .ok_or_else(|| anyhow::anyhow!("not a CONNECT request"))?;

    // 2. Open a TCP connection to the real server
    let mut upstream = TcpStream::connect(host_port).await?;

    // 3. Tell the client the tunnel is ready
    client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;

    // 4. Shovel bytes both ways
    copy_bidirectional(&mut client, &mut upstream).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    loop {
        let (sock, _) = listener.accept().await?;
        tokio::spawn(async move {
            let _ = handle(sock).await;
        });
    }
}
```

### Test

```sh
cargo run -p tls --bin p9-intercept

# In another terminal:
curl -x http://127.0.0.1:8080 https://example.com/
# Works. Returns the real HTML from example.com.
# Your proxy saw: only the CONNECT line. Zero plaintext after that.
```

**Milestone 1 passed when**: curl through your proxy returns real HTML from real HTTPS sites, and your proxy logs `CONNECT example.com:443` but nothing else.

## Stage 2 — Intercept with SNI + dynamic cert

Now instead of tunneling bytes, *terminate TLS at the proxy* using a fake cert. To do this you need to know *which hostname* the client is about to ask for. That's what SNI (Server Name Indication) is — the client puts it in the ClientHello.

### Why this is the hard stage

Two sequencing problems:

1. You need the SNI *before* you can pick a cert, but SNI lives inside the first TLS message.
2. You need to issue a cert *per host* on the fly, signed by a CA the client already trusts.

rustls solves both: a `ResolvesServerCert` trait fires once per connection with the parsed ClientHello, including the SNI.

### Pre-req: install your CA

From P7 (Certificate Authority):

```sh
cargo run -p tls --bin p7-ca -- init --name "My MITM CA"
# Writes ca.crt and ca.key

# macOS — add to system trust store:
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain ca.crt

# Linux:
sudo cp ca.crt /usr/local/share/ca-certificates/mitm-ca.crt
sudo update-ca-certificates

# For this project you can also just pass --cacert to curl:
curl --cacert ca.crt --proxy http://127.0.0.1:8080 https://example.com
```

### On-demand cert generator

```rust
use rcgen::{CertificateParams, KeyPair, Certificate};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

struct CertFactory {
    ca_cert: Certificate,
    ca_key:  KeyPair,
}

impl CertFactory {
    fn issue(&self, host: &str) -> (CertificateDer<'static>, PrivateKeyDer<'static>) {
        let params = CertificateParams::new(vec![host.into()]).unwrap();
        let key = KeyPair::generate().unwrap();
        let cert = params.signed_by(&key, &self.ca_cert, &self.ca_key).unwrap();

        let cert_der = CertificateDer::from(cert.der().to_vec());
        let key_der  = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der()));
        (cert_der, key_der)
    }
}
```

### Hook it into rustls with ResolvesServerCert

```rust
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use std::sync::Arc;

struct DynamicResolver { factory: Arc<CertFactory> }

impl ResolvesServerCert for DynamicResolver {
    fn resolve(&self, hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        let host = hello.server_name()?.to_string();
        let (cert, key) = self.factory.issue(&host);
        let signing_key = rustls::crypto::aws_lc_rs::sign::any_supported_type(&key).ok()?;
        Some(Arc::new(CertifiedKey::new(vec![cert], signing_key)))
    }
}
```

### Accept the inner TLS and talk to the real server

```rust
async fn intercept(mut client: TcpStream, factory: Arc<CertFactory>, host_port: &str)
    -> anyhow::Result<()>
{
    client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;

    // Inner TLS to the client, using SNI-driven cert generation
    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(DynamicResolver { factory }));
    let mut client_tls = TlsAcceptor::from(Arc::new(server_config))
        .accept(client).await?;

    // Now open TLS to the real server
    let (host, _port) = host_port.split_once(':').unwrap();
    let upstream_tcp = TcpStream::connect(host_port).await?;
    let client_config = /* webpki_roots-based ClientConfig, see Lesson 14 */;
    let mut upstream_tls = TlsConnector::from(Arc::new(client_config))
        .connect(host.to_string().try_into()?, upstream_tcp).await?;

    // Proxy the TLS-decrypted bytes (still HTTP, just plaintext now)
    tokio::io::copy_bidirectional(&mut client_tls, &mut upstream_tls).await?;
    Ok(())
}
```

### Test

```sh
curl --cacert ca.crt --proxy http://127.0.0.1:8080 https://example.com/ -v
# Look for: "subject: CN=example.com; issuer: CN=My MITM CA"
# That's your fake cert. The connection works because curl trusts your CA.
```

**Milestone 2 passed when**: curl gets real HTML, and `openssl s_client --proxy http://127.0.0.1:8080 -connect example.com:443` shows the cert's *issuer* is your CA, not the real one.

## Stage 3 — Log and cache

The proxy now has plaintext. Log what it sees, and avoid regenerating the same cert 1000 times per page load.

### Log HTTP in flight

Replace `copy_bidirectional` with your own pump that sniffs the first line of each direction:

```rust
async fn pump_logged(
    from: &mut (impl AsyncRead + Unpin),
    to:   &mut (impl AsyncWrite + Unpin),
    label: &str,
) -> anyhow::Result<()> {
    let mut buf = vec![0u8; 16 * 1024];
    let mut logged_first = false;
    loop {
        let n = from.read(&mut buf).await?;
        if n == 0 { return Ok(()); }
        if !logged_first {
            // The first chunk of an HTTP/1.1 message starts with the request/status line
            let first_line = std::str::from_utf8(&buf[..n])
                .ok()
                .and_then(|s| s.lines().next())
                .unwrap_or("<non-utf8>");
            println!("  {label} {first_line}");
            logged_first = true;
        }
        to.write_all(&buf[..n]).await?;
    }
}

// Run both directions concurrently:
tokio::try_join!(
    pump_logged(&mut client_read,   &mut upstream_write, "→"),
    pump_logged(&mut upstream_read, &mut client_write,   "←"),
)?;
```

(For brevity the split of `&mut TlsStream` into read/write halves is elided — `tokio::io::split` does it.)

### Cache generated certs

Cert generation is slow. Cache per hostname:

```rust
use std::sync::Mutex;
use std::collections::HashMap;

struct CertFactory {
    ca_cert: Certificate,
    ca_key:  KeyPair,
    cache:   Mutex<HashMap<String, Arc<CertifiedKey>>>,
}

impl CertFactory {
    fn issue_cached(&self, host: &str) -> Arc<CertifiedKey> {
        if let Some(hit) = self.cache.lock().unwrap().get(host) { return hit.clone(); }
        let ck = Arc::new(self.build_certified_key(host));
        self.cache.lock().unwrap().insert(host.to_string(), ck.clone());
        ck
    }
}
```

After the cache warms, per-connection cost drops to a `HashMap` lookup.

### Test

```sh
curl --cacert ca.crt --proxy http://127.0.0.1:8080 https://example.com/
curl --cacert ca.crt --proxy http://127.0.0.1:8080 https://example.com/about
curl --cacert ca.crt --proxy http://127.0.0.1:8080 https://httpbin.org/headers

# Your proxy logs:
#   CONNECT example.com:443
#   → GET / HTTP/1.1
#   ← HTTP/1.1 200 OK
#   CONNECT example.com:443        ← cert cache hit, no regeneration
#   → GET /about HTTP/1.1
#   ← HTTP/1.1 404 Not Found
#   CONNECT httpbin.org:443
#   → GET /headers HTTP/1.1
#   ← HTTP/1.1 200 OK
```

**Milestone 3 passed when**: the proxy logs every HTTP request and response line, and repeated visits to the same host skip cert generation.

## Why this works (the trust question)

```
Normal:
  Browser trusts: DigiCert, Let's Encrypt, ... (100+ root CAs)
  example.com's cert is signed by one of those → trusted ✓

With your proxy:
  Browser trusts: DigiCert, Let's Encrypt, ..., YOUR CA
  Proxy's cert for example.com is signed by YOUR CA → trusted ✓

  Install your CA on the client, and the browser has no way to
  distinguish your fake cert from the real thing. That's the whole
  point — and why installing CAs is a privileged operation.
```

## Ethical note

This tool is for **debugging your own traffic** and **authorized security testing** only. Intercepting someone else's traffic without consent is illegal in most jurisdictions.

Legitimate uses:
- Debugging API calls during development
- Security testing your own services
- Corporate network monitoring (with employee consent)
- CTF challenges and security research

## Exercises

### Exercise 1: Stages 1-3

Complete each milestone. Before moving to the next one, verify the test matches the expected output.

### Exercise 2: Streaming logger

The logger above only logs the *first* line of each direction. Extend it to log headers and a truncated body preview (first 200 bytes), so you see `Content-Type`, cookies, etc.

### Exercise 3: HTTP/2 ALPN

Modern servers negotiate HTTP/2 via TLS ALPN. Advertise `h2` and `http/1.1` on the server side of your proxy; negotiate the same with the upstream. What breaks if the two halves don't match? (Answer: framing mismatch — the client speaks HTTP/2 frames into a stream expecting HTTP/1.1 bytes.)

### Exercise 4: Pinning bypass demonstration

Modern apps pin a specific cert fingerprint instead of trusting the system CA list. Point an iOS/Android app or a CLI with cert pinning at your proxy. Observe that it *won't* connect — the fingerprint doesn't match. This is why pinning matters.

### Exercise 5: Request modification

Inject a header (`X-Intercepted-By: my-proxy`) into every outgoing request, or rewrite `Host:` on the fly. Now you're not just an observer — you're an active MITM. Consider what this means for bug bounty programs ("we disable pinning for debug builds only").

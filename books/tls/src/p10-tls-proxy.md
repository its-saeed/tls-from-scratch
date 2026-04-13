# Project: TLS Termination Proxy

> **Prerequisites**: Lesson 14 (tokio-rustls), P8 (Certificate Authority).

## What is this?

A TLS termination proxy sits in front of your backend services. It handles all the TLS complexity — certificates, handshakes, encryption — so your backends can stay simple (plain HTTP).

This is what nginx, HAProxy, Cloudflare, and AWS ALB do for millions of websites.

```
With TLS termination proxy:

  Internet              Proxy                   Backend
  (encrypted)           (terminates TLS)        (plaintext)
  │                     │                       │
  │── HTTPS ──────────►│── HTTP ──────────────►│
  │   (encrypted)       │   (plaintext, fast)   │  Backend is simple.
  │◄── HTTPS ──────────│◄── HTTP ──────────────│  No cert management.
  │                     │                       │  No TLS overhead.

Without proxy:

  Internet              Backend
  │                     │
  │── HTTPS ──────────►│  Backend must:
  │                     │  - Manage certificates
  │                     │  - Handle TLS handshakes
  │                     │  - Renew certs
  │                     │  - Each of 20 services does this independently
```

## What you're building

```sh
# Start a plaintext backend:
python3 -m http.server 8000 &

# Start the TLS proxy:
cargo run -p tls --bin p10-proxy -- \
  --listen 0.0.0.0:8443 \
  --backend 127.0.0.1:8000 \
  --cert server.crt --key server.key

# Client connects with HTTPS:
curl -k https://127.0.0.1:8443/
# Gets the response from the backend — but over TLS!
```

## Try it with existing tools

```sh
# nginx does this with ~5 lines of config:
# server {
#     listen 443 ssl;
#     ssl_certificate server.crt;
#     ssl_certificate_key server.key;
#     location / { proxy_pass http://127.0.0.1:8000; }
# }
# You're building this in Rust.
```

## Implementation guide

### Step 0: Project setup

```sh
touch tls/src/bin/p10-proxy.rs
```

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "tls-proxy", about = "TLS termination proxy")]
struct Cli {
    /// Address to listen on (e.g., 0.0.0.0:8443)
    #[arg(long)]
    listen: String,
    /// Backend address (e.g., 127.0.0.1:8000)
    #[arg(long)]
    backend: String,
    /// TLS certificate path
    #[arg(long)]
    cert: String,
    /// TLS private key path
    #[arg(long)]
    key: String,
}
```

### Step 1: Accept TLS connections

Same as P6, but instead of handling HTTP yourself, you forward to the backend:

```rust
let acceptor = TlsAcceptor::from(tls_config);
let listener = TcpListener::bind(&cli.listen).await?;

loop {
    let (client_tcp, addr) = listener.accept().await?;
    let acceptor = acceptor.clone();
    let backend = cli.backend.clone();

    tokio::spawn(async move {
        let client_tls = acceptor.accept(client_tcp).await.unwrap();
        proxy_connection(client_tls, &backend).await;
    });
}
```

### Step 2: Connect to backend and pipe data

The core of the proxy — just copy bytes both ways:

```rust
async fn proxy_connection(
    mut client: tokio_rustls::server::TlsStream<TcpStream>,
    backend_addr: &str,
) {
    let mut backend = TcpStream::connect(backend_addr).await.unwrap();

    // Pipe both directions:
    // client (encrypted) → proxy (decrypts) → backend (plaintext)
    // backend (plaintext) → proxy (encrypts) → client (encrypted)
    tokio::io::copy_bidirectional(&mut client, &mut backend).await.ok();
}
```

That's it. `copy_bidirectional` handles both directions. rustls transparently decrypts/encrypts.

### Step 3: Test it

```sh
# Start a backend:
python3 -m http.server 8000 &

# Start the proxy:
cargo run -p tls --bin p10-proxy -- \
  --listen 127.0.0.1:8443 \
  --backend 127.0.0.1:8000 \
  --cert certs/server.crt --key certs/server.key

# Test:
curl -k https://127.0.0.1:8443/
# Shows the same content as http://127.0.0.1:8000/ — but encrypted!

# Verify TLS:
echo | openssl s_client -connect 127.0.0.1:8443 2>/dev/null | grep -E "Protocol|Cipher"
```

### Step 4: Add logging

```rust
async fn proxy_connection(mut client: TlsStream<TcpStream>, backend_addr: &str, addr: SocketAddr) {
    let (_, conn) = client.get_ref();
    let proto = conn.protocol_version().map(|p| format!("{p:?}")).unwrap_or("?".into());
    let cipher = conn.negotiated_cipher_suite().map(|c| format!("{c:?}")).unwrap_or("?".into());

    let mut backend = TcpStream::connect(backend_addr).await.unwrap();
    println!("[{addr}] connected ({proto}, {cipher})");

    let result = tokio::io::copy_bidirectional(&mut client, &mut backend).await;
    match result {
        Ok((c2b, b2c)) => println!("[{addr}] done ({c2b} → backend, {b2c} ← backend)"),
        Err(e) => println!("[{addr}] error: {e}"),
    }
}
```

## Exercises

### Exercise 1: Basic proxy

Implement steps 1-3. Proxy HTTPS to a local HTTP server.

### Exercise 2: X-Forwarded-For header

The backend doesn't know the real client IP (it sees the proxy's IP). Inject `X-Forwarded-For` by reading the first HTTP request, adding the header, then forwarding:

```
Client → Proxy → Backend
                  sees: X-Forwarded-For: 192.168.1.100
```

This requires parsing the HTTP request before forwarding — you can't use `copy_bidirectional` directly.

### Exercise 3: SNI-based routing

Multiple backends behind one proxy, routed by domain name:

```
api.example.com  → 127.0.0.1:8001
web.example.com  → 127.0.0.1:8002
```

Read the SNI from the TLS ClientHello before completing the handshake. Route to the matching backend.

### Exercise 4: Health checks

Periodically check if backends are alive. If a backend is down, return 502 Bad Gateway instead of hanging.

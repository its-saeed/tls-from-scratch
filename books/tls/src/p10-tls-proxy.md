# Project: TLS Termination Proxy

> **Prerequisites**: Lesson 14 (tokio-rustls). Accept TLS connections, decrypt, forward plaintext to a backend.

## What you're building

A reverse proxy that handles TLS termination — like what nginx, HAProxy, or Cloudflare does for HTTPS.

```
Client (HTTPS)          Proxy                   Backend (HTTP)
  │                       │                        │
  │── TLS handshake ────►│                        │
  │                       │                        │
  │── encrypted GET / ──►│── plaintext GET / ───►│
  │                       │                        │
  │◄── encrypted resp ───│◄── plaintext resp ────│
```

```sh
# Start a plaintext backend:
python3 -m http.server 8000 &

# Start the TLS proxy:
cargo run -p tls --bin p10-proxy -- --listen 0.0.0.0:8443 --backend 127.0.0.1:8000

# Client connects with HTTPS:
curl -k https://127.0.0.1:8443/
# Gets the response from the backend, but over TLS!
```

## Why this exists

```
Without TLS proxy:
  Every backend service must:
  - Manage certificates
  - Handle TLS handshakes
  - Renew certs before expiry

With TLS proxy:
  One proxy handles ALL TLS.
  Backends are simple HTTP.
  Certs managed in one place.
```

## Implementation guide

### Core loop

```rust
let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));
let listener = TcpListener::bind(listen_addr).await?;

loop {
    let (client_tcp, _) = listener.accept().await?;
    let acceptor = tls_acceptor.clone();
    let backend = backend_addr.clone();

    tokio::spawn(async move {
        // 1. TLS handshake with client
        let mut client_tls = acceptor.accept(client_tcp).await.unwrap();

        // 2. Connect to backend (plaintext)
        let mut backend_tcp = TcpStream::connect(backend).await.unwrap();

        // 3. Pipe data both ways
        tokio::io::copy_bidirectional(&mut client_tls, &mut backend_tcp).await.unwrap();
    });
}
```

The key insight: `tokio::io::copy_bidirectional` handles both directions. Data from the client is decrypted by rustls, then forwarded as plaintext to the backend, and vice versa.

## Exercises

### Exercise 1: Basic proxy
Proxy HTTPS to a local HTTP server. Verify with `curl -k https://127.0.0.1:8443`.

### Exercise 2: Add X-Forwarded-For
Parse the HTTP request, inject `X-Forwarded-For: <client IP>` header before forwarding. This is how backends know the real client IP.

### Exercise 3: Multiple backends
Route by SNI hostname: `api.example.com` → backend A, `web.example.com` → backend B. Extract the SNI from the ClientHello before completing the handshake.

### Exercise 4: Access logging
Log each request: timestamp, client IP, method, path, response status, latency.

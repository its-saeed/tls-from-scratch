# Project: HTTPS Server

> **Prerequisites**: Lesson 8 (Certificate Generation), Lesson 14 (tokio-rustls). Serve a web page over TLS.

## What you're building

A web server that serves HTML over HTTPS — the most common use of TLS.

```sh
cargo run -p tls --bin p6-https-server
# Listening on https://127.0.0.1:8443
# Open in browser → your page, with padlock icon!
```

## Architecture

```
Browser                          Your HTTPS Server
  │                                    │
  ├── TCP connect to :8443 ───────────►│
  │                                    │
  │◄── TLS handshake ────────────────►│  rustls handles this
  │    (certificate, key exchange)     │
  │                                    │
  │── GET / HTTP/1.1\r\n ────────────►│  encrypted inside TLS
  │   Host: 127.0.0.1\r\n             │
  │   \r\n                             │
  │                                    │
  │◄── HTTP/1.1 200 OK\r\n ──────────│  your response
  │    Content-Type: text/html\r\n     │
  │    \r\n                            │
  │    <html>...</html>                │
```

## Implementation guide

### Step 1: Generate certificates

```rust
use rcgen::generate_simple_self_signed;

let cert = generate_simple_self_signed(vec!["localhost".into()])?;
let cert_pem = cert.cert.pem();
let key_pem = cert.key_pair.serialize_pem();
```

### Step 2: Configure rustls

```rust
use rustls::ServerConfig;

let certs = rustls_pemfile::certs(&mut cert_pem.as_bytes()).collect::<Result<Vec<_>, _>>()?;
let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())?.unwrap();
let config = ServerConfig::builder()
    .with_no_client_auth()
    .with_single_cert(certs, key)?;
```

### Step 3: Accept TLS connections

```rust
let acceptor = TlsAcceptor::from(Arc::new(config));
let listener = TcpListener::bind("127.0.0.1:8443").await?;

loop {
    let (tcp, addr) = listener.accept().await?;
    let acceptor = acceptor.clone();
    tokio::spawn(async move {
        let mut tls = acceptor.accept(tcp).await.unwrap();
        handle_request(&mut tls).await;
    });
}
```

### Step 4: Parse HTTP and respond

```rust
async fn handle_request(stream: &mut TlsStream<TcpStream>) {
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).await.unwrap();
    let request = String::from_utf8_lossy(&buf[..n]);

    let response = if request.starts_with("GET / ") {
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<h1>Hello from Rust HTTPS!</h1>"
    } else {
        "HTTP/1.1 404 Not Found\r\n\r\nNot Found"
    };

    stream.write_all(response.as_bytes()).await.unwrap();
}
```

## Testing

```sh
# With curl (skip certificate verification for self-signed):
curl -k https://127.0.0.1:8443/
# <h1>Hello from Rust HTTPS!</h1>

# With openssl:
openssl s_client -connect 127.0.0.1:8443 -servername localhost

# In a browser: https://127.0.0.1:8443
# You'll get a security warning (self-signed cert) — click "Advanced" → "Proceed"
```

## Exercises

### Exercise 1: Basic HTTPS server
Serve a static HTML page over TLS. Verify with `curl -k`.

### Exercise 2: Serve static files
Serve files from a directory. `GET /index.html` returns `./public/index.html`.

### Exercise 3: CA-signed certificate
Instead of self-signed, generate a CA + server cert (Lesson 8). Install the CA cert in your browser's trust store. The padlock should appear without warnings.

### Exercise 4: HTTP/1.1 keep-alive
Handle multiple requests on one connection (don't close after the first response). Parse `Content-Length` or use chunked transfer encoding.

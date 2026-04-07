# Lesson 12: Build a Simple HTTPS Client

## The full circle

In Lesson 1, you hashed a file. Now you'll connect to a real website over TLS — using every concept you've learned. When you run this program and see HTML from `example.com`, know that under the hood:

1. Your client and the server exchanged X25519 keys (Lesson 4)
2. The server's certificate was verified against a CA (Lesson 6)
3. The server signed the handshake (Lesson 3/8)
4. Session keys were derived with HKDF (Lesson 5)
5. All data is encrypted with AES-GCM or ChaCha20-Poly1305 (Lesson 2)
6. Each record has a sequence number nonce (Lesson 10)
7. The hash of the handshake transcript ties it all together (Lesson 1)

## What HTTPS actually is

HTTPS = HTTP over TLS over TCP. That's it.

```
Application:  HTTP (GET /index.html, 200 OK, headers, body)
Security:     TLS  (handshake, encrypt, authenticate)
Transport:    TCP  (reliable byte stream)
Network:      IP   (routing)
```

The HTTP protocol doesn't change at all. You send the same `GET / HTTP/1.1\r\n` request. The only difference is that the TCP stream is wrapped in TLS, so everything is encrypted.

## Server Name Indication (SNI)

One IP address can host many HTTPS websites (virtual hosting). But TLS handshake happens before HTTP, so the server doesn't know which site you want yet. **SNI** solves this: the client sends the hostname in the ClientHello (plaintext).

```rust
let server_name = "example.com".try_into().unwrap();
connector.connect(server_name, tcp_stream).await?;
//                ^^^^^^^^^^^^ sent in ClientHello
```

The server uses SNI to pick the right certificate. This is why SNI is visible to network observers — it's the one piece of metadata that leaks in TLS (Encrypted Client Hello / ECH aims to fix this).

## Root certificate stores

Your browser and OS ship with ~150 trusted root CA certificates. These are the trust anchors for the entire web.

In Rust, you have two options:
- **`webpki-roots`**: Mozilla's root store compiled into your binary. No system dependency.
- **`rustls-native-certs`**: Loads from the OS certificate store. Respects system-level CA additions/removals.

For a simple client, `webpki-roots` is easiest.

## Real-world scenarios

### What your browser does thousands of times a day

Every time you visit a website:
1. DNS lookup → IP address
2. TCP connect to port 443
3. TLS handshake (what you built in Lessons 4-8, plus certificate chain validation from Lesson 6)
4. Send HTTP request through the encrypted tunnel
5. Receive HTTP response
6. Render HTML

Your browser does this in ~100ms. The TLS handshake is typically 1 RTT (TLS 1.3) or 2 RTT (TLS 1.2).

### curl under the hood

When you run `curl https://example.com`, it does exactly what this lesson implements:
1. Connects TCP to example.com:443
2. Does a TLS handshake (using OpenSSL or rustls depending on build)
3. Sends `GET / HTTP/1.1\r\nHost: example.com\r\n\r\n`
4. Prints the response body

You're building curl's core networking in ~30 lines of Rust.

### API clients

Every REST API call over HTTPS follows this pattern. When your code calls `reqwest::get("https://api.github.com/users/octocat")`, it's doing a TLS handshake, sending HTTP, and parsing the response — exactly what you're implementing here.

## What you'll see in the output

```
HTTP/1.1 200 OK
Content-Type: text/html; charset=UTF-8
Content-Length: 1256
...

<!doctype html>
<html>
<head>
    <title>Example Domain</title>
...
```

Real HTML, fetched over real TLS, verified against real CA certificates.

## Exercises

### Exercise 1: HTTPS GET (implemented in 12-https-client.rs)
Connect to `example.com:443` over TLS, send an HTTP GET request, print the response.

### Exercise 2: Print TLS details
After the handshake, print:
- TLS protocol version (should be TLS 1.3)
- Negotiated cipher suite
- Server certificate subject name
- Certificate chain (list each cert's subject and issuer)

### Exercise 3: Try different sites
Connect to `google.com`, `github.com`, `cloudflare.com`. Compare the cipher suites and certificate chains. Some use ECDSA certificates, some use RSA. Some have 2-cert chains, some have 3.

### Exercise 4: Certificate pinning
Hardcode the expected SHA-256 fingerprint of `example.com`'s certificate. After the handshake, compute the fingerprint of the received certificate and compare. If they don't match, abort. This is certificate pinning — extra security beyond CA validation.

### Exercise 5: Connect without trusting the CA
Create a `ClientConfig` with an empty root store. Try connecting to `example.com`. The handshake should fail with a certificate verification error. This proves that the CA trust chain is enforced.

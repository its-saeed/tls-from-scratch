# Lesson 11: Real TLS with tokio-rustls

## The payoff

You've built TLS from scratch: key exchange (Lesson 4), key derivation (Lesson 5), encryption (Lesson 2), authentication (Lesson 8), replay defense (Lesson 10). Now use a production TLS library and see how everything maps.

## rustls vs OpenSSL

Two main TLS libraries in the Rust ecosystem:

| | rustls | OpenSSL (via openssl crate) |
|---|--------|---------------------------|
| Language | Pure Rust | C (with Rust bindings) |
| Safety | Memory-safe by default | Long history of CVEs (Heartbleed, etc.) |
| Performance | Competitive, sometimes faster | Hardware-accelerated AES |
| Ciphers | Modern only (TLS 1.2+) | Everything including legacy |
| Dependencies | No C toolchain needed | Requires OpenSSL headers/lib |

`rustls` is the natural choice for Rust projects. `tokio-rustls` wraps it for async I/O.

## How rustls maps to your lessons

When you call `TlsConnector::connect()`, here's what happens inside:

```
1. Client sends ClientHello
   → "I support X25519 key exchange, ChaCha20-Poly1305 encryption"       (Lessons 2, 4)

2. Server responds with ServerHello + Certificate + CertificateVerify
   → Server's DH public key                                               (Lesson 4)
   → Server's X.509 certificate                                           (Lesson 6)
   → Signature over handshake transcript                                   (Lesson 3)

3. Both sides derive keys
   → HKDF from DH shared secret                                           (Lesson 5)

4. Encrypted application data flows
   → ChaCha20-Poly1305 with counter nonces                                (Lessons 2, 10)
   → Server authenticated via certificate                                  (Lesson 8)
```

Everything you built by hand — rustls does automatically in ~2ms.

## The code change

Your Lesson 8 server/client required ~50 lines of handshake code. With tokio-rustls:

**Server:**
```rust
let acceptor = TlsAcceptor::from(Arc::new(server_config));
let (tcp_stream, _) = listener.accept().await?;
let tls_stream = acceptor.accept(tcp_stream).await?;
// tls_stream implements AsyncRead + AsyncWrite
// Just read and write plaintext — TLS handles everything
```

**Client:**
```rust
let connector = TlsConnector::from(Arc::new(client_config));
let tcp_stream = TcpStream::connect("127.0.0.1:7878").await?;
let tls_stream = connector.connect(server_name, tcp_stream).await?;
// Read and write plaintext
```

That's it. The entire handshake — DH, signatures, certificates, key derivation — happens inside `accept()` / `connect()`.

## Generating proper certificates

For Lesson 6, you generated a self-signed cert with openssl CLI. For rustls, you need:

1. A CA certificate (self-signed root)
2. A server certificate signed by that CA
3. The server's private key

You can use the `rcgen` crate to generate these in Rust, or use openssl:

```sh
# Generate CA key and cert
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout ca.key -out ca.crt -days 365 -subj "/CN=My CA"

# Generate server key and CSR
openssl req -newkey rsa:2048 -nodes \
  -keyout server.key -out server.csr -subj "/CN=localhost"

# Sign server cert with CA
openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key \
  -CAcreateserial -out server.crt -days 365
```

The client trusts `ca.crt`. The server presents `server.crt` + `server.key`.

## Real-world scenarios

### Adding TLS to any TCP service

You have a plaintext TCP service (database, cache, message broker). Adding TLS:

1. Generate certificates
2. Server: wrap `TcpListener` with `TlsAcceptor`
3. Client: wrap `TcpStream` with `TlsConnector`
4. No other code changes — the TLS stream implements the same Read/Write traits

This is how PostgreSQL, Redis, and gRPC add TLS. The application protocol doesn't change.

### The difference you can see

Run your Lesson 8 server and capture traffic:
```sh
sudo tcpdump -i lo0 port 7878 -w lesson8.pcap
```

Run your Lesson 11 server and capture traffic:
```sh
sudo tcpdump -i lo0 port 7878 -w lesson11.pcap
```

Open both in Wireshark. Lesson 8 shows your custom handshake (raw DH keys). Lesson 11 shows a standard TLS 1.3 handshake that Wireshark can parse and label: ClientHello, ServerHello, Certificate, Finished, Application Data.

## Exercises

### Exercise 1: TLS echo server (implemented in 11-real-tls-server.rs and 11-real-tls-client.rs)
Build an echo server using tokio-rustls. Generate certificates, configure server and client, verify encrypted communication works.

### Exercise 2: Inspect with openssl s_client
Start your TLS server, then connect with:
```sh
openssl s_client -connect 127.0.0.1:7878 -CAfile ca.crt
```
You'll see the full TLS handshake: protocol version, cipher suite, certificate chain. Type a message — it gets echoed through real TLS.

### Exercise 3: Cipher suite selection
Print which cipher suite was negotiated:
```rust
let (_, server_conn) = tls_stream.get_ref();
println!("Cipher: {:?}", server_conn.negotiated_cipher_suite());
```
Try configuring the server to only allow ChaCha20-Poly1305 or only AES-GCM and see what gets selected.

### Exercise 4: Compare with your hand-built TLS
Run both Lesson 8 and Lesson 11 servers. Use `time` to measure handshake latency. Compare code complexity. Think about what rustls handles that your implementation doesn't: cipher negotiation, session resumption, certificate chain validation, ALPN, SNI, etc.

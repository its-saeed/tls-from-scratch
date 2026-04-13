# Project: mTLS Service Mesh

> **Prerequisites**: Lesson 11 (mTLS), P8 (Certificate Authority).

## What is this?

In a microservice architecture, services talk to each other over the network. Without mTLS, any process that can reach the network can call any service. With mTLS, both sides prove their identity — unauthorized services are rejected.

```
┌──────────────────────────────────────────────────────────┐
│  Without mTLS:                                           │
│    Any process can call the payment service.             │
│    A compromised pod sends: POST /charge $10,000         │
│    Payment service: "OK!" (no identity check)            │
│                                                          │
│  With mTLS:                                              │
│    Payment service requires a client certificate.        │
│    Compromised pod has no valid cert → TLS handshake     │
│    fails → request never reaches the application.        │
│    Only "order-service" (with its cert) can call it.     │
└──────────────────────────────────────────────────────────┘
```

This is how Istio, Linkerd, and corporate zero-trust networks work.

## What you're building

Two services that authenticate each other:

```
┌──────────────────┐          mTLS          ┌──────────────────┐
│  Order Service   │ ◄─────────────────────► │  Payment Service │
│  cert: order.crt │  Both present certs     │  cert: payment.crt│
│  key:  order.key │  Both verify against CA │  key:  payment.key│
└──────────────────┘                         └──────────────────┘
              Both signed by: Company CA (from P8)
```

```sh
# Terminal 1 — Payment service (requires client cert):
cargo run -p tls --bin p9-mesh -- payment --port 9001 \
  --cert certs/payment.crt --key certs/payment.key --ca ca.crt

# Terminal 2 — Order service (presents its cert as client):
cargo run -p tls --bin p9-mesh -- order --payment-host 127.0.0.1:9001 \
  --cert certs/order.crt --key certs/order.key --ca ca.crt
# Connected to payment service ✓ (peer: CN=payment.internal)
# Charging $50... OK

# Terminal 3 — Unauthorized service (no cert):
cargo run -p tls --bin p9-mesh -- order --payment-host 127.0.0.1:9001 \
  --ca ca.crt
# ERROR: TLS handshake failed — client certificate required
```

## Implementation guide

### Step 1: Issue certs with your CA (from P8)

```sh
cargo run -p tls --bin p8-ca -- issue --domain order.internal
cargo run -p tls --bin p8-ca -- issue --domain payment.internal
```

### Step 2: Server that requires client cert

The key difference from P6: `with_client_cert_verifier` instead of `with_no_client_auth`:

```rust
use rustls::server::WebPkiClientVerifier;

let ca_certs = load_ca_certs("ca.crt");
let client_verifier = WebPkiClientVerifier::builder(Arc::new(ca_certs))
    .build()
    .unwrap();

let config = ServerConfig::builder()
    .with_client_cert_verifier(client_verifier)  // ← requires client cert!
    .with_single_cert(server_certs, server_key)?;
```

### Step 3: Client that presents its cert

```rust
let config = ClientConfig::builder()
    .with_root_certificates(ca_root_store)
    .with_client_auth_cert(client_certs, client_key)?;  // ← presents cert!
```

### Step 4: Extract peer identity

After the handshake, read WHO connected:

```rust
let (_, conn) = tls_stream.get_ref();
let peer_certs = conn.peer_certificates().unwrap();
let (_, cert) = X509Certificate::from_der(&peer_certs[0]).unwrap();
println!("Peer: {}", cert.subject());
// Use the subject for authorization: "Only order.internal can call /charge"
```

### Step 5: Authorization

Authentication tells you WHO. Authorization tells you WHAT they can do:

```rust
let peer_name = extract_cn(&peer_certs[0]); // "order.internal"

match (method, path) {
    ("POST", "/charge") if peer_name == "order.internal" => {
        // Allowed — order service can charge
    }
    ("POST", "/charge") => {
        // Denied — wrong service
        respond_403("Not authorized");
    }
    _ => respond_404(),
}
```

## Exercises

### Exercise 1: Two-service mTLS

Set up order + payment services. Order calls payment. Both verify certs. Show that an unauthorized service is rejected.

### Exercise 2: Service identity extraction

After the handshake, extract and log the peer's certificate CN. Use it for authorization decisions.

### Exercise 3: Three services

Add a third service (inventory). Order calls both payment and inventory. Each service only accepts calls from authorized peers:
- Payment accepts calls from: order
- Inventory accepts calls from: order
- Order accepts calls from: nobody (it's the entry point)

### Exercise 4: Certificate rotation

Reissue a service's cert (from P8). The service picks up the new cert without restarting. Other services continue to trust it (same CA).

# Project: mTLS Service Mesh

> **Prerequisites**: Lesson 11 (mTLS), P8 (Certificate Authority). Two services that authenticate each other.

## What you're building

Two microservices that communicate over mutual TLS. Each service has its own certificate issued by your CA. Unauthorized services are rejected.

```
┌──────────────────┐          mTLS          ┌──────────────────┐
│  API Service     │ ◄─────────────────────► │  DB Service      │
│  cert: api.crt   │  Both verify each other │  cert: db.crt    │
│  key:  api.key   │  using the shared CA    │  key:  db.key    │
└──────────────────┘                         └──────────────────┘
         Both signed by: My Root CA
         Unauthorized service → TLS handshake fails
```

## How it works

1. **CA setup** (from P8): generate Root CA
2. **Issue certs**: one for `api-service`, one for `db-service`
3. **Both services**: configure `rustls` with their cert + require client cert
4. **API calls DB**: both present certificates, both verify against the CA
5. **Unknown service tries to connect**: no cert from our CA → rejected

## Implementation

### Server config (requires client cert)

```rust
let config = ServerConfig::builder()
    .with_client_cert_verifier(
        WebPkiClientVerifier::builder(Arc::new(ca_root_store))
            .build()?
    )
    .with_single_cert(server_certs, server_key)?;
```

### Client config (presents its own cert)

```rust
let config = ClientConfig::builder()
    .with_root_certificates(ca_root_store)
    .with_client_auth_cert(client_certs, client_key)?;
```

## Exercises

### Exercise 1: Two services
Set up API and DB services with mTLS. API calls DB, DB responds. Both log the peer's certificate subject.

### Exercise 2: Unauthorized rejection
Try connecting a service with a cert signed by a different CA. The handshake should fail.

### Exercise 3: Service identity extraction
After the handshake, extract the peer's certificate subject name. Use it for authorization: "Only `api-service` can call the `/data` endpoint."

# Project: Certificate Authority

> **Prerequisites**: Lesson 7 (Certificates), Lesson 8 (Certificate Generation with rcgen).

## What you're building

A mini CA that can issue, list, and revoke certificates. Like a simplified version of what Let's Encrypt or your company's internal PKI does.

```sh
# Initialize the CA:
cargo run -p tls --bin p8-ca -- init
# Created ca.key + ca.crt (Root CA: "My Root CA")

# Issue a server certificate:
cargo run -p tls --bin p8-ca -- issue --domain api.internal.com
# Created api.internal.com.key + api.internal.com.crt
# Signed by: My Root CA
# Valid for: 90 days

# List issued certificates:
cargo run -p tls --bin p8-ca -- list
# api.internal.com    expires 2025-07-06   VALID
# db.internal.com     expires 2025-04-01   EXPIRED
# old-service.com     REVOKED

# Revoke a certificate:
cargo run -p tls --bin p8-ca -- revoke api.internal.com
# Revoked. Added to CRL.

# Verify a certificate:
openssl verify -CAfile ca.crt api.internal.com.crt
# api.internal.com.crt: OK
```

## Architecture

```
┌────────────────────────────────┐
│  Certificate Authority         │
│                                │
│  ca.key (private, PROTECT!)    │
│  ca.crt (public, distribute)   │
│  db.json (issued certs list)   │
│                                │
│  Commands:                     │
│    init   → create CA keypair  │
│    issue  → sign a new cert    │
│    list   → show all issued    │
│    revoke → mark as revoked    │
│    verify → check a cert       │
└────────────────────────────────┘
```

## Implementation guide

### CA initialization (with rcgen)

```rust
use rcgen::{CertificateParams, IsCa, BasicConstraints, KeyPair};

let mut params = CertificateParams::new(vec!["My Root CA".into()])?;
params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
let ca_key = KeyPair::generate()?;
let ca_cert = params.self_signed(&ca_key)?;

std::fs::write("ca.key", ca_key.serialize_pem())?;
std::fs::write("ca.crt", ca_cert.pem())?;
```

### Issue a certificate

```rust
let mut params = CertificateParams::new(vec![domain.into()])?;
params.is_ca = IsCa::NoCa;
let server_key = KeyPair::generate()?;
let server_cert = params.signed_by(&server_key, &ca_cert, &ca_key)?;
```

### Certificate database

Store issued certs in a JSON file:
```json
{
  "issued": [
    { "domain": "api.internal.com", "serial": "001", "expires": "2025-07-06", "revoked": false },
    { "domain": "old-service.com", "serial": "002", "expires": "2025-01-01", "revoked": true }
  ]
}
```

## Exercises

### Exercise 1: Basic CA
Implement init + issue. Verify with `openssl verify -CAfile ca.crt server.crt`.

### Exercise 2: Use with tokio-rustls
Start an HTTPS server using your CA's cert. Configure a client to trust your CA. Handshake should succeed.

### Exercise 3: Revocation
Add revoke + list. Implement a simple CRL (Certificate Revocation List) that clients can check.

### Exercise 4: Intermediate CA
Create Root CA → Intermediate CA → Server cert (3-level chain). Verify the full chain with `openssl verify -CAfile root.crt -untrusted intermediate.crt server.crt`.

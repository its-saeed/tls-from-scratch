# Lesson 8: Certificate Generation (rcgen)

> **Prerequisites**: Lesson 7 (Certificates & Trust). You understand what certificates contain and how trust chains work. Now create them in code.

## Real-life analogy: the notary's office

In Lesson 7, you learned to **read** certificates (like reading a passport). Now you're the notary — you **issue** them.

```
Passport office (Certificate Authority):
  1. Applicant proves identity ("I own example.com")
  2. Officer creates a document (certificate)
     - Name: example.com
     - Photo: public key
     - Issued by: Passport Office
     - Valid until: 2027-01-01
  3. Officer stamps and signs it (CA signature)
  4. Applicant uses the passport everywhere

You're building the passport office.
```

## What we're building

Instead of using the `openssl` CLI to generate certificates (like in Lesson 7), we'll do it entirely in Rust using the `rcgen` crate.

```
openssl CLI (Lesson 7):           rcgen (this lesson):
  openssl req -x509 ...             let cert = generate_simple_self_signed()?;
  → writes PEM files                → returns DER bytes + key pair
  → manual, external tool           → programmatic, in your application
```

## The certificate hierarchy

```
┌──────────────────────────────────────────┐
│  Root CA Certificate (self-signed)       │
│  Subject: "My Root CA"                   │
│  Issuer:  "My Root CA" (same = self)     │
│  Key:     CA key pair                    │
│  Can sign: other certificates (CA:TRUE)  │
└─────────────────┬────────────────────────┘
                  │ signs
                  ▼
┌──────────────────────────────────────────┐
│  Server Certificate                      │
│  Subject: "localhost"                    │
│  Issuer:  "My Root CA"                   │
│  Key:     server key pair                │
│  Can sign: nothing (CA:FALSE)            │
│  SANs: localhost, 127.0.0.1              │
└──────────────────────────────────────────┘
```

The root CA signs the server certificate. Clients trust the root CA → they trust the server certificate.

## rcgen basics

```rust
use rcgen::generate_simple_self_signed;

// Simplest: self-signed cert for localhost
let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
let cert = generate_simple_self_signed(subject_alt_names)?;

// Get the PEM-encoded certificate and key
let cert_pem = cert.cert.pem();
let key_pem = cert.key_pair.serialize_pem();

println!("Certificate:\n{}", cert_pem);
println!("Private key:\n{}", key_pem);
```

## Building a CA

A CA certificate needs special flags:

```rust
use rcgen::{CertificateParams, IsCa, BasicConstraints, KeyPair};

let mut ca_params = CertificateParams::new(vec!["My Root CA".to_string()])?;
ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);

let ca_key = KeyPair::generate()?;
let ca_cert = ca_params.self_signed(&ca_key)?;

println!("CA cert:\n{}", ca_cert.pem());
```

## Signing a server certificate with the CA

```rust
let mut server_params = CertificateParams::new(vec![
    "localhost".to_string(),
    "127.0.0.1".to_string(),
])?;
server_params.is_ca = IsCa::NoCa;

let server_key = KeyPair::generate()?;
let server_cert = server_params.signed_by(&server_key, &ca_cert, &ca_key)?;

println!("Server cert (signed by CA):\n{}", server_cert.pem());
```

## The full chain

```
Client connects to server:
  1. Server sends: [server_cert]
  2. Client checks: who signed server_cert? → "My Root CA"
  3. Client checks: do I trust "My Root CA"? → looks in trust store
  4. Client has ca_cert in trust store → YES → connection trusted
```

## Try it yourself

```sh
# Inspect an rcgen-generated cert (save PEM output to cert.pem first):
openssl x509 -in cert.pem -text -noout | head -20

# Verify a certificate chain:
openssl verify -CAfile ca.pem server.pem
# Should print: server.pem: OK

# See the chain relationship:
openssl x509 -in server.pem -text -noout | grep -A2 "Issuer"
openssl x509 -in ca.pem -text -noout | grep -A2 "Subject"
# Issuer of server.pem should match Subject of ca.pem
```

## Subject Alternative Names (SANs)

Modern TLS uses SANs instead of the CN field:

```
Certificate for a web server:
  SANs: DNS:example.com, DNS:www.example.com, IP:93.184.216.34

Certificate for localhost development:
  SANs: DNS:localhost, IP:127.0.0.1
```

```sh
# Check SANs of a real website:
echo | openssl s_client -connect google.com:443 2>/dev/null | \
  openssl x509 -noout -ext subjectAltName
```

## When you need this

- **Development**: generate self-signed certs for local HTTPS
- **Testing**: create CA + server certs for integration tests
- **Internal infrastructure**: issue certs for microservices
- **The intercepting proxy**: generate certs on-the-fly for any domain

## Exercises

### Exercise 1: Self-signed certificate

Use `rcgen::generate_simple_self_signed` to create a cert for localhost. Save the PEM to a file. Inspect it with `openssl x509 -in cert.pem -text -noout`.

### Exercise 2: CA + server certificate

1. Generate a CA certificate (with `is_ca = IsCa::Ca(...)`)
2. Generate a server certificate for "localhost"
3. Sign the server cert with the CA
4. Verify: `openssl verify -CAfile ca.pem server.pem`

### Exercise 3: Use with tokio-rustls

Take the CA + server cert from Exercise 2:
1. Server: load server cert + key into `rustls::ServerConfig`
2. Client: load CA cert into `rustls::ClientConfig` root store
3. Connect — the handshake should succeed
4. Change the server cert's SAN to "notlocalhost" — handshake should fail

### Exercise 4: Certificate inspector

Connect to a real website. After the TLS handshake, extract the peer's certificate chain. Parse each with `x509-parser` and print: subject, issuer, SANs, validity. Compare with `openssl s_client -connect <host>:443`.

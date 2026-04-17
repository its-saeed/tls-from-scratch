# Lesson 8: Certificate Generation (rcgen)

> **Alice's Bookstore — Chapter 8**
>
> Alice's bookstore is thriving. She now runs 12 microservices: inventory, payments, shipping, notifications, and more. Each one needs its own TLS certificate.
>
> *"I can't run openssl 12 times every time I deploy. And when I add a new service, I have to manually generate a cert, copy it to the right server... it's a nightmare."*
>
> Bob: *"You need to generate certificates from code. Your deployment script creates a cert automatically when a new service starts. No manual steps."*
>
> *"But can I just... make my own certificates? Don't I need Let's Encrypt?"*
>
> *"For internal services — services that talk to each other, not to the public internet — you create your own CA and issue your own certs. You're the passport office for your own company."*

> **Prerequisites**: Lesson 7 (Certificates & Trust). You understand what certificates contain and how trust chains work. Now create them in code.

## Why generate certificates in code?

In Lesson 7, you used `openssl` on the command line to create certificates. That works for a one-time setup. But what about:

```
Situations where you can't use the openssl CLI:
  
  Integration tests:
    "I need fresh certs every time tests run"
    → can't ask developers to run openssl manually before each test
  
  Dynamic services:
    "A new microservice spins up and needs a cert immediately"
    → can't wait for a human to run openssl
  
  The intercepting proxy (Project P9):
    "Client connects to google.com — I need a fake cert for google.com NOW"
    → must generate a cert in milliseconds, for any domain, on the fly
  
  Embedded devices:
    "IoT device boots up and needs a unique cert"
    → no openssl installed on the device
  
  CI/CD pipelines:
    "Build server needs TLS certs for staging environment"
    → must be automated, no manual steps
```

In all these cases, you need to generate certificates **programmatically** — from your Rust code, not from a terminal.

## Real-life analogy: printing your own ID badges

```
Lesson 7 (openssl CLI):
  You went to the government office.
  You waited in line.
  An officer printed your passport.
  One passport at a time, manual process.

This lesson (rcgen):
  You bought a badge printer for your company.
  Your software prints employee badges automatically.
  New employee joins → badge printed in seconds.
  No waiting, no manual work, any name/department.

  ┌────────────────────┐     ┌──────────────────────────┐
  │  Government office │     │  Your badge printer      │
  │  (openssl CLI)     │     │  (rcgen in Rust)         │
  │                    │     │                          │
  │  Manual            │     │  Automatic               │
  │  One at a time     │     │  Any quantity             │
  │  Slow              │     │  Instant                  │
  │  External tool     │     │  Part of your program     │
  └────────────────────┘     └──────────────────────────┘
```

## What we're building

A Rust program that creates certificates — the same certificates that `openssl` creates, but from code:

```rust
// One line to create a self-signed cert:
let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
// That's it. Same result as: openssl req -x509 -newkey rsa:2048 ...
```

## Two types of certificate

There are only two types, and the difference is one flag:

```
CA certificate (is_ca = true):
  "I am allowed to sign OTHER certificates"
  Like a notary — can stamp other documents
  Usually self-signed (signs itself)
  Installed in trust stores

Server certificate (is_ca = false):
  "I am a specific server (e.g., localhost)"
  Like an employee badge — identifies one entity
  Signed BY a CA
  Presented during TLS handshake
```

That's the entire difference. A CA cert has `is_ca = true`, which means it can sign other certs. A server cert has `is_ca = false`, which means it can't.

## The certificate hierarchy

```
┌──────────────────────────────────────────┐
│  Root CA Certificate (self-signed)       │
│  Subject: "My Root CA"                   │
│  Issuer:  "My Root CA" (same = self)     │
│  is_ca:   TRUE ← can sign other certs   │
│  Key:     CA key pair                    │
└─────────────────┬────────────────────────┘
                  │ signs (using CA's private key)
                  ▼
┌──────────────────────────────────────────┐
│  Server Certificate                      │
│  Subject: "localhost"                    │
│  Issuer:  "My Root CA" ← who signed me  │
│  is_ca:   FALSE ← cannot sign other certs│
│  Key:     server key pair (different!)   │
│  SANs:    localhost, 127.0.0.1           │
└──────────────────────────────────────────┘

The CA and server have DIFFERENT key pairs.
The CA signs the server cert with the CA's private key.
Clients verify the signature with the CA's public key.
```

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

## How trust works (the full chain)

```
Step 1: You generate a CA cert + server cert (this lesson)

Step 2: You install the CA cert on the CLIENT machine
  (add to browser trust store, or load in rustls root store)

Step 3: Client connects to server over TLS:

  Server sends:     "Here's my cert: localhost, signed by My Root CA"
                          │
  Client asks:      "Do I trust My Root CA?"
                          │
  Client checks     ┌─────▼─────────────────────────┐
  trust store:      │ Trusted CAs:                   │
                    │   DigiCert         ← no        │
                    │   Let's Encrypt    ← no        │
                    │   My Root CA       ← YES! ✓    │
                    └────────────────────────────────┘
                          │
  Client verifies:  Is the signature on the server cert valid?
                    (verify using My Root CA's public key)
                          │
                          ✓ → connection trusted
```

Without Step 2 (installing the CA cert), the client would reject the connection — it doesn't know "My Root CA".

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

## Why do we pass domain names to `CertificateParams::new()`?

You might have noticed: when we created certificates above, we passed domain names like `"localhost"` and `"127.0.0.1"` to `CertificateParams::new()`. What does `rcgen` do with them?

It puts them in the **Subject Alternative Names (SANs)** field — the list of domains and IPs this certificate is valid for. When a browser connects to `https://localhost`, it checks the SANs (not the Subject/CN field) to verify the hostname matches.

```
Old way (deprecated):
  Subject: CN=localhost          ← browsers used to check this
  SANs: (empty)

Modern way (what rcgen does):
  Subject: CN=localhost          ← mostly ignored by browsers now
  SANs: DNS:localhost, IP:127.0.0.1  ← THIS is what browsers check
```

If your cert's SANs don't include the hostname you're connecting to, the browser rejects it — even if the CN matches. That's why we pass the domain names when creating the cert.

A single cert can cover multiple domains:

```rust
// One cert for all your domains:
let params = CertificateParams::new(vec![
    "example.com".into(),
    "www.example.com".into(),
    "api.example.com".into(),
    "127.0.0.1".into(),
])?;
// rcgen automatically creates SANs for all of these
```

```sh
# See SANs of a real website — Google's cert covers dozens of domains:
echo | openssl s_client -connect google.com:443 2>/dev/null | \
  openssl x509 -noout -ext subjectAltName
# DNS:*.google.com, DNS:google.com, DNS:*.youtube.com, ...
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

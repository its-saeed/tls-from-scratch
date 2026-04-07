# Lesson 6: Certificates and Trust (X.509)

## The missing piece

You can now exchange keys (Lesson 4), derive encryption keys (Lesson 5), and encrypt data (Lesson 2). But there's a fatal flaw: **how does the client know it's talking to the real server?**

## The man-in-the-middle attack

Without authentication, an attacker (Mallory) sits between Alice and Bob:

```
Alice ←──DH──→ Mallory ←──DH──→ Bob
       key_1             key_2
```

- Alice thinks she did DH with Bob. She actually did DH with Mallory → `key_1`
- Bob thinks he did DH with Alice. He actually did DH with Mallory → `key_2`
- Mallory decrypts Alice's messages with `key_1`, reads them, re-encrypts with `key_2`, sends to Bob
- Neither Alice nor Bob detects anything wrong

All the encryption in the world doesn't help if you're encrypting to the wrong person.

## Certificates: binding identity to public keys

A certificate is a signed document that says:

```
┌──────────────────────────────────────┐
│ X.509 Certificate                    │
│                                      │
│ Subject:    server.example.com       │
│ Public Key: 0x3a8f7b...             │
│ Issuer:     Let's Encrypt            │
│ Valid:      2024-01-01 to 2025-01-01 │
│ Serial:     12345                    │
│                                      │
│ Signature:  0xab12... (signed by     │
│             issuer's private key)    │
└──────────────────────────────────────┘
```

The issuer (Certificate Authority) vouches: "I verified that the entity controlling `server.example.com` holds the private key corresponding to public key `0x3a8f7b...`."

## Chain of trust

Who vouches for the CA? Another CA, all the way up to a **Root CA**:

```
Root CA (pre-installed on your OS — Apple, Google, Mozilla maintain these lists)
  │
  └─ signs → Intermediate CA certificate (e.g., Let's Encrypt R3)
               │
               └─ signs → Server certificate (e.g., example.com)
```

Your browser/OS ships with ~150 trusted root CA certificates. When a server presents its certificate:

1. Read the server certificate → signed by Intermediate CA
2. Read the Intermediate CA certificate → signed by Root CA
3. Root CA is in the trusted store → **chain is valid**
4. Verify the server certificate's subject matches the hostname you're connecting to

If any link breaks — wrong signature, expired cert, hostname mismatch — the connection is rejected.

## Self-signed certificates

A self-signed certificate signs itself — it's both the subject and the issuer. No chain of trust; the client must explicitly trust it.

Used for:
- Development and testing
- Internal infrastructure (company VPNs, private services)
- Scenarios where you control both client and server

This is analogous to WireGuard: you manually exchange public keys rather than using a CA hierarchy.

## Real-world scenarios

### Alice visits her bank's website

1. Alice navigates to `https://bank.com`
2. Bank's server sends its certificate: "I am bank.com, here's my public key, signed by DigiCert"
3. Alice's browser checks:
   - Is DigiCert's certificate in the trusted root store? **Yes**
   - Does DigiCert's signature on bank.com's certificate verify? **Yes**
   - Is the certificate still valid (not expired)? **Yes**
   - Does the subject match the URL? `bank.com` == `bank.com` **Yes**
4. Browser proceeds with TLS handshake using the server's public key
5. The padlock icon appears

If Mallory tries to MITM this, she can't forge a certificate for `bank.com` — she doesn't have DigiCert's private key. She could present her own self-signed certificate, but the browser would show a scary warning.

### Bob deploys a private service

Bob runs an internal API at `api.internal.corp`. He doesn't want to (or can't) use a public CA.

1. Bob generates a self-signed CA certificate (his own private root)
2. Bob generates a server certificate for `api.internal.corp`, signs it with his CA
3. Bob installs his CA certificate on all client machines
4. Clients trust `api.internal.corp` because it chains to Bob's CA

This is common in corporate environments, Kubernetes clusters, and development setups.

### Certificate pinning (extra security)

Instead of trusting any CA to vouch for a server, the client hardcodes the expected certificate (or public key hash). Even if a CA is compromised, the attacker can't forge a pin-matching certificate.

Used by: banking apps, Signal, some browsers for critical services (Google pins its own certs in Chrome).

### The Let's Encrypt revolution

Before 2015, certificates cost money ($50-300/year) and required manual verification. Let's Encrypt automated the process:

1. You prove you control a domain (by placing a file on your web server or adding a DNS record)
2. Let's Encrypt issues a free certificate, valid for 90 days
3. Automated renewal via certbot

This made HTTPS the default for the entire web. Over 80% of web traffic is now encrypted, up from ~30% in 2014.

## Certificate formats

- **PEM**: Base64-encoded, delimited by `-----BEGIN CERTIFICATE-----`. Human-readable, used by most tools.
- **DER**: Raw binary encoding. Same data as PEM, just not base64-encoded.
- **PKCS#12 / PFX**: Bundles certificate + private key in one encrypted file. Common on Windows.

## Exercises

### Exercise 1: Parse a certificate (implemented in 6-certs.rs)
Generate a self-signed cert with openssl, read it in Rust, print subject and public key algorithm.

### Exercise 2: Certificate details
Extend the parser to also print:
- Issuer name (should equal Subject for self-signed)
- Validity dates (not before / not after)
- Serial number
- Signature algorithm

### Exercise 3: Verify the self-signature
For a self-signed certificate, the issuer's public key is the same as the subject's public key. Extract the public key and signature, and verify that the certificate's signature is valid. The `x509-parser` crate can help with this.

### Exercise 4: Download a real certificate
Use openssl to download a real website's certificate chain:
```sh
openssl s_client -connect google.com:443 -showcerts < /dev/null 2>/dev/null
```
Save the output, parse each certificate in the chain, and print the subject/issuer for each. You should see the chain: `google.com → GTS CA → GlobalSign Root`.

# Summary

[Introduction](./introduction.md)

# Phase 1: Cryptographic Building Blocks

- [Cryptography Fundamentals](./00-fundamentals.md)
- [Hashing (SHA-256)](./01-hash.md)
- [Symmetric Encryption (ChaCha20-Poly1305)](./02-encrypt.md)
- [Asymmetric Crypto & Signatures (Ed25519)](./03-sign.md)
- [Key Exchange (X25519)](./04-keyexchange.md)
- [Project: TOTP Authenticator](./p1-totp.md)
- [Project: Signed Git Commits](./p2-signed-commits.md)

# Phase 2: Putting Primitives Together

- [Key Derivation (HKDF)](./05-kdf.md)
- [Password-Based KDFs (PBKDF2/Argon2)](./06-password-kdf.md)
- [Certificates & Trust (X.509)](./07-certs.md)
- [Certificate Generation (rcgen)](./08-cert-generation.md)
- [Project: Certificate Inspector](./p3-cert-inspector.md)
- [Project: Password Manager Vault](./p4-password-vault.md)

# Phase 3: Build a Mini-TLS

- [Encrypted Echo Server](./09-echo-server.md)
- [Authenticated Echo Server](./10-echo-server.md)
- [Mutual TLS (mTLS)](./11-mtls.md)
- [Replay Attack Defense](./12-replay.md)
- [TLS Handshake Deep Dive](./13-handshake-deep-dive.md)
- [Project: Encrypted File Transfer](./p5-file-transfer.md)

# Phase 4: Real TLS

- [Real TLS (tokio-rustls)](./14-real-tls.md)
- [HTTPS Client](./15-https-client.md)
- [Project: HTTPS Server](./p6-https-server.md)
- [Project: TLS Scanner](./p7-tls-scanner.md)

# Phase 5: Capstone Projects

- [Certificate Authority](./p8-certificate-authority.md)
- [mTLS Service Mesh](./p9-mtls-service-mesh.md)
- [TLS Termination Proxy](./p10-tls-proxy.md)
- [HTTPS Intercepting Proxy](./p11-intercepting-proxy.md)

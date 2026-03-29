# Lesson 9: Mutual TLS (mTLS)

## What changes from Lesson 8

In Lesson 8, only the server proves its identity. The client is anonymous — the server has no idea who connected. This is how most HTTPS works: the server proves it's `google.com`, but Google doesn't know who you are (until you log in).

**Mutual TLS** adds client authentication: the client also has a long-term identity key pair and signs its DH public key. Now both sides verify each other before exchanging any data.

## The protocol

```
Client                                      Server
  │  (both have long-term Ed25519 keys)       │
  │                                           │
  │── client_dh_public (32 bytes) ──────────►│
  │                                           │
  │◄── server_dh_public (32 bytes) ──────────│
  │◄── server_signature (64 bytes) ──────────│  sign(server_id_key, server_dh_pub)
  │                                           │
  │  verify server signature ✓                │
  │                                           │
  │── client_signature (64 bytes) ──────────►│  sign(client_id_key, client_dh_pub)
  │── client_identity_pubkey (32 bytes) ────►│
  │                                           │
  │                    verify client signature ✓
  │                                           │
  │  (derive keys, encrypted communication)   │
```

Total handshake: 32 + 32 + 64 + 64 + 32 = **224 bytes** (was 128 in Lesson 8).

## Real-world scenarios

### Kubernetes service-to-service communication

In a Kubernetes cluster, services talk to each other over the network. How does Service A know it's really talking to Service B, and vice versa?

1. A certificate authority (often built into the service mesh like Istio or Linkerd) issues certificates to each service
2. When Service A calls Service B, both present their certificates
3. Service A verifies Service B's cert → "I'm talking to the real database service"
4. Service B verifies Service A's cert → "This request is from the authorized API service, not an attacker"
5. Only then does encrypted communication begin

Without mTLS, a compromised pod could impersonate any service.

### Corporate VPN / Zero Trust

Traditional VPNs: once you're on the network, you can access everything. Zero Trust: every connection requires mutual authentication.

1. Employee's laptop has a client certificate (often stored in a hardware TPM)
2. Corporate services require mTLS — they verify the client certificate
3. Even if an attacker gets on the corporate network, they can't access services without a valid client certificate
4. Each service also proves its identity to the client

### Banking APIs (PSD2/Open Banking)

European banking regulations (PSD2) require mutual TLS for third-party payment providers:

1. A fintech company registers with a bank and receives a client certificate
2. Every API call to the bank uses mTLS
3. The bank verifies the fintech's certificate → "This is an authorized payment provider"
4. The fintech verifies the bank's certificate → "This is the real bank, not a phishing server"
5. Financial data is only exchanged after mutual verification

## Client identity: pinned key vs certificate

In our implementation, the server needs to know the client's public key in advance (hardcoded or from a file). This is the **pinned key** model — simple but doesn't scale.

In real mTLS, the client sends a **certificate** signed by a CA that the server trusts. The server doesn't need to know every client's key — it just needs to trust the CA. This scales to thousands of clients.

```
Pinned key (our implementation):     Certificate (real mTLS):
  Server knows: [client_pubkey_1,      Server knows: [CA_pubkey]
                 client_pubkey_2,      CA signs each client's cert
                 client_pubkey_3]      Server verifies cert chain
  Doesn't scale                        Scales to thousands
```

## Exercises

### Exercise 1: Mutual authentication (implemented in 9-mtls-server.rs and 9-mtls-client.rs)
Extend Lesson 8: both sides have identity keys, both sign their DH public keys, both verify. Generate keys with `9-mtls-genkeys.rs`.

### Exercise 2: Authorized clients list
Modify the server to load a list of authorized client public keys from a file. Reject connections from unknown clients. Print which client connected.

### Exercise 3: Wrong client key
Generate a new client key but don't register it with the server. Connect — the server should reject the connection. This proves that only authorized clients can connect.

### Exercise 4: Revocation
Add a "revoked keys" file. Even if a client has a valid key, if it's in the revocation list, reject the connection. This simulates certificate revocation (CRL/OCSP in real TLS).

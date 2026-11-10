# Method map: three TLS credential fixtures

Immutable input: Game `4f0b6c62a789489fca6c37dd36d5917d096f328e`, directory `apps/game-server/tests/support/spell-save-tls-fixture/`.

These three binary files receive **full mixed semantic review in byte units**, not fictitious text-line ranges. Each complete input matches its immutable Git blob. `tls_fixture_semantics.py` and `tls-fixture-semantics.json` record the complete DER structural tree, exact SHA-256, byte count, certificate semantics and cryptographic relationship checks. No private-key integer is printed or copied to a report.

| Input | Full range | Manual semantic review and complete checks | Actual-use boundary |
|---|---:|---|---|
| `ca.der` | Bytes 1–736 | Full X.509 decoded certificate inspected: v3, self-issued Oteryn S1 Test CA, RSA 2048, SHA-256/RSA signature, 2025–2035 validity, critical CA true with pathlen 0. Complete canonical DER roundtrip, all outer TLVs consumed and self-signature verified. | Explicitly loaded as a test trust root in dispatch fixture descriptors. No production trust inference. |
| `client.der` | Bytes 1–750 | Full X.509 decoded certificate inspected: oteryn-game-test subject, same CA issuer, RSA 2048, same validity, critical CA false and the sole extended usage Client Authentication. Complete DER roundtrip and CA signature verified. No additional hidden extension. | Test client identity, not a server identity. No TLS session was run. |
| `client-key.der` | Bytes 1–1217 | Complete PKCS#8 and nested PKCS#1 TLVs validated; nine RSA integer fields have the expected structure. Cryptographic loader validates RSA; public key exactly matches client certificate; factor product and CRT relations checked. Exact canonical serialization equals original bytes. | Deliberately committed fixture key; no claim of a production secret or new vulnerability. Private parameters remain undisclosed. |

Actual manually read consumers, relative to the source root:

- `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs:1–60,125–215` builds the test scope/runtime and constructs a ProducerDescriptor using these exact three files, loopback port 1 and source.test. The fixture declaration does not establish a network server or successful handshake.
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/house_transfer.rs:1020–1080` sets up a test recovery peer. It explicitly creates an ephemeral server certificate, TLS 1.3 and no client authentication; the descriptor still uses the committed client/key pair. That peer's test setup must not be described as mutual-authentication coverage merely because client credentials are supplied.

The complete input structure and signatures/key correspondence passed a local read-only Python cryptographic check at the audit date, 2026-11-10. OpenSSL certificate decoding was inspected. Not run: Rust/native/SQL tests, TLS handshake, external service, production certificate deployment or production trust configuration. Grep-located additional fixture references receive no review credit.

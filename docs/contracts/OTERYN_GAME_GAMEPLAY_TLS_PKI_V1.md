# OTERYN_GAME_GAMEPLAY_TLS_PKI_V1

- Status: Candidate, governed by ADR-0024 (Proposed). Implementation and activation are not authorized by this document.
- Date: 2026-11-19
- Owner: Oteryn Game (gameplay transport trust)
- Related: ADR-0024, ADR-0003, ADR-0014, FND-02, `CLIENT-INSTALLER-0` §2.4, `OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §9, D172

## 1. Scope

The trust chain between the native client and a production game node on the gameplay
connection (`protocol-oteryn`, ALPN `oteryn-game/1`). Out of scope: Platform web TLS, the
Platform internal mTLS identities (node-to-Platform producers), the game database TLS, and
the preproduction development CA (D172 testing mode, unchanged).

## 2. Certificate profiles

| Item | Root | Intermediate | Leaf |
|---|---|---|---|
| Key | ECDSA P-256 | ECDSA P-256 | ECDSA P-256 |
| Signature | ecdsa-with-SHA256 | ecdsa-with-SHA256 | ecdsa-with-SHA256 |
| Validity | ≤ 10 years | ≤ 2 years | ≤ 30 days |
| Basic constraints | CA, critical | CA, pathLen 0, critical | not CA |
| Key usage | keyCertSign, cRLSign | keyCertSign, cRLSign | digitalSignature |
| Extended key usage | none | serverAuth | serverAuth only |
| Name | CN `Oteryn Gameplay Root <generation>` | CN `Oteryn Gameplay Intermediate <yyyy>` | SAN `DNS:game.oteryn.com` only |
| Name constraints | none | permitted DNS `game.oteryn.com` | n/a |
| Custody | offline (ADR-0024 Q1) | Oracle game host, Game operations only, file mode 0600 | node secrets directory |

The node serves `leaf || intermediate` as `[listener] certificate_chain_file`; the root is
never served.

## 3. Client validation

1. The release client embeds a bundle of one or two root certificates in the binary
   (`current`, and `next` during an overlap).
2. It validates with rustls' `WebPkiServerVerifier` built from that bundle only; the system
   trust store is not consulted.
3. The expected name is the login response's `tls_server_name`; the client refuses a route
   whose name is not `game.oteryn.com` in a release build.
4. `OTERYN_DEV_ROOT` is read only by development builds. A release build ignores it and
   logs nothing about it.
5. TLS 1.3 only; ALPN `oteryn-game/1` must be negotiated, as today.

## 4. Edge and node network

- `game.oteryn.com:443` is the public route. The Platform edge forwards it by SNI as raw
  TCP: no termination, no PROXY header. The route record's port is 443 and its
  `tls_server_name` is `game.oteryn.com` (World Registry, Platform contract §7.3).
- The node listener binds on the internal docker network only; nothing else publishes it.
- The node sees the edge as the peer address. Per-IP admission limits therefore apply to
  the edge as a whole; they are not a per-player control until PROXY v2 is decided.

## 5. Issuance and renewal

- Leaves are issued on the game host by Game operations tooling built on rcgen from a CSR
  (or key) generated on the host; the private key never leaves the host.
- Renewal at two-thirds of leaf life; the node picks the new chain up on its next restart
  through the existing supervisor path. A failed renewal leaves the old leaf serving until
  expiry and raises an operations alarm at 7 days remaining.
- Every issuance is logged (serial, not-before, not-after, SHA-256 of the leaf) in the
  host's operations log; the private key is never logged.

## 6. Rotation

- **Intermediate:** the owner signs a new intermediate with the offline root before the old
  one reaches 3 months of remaining life; new leaves come from it immediately. No client
  change.
- **Root:** the next root is generated offline and shipped as `next` in the client bundle at
  least one release (and at least 90 days) before the first intermediate signed by it is used.
  The old root is removed from the bundle one release after the switch.

## 7. Revocation

- No OCSP or CRL fetch by the client.
- Leaf or host-key compromise: issue a new leaf from a new host key and restart the node;
  the old leaf expires within 30 days.
- Intermediate compromise: the owner signs a new intermediate under a new name generation;
  the release client gains a deny entry for the old intermediate's SPKI hash in the next
  release; until then the 2-year intermediate remains a residual risk recorded by
  operations.
- Root compromise: ship a client release whose bundle contains only the replacement root;
  older clients stop connecting after the node switches chains (forced update).

## 8. Rollback

- A bad leaf: put the previous chain back (kept until its expiry) and restart.
- A bad client bundle: the previous client release still trusts the old root, so the node
  keeps serving a chain under the old root until the fixed release ships.
- Turning this contract off in preproduction is not needed: preproduction keeps the D172
  development root.

## 9. Acceptance evidence for the implementation (PR2)

- Client: a test that a release-profile verifier accepts a chain under the embedded root for
  `game.oteryn.com`, and rejects a chain under another root, a wrong SAN, a leaf without
  `serverAuth`, an expired leaf, and an intermediate outside its name constraint.
- Client: a test that a release build ignores `OTERYN_DEV_ROOT`.
- Tooling: a test that issued leaves match §2 exactly.
- Node: an end-to-end TLS handshake test on the issued chain with ALPN `oteryn-game/1`.

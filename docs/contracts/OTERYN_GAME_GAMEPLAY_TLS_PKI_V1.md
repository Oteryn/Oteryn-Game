# OTERYN_GAME_GAMEPLAY_TLS_PKI_V1

- Status: Candidate, governed by ADR-0024 (Proposed). Implementation and activation are not authorized by this document.
- Date: 2026-11-19
- Owner: Oteryn Game (gameplay transport trust)
- Related: ADR-0024, ADR-0003, ADR-0014, ADR-0021, FND-02, `CLIENT-INSTALLER-0` §2.4, `OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §9, D172, Platform #1980 (edge relay)

## 1. Scope

The trust chain between the native client and a production game node on the gameplay
connection (`protocol-oteryn`, ALPN `oteryn-game/1`), for the **single public endpoint**
`game.oteryn.com`. Distinct per-channel endpoints and TLS names (Platform multichannel
candidate §19.2) need an amendment of this contract before they are used.

Out of scope: Platform web TLS, the Platform internal mTLS producer identities (U15), the
game database TLS, the production login policy (U7), route-record acceptance (D3), and the
preproduction development CA (D172 testing mode, unchanged).

## 2. Certificate profiles

### 2.1 Issuance policy (enforced by the issuing tool)

| Item | Root | Intermediate | Leaf |
|---|---|---|---|
| Key | ECDSA P-256 | ECDSA P-256, a fresh key per intermediate | ECDSA P-256, a fresh key per leaf |
| Signature | ecdsa-with-SHA256 | ecdsa-with-SHA256 | ecdsa-with-SHA256 |
| Validity | ≤ 10 years | ≤ 2 years, never past the root | ≤ 30 days, never past the issuing intermediate |
| Basic constraints | CA, critical | CA, pathLen 0, critical | not CA |
| Key usage | keyCertSign, cRLSign | keyCertSign, cRLSign | digitalSignature |
| Extended key usage | none | serverAuth | exactly one: serverAuth |
| Name | CN `Oteryn Gameplay Root <generation>` | CN `Oteryn Gameplay Intermediate <yyyy>-<n>` | SAN `DNS:game.oteryn.com` only; no IP SAN |
| Name constraints | none | critical; permitted DNS `game.oteryn.com`; excluded IP `0.0.0.0/0` and `::/0` | n/a |
| Custody | owner, offline, one encrypted backup (ADR-0024) | §5.3 | node secrets directory |

The issuing tool sets every field itself. It takes no CSR and copies no requested
extension; it generates the leaf key on the game host (rcgen, current workspace features).
The permitted DNS subtree also covers names below `game.oteryn.com`; the single-name
boundary comes from this issuance policy and the client's name check (§3), not from the
constraint alone.

### 2.2 Runtime checks (enforced by the client)

Path validation checks validity dates, not maximum durations, and the embedded trust anchor
carries no root expiry. Duration limits are issuance policy (§2.1) and are proven by the
issuance tests (§9.3), not by the client.

The node serves `leaf || intermediate` as `[listener] certificate_chain_file`; the root is
never served.

## 3. Client validation

1. **Build boundary.** Trust is selected at compile time by a Cargo feature,
   `dev-gameplay-trust`, not by the Cargo profile and not by runtime configuration.
   - Player-channel artefacts are built **without** the feature. They trust only the
     embedded bundle (item 2) and never read `OTERYN_DEV_ROOT`.
   - Development and preproduction artefacts (the merge-group installer's dev channel
     included) are built **with** it and keep today's D172 behaviour.
   - The build records the feature in its build information; the player-channel packaging
     step fails if the feature is present.
2. The player build embeds a bundle of one or two root certificates in the binary
   (`current`, and `next` during an overlap, §6.2).
3. The verifier is the upstream rustls `WebPkiServerVerifier` built from that bundle only
   (no system trust store), wrapped by a Game verifier that, after a successful path:
   - requires the end-entity certificate to carry an EKU extension containing exactly
     `serverAuth` (webpki's own policy accepts an absent EKU and extra EKUs);
   - rejects the connection if any presented certificate's SPKI SHA-256 is on the embedded
     intermediate denylist (§7.2).
4. The expected name is the login response's `tls_server_name`; a player build refuses a
   route whose name is not `game.oteryn.com`.
5. The player build has no root input in its launch path: `native_entry/config.rs` does not
   require `OTERYN_DEV_ROOT`, `native_entry.rs` does not load a root file before OAuth, and
   the local login UI does not check a persisted certificate path.
6. TLS 1.3 only. The client sends the gameplay grant only after the handshake succeeds and
   ALPN `oteryn-game/1` was negotiated.

## 4. Edge and node network

- `game.oteryn.com:443` is the public route. The Platform edge (caddy-l4) relays a
  connection only when SNI is `game.oteryn.com` **and** the client offers ALPN
  `oteryn-game/1`, as raw TCP with no termination and no PROXY header, to the node listener
  at its internal address (Platform #1980: `172.18.0.11:7172`). Other HTTPS for the name is
  not relayed to the node.
- The route record's port is 443 and its `tls_server_name` is `game.oteryn.com` (World
  Registry, Platform contract §7.3). The node's configured `route_revision` equals the
  published one (runtime-status producer §9).
- The node listener publishes no host port. Other services on the same private docker
  network can still reach it; that network is part of the trusted host boundary.
- The node sees the edge as the peer address. The gameplay admission path has no per-IP
  limit today; one added later would count the edge, not players, until PROXY v2 is decided.

## 5. Issuance, activation and custody

### 5.1 Replacing the current certificate source

The Oracle supervisor today installs Caddy's ACME certificate for the game listener at
startup and on its daily refresh. PR2 replaces **both** paths with the private chain, in this
order: the private chain is installed and served first; the supervisor's ACME installation
for the game listener is removed second; the Caddy ACME site is removed last, and only once
the supervisor no longer depends on it to start the node.

### 5.2 Renewal and activation

- A host timer runs the issuing tool daily. It issues a new leaf when the served leaf has
  less than one third of its life left, retrying daily on failure.
- Install is transactional: the new key and chain are written to a new versioned directory
  and the node's configured paths switch to it with one atomic symlink rename. The pair is
  never replaced file by file.
- Activation is part of renewal: the node is restarted (through the existing supervisor
  path, outside peak hours) within 24 hours of issuance, and never later than 7 days before
  the old leaf expires.
- After activation a probe connects through the public route, completes TLS with ALPN
  `oteryn-game/1`, and compares the served leaf's SHA-256 with the issued one. A mismatch or
  a failure raises an operations alarm.
- Every issuance and activation is logged (serial, not-before, not-after, leaf SHA-256);
  private keys are never logged.

### 5.3 Intermediate key custody

- The intermediate key is a file owned by root, mode 0600, outside every path mounted into
  the node, ops or any other container; the issuing tool runs on the host as root.
- Loss or corruption of the intermediate key: the owner signs a new intermediate (fresh key)
  with the offline root within 7 days, which is inside the remaining life of the served leaf.

## 6. Rotation

### 6.1 Intermediate

The owner signs a new intermediate (fresh key) before the old one has 3 months left. New
leaves come from it at once; a leaf's not-after never passes its issuer's. No client change.

### 6.2 Root

Root rotation is gated on adoption, not on time:

1. Ship an **overlap** player build whose bundle holds `current` and `next`.
2. Raise Platform's minimum admitted client build (§7.1) to the overlap build. From then on
   every admitted client trusts both roots.
3. Switch the node to a chain under `next`.
4. Ship a **next-only** build, raise the minimum build to it, and only then retire `current`.

Player builds reach players through the existing maintenance updater, which verifies
releases against its build-pinned TUF root over Platform HTTPS, independently of gameplay
TLS. A player build with empty TUF trust bytes (`build_support/update_trust.rs` without
provisioning) is not a valid player release; the player packaging step fails on it.

## 7. Compromise

### 7.1 Minimum client build (dependency on Platform)

Platform's grant issuance refuses any login whose `client_build` is below a configured
minimum. It runs before a route or grant is returned, so a refused client never opens a
gameplay connection. Raising the minimum is the enforcement step of every revocation below.
PR2 depends on this gate existing; if Platform does not have it yet, it is requested from
Platform under its own contract before PR2 lands.

### 7.2 Responses

- **Leaf or host key:** issue a leaf from a new key and activate it (§5.2); the old leaf
  expires within 30 days. No OCSP or CRL fetch by the client.
- **Intermediate:** sign a replacement intermediate with a fresh key; ship a player build
  whose denylist (§3 item 3) holds the compromised intermediate's SPKI hash; raise the
  minimum build to it. Older builds are refused grants from then on, so the compromised
  intermediate is accepted only by clients that no longer receive routes.
- **Root:** ship a player build whose bundle holds only the replacement root, raise the
  minimum build to it, then switch the node chain. Older builds are refused grants and
  update through the TUF updater, which does not depend on gameplay trust.

## 8. Rollback

The rollback unit is the certificate chain **and its key** together. A rollback restores
only a previous pair that is not compromised, has not expired, and whose root is in the
bundle of the current minimum client build. With no such pair, the response is a new
issuance (§5.2), not a rollback. A bad player build: lower nothing; ship a fixed build and
keep the node on a chain the current minimum build trusts.

Preproduction keeps the D172 development root; nothing here changes it.

## 9. Acceptance evidence for the implementation (PR2)

### 9.1 Client

1. A player-feature verifier accepts a chain under the embedded root for `game.oteryn.com`.
2. It rejects: another root; a wrong SAN; an expired leaf; a leaf without an EKU extension;
   a leaf with only another EKU; a leaf with `serverAuth` plus another EKU; an intermediate
   outside its name constraint (constraint varied while SNI and the leaf SAN stay valid); a
   chain through a subordinate CA under a pathLen-0 intermediate.
3. Denylist: a chain through a denylisted intermediate key is rejected, and a chain through
   its replacement under the same root is accepted.
4. Rotation matrix: a current-only build against a current chain; an overlap build against
   both chains; a next-only build against a next chain and rejecting a current chain.
5. A player-feature build ignores `OTERYN_DEV_ROOT`, and a clean installation with no
   certificate file completes login.
6. A dev-feature build keeps today's D172 behaviour.
7. A failed TLS handshake and a missing ALPN `oteryn-game/1` each send no gameplay grant.
8. An install of the next player build from the previously shipped player build through the
   TUF updater.

### 9.2 Node and route

1. Readiness: the node does not report ready (boot) and stops reporting ready (runtime)
   when its served leaf is expired or expires within 24 hours, so Platform issues no grant
   for that route.
2. End-to-end on the preproduction topology: client → Platform login → Gateway route →
   edge relay → node, with the private chain, ALPN `oteryn-game/1`, and the published
   `route_revision` equal to the node's.
3. Renewal: a transactional install, an activation, and the public-route probe matching the
   issued leaf.
4. Rollback: restoring a chain without its key is refused before the node starts.

### 9.3 Issuance tool

1. Issued leaves, intermediates and the root each match §2.1 field by field, checked by an
   independent parser, including the critical name constraints.
2. A leaf's not-after never passes its issuer's.

# OTERYN_GAME_GAMEPLAY_TLS_PKI_V2

- Status: Candidate, governed by ADR-0024 (Proposed). Implementation and activation are not authorized by this document.
- Revision: 2. Supersedes the unmerged V1 candidate of Game PR #3327.
- Date: 2026-11-19
- Owner: Oteryn Game (gameplay transport trust)
- Related: ADR-0024, ADR-0003, ADR-0014, ADR-0021, FND-02, `CLIENT-INSTALLER-0` §2.4, `OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §9, D172, Platform #1980 (edge relay)

Platform #1982 merged the gate contract and design linked below. PR2 depends
on the gate being deployed.

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

The node serves `leaf || intermediate` for each active chain and omits the root.
During overlap the listener holds both pairs (§6.2).

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

**Verify in PR2.** rustls `ParsedCertificate` does not expose the leaf's EKU
list. Verify that the existing X.509 parsing dependencies expose the full
end-entity EKU extension and use it to enforce item 3.

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

- A host timer runs the issuing tool daily. Renew when the served leaf has less
  than the greater of one third of its original lifetime or 14 days remaining.
  Alarm immediately on issuance failure and retry daily.
- Write each chain and key into one immutable versioned directory and switch
  its active symlink with one atomic rename. At startup the node resolves the
  symlink once per pair, then opens both files from that directory. It checks
  that the private key matches the leaf SPKI and refuses to start on mismatch.
- Activate by restarting through the existing supervisor path outside peak
  hours within 24 hours of issuance and at least 7 days before the old leaf
  expires. The daily timer checks both deadlines and alarms if either is
  missed, including when issuance has failed and the old leaf is still served
  at the 7-day deadline.
- After activation, probe TLS through the public route with ALPN
  `oteryn-game/1` and compare the served leaf's SHA-256 with the issued one.
  Repeat after the next supervisor start and daily refresh. During overlap,
  probe each served chain. Alarm on a mismatch or a failed probe.
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

`R(A)`, `R(A+B)` and `R(B)` trust A only, both roots, and B only.
`s_AB` and `s_B` are the overlap and B-only release sequences in each update
channel. Follow [Platform gate contract] §7, planned rotation, in order.

1. Publish `R(A+B)` as the current release in the latest verified generation
   of every update channel in scope. The build sends `offer.release`. Leave
   the gate as is.
2. Serve B alongside A on every node in scope on the same listener. The player
   client sends the distinguished names of its embedded roots in TLS 1.3
   `certificate_authorities` ([RFC 8446 §4.2.4]). Use rustls 0.23
   `ResolvesServerCert` with `ClientHello::certificate_authorities()` on the
   server. In the pinned rustls client config, the Game
   `ServerCertVerifier::root_hint_subjects()` supplies the extension. Select
   B when its root matches, including when both roots match. For absent or
   unmatched hints, default to A until step 6. [Platform gate design] §4, option 1.
3. Prove that `R(A+B)` completes a session with each chain (test 12).
4. Set `enforcing` with minimum `s_AB` per update channel and
   `unknown_release = refuse`. Supply `chain_evidence` referencing step 3.
5. Wait for the grant lifetime plus clock skew. Drain nodes or use a staff
   kick so all sessions re-enter through admission. Fast reconnect bypasses
   the gate.
6. Retire A on every node in scope and make B the default.
7. Later, publish `R(B)`. Optionally raise the minimum to `s_B` under §7.1
   with evidence of B-only service (test 13).

### 6.3 Client updates

On `CLIENT_UPDATE_REQUIRED` with `update`, run the TUF updater for the
indicated update channel and sign in again after installation. Use the
channel's build-pinned TUF Root, Platform HTTPS metadata and Downloads
packages. Install the verified policy's current release only if its
sequence meets the required minimum. If update trust is unusable, show
`manual_download_url` and stop. A legacy client without `offer.release`
shows its generic update-required screen on `NATIVE_LOGIN_OFFER_UNSUPPORTED`.
[Platform gate contract] §§5–6.

Player packaging fails if the build has empty TUF trust bytes
(`build_support/update_trust.rs` without provisioning).

## 7. Compromise

### 7.1 Minimum client build (dependency on Platform)

The client sends its compiled `(update_channel, release_id, release_sequence)`
in `offer.release` after the Gateway and issuer support it. The issuer
compares integer `release_sequence` values within the same update channel.
`client_build` is diagnostic. [Platform gate contract] §§2–3.

Platform combines enforcing and maintenance records at global, World and
game-channel scopes into an effective rule for each game channel.
`maintenance` takes precedence. The effective minimum is the highest
applicable record or verified policy minimum for that update channel.
Any applicable record can refuse unknown releases. The verified policy's
`revoked_release_ids` also applies. The issuer checks maintenance after
ticket authentication and before candidate qualification. It checks the
build after candidate selection and before preparing or signing a grant,
including recovery. [Platform gate contract] §§4.1–4.3.

A reported release below the floor or revoked receives HTTP 409
`NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED` with `update`. In maintenance a client
with an older reported release receives the same refusal when a newer
current release is available. Otherwise a client with `offer.release`
receives `NATIVE_LOGIN_MAINTENANCE` with `Retry-After`. A legacy client
receives `NATIVE_LOGIN_ROUTE_UNAVAILABLE` in maintenance or
`NATIVE_LOGIN_OFFER_UNSUPPORTED` when refused as unknown. A refused request
gets no new grant and keeps its ticket unused. [Platform gate contract] §§4–5.

Platform requires a non-expired verified generation with an installable
release at or above the proposed floor and artifacts for all supported
client platforms. Writes require the current `expected_revision`.
A raise into `enforcing` requires an audited `chain_evidence` reference.
Stopping issuance with `maintenance` requires neither policy nor evidence.
[Platform gate contract] §7.

### 7.2 Responses

**Leaf or host key alone.** Issue a leaf with a fresh key and activate it
(§5.2). The old leaf expires within 30 days. The floor stays in place.
The client makes no OCSP or CRL fetch.

**Root or intermediate A.** Follow [Platform gate contract] §7, compromise
rollout, in order.

1. Set affected scopes to `maintenance` at once to stop new grants, even
   with no ready node. Account access and the updater remain available.
2. Drain the affected nodes to end sessions and fast reconnect on A.
3. Bring B into service as the only chain and prove it works. Publish
   a release that trusts B. If the roots differ, publish `R(A+B)` while both
   roots are still trusted, or `R(B)` when A's root must be removed. For a root
   compromise ship `R(B)` with the replacement root only. For an intermediate
   compromise use a fresh intermediate key and ship a build that denylists
   the compromised intermediate's SPKI (§3 item 3). It can keep the same
   root if that root is still trusted. Clients can update during maintenance
   via §6.3.
4. Move to `enforcing` with `unknown_release = refuse` and the replacement
   release's sequence as the minimum per update channel (`s_AB` or `s_B`
   for the corresponding bundle). Record step 3 in `chain_evidence` (§7.1).
   For an intermediate compromise, the floor must require the denylist
   build or later. Older builds receive no new grant.

## 8. Rollback

The rollback unit is the certificate chain **and its key** together. Restore
only an unexpired pair that the current minimum build trusts. A compromised
pair, a denylisted intermediate or a retired root also forbids restoration.
Keep the chains required by the current rollout step available. If no pair
qualifies, issue a new one (§5.2). For a bad player build, keep the floor and
serve a chain the current minimum build trusts while shipping a fixed build.

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
3. Renewal: verify the daily trigger, immediate issuance-failure alarm and daily
   activation deadline check. Verify the public-route probe matches the issued leaf
   after activation, the next supervisor start and the daily refresh.
4. Rollback: reject a chain without its key and a complete matching pair under a
   compromised or retired root before the node starts.
5. Switch the active symlink between the chain and key reads during boot. The
   node loads both from the originally resolved directory. Supply a mismatched
   key in that directory and verify startup fails before reporting ready.

### 9.3 Issuance tool

1. Issued leaves, intermediates and the root each match §2.1 field by field, checked by an
   independent parser, including the critical name constraints.
2. A leaf's not-after never passes its issuer's.

### 9.4 Joint preproduction smoke

Run tests 11–13 with Platform. [Platform gate contract] §8.

11. With a fresh attempt after planned step 4, an `R(A)` build below `s_AB` receives
    `NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED` with `update` before any grant is
    issued. The client opens no gameplay TLS connection. A pre-contract
    `R(A)` build receives `NATIVE_LOGIN_OFFER_UNSUPPORTED` and shows its
    update-required screen.
12. In planned step 3, an `R(A+B)` build completes a TLS handshake and
    gameplay session on the same listener with A as default (suppress the
    extension in the test), then with B selected by
    `certificate_authorities`. Verify the served chain in each case and
    verify that an unmatched hint also selects A.
13. After planned step 6, a preproduction `R(B)` build completes a TLS
    handshake and gameplay session. No node serves A and no `R(A)`
    session remains.

[Platform gate contract]: https://raw.githubusercontent.com/Oteryn/Oteryn-Platform/d8b2d3774c23eb773d9f7c9ae51db18464c98032/docs/contracts/OTERYN_V2_MINIMUM_CLIENT_BUILD_GATE_CONTRACT_PROPOSAL.md
[Platform gate design]: https://raw.githubusercontent.com/Oteryn/Oteryn-Platform/d8b2d3774c23eb773d9f7c9ae51db18464c98032/docs/architecture/MINIMUM_CLIENT_BUILD_GATE_DESIGN.md
[RFC 8446 §4.2.4]: https://www.rfc-editor.org/rfc/rfc8446.html#section-4.2.4

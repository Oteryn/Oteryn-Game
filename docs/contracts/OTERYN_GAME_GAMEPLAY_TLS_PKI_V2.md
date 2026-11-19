# OTERYN_GAME_GAMEPLAY_TLS_PKI_V2

- Status: Candidate, governed by ADR-0024 (Proposed). Implementation and activation are not authorized by this document.
- Revision: 2. Supersedes the unmerged V1 candidate of Game PR #3327.
- Date: 2026-11-19
- Owner: Oteryn Game (gameplay transport trust)
- Related: ADR-0024, ADR-0003, ADR-0014, ADR-0021, FND-02, `CLIENT-INSTALLER-0` §2.4, `OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §9, D172, Platform #1980 (edge relay)

Platform #1982 merged the gate proposal and design. Gate deployment and
qualification are prerequisites to ADR-0024 acceptance and U3 closure.

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
extension. It generates the leaf key on the game host. Enable rcgen's
`x509-parser` and `pem` features for the issuing tool to load the persisted
issuer with `Issuer::from_ca_cert_der` and write certificates with `Certificate::pem`.
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
   - Development and ordinary preproduction artefacts (including the merge-group
     installer's dev channel) are built **with** it and keep D172 behaviour.
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
   the local login UI does not check a persisted certificate path. Keep decoding
   the optional legacy `dev_root` in `native-login.json` and ignore its value in
   player builds. Keep strict decoding for unrelated fields.
6. TLS 1.3 only. The client sends the gameplay grant only after the handshake succeeds and
   ALPN `oteryn-game/1` was negotiated.

PR2 must parse the leaf's raw EKU DER sequence and require a single OID,
`serverAuth`. rustls `ParsedCertificate` lacks the list and x509-parser 0.18.1
deduplicates OIDs in its parsed summary.

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

**Expiry withdrawal.** At boot every served leaf must have more than 24 hours
left. At runtime, by the time any served leaf reaches 24 hours remaining,
stop ready heartbeats and new local admissions. Commit and report a newer
`ready = false` publication using the runtime-status producer §§4, 7–8.
Platform must stop issuing fresh grants for the route when it accepts the
withdrawal, or when the last ready report becomes stale, whichever comes
first. Qualify the deployed freshness bound F and its clock uncertainty
before rollout. Even if withdrawal cannot commit or reach Platform, fresh
grants must stop within F of the threshold. [Platform login contract] §§7.2–7.4.

## 5. Issuance, activation and custody

### 5.1 Replacing the current certificate source

Stage the private pair in its own versioned directory. Fence the supervisor's
startup and daily `--refresh-certificate` ACME writers, including in-flight
copies, before activating it. Switch both paths to private material as part
of the cutover. Missing private material fails closed. Verify the private
pair through both paths before removing the Caddy ACME site. [Platform supervisor]

### 5.2 Renewal and activation

- A host timer runs the issuing tool daily. Renew when the served leaf has less
  than the greater of one third of its original lifetime or 14 days remaining.
  Alarm immediately on issuance failure and retry daily.
- Write each chain and key into one immutable versioned directory and switch
  its active symlink with one atomic rename. At startup the node resolves the
  symlink once per pair, then opens both files from that directory. It checks
  that the private key matches the leaf SPKI and refuses to start on mismatch.
- Mount the pair directory read-only in the node, without a writable alias.
  Both files must satisfy `FileClass::Secret` (regular files, owner 1001 on
  Oracle, mode 0600).
- Activate by restarting through the existing supervisor path outside peak
  hours within 24 hours of issuance and at least 7 days before the old leaf
  expires. The daily timer checks both deadlines and alarms if either is
  missed, including when issuance has failed and the old leaf is still served
  at the 7-day deadline. At a missed deadline or an unsafe-chain incident,
  activate a verified replacement immediately, including during peak hours.
  If the current chain cannot safely remain in service, withdraw it and stop
  serving until a safe pair is active (§4, §7.2).
- After activation, probe TLS through the public route with ALPN
  `oteryn-game/1` and compare the served leaf's SHA-256 with the issued one.
  Repeat after the next supervisor start and daily refresh. During overlap,
  probe each served chain. Alarm on a mismatch or a failed probe.
- Every issuance and activation is logged (serial, not-before, not-after, leaf SHA-256);
  private keys are never logged.

### 5.3 Intermediate key custody

- The intermediate key is a file owned by root, mode 0600, outside every path mounted into
  the node, ops or any other container; the issuing tool runs on the host as root.
- On loss or corruption of the intermediate key, start replacement under
  the offline root immediately. Use the actual remaining safe leaf lifetime
  to decide when to withdraw. Apply the emergency path in §5.2 and treat a
  suspected compromise under §7.2.

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
   Apply boot, readiness and key-separation checks to both pairs. Include both
   loaded listener chains in the journal identity check. Runtime-status §16.2
   is pending. Before it takes effect, extend listener-key registration and
   the Game certificate set to cover both pairs.
   Fence TLS resumption across trust-policy changes. Discard old session state
   and tickets, or disable resumption, so the next connection verifies anew.
3. Prove that `R(A+B)` completes a session with each chain (test 12).
4. Record the share of recent logins below `s_AB` for each affected game
   channel, split by update channel. [Platform gate design] §5.
   Set `enforcing` with minimum `s_AB` per update channel and
   `unknown_release = refuse` after verifying every release this admits
   trusts B and preserves applicable denies (§7.2). Supply `chain_evidence`
   referencing step 3.
5. Wait for the grant lifetime plus clock skew. Drain nodes or use a staff
   kick so all sessions re-enter through admission. Fast reconnect bypasses
   the gate.
6. Retire A on every node in scope and make B the default.
7. Later, publish `R(B)`. Optionally raise the minimum to `s_B` under §7.1
   with evidence of B-only service (test 13).

### 6.3 Client updates

For login and recovery, extend the strict Gateway error decoder to accept
the exact §5 `NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED` 409 body and carry its
typed `update`. Also accept HTTP 503 `NATIVE_LOGIN_MAINTENANCE` with
`Retry-After`. Platform §5 specifies the “exact §11.1 body, with
`Retry-After`” and “a fresh sign-in after `Retry-After`”.
[Platform gate contract] §5. [Platform login contract] §11.1.

The 503 body has exactly three members. Use the request's protocol version
(3 in this example) and its canonical attempt reference when echoed, or
JSON `null` as shown here.

```json
{
  "protocol_version": 3,
  "error": { "code": "NATIVE_LOGIN_MAINTENANCE", "public_class": "TEMPORARILY_UNAVAILABLE" },
  "attempt_ref": null
}
```

On this 503, end the login or recovery attempt and honor the response's
`Retry-After` header before any fresh sign-in. Then use a fresh ticket
and attempt. Preserve the legacy `NATIVE_LOGIN_OFFER_UNSUPPORTED` screen.

Hand off the update channel, required minimum and validated manual URL to the
installed `oteryn-launcher.exe`. Pass no credentials or grants. The client
exits and releases its lifetime mutex before the launcher takes the existing
maintenance lock. The launcher uses the indicated channel, including beta.
It verifies the policy and package with its pinned channel TUF Root and
installs only the policy's current release at or above the required minimum.
On success, start a fresh sign-in with a fresh ticket and attempt. If update
trust or the installed launcher is unusable, or the verified policy's current
release is below the floor, show `manual_download_url` and stop.
[Platform gate contract] §§5–6. [Platform gate design] §6.

Player packaging validates the channel Root in the shipped launcher with the
updater's verifier. Today `apps/client/src/updater/install.rs:159` checks for
`oteryn-client.exe`, verifies the staged release and points `current.txt`
at it. It leaves the installed launcher in place.
`apps/client/src/updater.rs:110` handles explicit maintenance commands with
no forced-update handoff.

Enable `offer.release` for an installation only after installing a
handoff-capable launcher or verifying that the installed launcher supports
the handoff. If capability is absent or unverified, omit `offer.release`
and show the configured manual download URL. Qualify this check from the
previously installed launcher before enforcing a floor.

## 7. Compromise

### 7.1 Minimum client build (dependency on Platform)

The client sends its compiled `(update_channel, release_id, release_sequence)`
in `offer.release` after the Gateway and issuer support it and the installation
meets the launcher prerequisite in §6.3. The issuer compares integer
`release_sequence` values within the same update channel.
`client_build` is diagnostic. [Platform gate contract] §§2–3.

Platform combines enforcing and maintenance records at global, World and
game-channel scopes into an effective rule for each game channel.
`maintenance` takes precedence. The effective minimum is the highest
applicable record or verified policy minimum for that update channel.
Any applicable record can refuse unknown releases. The verified policy's
`revoked_release_ids` also applies. For login, the issuer checks maintenance
after ticket authentication and before candidate qualification. Filter the
qualified candidates by build before final selection, protocol 3 `channels`
construction or grant preparation, in the issuance transaction. With no
requested channel, select from the passing candidates. An explicitly requested
candidate that fails the gate, or an empty set caused by the gate, gets the
§5 refusal. [Platform gate contract] §§4.1–4.3.

For a new recovery attempt, authenticate the handle and any required device
proof. Check maintenance for the bound World after the character check.
Resolve the locator or bound target channel and check its maintenance before
testing target readiness. Apply the build rule to the qualified target
before preparing or signing a recovery grant.
Recovery keeps its resolved target. [Platform login contract] §19.4.
[Platform gate contract] §4.3 and §8 test 8.

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

**Leaf or host key alone.** Withdraw the unsafe pair immediately. Issue a
leaf with a fresh key and activate it via §5.2. The old leaf expires within
30 days. The floor stays in place.
The client makes no OCSP or CRL fetch.

**Root or intermediate A.** Follow [Platform gate contract] §7, compromise
rollout, in order.

1. Set affected scopes to `maintenance` at once to stop new grants, even
   with no ready node. Before draining, confirm effective maintenance covers
   explicit channel requests and implicit requests. Use World maintenance
   (or a covering global record) where `channel_id = null` can reach an affected
   channel. Account access and the updater remain available.
2. Drain the affected nodes to end sessions and fast reconnect on A.
   Keep them withdrawn. A committed attempt can replay its stored grant
   through reconciliation ([Platform gate contract] §4.3).
   Before declaring containment or reopening admission, allow every
   pre-maintenance login and recovery grant to age out. Wait from confirmed
   maintenance commit for the maximum grant lifetime plus accepted skew and
   clock uncertainty (FND-04 fresh-entry §7 and recovery §10). Confirm no
   compromised listener or session remains.
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

Before enforcing, verify the embedded roots and denylist of every shipped
release eligible under the effective per-channel rule. Each must trust B and
carry all applicable intermediate denies. After root compromise, each must
exclude the compromised root. Repeat the check on later publication, floor
changes and rollbacks. The floor gates new grants by reported release.
Clients can misreport it, and unupdated clients
retain their old trust. Leaf lifetime does not cap a compromised CA's
signing window. [Platform gate contract] §9.

## 8. Rollback

The rollback unit is the certificate chain **and its key** together. Restore
only an unexpired pair compatible with every admissible release. A compromised
pair, a denylisted intermediate or a retired root also forbids restoration.
Keep the chains required by the current rollout step available. If no pair
qualifies, issue a new one (§5.2). For a bad player build, keep the floor and
serve a chain compatible with every admissible release while shipping a fixed build.

Preproduction keeps the D172 development root; nothing here changes it.

## 9. Acceptance evidence for the implementation (PR2)

### 9.1 Client

1. A player-feature verifier accepts a chain under the embedded root for `game.oteryn.com`.
2. It rejects: another root; a wrong SAN; an expired leaf; a leaf without an EKU extension;
   a leaf with only another EKU; a leaf with `serverAuth` plus another EKU or two
   `serverAuth` OIDs; an intermediate outside its name constraint (constraint varied while
   SNI and the leaf SAN stay valid); a chain through a subordinate CA under a pathLen-0
   intermediate.
3. Denylist: a chain through a denylisted intermediate key is rejected, and a chain through
   its replacement under the same root is accepted.
4. Rotation matrix: a current-only build against a current chain; an overlap build against
   both chains; a next-only build against a next chain and rejecting a current chain.
5. A player-feature build ignores `OTERYN_DEV_ROOT`, and a clean installation with no
   certificate file completes login. A legacy `native-login.json` with an unreadable
   `dev_root` also completes player login.
6. A dev-feature build keeps today's D172 behaviour.
7. A failed TLS handshake and a missing ALPN `oteryn-game/1` each send no gameplay grant.
8. Start on the previously installed Windows launcher and client. Pass a real Gateway
   409 with `update` for beta. Confirm client exit, mutex release, beta installation
   from the pinned Root and a fresh sign-in. With a verified current release below
   the floor, show the download page without installing or retrying sign-in.
9. Update the client in place while retaining a launcher without handoff
   support. Verify the client omits `offer.release` and shows the manual
   download URL. Install or verify a capable launcher and confirm the
   client can send the member.
10. For Gateway login (protocols 2 and 3) and recovery (protocol 3), accept
    the exact three-member 503 body with `Retry-After`. Reject a 503 body
    with an added `update` member. Verify no updater starts and no fresh
    sign-in begins before the delay expires. After it expires, verify the
    next sign-in uses a fresh ticket and attempt.

### 9.2 Node and route

1. Readiness: a leaf at or below 24 hours blocks boot readiness. Cross that
   threshold at runtime on either chain. Verify committed withdrawal and Gateway
   refusal. With commit or delivery blocked, verify heartbeat cessation, stale
   report and no fresh grants past §4's bound.
2. End-to-end on the preproduction topology: client → Platform login → Gateway route →
   edge relay → node, with the private chain, ALPN `oteryn-game/1`, and the published
   `route_revision` equal to the node's. Repeat with a mismatched route revision
   and verify the Gateway returns no grant for that route.
3. Renewal: verify the daily trigger, immediate issuance-failure alarm and daily
   activation deadline check. Verify the public-route probe matches the issued leaf
   after activation, the next supervisor start and the daily refresh. With a
   missed deadline, verify immediate activation or withdrawal of the unsafe pair.
4. Rollback: reject a chain without its key and a complete matching pair under a
   compromised or retired root before the node starts.
5. Switch the active symlink between the chain and key reads during boot. The
   node loads both from the originally resolved directory. Supply a mismatched
   key in that directory and verify startup fails before reporting ready.
6. Verify that the journal identity check rejects reusing the B listener key.

### 9.3 Issuance tool

1. Issued leaves, intermediates and the root each match §2.1 field by field, checked by an
   independent parser, including the critical name constraints.
2. A leaf's not-after never passes its issuer's.

### 9.4 Joint preproduction smoke

PR2 must add the **Gameplay PKI joint preproduction smoke** qualification.
Run tests 11–13 through the real Gateway, Registry route and preproduction
edge in isolated qualification scopes. Build player-equivalent
artefacts with embedded qualification roots and without `dev-gameplay-trust`.
Resolve `game.oteryn.com:443` to the qualification edge in that environment.
Ordinary preproduction retains D172 trust. [Platform gate contract] §8.

11. With a fresh attempt after planned step 4, an `R(A)` build below `s_AB` receives
    `NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED` with `update` before any grant is
    issued. The client opens no gameplay TLS connection. A pre-contract
    `R(A)` build receives `NATIVE_LOGIN_OFFER_UNSUPPORTED` and shows its
    update-required screen.
12. In planned step 3, an `R(A+B)` build completes a TLS handshake and
    gameplay session on the same listener with A as default (suppress the
    extension in the test), then with B selected by
    `certificate_authorities`. Verify the served chain in each case and
    verify that an unmatched hint also selects A. Use full handshakes.
    Confirm that a ticket from before the trust transition cannot resume.
13. After planned step 6, a qualification `R(B)` build completes a full TLS
    handshake and gameplay session. No node serves A and no `R(A)`
    session remains.

Also run §9.2 test 2 and these cases through the same Gateway:

- With two eligible routes before the build filter, a sequence-42 offer
  without `channel_id` skips the first route with minimum 43. It gets the
  second and the protocol 3 list omits the first.
- Recovery with an authenticated handle and device proof refuses a below-floor
  offer on a ready target. With maintenance and no ready target, verify the
  §5 refusal for both locator and bound-channel recovery. Test a fresh
  `channel_id = null` login under World maintenance with no ready node.
- Replay a committed attempt during maintenance and prove its grant is
  unusable after the containment wait. Verify a higher release with a
  regressed bundle or denylist cannot be promoted.

[Platform gate contract]: https://raw.githubusercontent.com/Oteryn/Oteryn-Platform/d8b2d3774c23eb773d9f7c9ae51db18464c98032/docs/contracts/OTERYN_V2_MINIMUM_CLIENT_BUILD_GATE_CONTRACT_PROPOSAL.md
[Platform gate design]: https://raw.githubusercontent.com/Oteryn/Oteryn-Platform/d8b2d3774c23eb773d9f7c9ae51db18464c98032/docs/architecture/MINIMUM_CLIENT_BUILD_GATE_DESIGN.md
[Platform login contract]: https://raw.githubusercontent.com/Oteryn/Oteryn-Platform/d8b2d3774c23eb773d9f7c9ae51db18464c98032/docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
[Platform supervisor]: https://github.com/Oteryn/Oteryn-Platform/blob/d8b2d3774c23eb773d9f7c9ae51db18464c98032/deploy/oracle/game-supervisor.py#L216-L242
[RFC 8446 §4.2.4]: https://www.rfc-editor.org/rfc/rfc8446.html#section-4.2.4

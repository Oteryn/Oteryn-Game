# SEC-CLIENT-01 Client integrity and anti-bot layers

- Decision: `SECCLIENT01-LAYERED-CLIENT-INTEGRITY-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (security,
  privacy, protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the `SEC-CLIENT-01` gate of `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`
  (`REQUIRED_FOR_ALPHA`) and owner answer **5b, layered version** (#162 5929226991): (1) a signed
  client and a challenge at login, (2) input telemetry, (3) silent flagging with ban waves instead
  of instant kicks, (4) no kernel driver and no third-party vendor; the alpha gets layers 1 and 2,
  open beta adds in-game challenges and layer 3; the browser gets telemetry and server analysis
  only; telemetry carries statistical summaries, never typed text or anything outside the game
  window; the protocol reserves a place for client attestation now.
- Builds on: FND-02 §9, §11, §20 (capabilities, `ClientBootstrap`, privacy); FND-04 and FND-04A-C
  ("no kernel driver, invasive anti-cheat or mandatory device fingerprint"); ADMIT-0 (the capability
  predicate and `ADMISSION_CAPABILITY_REQUIRED`); ANL-01 and ANL-03 (read-only
  detectors, hypothesis signals, human review, privacy classes, optional diagnostics are
  non-adverse); `CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md`; ADR-0006; ADR-0018 (browser,
  proposed); MOVE-RL-11 (server-side visibility); the horizon gates `PROD-COMPAT-01`,
  `EXP-UPDATE-01`, `OPS-GM-01`, `DATA-PRIVACY-01`; WRITE-0 §6 (reports).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on | Phase |
|---|---|---|---|---|
| SEC-REL-1 | hard (security), security review | the signed client release manifest, the per-build challenge corpus and the Game-side trust store with its revision floor and revocation (§3) | PROD-COMPAT-01's release train | alpha |
| SEC-CHAL-1 | hard (protocol, security), protocol and security review | capability `CLIENT_INTEGRITY_V1`: the login challenge and its evaluation (§4) | SEC-REL-1; ADMIT-CAP-1 | alpha |
| SEC-TELEM-1 | hard (protocol, privacy), protocol and privacy review | capability `INPUT_TELEMETRY_V1`, the summary format, ingestion as an ANL-01 security event, the retention profiles of §5.1 and their purge (§5) | ANL-01; DATA-PRIVACY-01's disclosure (production collection gate, §5.1) | alpha |
| SEC-DETECT-1 | impl, security and privacy review | ANL-03 detectors over telemetry and server-side timing; the flag queue (§6) | SEC-TELEM-1; ANL-03 | alpha (detectors), beta (queue to GM) |
| SEC-CHAL-2 | impl, security review | in-game challenges at random intervals (§4.3) | SEC-CHAL-1 | before open beta |

Tests: a native bootstrap without either capability is refused `ADMISSION_CAPABILITY_REQUIRED`; a
wrong, late or missing challenge answer never disconnects, mutates gameplay or sanctions, and
produces exactly one observation per server-issued challenge; any number of replies to unknown,
expired or answered challenge ids yields at most one coalesced observation per
`SECCLIENT01-RL-09` window and never a terminal outcome; no sanction is
proposed from challenge outcomes alone (a `WRONG` flag opens a case labelled "release possession
only"); a telemetry summary over its byte bound is dropped and
observed; no telemetry field can carry a key code, text, a screen position outside the game window
or a process list; disabling optional crash diagnostics changes nothing here; a non-empty
`client_attestation` is ignored; a build leaving the admitted set keeps its corpus until its last
session and challenge end, and a session whose corpus is absent is not challenged and yields no
observation; a rotation record at or below the floor, or signed by a release key, is rejected; a
revoked key's manifests stop being accepted; after a database restore to a revision below the trust
log head, no challenge is sent until the node has re-applied the log up to its head; an answer after
the deadline yields `MISSING` and nothing more; a second telemetry summary within
`SECCLIENT01-RL-13` of the last accepted one is not ingested and is coalesced; every artifact past
its §5.1 retention is purged; a production scope without the §5.1 gate met does not offer
`INPUT_TELEMETRY_V1`.

Later, each with its own decision: client attestation (the reserved slot, §7), a third-party
anti-cheat or kernel component (rejected for now, §9), sanctions and ban waves (OPS-GM-01).

## 1. Question

How does the server raise the cost of automation and learn which sessions are driven by a
person, without trusting the client, without invasive software, and without punishing anyone
automatically?

## 2. Facts

**PROVEN**

- Horizon gate `SEC-CLIENT-01` must decide signature verification, tamper and automation
  detection, telemetry privacy and false positives, sanctions with `OPS-GM-01`, update
  interaction, and must preserve: "client integrity signals are not authoritative gameplay truth";
  "no autonomous punishment solely from an opaque anomaly score"; "secrets and trust anchors are
  not embedded as reusable server credentials".
- FND-04, FND-04B §(privacy), FND-04C: no kernel driver, invasive anti-cheat or mandatory device
  fingerprint is required or authorized.
- FND-02 §11: `ClientBootstrap` carries a bounded `client_build_id`, "diagnostic/build identity,
  never authority"; fields 8 to 15 are reserved; transport profile 1 is native TCP + TLS 1.3.
- ANL-03 §2: detectors, signals and cases are read-only and never sanction; §4 behavioural input
  stays hypothesis-class; §14 optional client diagnostics are non-adverse.
- The server already decides everything that matters: movement, combat, items and visibility
  (MOVE-RL-11 sends only what the character can see), so a modified client gains no map, light or
  speed hack. What remains is automation (bots and macros driving a client or speaking the
  protocol).
- The official client binaries are distributed publicly. Anyone can keep a pristine copy and hash
  any region of it without running it, so no challenge over release bytes can show what code is
  actually running (§4.0).

## 3. The signed release (SEC-REL-1; layer 1)

- **Release manifest.** Every official native client build has a manifest: build id, platform,
  the hash of every executable and library, and its challenge corpus reference. The release
  pipeline signs it with an offline release key; the client distribution also carries the OS code
  signatures (Authenticode, Apple notarization), which are distribution concerns of
  `EXP-UPDATE-01`.
- **Trust store (Game).** The Game nodes hold the release signing public keys and the accepted
  manifests of the builds `PROD-COMPAT-01` admits. A manifest that fails verification is not
  accepted; its builds count as unknown.
- **Key rotation and revocation.**
  - *Signer separation:* release signing keys sign manifests only. The set of trusted release keys
    changes only through a **trust record** `{revision, previous_revision, active_keys,
    revoked_keys}`, signed by a separate offline **trust root** key that never signs manifests. The
    trust root public key is pinned in the Game node build.
  - *Ordering:* `revision` is a uint64 that rises by exactly one per record, and `previous_revision`
    must equal the node's current revision. A node applies records in order only.
  - *Rollback floor and its anchor:* the highest applied revision is persisted in the Game
    database, and each Game node build embeds the revision current at its build time. A record at
    or below the higher of the two is rejected and observed as an operational security event. The
    database can be restored, so it is not the anchor. The anchor is the release pipeline's
    **trust log**: an append-only publication of every trust record, outside the Game database
    and never restored with it.
  - *Fail closed:* at start and every `SECCLIENT01-RL-14` (1 hour), a node reads the trust log
    head and applies any missing records in order. The trust store is `VERIFIED` only while its
    revision equals the head the node last read within `SECCLIENT01-RL-14`. Otherwise it is
    `UNVERIFIED`: the log could not be read, the head is ahead, or the database revision is behind
    the log after a restore. While it is `UNVERIFIED`, the node sends no challenge and records no
    challenge outcome. A database restore therefore cannot reinstate a revoked key. Admission is
    unaffected, because the trust store feeds only challenge evidence and build admission belongs
    to PROD-COMPAT-01.
  - *Emergency revocation:* a compromised release key is listed in `revoked_keys` of the next record,
    published out of the release train. On applying it, manifests signed only by a revoked key stop
    being accepted and their builds count as unknown; PROD-COMPAT-01 decides whether those builds
    stay admitted. Challenge outcomes for them are not observed (§4.1).
  - *Trust root compromise:* the root is replaced by a Game node release with a new pinned root and
    a raised embedded floor; records signed by the old root are then rejected.
- **Challenge corpus.** For each build the pipeline extracts the code regions the challenges may
  name. The Game nodes keep these region bytes (or the build's binaries) server-side and never send
  them to clients. The corpus is derived from public binaries, so it is not a secret and not a
  credential.

## 4. Challenges (SEC-CHAL-1, SEC-CHAL-2; layer 1)

### 4.0 What a challenge proves

A correct answer proves only that the peer **can obtain the bytes of an official release build**
for the build it declared. It does not prove that this build is running, unmodified, or driven by a
person: a bot or a modified client can answer from a stored pristine copy. The layer raises the cost
of automation (a bot must track every release and map each build's regions) and nothing more. Hence:

- A challenge outcome is never adverse evidence on its own. `VALID` is never exculpatory either.
- Binding answers to the code actually running needs a separately trusted mechanism: platform
  attestation in the reserved slot (§7), under its own later decision.

This is owner answer 5b as given: client signals only raise the cost of botting, and evidence for a
ban is always server-side.

### 4.1 Capability and admission

- Capability **`CLIENT_INTEGRITY_V1`** (number reserved on #162 at allocation): one server message
  `IntegrityChallenge {challenge_id, nonce, region_selector}` and one client message
  `IntegrityResponse {challenge_id, digest}`, where `digest = SHA-256(nonce || region bytes)`.
- **Mandatory per transport profile.** ADMIT-0's predicate checks, for each session, the channel's
  effective set plus a transport profile set: for transport profile 1 (native)
  `CLIENT_INTEGRITY_V1` and `INPUT_TELEMETRY_V1`, each once the build offers it. A native bootstrap
  that does not declare them is refused `ADMISSION_CAPABILITY_REQUIRED` (a protocol requirement,
  the same refusal an outdated client gets, not a sanction). A future browser profile (ADR-0018)
  requires `INPUT_TELEMETRY_V1` only. The check is ADMIT-0's predicate at its points, unchanged: fresh
  admission after authentication, ownership and world eligibility (FND-04A step 14, and the final
  revalidation), same-session reconnect and recovery, with ADMIT-0's three codes
  (`ADMISSION_`, `RECONNECT_`, `RECOVERY_CAPABILITY_REQUIRED`). No new FND-04 step or code is
  added. Amended: ADMIT-0 §3.1 (pending on its acceptance).
- **Registration.** SEC-CHAL-1 registers the capability, its two message types with their
  `foundation`-style proto definitions and size limits, and the challenge outcome ANL-01 event in
  `PROTOCOL_OTERYN_V1_REGISTRY.json` and `GAME_EVENT_FOUNDATION_REGISTRY.json`, numbers reserved on
  #162 at allocation; SEC-TELEM-1 does the same for its capability, `InputSummaryV1` (an allowlist
  proto: only the fields of §5, so a forbidden field cannot be encoded) and its event.
- **Builds.** The corpus exists for the builds `PROD-COMPAT-01` currently admits (at most
  `SECCLIENT01-RL-07`), at most `SECCLIENT01-RL-08` per build. A client declaring an admitted old
  build is challenged against that build; one declaring a build outside the set is refused by
  `PROD-COMPAT-01`'s version rule before admission.
- **Retiring builds.** When a build leaves the admitted set, no new session can declare it, but its
  corpus is kept until the last session that declared it has ended and that session's last
  challenge has reached its terminal outcome. Retiring corpora do not count against
  `SECCLIENT01-RL-07`; their number is bounded by the sessions still open, and they are deleted at
  the latest when the node restarts.
- **No corpus, no challenge.** A session whose declared build has no corpus on its node (an accepted
  manifest is missing or its key was revoked, or the node restarted after the build retired) is not
  challenged, and no observation is recorded for it. Such cases are operational events about the
  release pipeline, never security observations about the session.

### 4.2 At login (alpha)

- Within the first `SECCLIENT01-RL-01` (5 s) after `ServerAccepted`, the node sends one challenge
  for the session's declared `client_build_id`, with a fresh random nonce and a region drawn from
  that build's corpus.
- The answer is checked against the corpus. The outcome is `VALID` or `WRONG` when the first reply
  carrying the challenge id arrives before the deadline `SECCLIENT01-RL-02` (10 s). The outcome is
  `MISSING` when the deadline passes first. A reply after the deadline is an unsolicited reply (see
  below), so the arrival time alone decides the outcome.
- **One terminal outcome per server-issued challenge.** The first reply carrying the outstanding
  challenge id, or the deadline, ends the challenge, and exactly one observation is recorded.
- **Unsolicited replies.** A reply with an unknown, expired or already answered id has no outcome
  and never counts as `WRONG`. The node only counts such replies per session; it records at most
  one coalesced `UNSOLICITED_REPLY {count}` observation per session per `SECCLIENT01-RL-09`
  window, and a final one at session end if the count is non-zero. These replies are protocol-fault
  evidence. They are also subject to FND-02's ordinary inbound message limits, which drop excess
  messages before they reach this counter.
- **The outcome never changes the session.** Nothing is refused, disconnected or slowed; the
  outcome is recorded as one ANL-01 security observation (§6). This is the owner's silent
  flagging: the bot author does not learn what gave them away.

### 4.3 In game (before open beta, SEC-CHAL-2)

The same exchange at random intervals, at most `SECCLIENT01-RL-03` per session in any sliding 60
minutes (a reconnect keeps the session's count), with the interval drawn by the node; same
outcomes, same silence. One challenge is outstanding at a time. Replies follow §4.2: one terminal
outcome per challenge, unsolicited replies coalesced. A session whose corpus is absent (§4.1) is
not challenged.

## 5. Input telemetry (SEC-TELEM-1; layer 2)

- Capability **`INPUT_TELEMETRY_V1`**: the client sends one `InputSummaryV1` every
  `SECCLIENT01-RL-04` (60 s) of play, at most `SECCLIENT01-RL-05` (2 KiB).
- **Server-owned windows.** The node, not the client, decides which summaries count. A summary is
  accepted only if at least `SECCLIENT01-RL-13` (45 s) of play has passed since the session's last
  accepted summary, or since admission for the first one. An accepted summary is ingested and
  stamped with the node's own window bounds; any time the client claims is ignored. Any other
  summary is not ingested. The node counts such summaries and records them as one coalesced
  `TELEMETRY_EXCESS {count}` observation per session per `SECCLIENT01-RL-09` window, plus a final
  one at session end. A summary is missing when no summary is accepted for twice `RL-04` of play.
- **Closed content**, statistical only, about input inside the game window: counts and fixed-bucket
  histograms of inter-press intervals for game actions (by action class, never by key code), of
  pointer movement speed and path curvature between actions, of the delay between a server event
  shown and the next game action, and the share of actions issued by hotkey, by click and by the
  client's own automation features. It never carries key codes, typed text, chat, coordinates
  outside the game window, screenshots, process or window lists, hardware identifiers or anything
  from outside the game window. Adding a field needs a new revision of this decision.
- **Server-side timing** (no client involvement, cannot be forged): the node records the same
  kinds of statistics from command arrival times and the events it sent. Detectors weigh it above
  client summaries, which a bot can fake.
- **Not optional diagnostics.** Input telemetry is a gameplay protocol obligation of the native
  and browser profiles, disclosed in the privacy policy (`DATA-PRIVACY-01`); it is separate from
  the opt-out crash diagnostics, whose non-adverse rule (ANL-03 §14) is unchanged. A missing or
  malformed summary while the capability is selected is a protocol fault, not an opted-out
  diagnostic. It is counted into the same coalesced observation as excess summaries.
- **Privacy class:** `SECURITY_SENSITIVE` (ANL-03 §14), pseudonymous by AnalyticsActorId. Retention
  follows §5.1.

### 5.1 Retention profiles (ANL-01 §16)

These profiles apply to the input summaries, the server-side timing records and the challenge
outcome events of §4. Each is bound to its events in the ANL retention registry by SEC-TELEM-1, or
by SEC-CHAL-1 for challenge outcomes.

| Element | Raw events (`SECCLIENT01-RAW-1`) | Detector signals (`SECCLIENT01-SIG-1`) |
|---|---|---|
| Purpose | Detecting automation, and calibrating those detectors. Not balance analytics, product analytics, marketing or any other model. | The same purpose. |
| Class | `SECURITY_SENSITIVE`, pseudonymous by AnalyticsActorId | `SECURITY_SENSITIVE`, pseudonymous |
| Ordinary retention (ceiling) | `SECCLIENT01-RL-06` (90 days) from collection | `SECCLIENT01-RL-10` (180 days) from creation |
| Allowed roles | The detector service (read, write signals). The security analyst role, reading one actor's rows only when a signal or case names that actor; every read is access-logged. No GM, support, product, Platform or Atlas access. | The detector service; the security analyst role; GMs only through a case (OPS-GM-01). |
| Aggregation | After retention, only calibration aggregates are kept: bucket counts over at least `SECCLIENT01-RL-11` (50) distinct actors, with no AnalyticsActorId, for at most `SECCLIENT01-RL-12` (2 years). | None. |
| Deletion and anonymisation | A daily purge deletes rows past retention. Account deletion deletes the actor's rows within 30 days, or unlinks the pseudonym from them within the same 30 days; a legal hold overrides this. | Same as raw events. |
| Export and redaction | No bulk export. A case may copy only the rows it cites, and the copy then follows the case profile. A data subject request returns the player's own summaries in their closed field form. | A case copies the signals it cites. |
| Legal hold | Only by an explicit hold record naming who set it, the reason, the scope and an expiry. The record is audited, and the held rows are purged at expiry. | Same as raw events. |
| Rollout and rollback | Profile revision 1. A revision that adds purpose, a role, a field or retention needs a new revision of this decision. A shorter retention applies at the next purge. A rollback never keeps a row longer than the profile in force when it was collected allows. | Same as raw events. |

**The other artifacts (ANL-03 §14.6).** These have the same purpose, class, legal hold and rollout
rules as `SECCLIENT01-RAW-1`, and are purged by the same daily purge:

| Artifact | Profile | Ordinary retention (ceiling) | Roles and limits |
|---|---|---|---|
| Detector features (per-actor values derived from raw events) | `SECCLIENT01-FEAT-1` | the shorter of the source raw rows' retention and `SECCLIENT01-RL-06` | detector service only; deleted with their source rows on account deletion |
| Signal dispositions (the team's or a GM's audited verdict on a signal, including alpha review without a case) | `SECCLIENT01-DISP-1` | `SECCLIENT01-RL-15` (1 year) from the disposition | security analyst role writes; detector service reads them for calibration only, without AnalyticsActorId once the signal is purged |
| Case evidence | `OPS-GM-01`'s case profile | per that profile | **Prohibited until that profile is accepted**: no case is opened and nothing is copied into one |
| Identity-resolution logs (each mapping of an AnalyticsActorId to an account or character, with who, why and which signal or case) | `SECCLIENT01-IDRES-1` | `SECCLIENT01-RL-16` (2 years) | written by the resolution service; read only by the security audit role; resolution is allowed only for a signal under review or a case |
| Exports (data subject responses; nothing else) | `SECCLIENT01-EXPORT-1` | the response copy is kept `SECCLIENT01-RL-17` (30 days) after delivery | prepared by the privacy role and delivered to the player only; any other export is prohibited |

**Production collection gate.** A production scope opens the offer gate of `INPUT_TELEMETRY_V1`,
which also turns on the requirement of §4.1 for that scope, only when both of these hold:

- every profile of §5.1 except the case profile is bound in the ANL retention registry:
  `RAW-1`, `FEAT-1`, `SIG-1`, `DISP-1`, `IDRES-1` and `EXPORT-1`;
- `DATA-PRIVACY-01`'s disclosure of this telemetry is published.

Until then the capability is not offered there, nothing is collected, and native clients are not
refused for lacking it. Challenges have the same kind of gate: until `SECCLIENT01-RAW-1` is bound
to the challenge outcome events, a production scope does not offer `CLIENT_INTEGRITY_V1` and sends
no challenge.

## 6. Flags (SEC-DETECT-1; layer 3)

- Challenge outcomes, telemetry summaries and server timing are ANL-01 security events. ANL-03
  detectors turn them into **hypothesis signals**: never sanctions, never gameplay mutations
  (ANL-03 §2).
- **Challenge outcomes are never sanction evidence on their own (§4.0).** A `WRONG` outcome flags
  the character for review, as owner answer 5b says. The case is labelled "release possession
  only", and a GM cannot sanction from it: a sanction needs server-side evidence, meaning server
  timing or behaviour the server observed. `MISSING` only adds context to a case opened for
  another reason. `VALID` never clears anyone.
- **Alpha:** detectors run and calibrate; signals are reviewed by the team, no ban waves.
- **Before open beta:** signals above a detector's threshold open a case in the `OPS-GM-01` queue
  with its evidence; a GM decides, and sanctions are applied in waves on a schedule
  `OPS-GM-01` sets, not at detection time. Player Rule Violation reports (WRITE-0 §6, the GM tools
  decision) feed the same queue.
- No signal ever disconnects, refuses or penalizes on its own.

## 7. Attestation slot (FND-02, FND-04; reserved now)

- `ClientBootstrap` field 8 (inside today's `reserved 8 to 15`; 9 to 15 stay reserved) is set
  aside for `bytes client_attestation`: SEC-CHAL-1 adds it to `foundation.proto`. In v1 it has no
  meaning: a non-empty value is ignored (never authority, never a refusal) and recorded as one
  observation, so no new error code is needed. The grant claim name
  `client_attestation_ref` is set aside for the same later decision; today it is an unknown claim,
  which the grant profile's exact claim membership already refuses as `ADMISSION_GRANT_MALFORMED`.
  Amended: FND-02 §11.
- A later decision may fill them with platform attestation (for example a TPM or Apple App Attest
  quote) under its own review.

## 8. Rows

| Row | Value |
|---|---|
| `SECCLIENT01-RL-01` login challenge sent after `ServerAccepted` | within 5 s |
| `SECCLIENT01-RL-02` challenge answer deadline | 10 s |
| `SECCLIENT01-RL-03` in-game challenges per session per hour | 6 (beta) |
| `SECCLIENT01-RL-04` telemetry summary period | 60 s of play |
| `SECCLIENT01-RL-05` telemetry summary size | 2 KiB; larger is dropped and observed |
| `SECCLIENT01-RL-06` raw telemetry retention | 90 days |
| `SECCLIENT01-RL-07` builds with a corpus | 8 (the builds PROD-COMPAT-01 admits) |
| `SECCLIENT01-RL-08` corpus per build | 64 MiB |
| `SECCLIENT01-RL-09` coalescing window for unsolicited replies | 10 minutes |
| `SECCLIENT01-RL-10` detector signal retention | 180 days |
| `SECCLIENT01-RL-11` minimum distinct actors in a calibration aggregate | 50 |
| `SECCLIENT01-RL-12` calibration aggregate retention | 2 years |
| `SECCLIENT01-RL-13` minimum play between accepted telemetry summaries | 45 s |
| `SECCLIENT01-RL-14` trust log check interval and freshness | 1 hour |
| `SECCLIENT01-RL-15` signal disposition retention | 1 year |
| `SECCLIENT01-RL-16` identity-resolution log retention | 2 years |
| `SECCLIENT01-RL-17` data subject response copy retention | 30 days |

Each with max and max+1 tests.

## 9. Rejected options

- **Refusing or kicking on a failed challenge.** The owner chose silent flagging; instant feedback
  teaches bot authors and punishes honest players with odd setups.
- **Kernel drivers or a vendor (BattlEye, EasyAntiCheat).** Owner answer 5b; also FND-04's
  prohibition; Linux and macOS problems; privacy.
- **Trusting the client's integrity answer as authority, or as integrity evidence.** Horizon
  "must preserve"; the server stays authoritative. The answer proves only access to the release
  bytes (§4.0).
- **Device fingerprints.** FND-04 forbids a mandatory one.

## 10. Architect rulings (owner answer 5b; owner rule 5905825574)

- **R1. Missing capability on a native bootstrap.** a) Refuse as an outdated client (recommended:
  a protocol rule, not a sanction); b) admit and flag. **Ruled a).**
- **R2. Telemetry opt-out.** a) No opt-out: a disclosed gameplay obligation with closed,
  statistical content (recommended: an opt-out would be the bot's switch); b) opt-out like crash
  diagnostics. **Ruled a).**

## 11. Owner questions

None. Owner answer 5b set the layers, the timing and the privacy limits.

## 12. Decision test

- **Must decide now:** YES. The horizon marks the gate `REQUIRED_FOR_ALPHA`; the alpha needs layers
  1 and 2 and the reserved slot.
- **Minimum sufficient:** two capabilities, one signed manifest with a trust record chain, ANL
  events with their retention profiles, and ANL-03 detectors; no new authority.
- **Superseding evidence:** measured bot prevalence after beta that the layers do not contain.
- **Deliberately not decided:** attestation, vendors, sanctions (OPS-GM-01).

## 13. Before-freeze checklist

1. **Contract amendments:** FND-02 §11 (field 8 set aside); ADMIT-0 §3.1 (a transport profile
   set). Applied in this PR. Registry, proto and event registrations are the children's (§4.1).
2. **Serialization:** challenges and summaries are session messages; outcomes are events, never
   state changes.
3. **Restart:** nothing durable but ANL events and the trust record floor; after a restart or a
   database restore the trust store is `UNVERIFIED` until the trust log is re-read; a challenge in flight at
   a restart is `MISSING` and ignored by detectors (a known restart window); retiring corpora are
   dropped and their sessions are no longer challenged.
4. **Typed references:** build ids from signed manifests; AnalyticsActorId for analytics.
5. **Wire:** two capabilities, three messages, one reserved field.
6. **Split work:** none.

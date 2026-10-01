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
  ("no kernel driver, invasive anti-cheat or mandatory device fingerprint"); ADMIT-0 (PR #1440, the
  mandatory capability floor and `ADMISSION_CAPABILITY_REQUIRED`); ANL-01 and ANL-03 (read-only
  detectors, hypothesis signals, human review, privacy classes, optional diagnostics are
  non-adverse); `CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md`; ADR-0006; ADR-0018 (browser,
  proposed); MOVE-RL-11 (server-side visibility); the horizon gates `PROD-COMPAT-01`,
  `EXP-UPDATE-01`, `OPS-GM-01`, `DATA-PRIVACY-01`; WRITE-0 §6 (reports).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on | Phase |
|---|---|---|---|---|
| SEC-REL-1 | hard (security), security review | the signed client release manifest, the per-build challenge corpus and the Game-side trust store (§3) | PROD-COMPAT-01's release train | alpha |
| SEC-CHAL-1 | hard (protocol, security), protocol and security review | capability `CLIENT_INTEGRITY_V1`: the login challenge and its evaluation (§4) | SEC-REL-1; ADMIT-CAP-1 | alpha |
| SEC-TELEM-1 | hard (protocol, privacy), protocol and privacy review | capability `INPUT_TELEMETRY_V1`, the summary format, ingestion as an ANL-01 security event (§5) | ANL-01; DATA-PRIVACY-01's disclosure | alpha |
| SEC-DETECT-1 | impl, security and privacy review | ANL-03 detectors over telemetry and server-side timing; the flag queue (§6) | SEC-TELEM-1; ANL-03 | alpha (detectors), beta (queue to GM) |
| SEC-CHAL-2 | impl, security review | in-game challenges at random intervals (§4.3) | SEC-CHAL-1 | before open beta |

Tests: a native bootstrap without either capability is refused `ADMISSION_CAPABILITY_REQUIRED`; a
wrong, late or missing challenge answer never disconnects, mutates gameplay or sanctions, and
produces exactly one security observation; a telemetry summary over its byte bound is dropped and
observed; no telemetry field can carry a key code, text, a screen position outside the game window
or a process list; disabling optional crash diagnostics changes nothing here; a second answer to
one challenge, or an answer to a never-sent challenge, yields no second observation of `VALID`; a
non-empty `client_attestation` is ignored; a build leaving the admitted set loses its corpus.

Later, each with its own decision: client attestation (the reserved slot, §7), a third-party
anti-cheat or kernel component (rejected for now, §9), sanctions and ban waves (OPS-GM-01).

## 1. Question

How does the server learn that a session comes from the official client and is driven by a person,
without trusting the client, without invasive software, and without punishing anyone
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

## 3. The signed release (SEC-REL-1; layer 1)

- **Release manifest.** Every official native client build has a manifest: build id, platform,
  the hash of every executable and library, and its challenge corpus reference. The release
  pipeline signs it with an offline release key; the client distribution also carries the OS code
  signatures (Authenticode, Apple notarization), which are distribution concerns of
  `EXP-UPDATE-01`.
- **Trust store (Game).** The Game nodes hold the release public keys (pinned, rotatable by a
  signed key-rotation record) and the accepted manifests of the builds `PROD-COMPAT-01` admits. A
  manifest that fails verification is not accepted; its builds count as unknown.
- **Challenge corpus.** For each build the pipeline extracts the code regions the challenges may
  name. The Game nodes keep these region bytes (or the build's binaries) server-side; they are
  never sent to clients and are not a credential: a leaked corpus only helps forge answers for that
  build, and every release replaces it.

## 4. Challenges (SEC-CHAL-1, SEC-CHAL-2; layer 1)

### 4.1 Capability and admission

- Capability **`CLIENT_INTEGRITY_V1`** (number reserved on #162 at allocation): one server message
  `IntegrityChallenge {challenge_id, nonce, region_selector}` and one client message
  `IntegrityResponse {challenge_id, digest}`, where `digest = SHA-256(nonce || region bytes)`.
- **Mandatory per transport profile.** For transport profile 1 (native), `CLIENT_INTEGRITY_V1` and
  `INPUT_TELEMETRY_V1` join ADMIT-0's mandatory floor: a native bootstrap that does not declare
  them is refused `ADMISSION_CAPABILITY_REQUIRED` (a protocol requirement, the same refusal an
  outdated client gets, not a sanction). A future browser profile (ADR-0018) requires
  `INPUT_TELEMETRY_V1` only. The check is ADMIT-0's predicate at its points, unchanged: fresh
  admission after authentication, ownership and world eligibility (FND-04A step 14, and the final
  revalidation), same-session reconnect and recovery, with ADMIT-0's three codes
  (`ADMISSION_`, `RECONNECT_`, `RECOVERY_CAPABILITY_REQUIRED`). No new FND-04 step or code is
  added. Amended: ADMIT-0 §3.1 (pending on its acceptance).
- **Registration.** SEC-CHAL-1 registers the capability, its two message types with their
  `foundation`-style proto definitions and size limits, and the challenge outcome ANL-01 event in
  `PROTOCOL_OTERYN_V1_REGISTRY.json` and `GAME_EVENT_FOUNDATION_REGISTRY.json`, numbers reserved on
  #162 at allocation; SEC-TELEM-1 does the same for its capability, `InputSummaryV1` (an allowlist
  proto: only the fields of §5, so a forbidden field cannot be encoded) and its event.
- **Builds.** The corpus exists only for the builds `PROD-COMPAT-01` currently admits (at most
  `SECCLIENT01-RL-07`), at most `SECCLIENT01-RL-08` per build; a build's corpus is deleted when the
  build leaves the admitted set. A client declaring an admitted old build is challenged against
  that build; one declaring a build outside the set is refused by `PROD-COMPAT-01`'s version rule
  before admission, and a declared build without a corpus is `UNKNOWN_BUILD`.

### 4.2 At login (alpha)

- Within the first `SECCLIENT01-RL-01` (5 s) after `ServerAccepted`, the node sends one challenge
  for the session's declared `client_build_id`, with a fresh random nonce and a region drawn from
  that build's corpus.
- The answer is checked against the corpus. The outcome is one of `VALID`, `WRONG`, `LATE`
  (after `SECCLIENT01-RL-02`, 10 s), `MISSING`, `UNKNOWN_BUILD` (no accepted manifest for the
  declared build).
- **The outcome never changes the session.** Nothing is refused, disconnected or slowed; the
  outcome is recorded as one ANL-01 security observation (§6). This is the owner's silent
  flagging: the bot author does not learn what gave them away.

### 4.3 In game (before open beta, SEC-CHAL-2)

The same exchange at random intervals, at most `SECCLIENT01-RL-03` per session in any sliding 60
minutes (a reconnect keeps the session's count), with the interval drawn by the node; same
outcomes, same silence. One challenge is outstanding at a time; an answer to an unknown, expired or
already answered challenge id is ignored and counted in the outcome as `WRONG`.

## 5. Input telemetry (SEC-TELEM-1; layer 2)

- Capability **`INPUT_TELEMETRY_V1`**: the client sends one `InputSummaryV1` every
  `SECCLIENT01-RL-04` (60 s) of play, at most `SECCLIENT01-RL-05` (2 KiB).
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
  the opt-out crash diagnostics, whose non-adverse rule (ANL-03 §14) is unchanged. A summary that
  is missing or malformed while the capability is selected is itself recorded as an observation
  (it is a protocol fault, not an opted-out diagnostic).
- **Privacy class:** `SECURITY_SENSITIVE` (ANL-03 §14), pseudonymous by AnalyticsActorId; raw summaries kept
  `SECCLIENT01-RL-06` (90 days), signals and cases by ANL-03's profiles.

## 6. Flags (SEC-DETECT-1; layer 3)

- Challenge outcomes, telemetry summaries and server timing are ANL-01 security events. ANL-03
  detectors turn them into **hypothesis signals**: never sanctions, never gameplay mutations
  (ANL-03 §2). A `WRONG` digest from a known build is the strongest single signal; it is still
  reviewed by a person.
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

Each with max and max+1 tests.

## 9. Rejected options

- **Refusing or kicking on a failed challenge.** The owner chose silent flagging; instant feedback
  teaches bot authors and punishes honest players with odd setups.
- **Kernel drivers or a vendor (BattlEye, EasyAntiCheat).** Owner answer 5b; also FND-04's
  prohibition; Linux and macOS problems; privacy.
- **Trusting the client's integrity answer as authority.** Horizon "must preserve"; the server
  stays authoritative and the answer is evidence only.
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
- **Minimum sufficient:** two capabilities, one signed manifest, ANL events and ANL-03 detectors;
  no new authority.
- **Superseding evidence:** measured bot prevalence after beta that the layers do not contain.
- **Deliberately not decided:** attestation, vendors, sanctions (OPS-GM-01).

## 13. Before-freeze checklist

1. **Contract amendments:** FND-02 §11 (field 8 set aside); ADMIT-0 §3.1 (floor per transport
   profile), applied once #1440 is on main (this branch merges main before its freeze). Registry,
   proto and event registrations are the children's (§4.1).
2. **Serialization:** challenges and summaries are session messages; outcomes are events, never
   state changes.
3. **Restart:** nothing durable but ANL events; a challenge in flight at a restart is `MISSING`
   and ignored by detectors (a known restart window).
4. **Typed references:** build ids from signed manifests; AnalyticsActorId for analytics.
5. **Wire:** two capabilities, three messages, one reserved field.
6. **Split work:** none.

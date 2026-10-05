# ARCH-LOGIN-FIRST-0 Native login first: N4-P acceptance, N8 wire and packets

- Decision: `ARCH-LOGIN-FIRST-PACKETS-V1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES**, after exact-head validation, the independent
  review on the frozen head and protected integration. §1.2 (the N8 wire) changes protocol-oteryn
  and also needs owner protocol acceptance (§1.4, question 1) before N8-1 is allocated.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control plane D721 (#162): accept N4-P for testing and preproduction, fix the N8
  admission-result wire, and packet the shortest path to "log in, see the join snapshot and walk"
  on a local full stack.
- Accepts: Platform `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` at
  `Oteryn/Oteryn-Platform@f880cd744f8846f3efb8e7c1edadf1ea048656ce` (§1.1).
- Runtime, migration, Platform and production authority: NONE. Each packet needs its own #162
  allocation; Platform pieces need Platform authority under Oteryn/Oteryn-Platform#1419.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Implementation brief

1. **This PR** re-pins the N4-P lock entry to Platform `f880cd74` (adds Amendment N4P-3) and marks
   it accepted for testing and preproduction (owner D721). ADR-0020 gets a dated note: N4 is
   unblocked. Production entry is unchanged (§1.1).
2. **N8-1 (hard).** Before admission, the server answers a refusal it can classify with one
   `ProtocolError` (type 14) carrying a new code 1100..1116, one per FND-04A §11 row, disposition
   `TRANSPORT_FATAL`, `connection_generation` 0, then closes. Anything it cannot classify closes
   with no frame, as today. No new message type, no `foundation.proto` change. Registry, crate,
   server mapping and client decode land in one PR (§1.2, §2.2).
3. **OPS-ASSIGN-REPORT-1 (hard).** `oteryn-game-ops` reports each committed scope assignment to
   Platform (`ReportScopeAssignmentV1`), so the node's runtime-status report is ownership-bound
   and Platform can route. Without it no grant is ever issued (§2.3).
4. **N4-1 (hard).** `crates/platform-client` gets the native login: OAuth code + PKCE S256 over a
   loopback redirect, native ticket, Gateway `POST /v1/login` with a reused `attempt_ref`, then
   `oteryn-session-tcp` connect to the returned endpoint (dev root) and `Session::admit` with the
   grant. The client maps N8 codes to FND-04A public classes; everything else fails closed (§2.4).
5. **N5-1 (impl).** Allow the `oteryn-client -> oteryn-session-tcp` edge in
   `workspace-boundaries.toml` and keep every closure negative failing (§2.5).
6. **N2N3-1 (impl).** `apps/client` renders the admitted join snapshot (own position on a
   placeholder grid) and walks by click and arrow keys over the session; no map-track path (§2.6).
7. **RUNBOOK-1 (impl).** `tools/qualification/login_local/` brings up Platform (pinned
   `>= 71bbe6c`), the gateway, the issuer in modes 33a/34a and one game node, then runs the
   client to "walked one step". It is a local runbook, not CI (§2.7).

Order: P1 (this PR) -> N8-1 and OPS-ASSIGN-REPORT-1 (parallel) -> N4-1 -> N5-1 -> N2N3-1 ->
RUNBOOK-1. N5-1 may land any time before N4-1 merges if N4-1 is split; it is listed after it
only because N4-1 is the first change that needs the edge (§2.1).

Platform pieces (separate authority, #1419): U1 native OAuth client and native ticket issuance
(blocks N4-1 end to end); the contract Status line update; LCFA-1 later (replaces mode 33a).

Owner questions (§1.4): 1. accept the N8 wire; 2. U1 native OAuth client shape.

## 1. Rulings

### 1.1 N4-P acceptance (lock entry `OTV2-20260929-N4P-NATIVE-GATEWAY-LOGIN`)

The lock pinned the candidate at `ed6c0d38` (#1420). Platform `f880cd74` (#1427, on Platform
`main@71bbe6c`) is the current revision of the same file; its delta is:

- U16 now carries the #1426 review finding (the assignment epoch is global);
- new §17 Amendment N4P-3: mode 33a (D171, Character ownership not verified, one configured
  world) and mode 34a (D172, route record on `game_channels`, published only through
  `NativeTopologyRegistry::publishRouteForPreproduction`, per-scope `native_login_enabled`
  default false), both `testing`/`preproduction` only and default off; issuer lock wait bounded
  at 3 s (1..10);
- Non-authorization renumbered to §18.

Ruling: the contract at `f880cd74` (file sha256
`4c49b08c0348fe37c42676794ef492777729a907172d491d31cfbbb6f353c747`) is **accepted by the Game
architect and the owner (D721) for testing and preproduction**. That is rollout step 1 of the lock
entry; it unblocks N4 (ADR-0020 §7) and the Game producer and consumer packets below. It does not
accept the two Game companion candidates beyond what their own merged text already says, and it
does not change `accepted_for_fnd02` (stays `false`).

Production entry still needs, unchanged: U1, U3 (gameplay CA), U8 (production topology), U9 (key
custody); the mode 33a release gate CHAR-NAME-1 -> LCFA-1 -> full §5.4 issuance check; the mode
34a release gate U3 -> D3 -> U7; and ADR-0020 §7 (N1..N8, N6, N7). The contract's own Status line
still reads CANDIDATE; changing it is a Platform-side edit and does not gate Game work, because the
lock records acceptance by digest.

U16 (Game architect): epoch raises stay an operator-only action in `oteryn-game-ops`, never a
by-product of an ordinary assignment report. OPS-ASSIGN-REPORT-1 therefore never raises the
epoch; a restore that needs a raise is an explicit operator command (§2.3).

### 1.2 N8 wire: bounded admission refusal

Today a fresh-admission refusal closes the socket with no frame
(`ConnectionEnd::AdmissionRefused`), and the verifier's `Fnd04ConsumerError` is discarded
(`gameplay_transport/mod.rs`, `verify_fresh_grant_durability_v1(..).map_err(|_| Rejected)`). The
client cannot keep the FND-04A per-code progression that ADR-0020 §3 requires.

**Message.** The existing `ProtocolError` (type 14, phase `ANY`, `SERVER_TO_CLIENT`,
unsequenced). Before `ServerAccepted` the envelope `connection_generation` is 0; `error_code` is
the code below; `disposition` is `TRANSPORT_FATAL` (4); `related_command_id`,
`expected_command_id` and `expected_server_sequence` are 0 (absent). No payload text, no JWT, no
generation, no fencing value, no identifier. The server writes exactly this one frame, flushes
within the entry deadline, and closes. A new message type was rejected: type 14 already has the
right phase, direction and sequencing, and the existing pre-admission `reject(stream, error, 0)`
path already writes it.

**Codes.** New registry `error_codes` entries, in FND-04A §11 row order. Names are the FND-04A
names; category is the FND-04A category (three categories are new to the registry:
`AUTHENTICATION_FAILED`, `SESSION_REJECTED`, `DEPENDENCY_UNAVAILABLE`); every
`default_disposition` is `TRANSPORT_FATAL`. Each entry also gets `progression` and
`public_class` members (only these entries; FND-02 codes keep their shape).

| Code | Name | Category | Progression | Public class |
|---|---|---|---|---|
| 1100 | `ADMISSION_GRANT_MALFORMED` | `INVALID_INPUT` | `TERMINAL` | `RETRY_LOGIN` |
| 1101 | `ADMISSION_GRANT_AUTHENTICATION_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | `AUTHENTICATION_REQUIRED` |
| 1102 | `ADMISSION_GRANT_BINDING_MISMATCH` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | `RETRY_LOGIN` |
| 1103 | `ADMISSION_GRANT_NOT_YET_VALID` | `SESSION_REJECTED` | `RETRYABLE` | `TEMPORARILY_UNAVAILABLE` |
| 1104 | `ADMISSION_GRANT_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` | `RETRY_LOGIN` |
| 1105 | `ADMISSION_GRANT_REPLAYED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | `SESSION_UNAVAILABLE` |
| 1106 | `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | `TEMPORARILY_UNAVAILABLE` |
| 1107 | `ADMISSION_GRANT_SECURITY_STATE_REVOKED` | `SESSION_REJECTED` | `SECURITY_TERMINAL` | `AUTHENTICATION_REQUIRED` |
| 1108 | `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | `TEMPORARILY_UNAVAILABLE` |
| 1109 | `ADMISSION_GRANT_ROUTE_STALE` | `STALE_GENERATION` | `TERMINAL` | `RETRY_LOGIN` |
| 1110 | `ADMISSION_GRANT_RUNTIME_GENERATION_STALE` | `STALE_GENERATION` | `TERMINAL` | `RETRY_LOGIN` |
| 1111 | `ADMISSION_GRANT_WORLD_STALE` | `STALE_GENERATION` | `TERMINAL` | `RETRY_LOGIN` |
| 1112 | `ADMISSION_GRANT_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | `CLIENT_UPDATE_REQUIRED` |
| 1113 | `ADMISSION_ACCOUNT_CHARACTER_CONFLICT` | `CONFLICT` | `TERMINAL` | `SESSION_UNAVAILABLE` |
| 1114 | `ADMISSION_INCUMBENT_PROTECTED` | `CONFLICT` | `TERMINAL` | `CHARACTER_ALREADY_ACTIVE` |
| 1115 | `ADMISSION_CAPACITY_EXCEEDED` | `CAPACITY_EXCEEDED` | `RETRYABLE` | `TEMPORARILY_UNAVAILABLE` |
| 1116 | `ADMISSION_CAPABILITY_REQUIRED` | `UNSUPPORTED_REVISION` | `TERMINAL` | `CLIENT_UPDATE_REQUIRED` |

1116 is assigned now so the range stays contiguous, but no server path emits it until ADMIT-0 is
accepted and implemented. Codes 1117..1199 stay unassigned for later admission rows; registry
invariant "never reused" applies.

**Server mapping (N8-1).** Fresh admission keeps its `AdmissionRefusal` internally but carries an
optional code:

- each `Fnd04ConsumerError::Fresh*` variant maps to its row (`Malformed` 1100,
  `AuthenticationFailed` 1101, `BindingMismatch` 1102, `RevisionUnsupported` 1112, `NotYetValid`
  1103, `Expired` 1104, `SecurityEvidenceStale` 1108, `SecurityStateRevoked` 1107,
  `AccountCharacterConflict` 1113, `WorldStale` 1111, `RouteStale` 1109, `RuntimeStale` 1110);
  the `.map_err(|_| Rejected)` on the verifier call stops discarding it;
- the durable commit outcome (`FreshAdmissionDurableOutcomeV1`): `RejectedReplayConflict` 1105,
  `RejectedIncumbent` 1114, `RejectedStaleAuthority` on the last round 1110;
- `reserve_runtime_player` refused with `CarrierError::CapacityExceeded` 1115;
- the existing direct mappings: kid/utf8/character-id decode failures 1100, world mismatch 1111.

No fresh-admission path emits 1106 or 1116 yet: 1106 is the FND-04A row for an unproven Platform
issuance outcome, and the server's own commit ambiguity is not that row.

Every other refusal (an `Unavailable` the server cannot attribute to one row, a reconciliation
`Unknown`, `DurableNotOwned`, `RejectedCollision`, an internal error) sends **no frame** and closes, exactly as today:
N8 never invents a code that FND-04A does not define, and the client treats a frameless close as
generic `TEMPORARILY_UNAVAILABLE` with no automatic retry. Resume refusals stay frameless
(`ResumeUnavailable`); recovery codes are out of scope.

**Precedence and disclosure.** The server emits the code its existing verifier precedence
(FND-04A §5: malformed -> authentication failed -> authenticated schema -> binding -> revision ->
...) already produced; N8 changes no check order. Before authentication only 1100 and 1101 are
reachable, so an unauthenticated peer learns nothing about accounts, characters or routes. After
authentication the exact code is sent, including the `SECURITY_TERMINAL` rows: FND-04A §11 assigns
each a distinct public class and ADR-0020 §3 requires the client to keep it. Platform collapses its
own security rows for an unauthenticated HTTP caller; that rule does not transfer to a holder of
an authenticated grant.

**Client mapping.** `crates/session` returns
`SessionError::AdmissionRefused { code: u32 }` when the first server frame is `ProtocolError`
with a code in 1100..1199, and the frameless close as the existing I/O error. `crates/protocol-oteryn`
exposes `admission_refusal_class(code) -> Option<(Progression, PublicClass)>` from the same table;
an unknown code in the range maps to `TEMPORARILY_UNAVAILABLE`, `TERMINAL` (no retry). Retry:
`RETRYABLE` rows 1103 and 1108 retry the **same unconsumed grant** while it is unexpired, at most 3
times, with backoff 1 s, 2 s, 4 s (bounded by the grant's remaining validity); 1106 retries the
Gateway call with the same `attempt_ref` (Platform §6 returns the identical grant); 1115 retries
the same grant once after 2 s, then needs a new Gateway attempt. No other code retries
automatically. There is no retry-after field.

**Old peers (checklist 5).** A new server and an old client: the old `Session::admit` already
returns `NotAdmitted(ProtocolError)` for any non-`ServerAccepted` first frame and never decodes its
payload, so it fails closed as before. A new client and an old server: no frame, generic class.
No capability or protocol major is needed. `foundation.proto` is unchanged (its locked sha256
stays).

**Why registry and crate in one packet.** `error_codes_match_the_registry`
(`crates/protocol-oteryn/src/lib.rs`) requires the crate enum to equal the registry. A
registry-only change in this decision PR would break CI, so this decision fixes the exact wire and
N8-1 applies registry, crate, server and client together.

### 1.3 U1 recommendation (Platform + owner)

Ticket issuance from OAuth exists (`IssueGameLoginTicketFromOAuth`, scope `game:ticket`), but it
issues the Canary-audience ticket and the existing "Oteryn OTClient" Passport client has the fixed
redirect `http://127.0.0.1/callback`. Recommendation: a **separate Passport public client** for the
native Rust client, PKCE S256 required, redirect `http://127.0.0.1/callback` matched on any port
(RFC 8252 §7.3 loopback), scopes `game:ticket` only, and native ticket kind selected by that client
id (Platform §4, U1). Reusing the OTClient client would mix Canary and native ticket audiences on
one client id, which Platform §16 tests must keep isolated.

### 1.4 Owner questions (batched to the control plane)

1. **N8 wire (§1.2).** a) accept as written; b) accept but collapse the `SECURITY_TERMINAL` rows
   (1101, 1102, 1105, 1107) into 1101 on the wire; c) reject. Recommendation: a.
2. **U1 native OAuth client (§1.3).** a) a separate public loopback client, any port; b) reuse
   the existing OTClient client with a fixed port. Recommendation: a.

### 1.5 Checklist before freeze

1. Amendments in the owning contract: the lock entry and ADR-0020 note are applied in this PR;
   the N8 registry entries are applied in N8-1 with the crate (§1.2, last paragraph); the
   Platform Status line is a Platform edit.
2. Concurrency: the N8 frame is written by the one connection task that owns the socket, before
   close; OPS-ASSIGN-REPORT-1 runs after the assignment commit, never inside it.
3. Restart: N8 adds no state. OPS-ASSIGN-REPORT-1 derives the report from the durable assignment
   alone, so a restarted ops command re-sends the identical report (§2.3).
4. Typed references: every reference above names the file and section.
5. Older peers: §1.2 "Old peers"; the client N4-1 fails closed against a gateway with the native
   branch off (Platform `GATEWAY_NATIVE_LOGIN_ENABLED=false`).
6. Split work: N8-1 is one PR (registry + crate + server + client); OPS-ASSIGN-REPORT-1 is
   observable on its own through Platform's runtime-status read model.

## 2. Packets

### 2.1 Dependency order

| # | Packet | Class | Depends on | Platform piece |
|---|---|---|---|---|
| P1 | this decision (lock + ADR-0020 note) | architect | D721 | Status line (non-gating) |
| 2 | N8-1 admission refusal wire | hard | P1, owner Q1 | none |
| 3 | OPS-ASSIGN-REPORT-1 assignment report | hard | P1 | none (endpoint exists) |
| 4 | N4-1 native login in client | hard | P1; N8-1 for code mapping | U1 client + native ticket |
| 5 | N5-1 client edge to session-tcp | impl | P1 | none |
| 6 | N2N3-1 render join snapshot and walk | impl | N4-1, N5-1 | none |
| 7 | RUNBOOK-1 local full stack | impl | N4-1, OPS-ASSIGN-REPORT-1, N2N3-1 | U1 merged; config only |

No packet owns a map-track path (MAP01-VIEWPORT, MAP-CUTOVER-1, `crates/world-bundle`,
`tools/world-bundle-compiler`, `apps/game-server/src/gameplay_transport/world_spatial*`).

### 2.2 N8-1 admission refusal wire (hard worker)

- owned_paths: `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` (17 `error_codes` entries only);
  `crates/protocol-oteryn/src/lib.rs`; `crates/session/src/lib.rs`;
  `apps/game-server/src/gameplay_transport/mod.rs` (fresh admission refusal mapping);
  `apps/game-server/src/gameplay_transport/connection.rs` (`admit_frame` refusal write);
  `apps/game-server/src/foundation/admission.rs` only if a variant needs a code accessor.
- scope: §1.2 exactly; the crate test table includes the 17 rows; a test per row that the server
  writes exactly one `ProtocolError` frame with that code, generation 0, `TRANSPORT_FATAL`, then
  EOF; a test that an unattributed `Unavailable` writes no frame; a session test for each class
  and for an unknown in-range code; a test that the frame carries no bytes beyond the four fields.
- validation: `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo test -p oteryn-protocol-oteryn -p oteryn-session -p oteryn-game-server`; the PG
  admission tests selected by `tools/repository/classify_pr_test_lanes.py`.

### 2.3 OPS-ASSIGN-REPORT-1 scope assignment report (hard worker)

- owned_paths: `apps/game-server/src/bin/oteryn-game-ops.rs`;
  `apps/game-server/src/native_admission_source/` (new `scope_assignment.rs`, reuse of
  `http1_mtls`); its tests.
- scope: implement `ReportScopeAssignmentV1` of
  `docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §5 to Platform
  `POST /internal/v1/game-auth/native-scope-assignments` with the ownership-authority client
  certificate. `ops assignment assign` sends it after its commit; a new
  `ops assignment report --world --channel` re-sends the current durable assignment. The body is
  derived only from the durable assignment row, so a re-send is byte-identical; retries with
  bounded backoff until a definite 2xx (`accepted` or `superseded`) or a definite 4xx, which is
  reported and stops. The epoch is never raised here (§1.1, U16).
- validation: as N8-1 for `oteryn-game-server`; a test server for the mTLS exchange (accepted,
  superseded, 4xx, timeout then success, identical bytes on re-send).

### 2.4 N4-1 native login in the client (hard worker)

- owned_paths: `crates/platform-client/src/` (new `native_login` module); `crates/platform-client/Cargo.toml`;
  `crates/identity/src/lib.rs` (loopback redirect helper only, if needed); `apps/client/src/lib.rs`;
  `apps/client/Cargo.toml` (add `oteryn-session-tcp`); `apps/client/src/main.rs` (config);
  `crates/session-tcp/src/` only for a configured root file, if missing.
- scope, in ADR-0011 order (availability before credential before connect):
  1. directory fetch shows the World available, else stop with no credential request;
  2. OAuth authorization code + PKCE S256 (`PkceMaterial`) in the system browser, loopback
     listener on `127.0.0.1:0`, one exact `state`, bounded wait; token exchange; scope
     `game:ticket`;
  3. `POST /v1/game-auth/tickets` with the bearer; native ticket;
  4. Gateway `POST /v1/login` exactly as Platform §3 (protocol_version 2, `attempt_ref` UUIDv7
     generated once and reused on every retry of this attempt, `character_id` from config in mode
     33a, `channel_id` null, the offer); strict decode (unknown member, duplicate, wrong version
     rejected); Platform §11 errors mapped to their public class; this module bypasses the
     directory's forbidden-field filter only for its own typed response;
  5. `oteryn-session-tcp` connect to `endpoint.host:port`, TLS 1.3, SNI and verification against
     `endpoint.tls_server_name` and the configured dev root file only, ALPN `endpoint.alpn`;
  6. `Session::admit` with the grant bytes as `admission_material`; N8 codes mapped per §1.2.
  Tokens, tickets and grants are held in memory only, never logged, dropped after use.
  `request_gameplay_entry` returns the admitted session or a public class; no path skips a step.
- validation: unit tests with a local mock OAuth/Gateway (each Platform §11 error, retry reuses
  `attempt_ref`, byte-identical grant accepted, unknown member rejected); a session-tcp test with a
  self-signed dev root (wrong name rejected); `cargo test -p oteryn-platform-client -p oteryn-client`;
  `cargo run -p oteryn-architecture-check -- workspace .` after N5-1.

### 2.5 N5-1 client edge to session-tcp (impl worker)

- owned_paths: `workspace-boundaries.toml` (`[edges] oteryn-client` gains `oteryn-session-tcp`);
  `tools/architecture-check/src/` tests; the closure steps in `.github/workflows/merge-gate.yml`,
  `merge-group-gate.yml` and `rust.yml` only if they list allowed crates.
- scope: the production closure of `oteryn-client` may contain `oteryn-session-tcp` and, through
  it, `oteryn-session` and `oteryn-protocol-oteryn`; `forbidden_package_fragments` is unchanged
  (no closure crate name contains `transport`); the canary, tool and synthetic negatives
  (including `oteryn-synthetic-assets` and `oteryn-dev-client`) still fail, with a negative test
  each.
- validation: `cargo run -p oteryn-architecture-check -- workspace .`; `cargo test -p oteryn-architecture-check`;
  the three workflow closure scripts run locally.

### 2.6 N2N3-1 render the join snapshot and walk (impl worker)

- owned_paths: `apps/client/src/scene.rs`, `apps/client/src/input.rs`,
  `apps/client/src/windows_shell.rs`, a new `apps/client/src/play.rs`; `crates/renderer/src/`
  only for a tile/sprite call it lacks.
- scope: after admission, build the view from the session's join snapshot
  (`JoinSnapshot.world_spatial.actor_position`, and `world_object_overlay` entries as markers):
  a placeholder ground grid centred on the own position, drawn with the existing
  `TileBatch`/`SpriteBatch` and placeholder assets; `ClickWalk` and arrow keys call
  `Session::step`, and the step outcome and domain-1 deltas move the view; on any session error
  return to the login state with its public class. Real map tiles arrive with the map track
  (MAP01-VIEWPORT) and are not drawn here. No new protocol, no map loading, no creature or chat
  rendering (N6, N7).
- validation: `cargo test -p oteryn-client` (view from a snapshot fixture, click to step, step
  result moves the view, error returns to login); Windows build of `oteryn-client` in CI.

### 2.7 RUNBOOK-1 local full stack (impl worker)

- owned_paths: `tools/qualification/login_local/` (README, `run.sh`, compose override).
- scope: extends `tools/qualification/node_boot/` patterns: pins a Platform SHA `>= 71bbe6c`
  carrying U1; Platform with `APP_ENV=preproduction`, the native admission issuer key file and its
  public key published with `publishTrustedKey`, mode 33a
  (`GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_OWNERSHIP=true`, `..._WORLD_ID` = the node's
  world), the route published with `publishRouteForPreproduction` and `native_login_enabled=true`;
  the Go gateway with `GATEWAY_NATIVE_LOGIN_ENABLED=true`; one game node with `runtime_status`
  configured and its certificate; `ops assignment assign` (which now reports); a test account and
  its Character; then the client with the dev root, the configured `character_id` and the
  Platform URLs. The result line is `LOGIN_LOCAL_RESULT=WALKED` only after a server step result.
  Secrets are generated per run and never committed.
- validation: `bash -n`; `shellcheck`; one documented local run recorded in the PR.

## 3. Non-authorization

This decision authorizes no code, migration, registry entry, configuration, deployment, Platform
change or production entry. Each packet needs its own #162 allocation.

# GPE2E-1 — Game ↔ Platform end-to-end qualification plan

Documentation snapshot: 2026-11-12. Platform main verified: `a85b5da4de23f3e8633cfb25a2db1ad7f6133acd` (#1833; includes #1828, #1827 and #1826). Game base: `aee3f57a5697eea525cd8109b3ae8cc93691216e`.
No implementation, schema, flag, pin or deployment change is part of this task.


GPE2E-2e current-pin correction: Platform #1833 supplies channel/route CLI seeds
(`N:portal-rs/bin/oteryn-portal/src/seed.rs:397`, `:406`); #1825 supplies native
admission grants (`N:portal-rs/crates/gameauth/src/admission/grant.rs:20`, `:85`).
Earlier M observations below remain historical evidence, not current Platform blockers.
The live ac3bb477 run proved web identity and account isolation before the old T19c
blocker. The new lane validates route format/replay/conflict, then remains BLOCKED
on Game control-scope grant issuance (`G:tools/qualification/node_boot/run.sh:999`);
no manual grants, assignment or admission success are claimed. Grant integration also
requires encrypted custody/wrapping files (`N:portal-rs/crates/gameauth/src/admission/grant.rs:24`).

## Evidence coordinates and interpretation

- `G:path:line` means Oteryn/Oteryn-Game at the base above.
- `P:path:line` means Oteryn/Oteryn-Platform at node-boot's pin,
  `3bc2d0e923db7472ec1f9b534ebe87aaffa1ef05` (#1665).
- `N:path:line` means the newly verified Platform main in the snapshot line; cloned read-only for source inspection. The lane now pins this revision (#1833).
- `M:path:line` means the earlier Platform main as inspected,
  `24bdde3f1fd727d51a3d61a32e85d80b45757c52` (#1817).
  Throughout, proposed `client_phases/<file>.rs` means
  `apps/game-server/src/qualification/client_phases/<file>.rs`.
  These prefixes are revision-qualified source citations, not local checkout aliases.
  Inspect P with `git show <pin>:<path>`; do not interpret its lines against M.
- Platform was cloned with `git clone --depth 50 https://github.com/Oteryn/Oteryn-Platform /tmp/platform`;
  the pin was outside that history and was fetched explicitly with depth 1. The
  Platform working tree was not edited. Routes and their implementations were read,
  not inferred from keyword matches.
- **RUNNABLE NOW** means the bounded test can be implemented against existing real
  product callers and handlers. It does not mean the test exists or passed. **BLOCKED**
  means the named successful flow needs product work or a contract decision first.
  A runnable refusal test is separately named; it never makes a blocked positive flow green.
- Runtime estimates below are proposed warm-stack budgets, not measurements. Allow
  5–15 minutes shared stack provisioning plus existing build time (UNKNOWN, not timed).
  Keep per-flow deadlines and report boot time separately from scenario time.
- ACCEPTED does not imply production authority. Where an old document still says
  Candidate, retain that wording and cite its acceptance/merge evidence separately.
  `G:docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json:5` requires merged canonical
  revisions; line 7 explicitly says LOCKED is not FND-02 acceptance. Historical runtime
  status fields in that lock (e.g. line 94) are not current executable coverage.

## Existing lifecycle and the portal-rs prerequisite

Reuse `G:tools/qualification/node_boot/run.sh:14` (pin), `:30` (Compose layers),
`:310` (Game PostgreSQL 18.6 with verified TLS), `:355` (shipping binaries), `:1027`
(real Platform operator intents and Game control socket), `:1046` (seam against
running node), and `:1082` (shutdown). The real World pass supplies its bundle at
`:1055`. `G:tools/qualification/wp5_s3b/run.sh:4` describes the same producer seam;
`:10` explicitly says the harness signs the fresh grant. `G:apps/game-server/src/qualification.rs:89`
prints `SEAM_EVIDENCE`; node boot's printer is `G:tools/qualification/node_boot/run.sh:179`.

**The current topology is not portal-rs ↔ PostgreSQL.** Its Platform database is
MariaDB and its process is PHP-FPM (`G:tools/qualification/wp5_s3a/compose.yml:3`,
`:37`, `:57`). Its proxy exposes only native evidence and bootstrap reads
(`G:tools/qualification/wp5_s3b/nginx.conf:26`, `:45`, `:64`). Merely changing the
Platform SHA cannot work: M removed the Laravel app/Go gateway (#1811), while node
boot still runs Artisan (`G:tools/qualification/node_boot/run.sh:341`, `:1033`).

GPE2E-2 adds `tools/qualification/game_platform/run.sh`, reusing
node_boot's lifecycle and real-bundle/seam stages, with a reviewed lifecycle hook
for the Platform process and provisioning. Run actual `oteryn-portal migrate`,
`serve`, `worker` and `gateway` roles against a separate disposable Platform
PostgreSQL database; roles are at `M:portal-rs/bin/oteryn-portal/src/main.rs:9`, `:34`,
`:60`. Delegate Platform boot to `M:portal-rs/e2e/start.sh` and parse its JSON
report (listeners, TLS paths, seed outputs and pending steps), never its logs.
Use `oteryn-portal seed account|world|oauth-native-client|signing-trust|bootstrap-intent`
for testing-only provisioning. Channel/route seeds were pending at M; N #1833 supplies them at
`N:portal-rs/bin/oteryn-portal/src/seed.rs:397`, `:406`. Use the real seeds and
verify revision/replay/conflict; never insert manual route rows.
Fresh account evidence uses purpose `platform_security`, scope `fresh_admission`
(`M:portal-rs/e2e/test.env:15`). Keep Game and Platform database roles isolated. Do not copy the boot script,
keep PHP SQL against PostgreSQL, bypass the exact-pin check, or replace missing
handlers with test servers. The fixed-path `NODE_BOOT_LIFECYCLE=game_platform`
hook selects a separate readonly M pin; the default pin and PASS printer remain.
Keep the existing pinned seam as a regression lane and add a separately immutable
portal-rs lane. This shared test-tooling prerequisite applies to every M-based
RUNNABLE NOW row; no product completion is implied by it.

Provision real disposable accounts through Platform registration/identity services,
with random per-run passwords and real password hashes; use the normal development
OAuth registration, account eligibility and trust-management paths. Create Characters
through the real Platform command producer and Game authority (section 5a), then
publish from Game. Never insert fabricated character projections, grants, journal
records, receipts or Store operations to make an assertion pass. Database reads are
independent assertions, not an alternate caller. A scenario needing an issuer that
M lacks stays blocked. Never log credentials, tickets, grants or proof bytes.

Use separate per-purpose certificates, real TLS listeners and PostgreSQL transactions.
M's internal routes are mounted before `internal_tls::start`, then removed from the
public/native app (`M:portal-rs/bin/oteryn-portal/src/main.rs:407`, `:417`, `:427`).
A public header is not a verified certificate. Include missing/wrong-purpose cert,
flag-off and wrong-environment cases without weakening production guards.

**Environment discrepancy:** ticket and LCFA configuration accept `OTERYN_ENV=preprod`
(`M:portal-rs/crates/player-api/src/lib.rs:41`,
`M:portal-rs/crates/characters/src/overview/mod.rs:227`), whereas #1814's command and
service exchanges require `testing|preproduction`
(`M:portal-rs/bin/oteryn-portal/src/game_routes.rs:88`). Use `testing` for initial
cross-route qualification. For the requested preproduction lane, record exact
values; qualify isolated preprod routes separately, restarting the real Platform
between isolated stages if needed. A single preproduction end-to-end login/create
configuration remains blocked on Platform reconciling these gates; do not silently
rename an environment or call a testing run preproduction.

`G:tools/qualification/e2e_all/run.sh:16` already delegates boot to node_boot and
`:21` rejects missing captures; extend that reporting pattern, not a parallel
coverage claim. Existing phase mounts are explicit at
`G:apps/game-server/src/qualification.rs:2666`; add sorted mounts and invoke each new
phase in the orchestrator in its future implementation task. A new `.rs` file alone
is not executable coverage. New evidence strings proposed below are specifications,
not strings claimed to exist today, except the mounted identity driver below.
GPE2E-2 runtime qualification is **not proven** here: the lane returned
`GAME_PLATFORM_RESULT=FAIL flow=preflight reason=missing_binary_sudo`.
The original node_boot regression attempt returned `NODE_BOOT_RESULT=NODE_BOOT_FAIL`:
Docker Compose could not create `/home/agent/.docker/buildx` on the read-only
filesystem; cleanup also reported `sudo: command not found`. No login PASS is claimed.

## 1. Account login — Platform web identity (highest risk)

**Game / contract.** Platform owns identity, OAuth and sessions under ACCEPTED
`G:docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md:5`, `:19`.
Game has no password authenticator. Its downstream consumer is
`G:crates/platform-client/src/native_login.rs:491` (PKCE authorization URL) and
`:513` (code exchange), with credentials held/redacted at `:49`.

**Platform P → M.** P mounts GET/POST `/login` and POST `/logout` at
`P:routes/web.php:36`; `P:app/Http/Controllers/Identity/SessionController.php:21`
checks identity/MFA and establishes the web session, `:43` invalidates it.
M mounts localized login/logout at `M:portal-rs/crates/identity/src/login_http.rs:20`,
`:35`; `:162` authenticates and `:170` rotates the session; `:185` logs out.
The binary mounts it at `M:portal-rs/bin/oteryn-portal/src/main.rs:150`.
OAuth routes are `M:portal-rs/crates/gameauth/src/oauth/oauth_http.rs:25`.

**Coverage today.** Not covered. Node boot seeds identities with random passwords
through PHP (`G:tools/qualification/node_boot/run.sh:341`); no browser login happens.
There is no honest NODE_BOOT_EVIDENCE/SEAM_EVIDENCE login-success line.

**Status: RUNNABLE NOW**, bounded ordinary web login + PKCE handoff, after the shared
harness work. OAuth sign-in challenge completeness is UNKNOWN; do not infer it from
ordinary login. Web MFA has challenge creation and completion at
`M:portal-rs/crates/identity/src/mfa.rs:341`, `:432`; its web adapter rewrites
login at `M:portal-rs/crates/identity/src/mfa_http.rs:90` and completes it at `:459`.
The old password-only adapter comment is stale for this composed web flow.
The web MFA extension is **RUNNABLE NOW against N**: testing-only
`oteryn-portal seed account-mfa` returns `totpSecret` and `recoveryCodes`
(`N:portal-rs/bin/oteryn-portal/src/seed.rs:120`, `:164`, `:727`, `:731`; #1828).
Use those private outputs to complete the real web TOTP challenge; never log them.

**Separate native MFA test — `native_login_mfa_account`: RUNNABLE NOW against N.**
The requested BLOCKED status is superseded by merged Platform #1826: the code
now refuses only when MFA is required and the browser has not proven it
(`N:portal-rs/crates/gameauth/src/oauth.rs:230`). The HTTP adapter checks a live
account-bound `MfaSession` (`N:portal-rs/crates/gameauth/src/oauth/oauth_http.rs:162`)
and passes its proof to approval (`:261`); proven MFA can issue a code (`:264`).
Test missing/expired proof refusal and successful web TOTP → consent → native
PKCE exchange. This is current-main readiness, not evidence at the unchanged lane pin.
`G:docs/contracts/OTERYN_NATIVE_LOGIN_CHALLENGE_V1.md:48` lists the additional
Platform obligations for a challenge-specific successor.

**Driver added, runtime unverified — `web_identity_pkce_isolation`.**
`G:crates/platform-client/tests/web_identity_pkce_isolation.rs` is invoked by
`tools/qualification/game_platform/run.sh` before the dev-client phase. Drive a real
cookie-jar HTTP browser journey: GET form/CSRF, wrong password, real password,
authorize/consent, loopback callback and Game's `NativeLoginClient` token exchange.
Assert generic failure without a session for wrong credentials, CSRF rejection,
rotation of session identity on login/logout, correct account for the token,
PKCE/state mismatch rejection, code one-use, logout invalidation, account B cannot
use A's session. Inspect persisted sessions/tokens in Platform PostgreSQL; assert
Game has zero admissions until its own admission flow. Budget 30–60 s.
Required success evidence (not observed here):
`NODE_BOOT_EVIDENCE flow=web_identity result=pass account_isolation=1`.

## 2. Game ticket / native gateway login — mode 34a, preprod

**Game / contract.** Real caller: `G:crates/platform-client/src/native_login.rs:531`
(ticket) and `:593` (Gateway protocol 2); World availability precedes credentials at
`G:crates/platform-client/src/gameplay_entry.rs:43`. ACCEPTED testing/preproduction
native gateway contract is locked at
`G:docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json:158`, `:177`, canonical Platform
`f880cd744f8846f3efb8e7c1edadf1ea048656ce`. Its copy still says CANDIDATE at
`M:docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md:5`; the lock records the
accepted scope. Production issuance amendment #1817 is only a proposal, not a
reason to enable production.

**Platform P → M.** Ticket route `P:routes/api.php:23`, handler
`P:app/Http/Controllers/GameAuth/GameLoginTicketIssueController.php:15` binds the
OAuth identity and prohibits submitted account IDs. Admission forwarding target:
`P:routes/internal.php:33` and
`P:app/Http/Controllers/GameAuth/NativeAdmissionController.php:26`.
M uses `M:portal-rs/crates/player-api/src/lib.rs:47` through the real guarded OAuth
authority (`M:portal-rs/bin/oteryn-portal/src/game_login.rs:32`, `:64`);
Gateway `/v1/login` is `M:portal-rs/crates/gateway/src/lib.rs:54`, forwarding at
`:119`; the service-authenticated private handler is
`M:portal-rs/crates/gameauth/src/admission/http.rs:80`, `:117`.
**Current pin:** a85b5da4 includes route/channel seeds and admission signing.
The lane invokes the real signing-custody seed and loads its private encrypted
issuer files; it does not synthesize signing snapshots or grants. The refusal
journey still has no owner LCFA snapshot, so Gateway must return exact
NATIVE_LOGIN_ROUTE_UNAVAILABLE and roll back redemption with zero admissions.

**Coverage today.** Not covered by node_boot/seam: its signer is
`G:apps/game-server/src/qualification.rs:346` and no ticket is obtained.
Related manual harness `G:tools/qualification/login_local/run.sh:4` uses mode 33a/34a,
a different Platform pin (`:32`) and the Go gateway. Its `:649` prints
`LOGIN_LOCAL_EVIDENCE node assigned=operator report=ReportScopeAssignmentV1 assignment_epoch=1 readiness=true runtime_status=configured`;
`:660` prints projection acceptance. These are not NODE_BOOT_EVIDENCE, not automated
web-login proof, and not evidence for portal-rs main.

**Current pinned refusal slice.** **Runnable subtest:**
`native_ticket_and_gateway_fail_closed` exercises actual OAuth → ticket → real
Gateway and asserts the current unavailable response, zero Game admissions and no
successful admission row. Ticket persistence/revocation checks are at
`M:portal-rs/crates/gameauth/src/admission/tickets.rs:68`, `:81`, and transaction
rollback on issue failure at `M:portal-rs/crates/gameauth/src/admission/store.rs:151`.
The refusal subtest mounts in the wrapper and has a 30–60 s budget.

**Positive test to add — `native_login_34a_preprod`.** New
`client_phases/platform_native_login.rs`, invoked by the portal-rs node-boot lane;
run Game's real native client through web OAuth, character selection, ticket,
Gateway and TLS Game bootstrap. Obtain every grant from Platform, never `sign_grant`.
Assert protocol-2 profile, Registry endpoint/SNI/ALPN, exact Character/World/Channel
and route/runtime revisions; one GameSession, baseline and an acknowledged real
movement; same-attempt retry byte identity; conflicting attempt, spent/expired
60-second ticket, stale generation and disabled/production mode refuse without
new authority. Budget 60–120 s including one real ticket expiry. Mount the runnable
refusal subtest in the wrapper first; do not treat its pass as this positive test.

Repin5: lane Platform pin is `a85b5da4de23f3e8633cfb25a2db1ad7f6133acd`.
Real signing-custody seed now provisions private per-run issuer files; the old
custody blocker is removed. Missing control-scope grant issuer at
`tools/qualification/node_boot/run.sh:999` remains the sole expected terminal blocker.

## 2a. Remembered native-device enrollment, rotation and sign-out

**Game / contract.** ACCEPTED protocol 1, no production rollout authority:
`G:docs/contracts/OTERYN_V2_NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT.md:3`.
Actual native client calls enrollment, character rotation, ticket rotation and revoke
at `G:crates/platform-client/src/native_login/owner_characters/device_transport.rs:87`,
`:136`, `:164`, `:199`; strict credential/successor validation is
`G:crates/platform-client/src/device_sessions.rs:49`, `:62`.

**Platform P → M.** P device routes are `P:routes/api.php:11` (enroll, characters,
ticket, revoke). M uses `M:portal-rs/crates/gameauth/src/devices_http.rs:53`, real
OAuth/native-client authority at
`M:portal-rs/crates/gameauth/src/devices_authority.rs:10`, `:26`, and the direct
verified-HTTPS listener at `M:portal-rs/crates/gameauth/src/devices_tls.rs:16`.
Mount is `M:portal-rs/crates/web/src/lib.rs:478`, binary TLS start at
`M:portal-rs/bin/oteryn-portal/src/main.rs:430`. M uses the new explicit remembered
flag plus APP_ENV and OAuth allowlist; the old PHP flag does not activate it
(`G:docs/contracts/OTERYN_V2_NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT.md:20`).

**Current lane driver — native_device_rotation_replay.**
`crates/platform-client/tests/native_device_rotation_replay.rs` runs after real
browser OAuth and ticket/Gateway refusal. At a85b5da4, enrollment and proofless
rotation must succeed; old schema-refusal BLOCKED outcomes are no longer accepted.
Byte-exact transport capture, account-bound tickets, four credential rotations,
family/absolute-deadline continuity, exact replay/revoke/generation refusal,
disabled OAuth, and offline shipping-client sign-out remain mandatory. Independent
observers require zero Game and Platform admissions. Persistent restart recovery
(U23) and long-expiry soaking remain outside this success claim. Budget 120s.

Repin5 device status: #1844/#1845 accept the contract's original two-member
bodies without proof. Enrollment/rotation must PASS; any refusal is FAIL.
Locator maps remain unset: Platform `runtime.rs:725-748` makes them optional
behind disabled protocol-3/recovery flags; none of these flows calls the locator.

## 3. Admission grants and recovery continuity

**Game / contracts.** Fresh profile:
`G:docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md:3`; recovery profile:
`G:docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md:3`.
Both are ACCEPTED merged baseline profiles with retained conditional Candidate
headers; Game history contains finalization
commits `cae318b88` (fresh) and `e6282b9c4` (recovery), both verified ancestors of
the Game base. Their runtime baselines are
consumed by the accepted gateway contract above; do not derive new production
permission from those old headers. Fast-proof amendment is explicitly **ACCEPTED,
GAME-SIDE implementation only, production NONE** at
`G:docs/contracts/FND-04B_AMEND-1_GAME_RECONNECT_PROOF_V1.md:4`.
Production fresh admission consumes/verifies the grant at
`G:crates/gameplay/dispatch/src/gameplay_transport/composed_admission/admission.rs:5`,
`:134`; the qualification composition is `G:apps/game-server/src/qualification.rs:1712`;
real production recovery resolves current session/fences at
`G:crates/gameplay/dispatch/src/gameplay_transport/resume.rs:253`, `:268`, `:279`.

**Platform P → M.** P has a real fresh signer at
`P:app/GameAuth/NativeAdmission/NativeAdmissionGrantIssuer.php:68`, trust/self-check
at `:11`, TTL 5–30 s at `:33`. M fresh issuance is available with the seeded custody described in section 2; Game control-scope grants remain unavailable.
M native-evidence handler supports separate recovery trust/account profiles at
`M:portal-rs/crates/gameauth/src/evidence.rs:199`, `:207`, but that is a verifier's
source, not a reauthenticated recovery grant issuer. Accepted gateway v1 explicitly
excludes recovery; its protocol-3 recovery amendment remains candidate
(`M:docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md:30`).
No P or M reauthentication-to-recovery signer was established: **UNKNOWN** after
reading the P NativeAdmission issuer, M gameauth module exports
(`M:portal-rs/crates/gameauth/src/lib.rs:3`), admission authority and gateway routes.
Game-only reconnect proofs should not have a Platform issuing route.

**Coverage today (partial, exact scope).** Node boot invokes the seam at
`G:tools/qualification/node_boot/run.sh:1049`; the same phases are composed at
`G:apps/game-server/src/qualification.rs:2764`. Existing evidence:

- `G:apps/game-server/src/qualification/client_phases/admission.rs:95`: `SEAM_EVIDENCE admission=committed server_accepted=1 snapshot=baseline_0_0_0_rev1 step_east=moved_1_0_0_rev2 step_west=moved_0_0_0_rev3 step_north=blocked step_south=blocked spell_cast=rejected_gated unknown_command=rejected command_gap=closed_sequence_gap admissions=1`.
- `G:apps/game-server/src/qualification/client_phases/admission.rs:150`: `SEAM_EVIDENCE replayed_grant=refused admissions=1`;
  `:193`: `SEAM_EVIDENCE concurrent_same_grant attempts=3 accepted=1 admissions=2`.
- `G:apps/game-server/src/qualification/client_phases/recovery.rs:90`: `SEAM_EVIDENCE resume same_session=resumed generation=2 next_command_id=7 server_sequence=8 position=0_0_0_rev3 step_east=moved_1_0_0_rev4 admissions=2`;
  `:98`: `SEAM_EVIDENCE resume replayed_credential=refused`.
- `G:apps/game-server/src/qualification/client_phases/disconnect_resume.rs:30` fetches real Platform generation but
  `:31` signs recovery locally. Its `:57` prints
  `SEAM_EVIDENCE mechanic=disconnect_resume phase=replacement_transport_joined`.
  This does not prove Platform reauthentication or Game-issued fast proof.

**Statuses and tests to add.** Mount separate phases, never collapse all three credentials:

1. **Fresh: RUNNABLE NOW at N (#1825/#1833); historical M was BLOCKED — Platform T19c.** `platform_grant_commit_fences` in
   `client_phases/platform_admission.rs`, sharing the real caller from section 2.
   Admit with Platform's grant, race two real sockets, expire a real grant, revoke
   trust/change authoritative binding between PREPARE and COMMIT, restart and replay.
   Assert one nonce consumption/session, no actor on refusal, current DB fences,
   typed error and no fallback to recovery. Budget 60–120 s.
2. **Reauthenticated recovery: BLOCKED — Platform** (issuer follows T19c;
   CANDIDATE §19 remains default off) real issuer/accepted protocol-3
   recovery exchange (contract decision also needed). `platform_reauth_recovery`
   in `client_phases/platform_recovery.rs`: real browser reauth, actual issued
   recovery grant, drop socket, resume with the dev-client runtime. Assert same
   session/position, higher generation, monotonic sequence, no new admission,
   single-use and old-writer rejection after reload. Budget 60–120 s.
3. **FND-04B fast proof: BLOCKED — Game server and Game client.** At this base,
   `G:crates/gameplay/dispatch/src/gameplay_transport/connection.rs:765` hands
   reconnect_material to the recovery path; `resume.rs:253` parses the signed
   recovery token. The existing negative at
   `G:apps/game-server/src/qualification/client_phases/admission.rs:111` uses
   unissued material and `:127` reports refusal. No successful Game-profile
   issuance/rotation path was established in dispatch, dev-client or client runtime;
   do not count synthetic proof bytes as delivery. `game_fast_reconnect_rotation`
   in `client_phases/platform_fast_reconnect.rs` must retain material from the actual
   ServerAccepted/ServerResumeAccepted, disconnect and return it through the real
   client. Assert same actor, proof rotation, stale/current-candidate replay,
   grace expiry, restart reconstruction and purpose separation at both commits.
   Budget 60–120 s. Requires section 4 for the full ban/kick invalidation assertion.

## 3a. Runtime status, scope assignment and revocation — route-authority dependency

**Game / contract.** Game node starts the real reporter only after committed readiness
(`G:apps/game-server/src/node/serve.rs:3367`); reporter lifecycle is
`G:apps/game-server/src/node/runtime_status.rs:214`. Ops sends assignment/revocation
through `G:crates/gameplay/native-admission-source/src/scope_assignment.rs:290`, `:305`.
The accepted base and RS-A1 scope are recorded, alongside the still-pending §16.2,
at `G:docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md:3`;
compiled routes are `:30`. Platform semantic contract is ACCEPTED at
`P:docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md:5`.

**Platform P → M.** P routes are `P:routes/internal.php:47`, `:54` (runtime and
assignment). M decodes real reports at `M:portal-rs/crates/gameauth/src/runtime.rs:190`
and mounts runtime at `:504`. Assignment and revocation handlers exist at
`M:portal-rs/crates/gameauth/src/scopes.rs:277`, `:308`, `:312`; they and runtime
are mounted indirectly by `M:portal-rs/crates/web/src/lib.rs:476`. The assignment
flag/identity configuration is `M:portal-rs/crates/gameauth/src/scopes.rs:447`.
The actual independent assignment publication, not a manually populated runtime
table, must authorize the observation. T19c still lacks route selection at M (section 2); Platform reports code ready,
PR pending, with route/channel seeds after merge.

**Coverage today.** Not covered cross-repository by node boot: its proxy denies these
paths. `NODE_BOOT_EVIDENCE node registered=1 assignment=operator ungranted=refused content_activation=sequence_1 content_ungranted=refused content_replay=exact content_stale=refused readiness=true control_socket=uid${SERVICE_UID}_0600`
at `G:tools/qualification/node_boot/run.sh:1008` proves Game-local control only.

**RUNNABLE NOW:** `runtime_reports_owner_fences` drives actual ops assignment/revoke
and node readiness/heartbeat through those handlers, asserting real Platform
PostgreSQL assignment/runtime state, wrong-node/older-generation rejection, revoke
and shutdown readback. Mount in the wrapper, budget 60–120 s. No Gateway success
is claimed. **BLOCKED — Platform:** route-selection/issuance extension;
**contract decision** for pending §16.2 must stay explicit. `runtime_route_fencing`
in `tools/qualification/game_platform_runtime/run.sh`, reusing the shared lifecycle,
drives real `ops assignment assign/revoke`, node heartbeat and real Gateway.
Assert no issuance before assignment+readiness, wrong-node/old-generation reports
refused, stale heartbeat and revoke remove eligibility, replacement node cannot
revive an old assignment, shutdown ready=false; no manually seeded runtime rows.
Budget 90–180 s, with real freshness windows.

## 4. Security revocation / Platform staff kick (#1815)

**Game / contract.** **PROPOSED**, Platform producer/staff-kick implementation granted,
Game consumer NONE, production NONE:
`M:docs/contracts/OTERYN_V2_ACCOUNT_SECURITY_REVOCATION_STREAM_CONTRACT_PROPOSAL.md:3`.
Required Game terminalization and cause-3 semantics are at `:76`. Game's current
native-evidence consumer checks generation (`G:apps/game-server/src/qualification.rs:247`)
and recovery reads current sessions (`G:crates/gameplay/dispatch/src/gameplay_transport/resume.rs:268`);
neither is a continuous revocation poller. Game's local staff-kick durability is a
different authority (`G:crates/gameplay/durability/src/durability/staff/executor.rs:439`).
No revocation-stream consumer is exported by the real integration module
`G:crates/gameplay/native-admission-source/src/lib.rs:3`.

**Platform P → M.** No P security-revocation route in `P:routes/internal.php:33`
through `:117`; P admission/generation evidence is not this stream.
**#1815 MERGED**, proved by Platform main log commit
`640b22b2b0e31ae677f142b862d722c97c6da4d5` (subject includes #1815), not merely an open PR.
M mounts export at `M:portal-rs/bin/oteryn-portal/src/main.rs:411`;
route/flag at `M:portal-rs/crates/identity/src/security_revocations.rs:25`, `:35`,
ordered PostgreSQL read/reconciliation at `:228`, staff-kick generation mutation at
`:171`. Real admin permission/recheck/audit transaction is
`M:portal-rs/crates/admin/src/players/mod.rs:303`, `:322`, `:330`;
HTTP action dispatch is `M:portal-rs/crates/admin/src/players/http.rs:256`.

**Coverage today.** Not covered; no NODE_BOOT_EVIDENCE/SEAM_EVIDENCE staff kick or
stream terminalization assertion. Stale-evidence admission rejection cannot prove
that an already admitted actor becomes terminal.

**Status: BLOCKED — contract decision + Game server**, no accepted/authorized Game
consumer, plus fast-proof delivery from section 3. `security_revocation_terminal`
in `client_phases/platform_revocation.rs` will admit through the real login path,
then use real Platform password-change, sanction and MFA-confirmed admin kick
routes. Let Game's actual poller consume the stream; never call its apply helper
from the test. Assert persisted ordered cursor only after terminal effects, duplicate
revocation idempotency, cause 3 closes even at equal generation, active and
reconnectable sessions terminate, old/candidate proofs and old writers cannot
resume, other account unaffected, restart does not resurrect. Include denied
permission/MFA and rolled-back kick (no audit/generation/stream partial commit).
During a real export outage, assert current play continues, fast reconnect fails
closed once the registered freshness proof expires, and no session revives. After
password change/ban lift/kick resolution, a real fresh login must succeed under
current permitted generation; this is required at
`M:docs/contracts/OTERYN_V2_ACCOUNT_SECURITY_REVOCATION_STREAM_CONTRACT_PROPOSAL.md:85`.
Owner decision 3 at `:105` already chooses fail-closed reconnect; the numeric bound
is to be registered by the Game implementation.
Budget 120–180 s. The numeric polling/terminalization SLA is **UNKNOWN**;
do not invent a five-second live-kick deadline from FND-04 admission freshness.
A real admin/export-only test can run now, but does not satisfy this Game E2E.

## 5. Character list and purpose-separated internal routes (#1814)

**Game / contracts.** ACCEPTED LCFA revision 2 for testing/preproduction at
`G:docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md:3`;
authority/owner-only limits at `:29`, certificate separation at `:111`.
Real producer: `G:apps/game-server/src/node/serve.rs:1117`, durable store at `:1142`,
real mTLS sink `G:crates/gameplay/native-admission-source/src/account_characters.rs:291`.

**Platform P → M.** P snapshot/watermark routes: `P:routes/internal.php:61`, `:68`,
handler `P:app/Http/Controllers/GameAuth/NativeAccountCharactersController.php:25`.
M LCFA handler mounts snapshot/watermark at
`M:portal-rs/crates/characters/src/lcfa/http.rs:63`, `:82`; real binary composition is
`M:portal-rs/bin/oteryn-portal/src/game_login.rs:86`. Owner-facing GET list and ticket
are `M:portal-rs/crates/player-api/src/lib.rs:49`; issuer checks freshness and
ownership at `M:portal-rs/crates/gameauth/src/admission/tickets.rs:23`.
**#1814 MERGED**, Platform main log commit
`445c0c9b941f8633d2ece2dc8ccbf9d66cf3d135`. It mounts command/restore/bootstrap,
premium/loyalty/Store and online/daily-kill routes with independent default-off
flags (`M:portal-rs/bin/oteryn-portal/src/game_routes.rs:8`, `:129`, `:137`, `:168`,
`:180`). LCFA itself was wired by #1812, not newly invented by #1814.

**Coverage today.** Not covered by node_boot proxy. The manual login_local
projection-accepted line at `G:tools/qualification/login_local/run.sh:660` is related
legacy evidence only; no NODE_BOOT_EVIDENCE owner-list isolation assertion exists.

**Status: RUNNABLE NOW** for LCFA ingestion and owner list in `testing`/isolated
`preprod`; successful grant issuance remains blocked independently.
`lcfa_owner_freshness_and_routes` in `client_phases/platform_characters.rs` drives
real Game Character creation/mutation, lets the actual publisher send snapshots
and watermarks, then calls Game's native owner-character consumer with real OAuth.
Assert A's real characters/world/name/availability, B isolation, missing snapshot
is unavailable (not fabricated empty), stale watermark denies, identical retry
accepted, conflicting same revision quarantines, older epoch cannot roll back.
Restart publisher with its real fence and verify resync/recovery. Probe #1814 routes
with the LCFA cert and assert refusal, and public listener cannot reach internal
handlers. Budget 90–150 s including >30 s stale-feed observation. Receipt/list
success without a live watermark is not a pass.

## 5a. Character creation, bootstrap intents and command exchange

**Game / contract.** ACCEPTED commands at
`G:docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md:3`; boundary ACCEPTED
at `G:docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md:5`.
Game polls/decides/publishes real receipts at
`G:apps/game-server/src/node/serve.rs:1481`, `:1516`; transport is
`G:crates/gameplay/native-admission-source/src/character_commands.rs:421`.
Bootstrap input consumer is `G:crates/gameplay/durability/src/character_bootstrap_intent.rs:1`.
The Platform bootstrap contract defines only operator control-plane issuance, not
web creation (`P:docs/contracts/OTERYN_V2_CHARACTER_BOOTSTRAP_INTENT_PRODUCER_CONTRACT.md:5`).

**Platform P → M.** P command pending/read/receipt register at
`P:app/CharacterAuthority/Http/CharacterCommandRoutes.php:15`; bootstrap read at
`P:routes/internal.php:75`, handler
`P:app/Http/Controllers/GameAuth/CharacterBootstrapIntentController.php:16`.
M mounts commands through `M:portal-rs/bin/oteryn-portal/src/game_routes.rs:98`, `:129`;
handlers at `M:portal-rs/crates/characters/src/commands/http.rs:28`.
Real web creation is `M:portal-rs/crates/characters/src/create_web/http.rs:50`, `:81`;
its separate producer flags differ from the exchange flags. M bootstrap HTTP is read-only
(`M:portal-rs/crates/characters/src/bootstrap/mod.rs:1`, `:50`), mounted at
`M:portal-rs/bin/oteryn-portal/src/game_routes.rs:137`.
The governing producer contract is
`M:docs/contracts/OTERYN_V2_CHARACTER_BOOTSTRAP_INTENT_PRODUCER_CONTRACT.md:5`
(contract version 3; the test name need not encode its version).
N supplies the production `oteryn-portal bootstrap-intent issue --actor <uuidv7> [--input file]`
command (#1828), dispatched at `N:portal-rs/bin/oteryn-portal/src/main.rs:32`.
Input is JSON (stdin by default) with `identityId` and the other required intent
fields (`N:portal-rs/bin/oteryn-portal/src/bootstrap_intent.rs:104`, `:196`).
`GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_ENABLED` is default off; enable it only
inside the lane's disposable harness, with explicit
`GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS` in 1..=300 (`:138`, `:165`).
Canonical intent bytes go to stdout, structured errors to stderr (`:257`).
**RUNNABLE NOW against N**: `operator_bootstrap_replay` must cover exit 2 bad input,
3 conflict, 4 unavailable (`:52`–`:54`), exact issuance replay returning the same
canonical bytes, and Game consumption/receipt replay without duplicate effects.
This is planned harness enablement against N; the audited staff actor must hold `game.player.manage` (#1831); the lane does not
enable the issuer flag.

**Coverage today.** Real P bootstrap is covered: node_boot `character_bootstrap`
prints `NODE_BOOT_EVIDENCE characters=${#BOOTSTRAP_ACCOUNTS[@]} source=platform_intents path=control_socket retry=idempotent`
(`G:tools/qualification/node_boot/run.sh:1027`, `:1043`). The seam prints
`SEAM_EVIDENCE owners=composed holder=assigned characters=2 source=platform_operator_intents`
(`G:apps/game-server/src/qualification.rs:1709`). This covers neither browser
CreateCharacter nor M bootstrap issuance.

**Statuses.** **RUNNABLE NOW**: bounded CreateCharacter exchange in `testing`.
**RUNNABLE NOW against N**: operator-bootstrap E2E through the production issuer;
never insert a canned `intent_json` row as its replacement.

**Tests to add.** `character_create_receipt_idempotency` in
`client_phases/platform_character_commands.rs`: browser submits real CreateCharacter,
Game's consumer pulls and commits, Platform stores Game receipt; assert one Game
UUID/name reservation/root and one immutable receipt, pending → terminal UX,
operation retry, expired intent rejection, duplicate-name race, unauthorized account
and wrong-purpose certificate refusal. Read authoritative Game PostgreSQL and then
LCFA convergence; budget 90–150 s. `operator_bootstrap_replay` mounts in the wrapper
once an issuer exists: issue through it, real ops bootstrap twice, compare Character,
explicit sex/name/world/interpretation bindings, expired/changed-operation denial;
budget 30–60 s. Keep the existing P operator test runnable separately.

## 5b. Restore notice and reopened command receipts

**Game / contract.** ACCEPTED restore reconciliation default-off contract at
`M:docs/contracts/OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md:5`, paired with
accepted Game commands (`G:docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md:3`).
Real Game sender is `G:crates/gameplay/native-admission-source/src/restore_notice.rs:234`,
repeat-until-ack at `:280`.

**Platform P → M.** P registers notice at `P:routes/internal.php:112` and reopened
exchange through CharacterCommandRoutes. M composite handler is
`M:portal-rs/crates/characters/src/restore/http.rs:43`, configured at
`M:portal-rs/bin/oteryn-portal/src/game_routes.rs:121`, `:129`.

**Coverage today.** Not covered; node_boot restart-without-supersession at
`G:tools/qualification/node_boot/run.sh:1107` is a Game custody check, not Platform
restore acknowledgment. No restore notice evidence line exists in these phases.

**Status: RUNNABLE NOW**, bounded accepted restore exchange in testing.
`restore_reopens_exact_receipts` in `tools/qualification/game_platform_restore/run.sh`
reuses node_boot lifecycle, runs a real completed command, makes/restores a disposable
Game backup through the actual restore tool and sends its actual notice. Assert
Platform ack only after reconciling affected operations, immutable notice retry,
reopened command converges to existing Game effect rather than repeating it,
wrong generation/source fails, unavailable Platform keeps retry state across restart.
Budget 120–240 s. Do not reuse the retained environment database for this exercise.

## 5c. Account closure and Bazaar custody — destructive cross-repository sagas

**Game / contracts.** Separate **PROPOSED/CANDIDATE** finalization and Bazaar contracts:
`G:docs/contracts/OTERYN_GAME_ACCOUNT_DELETION_FINALIZATION_V1.md:3`,
`G:docs/contracts/OTERYN_GAME_BAZAAR_AUTHORITY_V1.md:3`; lock confirms nonacceptance
at `G:docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json:333`, `:363`.
Actual Game integration surfaces are closure publication alongside command consumption
(`G:apps/game-server/src/node/serve.rs:1509`) and Bazaar transport
(`G:crates/gameplay/native-admission-source/src/bazaar/mod.rs:1`,
`G:crates/gameplay/native-admission-source/src/descriptor.rs:134`).

**Platform P → M.** P has generic account-transfer command routes at
`P:app/CharacterAuthority/Http/CharacterCommandRoutes.php:18`; that is not proof of
the later closure/Bazaar-v2 private handshake. Those later P handlers are UNKNOWN
from the pinned route inventory `P:routes/internal.php:105`.
M closure boundary and dedicated listener:
`M:portal-rs/bin/oteryn-portal/src/account_closure.rs:33`, `:63`;
real Bazaar runtime/worker/listener:
`M:portal-rs/crates/bazaar/src/game/runtime/mod.rs:30`,
`M:portal-rs/crates/bazaar/src/game/runtime/tls.rs:11` and binary start
`M:portal-rs/bin/oteryn-portal/src/main.rs:93`. Existing implementation does not
supersede the unaccepted cross-repository contracts.

**Coverage today.** Neither saga is covered by node_boot/seam/dev-client phases;
no corresponding evidence line.

**Status: BLOCKED — contract decision** for each full saga; exact live compatibility
also UNKNOWN, requiring a follow-up exact-schema qualification, not stub evidence.
`closure_cancel_finalize_race` and `bazaar_custody_settlement_replay` mount in separate
`tools/qualification/game_platform_closure/run.sh` and
`tools/qualification/game_platform_bazaar/run.sh`, sharing node_boot lifecycle.
Drive real authenticated termination/cancel and Bazaar listing/bid/worker actions;
let Game pull actual intents. Closure asserts admission closed, reversible quarantine
before finalization, cancel/worker one winner, LCFA/hash/generation proof before
Platform finalization and no destructive effect on stale evidence. Bazaar asserts
single escrow custody, immutable receipts, one settlement, losing bidder released,
no double gold/coin effect on crash/retry and owner change reflected privately.
Budget 180–300 s each with accepted test clocks for long business deadlines;
otherwise report real-duration prerequisite, never shorten production timeouts.

## 5d. Coin transfers and coin-market settlement

**Game / contract.** ACCEPTED `G:docs/contracts/PLATFORM_COIN_WIRE_V1.md:3`,
Game gift/market ownership at `G:docs/contracts/COIN_TRANSFER_GAME_V1.md:3` and
`G:docs/contracts/COIN_MARKET_GAME_V1.md:3`. Real mTLS caller:
`G:crates/gameplay/native-admission-source/src/coins.rs:45`; Game owns gold,
matching and retry, Platform owns every coin.

**Platform P → M.** P internal route inventory `P:routes/internal.php:33` through
`:117` contains no coin wire; no P Wallet handler for this accepted wire established.
M mounts actual guarded Wallet routes at
`M:portal-rs/bin/oteryn-portal/src/main.rs:405`; route families/transfers, market,
operation reconciliation, balance/history are
`M:portal-rs/crates/wallet/src/wire.rs:38`, `:49`.

**Coverage today.** Not covered cross-repository. Existing Game `client_phases/market.rs`
is not evidence of Platform coin settlement; pinned proxy has no coin paths.

**RUNNABLE NOW bounded test:** `coins_wire_transfer_replay` calls the actual Game
`CoinClient::execute` (`G:crates/gameplay/native-admission-source/src/coins.rs:116`)
over real mTLS to Platform, including balance/history and operation retry. Fund
disposable accounts through the real authorized Wallet ledger service; never
insert balance rows. It asserts Platform debit/credit, transferable split, receipt
idempotency and changed-operation conflict, but claims no Game gold or native UI
coverage. Mount in the wrapper, budget 30–60 s.

**Full gameplay status: BLOCKED — Platform admission (section 2).**
`coins_transfer_market_exactly_once` in `client_phases/platform_coins.rs` drives real
client transfer/market commands and actual Game retry transport. Assert transferable
split, exact recipient account resolved from a real Character, insufficient/own-account
refusal, one debit+credit or reservation/settlement, one Game gold effect, equal
operation retry idempotent, conflict rejected, restart reconciliation from real
receipt and balance/history. Budget 90–180 s. No fake Wallet or SQL coin balances.

## 5e. Premium entitlement and loyalty evidence

**Game / contracts.** Premium pull uses the real client and accepted PREM-P binding
at `G:crates/gameplay/social/src/premium/client.rs:1`, `:17`; default-unconfigured
Free behavior is at `:9`. Accepted producer reconciliation is recorded in
`G:docs/architecture/reviews/OTERYN_GAME_PREMIUM_DELIVERY0_PREMIUM_EVIDENCE_TRANSPORT_DECISION_2026-09-30.md:16`
(despite its old conditional Candidate header at `:4`). Server composition is
`G:apps/game-server/src/node/serve.rs:3406`. Do not confuse this internal pull with
the supported native Premium overview at
`G:docs/contracts/OTERYN_PREMIUM_PANEL_V1.md:11`: capability 97 / command 113
op 5 reads the existing Game Premium fence fed by `premium-snapshot-v1`.
It needs no separate Platform JSON producer; the retained pre-login projection
in the same contract is a different surface.
Loyalty snapshot remains **PROPOSED** at
`M:docs/contracts/OTERYN_V2_LOYALTY_SNAPSHOT_CONTRACT.md:3`; Game's skill projection
reads its durable state at
`G:crates/gameplay/dispatch/src/gameplay_transport/loyalty_skills.rs:57`, not proof of
a live Platform stream consumer.

**Platform P → M.** P Premium read route `P:routes/internal.php:81`.
M actual Premium and loyalty snapshot routes/flags:
`M:portal-rs/crates/store/src/premium_http.rs:22`, `:31` and
`M:portal-rs/crates/store/src/loyalty_http.rs:22`, `:32`, mounted at
`M:portal-rs/bin/oteryn-portal/src/game_routes.rs:148`.
M also mounts loyalty stream export at
`M:portal-rs/bin/oteryn-portal/src/main.rs:414`; no corresponding P stream route
was established from its internal route inventory.

**Coverage today.** Not covered by node_boot proxy; no Premium/loyalty end-to-end
NODE_BOOT_EVIDENCE/SEAM_EVIDENCE. Feature selection alone is not evidence.

**Statuses and tests.** **RUNNABLE NOW:** `premium_snapshot_fail_closed` in
`client_phases/platform_entitlements.rs`, real Platform entitlement service changes
on disposable accounts and actual Game PremiumClient/coordinator pull, using
`M:portal-rs/crates/store/src/premium/repository.rs:50` with real authorized staff
context, not a test producer. This bounded service test needs no admitted session.
Assert no entitlement → Free, valid bounded lease → current, expiry/revocation,
stale/older/conflicting snapshot and unavailable producer cannot grant benefits,
restart preserves revision floor. The additional real headless Premium-gated
action assertion is BLOCKED — Platform admission; do not count a coordinator
service test as native gameplay coverage. Budget
90–180 s; use actual entitlement expiry under accepted test clock support.
**BLOCKED — contract decision + Game server:**
`loyalty_stream_converges` until the chosen snapshot/stream contract and real
consumer are established. Drive actual Platform credit/referral events, Game pull
and headless skill query; assert account-wide points, exactly-once ingestion,
no duplicate skill gain after restart and stale evidence fails closed. Mount in
the same phase, budget 90–150 s.
**RUNNABLE NOW — Game side:** `native_premium_overview_refresh`, 30–60 s,
drives capability 97 / command 113 op 5 in an authenticated Game session against
the real Premium snapshot from Platform's portal-rs/e2e harness. Refresh observes
the existing fence's expiry/revision and fails closed for stale/unavailable
evidence. See `G:docs/contracts/OTERYN_PREMIUM_PANEL_V1.md:11`, `:24`, `:72`.
The shared harness/admitted-session prerequisite still applies; this is runnable
Game work, not an executed E2E success or a separate Platform JSON consumer.

## 5f. Public projections and privacy watermarks

**Game / contracts.** ACCEPTED V1 families and V2 deaths/guild/house:
`G:docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md:3`,
`G:docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V2.md:3`. Game producer loop:
`G:apps/game-server/src/node/serve.rs:1219`; compiled routes include the additional
online-session/daily-kill/totals families at
`G:crates/gameplay/native-admission-source/src/descriptor.rs:120`.
Acceptance of those extensions is UNKNOWN from V2's narrowly enumerated acceptance;
do not generalize it to all compiled families.

**Platform P → M.** P public snapshot/watermark routes at
`P:routes/internal.php:87`, `:99`. M mounts community public families at
`M:portal-rs/bin/oteryn-portal/src/main.rs:106` and `:125`;
#1814 additionally composes v2 online sessions/daily kills and dispatches shared
watermarks at `M:portal-rs/bin/oteryn-portal/src/game_routes.rs:180`, `:198`, `:214`.

**Coverage today.** Not covered cross-repository by the pinned proxy; no public
privacy/freshness evidence line in node_boot or the inspected client phases.

**Status: RUNNABLE NOW** for the bounded accepted profile/highscore/watermark slice;
positive V2 deaths require the admission dependency below. V2 guild/house remain
within accepted scope and need their own real lifecycle actions for qualification;
**BLOCKED — contract decision** for extension acceptance not established above.
`public_projection_privacy_floor` in `client_phases/platform_public.rs` drives real
Game ops/Character-authority mutations, the running Game publisher and actual
public portal HTTP reads; qualify character profiles/highscores and privacy
watermarks without fabricating live sessions or death records. Assert
only accepted allowlisted fields, no AccountId/session/placement leakage, real
visibility/owner changes suppress data, stale feed hides rather than serves stale
presence, same revision conflict fails closed, restart resync cannot lower epoch.
No constructed snapshots as the producer. Budget 90–180 s. A positive death/online
session assertion additionally needs real gameplay, BLOCKED — Platform admission. Later
`public_online_daily_totals` uses actual login/logout and kill/total producers,
checks shared watermark family dispatch and exactly-once counters after retries;
same mount, 90–180 s after acceptance and the admission dependency are resolved.

## 5g. World-event outcomes and guild-season feed (T94)

**Game / contract.** **PROPOSED/CANDIDATE** at
`G:docs/contracts/OTERYN_GAME_WORLD_EVENT_OUTCOMES_AND_GUILD_SEASON_V1.md:3`,
lock `G:docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json:393`.
Real publisher/transport abstraction:
`G:crates/gameplay/native-admission-source/src/world_event_outcomes/publisher.rs:73`,
`:120`; records explicitly are not source authority at
`G:crates/gameplay/native-admission-source/src/world_event_outcomes.rs:1`.

**Platform P → M.** No P event/season routes in the pinned internal route assembly
`P:routes/internal.php:87`; a prior complete equivalent is UNKNOWN.
M feed paths and authenticated admission are
`M:portal-rs/crates/minigames/src/league/admission.rs:8`, `:50`; current durable
feed fences at `M:portal-rs/crates/minigames/src/league/runtime.rs:34`.

**Coverage today.** Not covered; no T94 evidence line in node_boot/seam.
**Status: BLOCKED — contract decision**, plus exact authoritative Game-source and
listener qualification UNKNOWN. `league_outcome_correction_fences` in
`tools/qualification/game_platform_league/run.sh` reuses node lifecycle, drives a
real Game event completion/guild-season producer and real Platform worker/owner
read. Assert retained retry identity, current epoch/recovery fence, correction
provenance, stale/conflict suppression and no monetary/reward mutation. Budget
120–240 s after accepted source/test-clock decisions; never fabricate outcome records.

## 6. Private journal T72 (G-9 / G-10 / G-11)

**Game / contracts.** Platform journal sessions/items/progress contracts are all
**ACCEPTED revision 3** for sessions/items and **ACCEPTED revision 4** for
progress at line 1 of
`M:docs/contracts/OTERYN_V2_GAME_JOURNAL_SESSIONS_CONTRACT_PROPOSAL.md`,
`M:docs/contracts/OTERYN_V2_GAME_JOURNAL_ITEMS_CONTRACT_PROPOSAL.md`, and
`N:docs/contracts/OTERYN_V2_GAME_JOURNAL_PROGRESS_CONTRACT_PROPOSAL.md`.
N verifies the progress revision 4 dependency (Platform #1827): `result` is
`accepted|refused`, with sole terminal refusal reason `CHARACTER_SCOPE_DELETED`
(`N:docs/contracts/OTERYN_V2_GAME_JOURNAL_PROGRESS_CONTRACT_PROPOSAL.md:15`;
`N:portal-rs/crates/playerdata/src/store.rs:232`, `:934`). Exact accepted replay
keeps its stored result; terminal refusal consumes its revision. Sessions/items
remain revision 3.
Their filenames saying PROPOSAL do not undo acceptance; acceptance explicitly does
not start Game producers. Game's existing quest source is
`G:crates/gameplay/quests/src/quest/log/journal_source.rs:1`; the real dev-client
quest query is `G:apps/game-server/src/qualification/client_phases/quest_journal.rs:2`.
Neither is a T72 owner-private canonical ingest producer. The integration module's
actual exports (`G:crates/gameplay/native-admission-source/src/lib.rs:3`) do not
establish that producer.

**Platform P → M.** No P journal ingest in `P:routes/internal.php:33` through `:117`;
any other pinned private listener is UNKNOWN. M exposes an adapter for records/cursor
at `M:portal-rs/crates/playerdata/src/items/ingest.rs:27`, `:41`, but baseline/transfer
Evidence deliberately denies (`M:portal-rs/crates/playerdata/src/items/store.rs:104`).
Dashboard runtime installs Unavailable (`M:portal-rs/crates/playerdata/src/dashboard/mod.rs:217`).
G-11 projection calls still require T72 canonical admission
(`M:portal-rs/crates/playerdata/src/logins.rs:204`). Tables/migrations and these
feature adapters are not a mounted, authorized journal authority.

**Coverage today.** Not covered across Game/Platform. `quest_journal.rs:18` exercises
Game QuestLog List and `:42` captures its payload for client-runtime replay;
`G:tools/qualification/e2e_all/run.sh:21` checks captures. This is not T72 receipt,
cursor, privacy or portal readback evidence. No corresponding cross-repository
NODE_BOOT_EVIDENCE/SEAM_EVIDENCE exists.

**Status: BLOCKED — Game server + Platform.** Game needs real G-9/G-10/G-11 producers,
durable outbox/baseline/transfer fences; Platform needs T72 admission/listener,
ownership evidence and consumer assembly. Accepted semantics do not remove these
implementation blockers. Open producer choices remain UNKNOWN per the contracts.

**Test to add — `journal_t72_owner_replay_privacy`.**
`client_phases/platform_journal.rs`, after actual admission and real World actions:
perform movement/hunt, real item/bank transaction and real progress/login events;
let actual Game outbox deliver to T72, restart sender/consumer around an ack, and
read the owner portal with a real web session. Assert exact source records and
cursor, no duplicate after replay/lost ack, gaps withhold completeness, account B
cannot read A, opt-out/transfer hides immediately, purge within accepted 24 h with
content-free tombstones, replay cannot resurrect erased data. Bank account scope
must survive Character transfer; never use G-10 as admin analytics. Budget 120–240 s
for live ingest/replay; purge is a separate accepted-clock test or real 24 h soak.
Use actual source IDs/receipts, never placeholder journal rows or permissive Evidence.

## 6a. Offline catalogue → Platform verified cache

**Game / contract.** Contract text says PROPOSED-until-merged at
`G:docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md:9`; merged canonical
reference is LOCKED at `G:docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json:121`,
`:125`, and `:152` records the merged reference producer. This is an accepted
physical compatibility baseline, not permission to activate populated content.
Real exporter invokes CONTENT at `G:tools/game-platform-catalog/build.py:17`, `:39`;
current capability completeness stays unsupported/unknown
(`G:tools/game-platform-catalog/README.md:9`).

**Platform P → M.** P has the importer/semantic-validator source under
`P:app/GameCatalog/Application/Import/Native/NativeCatalogEnvelopeValidator.php:29`
(bounded read and artifact-hash validation);
M verified cache/refresh implementation is
`M:portal-rs/crates/world/src/catalog/mod.rs:1`,
`M:portal-rs/crates/world/src/catalog/refresh.rs:35`, invoked by the real binary at
`M:portal-rs/bin/oteryn-portal/src/main.rs:76`. This flow is artifact/cache ingestion,
not a game-session HTTP RPC.

**Coverage today.** Not covered by node_boot/seam; no catalogue-to-Platform evidence.
**Status: RUNNABLE NOW** for honest unsupported-artifact fidelity/refusal;
**BLOCKED — Game server/content producer** for populated supported content.
`catalog_real_export_cache_fidelity` in `tools/qualification/game_platform_catalog/run.sh`
reuses stack provisioning, runs the actual exporter on committed Game content,
serves its exact artifact through the configured local artifact origin and lets
Platform's actual refresh job import it. Assert semantic+transport digest/provenance,
PostgreSQL cache readback, unsupported remains unsupported and no false complete
catalogue is published. Corrupt the transport copy, not the canonical source, to
assert refusal and preserved last-good state. Budget 60–180 s plus exporter build.
A static artifact origin is transport, not a fake Game producer.

## 7. Store purchase delivery (last; default off)

**Game / contract.** **PROPOSED, NOT ACCEPTED**, exact-head review remains required
at `M:docs/contracts/OTERYN_V2_STORE_PURCHASE_DELIVERY_CONTRACT.md:5`, `:16`;
`:18` authorizes no activation, charge or mint. Existing Game transport is
`G:crates/gameplay/native-admission-source/src/store.rs:13`, `:306`, `:419`, `:473`;
compiled purchase/balance/history paths at `:17`. Code existence does not accept
the contract.

**Platform P → M.** P internal route set at `P:routes/internal.php:75` through `:117`
has Premium/commands/restore but no Store wire. M has real purchase/balance/history
handlers at `M:portal-rs/crates/store/src/purchase_http.rs:20`, delivery
pending/read/receipt/reconcile at `M:portal-rs/crates/store/src/delivery_http.rs:19`,
`:41`; #1814 mounts delivery at
`M:portal-rs/bin/oteryn-portal/src/game_routes.rs:168`. Default purchase/delivery
flags are false at `M:portal-rs/crates/store/src/purchase.rs:62`.

**Coverage today.** Explicitly skipped:
`G:apps/game-server/src/qualification/client_phases/store.rs:8` prints
`SEAM_EVIDENCE mechanic=store skipped=pinned_platform_has_no_store_api`.
This is a coverage gap, never Store success.

**Status: BLOCKED — contract decision** for positive delivery; pinned topology also
lacks endpoints. **RUNNABLE NOW refusal slice:** `store_default_off_no_effect`
uses the real caller against M with both flags false, asserts unavailable/no
purchase, no Wallet debit, operation, Game item or receipt. Mount in the wrapper
using the actual Game StoreClient, budget at most 30 s. Do not enable the flags
to produce a positive demonstration in this documentation task.

**Test to add after acceptance — `store_purchase_delivery_replay`.**
`client_phases/platform_store_delivery.rs` mounted by node boot's portal-rs lane.
Headless client buys an actual Game-content offer, Game calls actual Platform
purchase, then its real delivery consumer claims the real pending operation and
publishes the durable receipt. Assert actual quote/revision/price, ledger debit
once, one intent, claim on allowed profile/world, one DUR-03 mint or entitlement
application, identical operation retry, changed-payload conflict, lost receipt/
restart reconciliation, refund/revoke-unclaimed and reclaim shortfall semantics,
restored-backup purchase latch blocks new purchase until actual reconciliation.
Use authorized disposable Wallet funding; no payment provider, fake delivery or
hand-inserted operation. Budget 120–240 s plus separate restore lane. Default-off
refusal belongs first in this phase and runs even while success stays blocked.

## Ordered build list (runnable work first)

1. Add the portal-rs/PostgreSQL lifecycle adapter to node_boot and wrapper described
   above; retain existing P seam and per-purpose TLS. Print exact immutable Game,
   Platform, topology, database-version and World-bundle provenance. Refuse a missing
   product handler; never fall back to P for an M success claim. Implementation
   added in GPE2E-2, runtime unverified: `GAME_PLATFORM_RESULT=FAIL flow=preflight
   reason=missing_binary_sudo`. Platform lifecycle delegates to `portal-rs/e2e/start.sh`.
2. `web_identity_pkce_isolation`, including web MFA using testing-only
   `seed account-mfa` private TOTP/recovery outputs; add `native_login_mfa_account`
   against N (#1826 proven-session step-up), then runnable
   `native_ticket_and_gateway_fail_closed` and `native_device_rotation_replay`.
   These establish identity and honest
   current Gateway failure before any downstream success assertion.
3. `operator_bootstrap_replay` against N production `bootstrap-intent issue`:
   disposable-harness-only enable flag, explicit TTL 1..=300, JSON `identityId`,
   canonical stdout, exit 2/3/4 and exact replay assertions; then
   `runtime_reports_owner_fences`, then `character_create_receipt_idempotency` →
   `lcfa_owner_freshness_and_routes` in
   testing, then isolated preprod owner-list qualification. These produce real
   Character data needed by the later native-login success test.
4. `restore_reopens_exact_receipts`, `coins_wire_transfer_replay`,
   `premium_snapshot_fail_closed`, `native_premium_overview_refresh`,
   `public_projection_privacy_floor`,
   `catalog_real_export_cache_fidelity`, `store_default_off_no_effect`.
   Keep disabled/proposed families separate from accepted positive slices.
5. Use N #1833 route seeds and #1825 admission signer; qualify Game assignment,
   encrypted signing custody and environment gates; add
   `runtime_route_fencing` → `native_login_34a_preprod` →
   `platform_grant_commit_fences`, then the full client/gold test
   `coins_transfer_market_exactly_once` and live Premium/public gameplay assertions.
   No manual runtime snapshots or test-signed grants.
6. After T19c, unblock real recovery issuance (CANDIDATE §19, default off)
   and Game proof delivery; add
   `platform_reauth_recovery` and `game_fast_reconnect_rotation`.
7. Accept/authorize Game security-stream consumption and add
   `security_revocation_terminal`. This remains ahead of lower-risk blocked feeds.
8. Resolve closure/Bazaar contracts and loyalty
   source/consumer, then build their tests. Resolve public extension acceptance,
   T94 authoritative-source qualification and populated catalogue producer separately.
9. Complete T72 Game producers and Platform assembly against progress rev 4
   (#1827, `result: accepted|refused`, sole refusal `CHARACTER_SCOPE_DELETED`);
   add its privacy/replay test.
10. Accept Store delivery on an exact reviewed contract revision, then implement
    `store_purchase_delivery_replay`; retain default-off until separate enablement.

| Flow | Status | Blocker owner | Test name |
|---|---|---|---|
| Account web login + PKCE | RUNNABLE NOW; GPE2E-2b repairs root-owned state base and fail-fast operator setup; callback state uses shipping IdentityFlow | Live rerun required; no PASS evidence | `web_identity_pkce_isolation` |
| MFA-protected web login success | RUNNABLE NOW | N #1828: testing-only `seed account-mfa` returns `totpSecret` + recovery codes; web TOTP exists | `web_identity_pkce_isolation` (MFA extension) |
| Native OAuth for an MFA account | RUNNABLE NOW | N #1826: proven web MFA accepted; unproven required MFA refused (`oauth.rs:230`); lane pin now includes #1833 | `native_login_mfa_account` |
| Ticket + Gateway refusal slice | DRIVER IMPLEMENTED at a85b5da4; custody provisioned with real signing-custody seed | Exact ROUTE_UNAVAILABLE until real owner projection/control grant; zero admissions and ticket rollback required | `native_ticket_and_gateway_fail_closed` |
| Remembered native-device session | DRIVER IMPLEMENTED at a85b5da4; PASS required for two-member enrollment and proofless rotation; U23 excluded | Live rerun required; no Platform device blocker accepted; exact ownership/replay/revoke/generation and zero-admission checks | `native_device_rotation_replay` |
| Native Gateway mode 34a preprod success | RUNNABLE NOW | N #1833 route seeds + #1825 admission signer; Game assignment/custody and environment qualification still required | `native_login_34a_preprod` |
| Fresh admission from Platform grant | RUNNABLE NOW | N #1833 route seeds + #1825 admission signer; Game assignment/custody and environment qualification still required | `platform_grant_commit_fences` |
| Reauthenticated recovery | BLOCKED | Platform: recovery issuer follows T19c; CANDIDATE §19, default off; contract decision | `platform_reauth_recovery` |
| Game fast reconnect proof | BLOCKED | Game server + Game client: issuance/delivery/rotation | `game_fast_reconnect_rotation` |
| Runtime/assignment/revocation reporting | RUNNABLE NOW | — (existing identity rules) | `runtime_reports_owner_fences` |
| Runtime/assignment/revocation → route | RUNNABLE NOW | N #1833 route seeds + #1825 admission signer; Game assignment/custody and environment qualification still required | `runtime_route_fencing` |
| Security revocation/staff kick → Game | BLOCKED | Contract decision + Game server consumer | `security_revocation_terminal` |
| LCFA owner list/internal isolation | RUNNABLE NOW | —; full preproduction gate discrepancy separate | `lcfa_owner_freshness_and_routes` |
| Browser Character command/receipt | RUNNABLE NOW | — (testing slice) | `character_create_receipt_idempotency` |
| Operator bootstrap on portal-rs | RUNNABLE NOW | N #1828: production issuer; disposable harness enables default-off gate; TTL 1..=300; exit 2/3/4 + replay | `operator_bootstrap_replay` |
| Restore notice/reopened receipts | RUNNABLE NOW | — | `restore_reopens_exact_receipts` |
| Account closure saga | BLOCKED | Contract decision | `closure_cancel_finalize_race` |
| Bazaar custody/settlement saga | BLOCKED | Contract decision | `bazaar_custody_settlement_replay` |
| Coin transfer/market wire | RUNNABLE NOW | — (bounded service caller) | `coins_wire_transfer_replay` |
| Coin market with Game gold/native client | RUNNABLE NOW | N #1833 route seeds + #1825 admission signer; Game assignment/custody and environment qualification still required | `coins_transfer_market_exactly_once` |
| Internal Premium snapshot | RUNNABLE NOW | — (service/coordinator slice) | `premium_snapshot_fail_closed` |
| Premium-gated live action | RUNNABLE NOW | N #1833 route seeds + #1825 admission signer; Game assignment/custody and environment qualification still required | `premium_snapshot_fail_closed` (gameplay extension) |
| Loyalty live ingestion | BLOCKED | Contract decision + Game server consumer | `loyalty_stream_converges` |
| Native Premium overview | RUNNABLE NOW | Game side; real harness snapshot → existing Game fence; admitted-session prerequisite | `native_premium_overview_refresh` |
| Public profile/highscores/privacy watermark | RUNNABLE NOW | — (authority/ops source) | `public_projection_privacy_floor` |
| Public live death/online-session assertions | RUNNABLE NOW | N #1833 route seeds + #1825 admission signer; Game assignment/custody and environment qualification still required | `public_projection_privacy_floor` (gameplay extension) |
| Public online/daily/totals extensions | BLOCKED | Contract decision: acceptance UNKNOWN | `public_online_daily_totals` |
| T94 event/season feed | BLOCKED | Contract decision; source qualification UNKNOWN | `league_outcome_correction_fences` |
| Private journal T72 | BLOCKED | Game server + Platform; depends on T72 rev 4 (Platform #1827): `result: accepted\|refused`, sole refusal reason `CHARACTER_SCOPE_DELETED` | `journal_t72_owner_replay_privacy` |
| Catalogue unsupported-artifact fidelity | RUNNABLE NOW | —; populated content needs Game producer | `catalog_real_export_cache_fidelity` |
| Store default-off refusal | RUNNABLE NOW | — | `store_default_off_no_effect` |
| Store positive purchase/delivery | BLOCKED | Contract decision | `store_purchase_delivery_replay` |

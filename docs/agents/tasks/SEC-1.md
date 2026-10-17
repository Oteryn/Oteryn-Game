# SEC-1

```yaml
task_id: SEC-1
title: "SEC-1: server-side client integrity and anti-bot checks (skeleton)"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/sec-1
issue: 1622
pr: 2188
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 6 SEC-1 (row 135); App. 7 SEC-1; wave 1"
class: "hard (child graph SEC-REL-1, SEC-CHAL-1, SEC-TELEM-1, SEC-DETECT-1, SEC-CHAL-2)"
migration: no
registry: yes
owner: "/root/gm_compile_fix (sole allocated leaf writer; family lead /root / chat I)"
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-17
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/SEC-1.md
  - apps/game-server/src/security/mod.rs (new, unregistered leaf root only)
  - apps/game-server/src/security/trust_policy.rs (new)
  - apps/game-server/src/security/trust_policy_tests.rs (new)
  - apps/game-server/src/security/release_policy.rs (new)
  - apps/game-server/src/security/release_policy_tests.rs (new)
slot_only_paths: []
public_contracts: []
depends_on:
  - "CLOCK-1 residue: MERGED #2114"
  - "ITEM-LOOK-1: MERGED #2182; WEATHER-1: draft/open #2202 at 4b8688fe; runtime wiring remains REPORT_ONLY"
  - "ADMIT0 ACCEPTED; ADMIT-CAP-1 implementation remains a SEC-CHAL-1/2 gate"
  - "SEC_CLIENT01 ACCEPTED (D1034); acceptance is not child implementation authority"
  - "SEC-TRUST-1 node floor/checkpoint seed: separately allocated; fail closed pending qualification"
  - "D1032 release-train reconciliation before SEC-REL-1 slot; PROD-COMPAT admitted-build source before SEC-CHAL-1"
  - "ANL-01, ANL-03: child prerequisites; DATA-PRIVACY-01 disclosure: production collection gate only"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Current leaf allocation is **IMPLEMENT**, limited to the new files listed above
and this owned record, under the [CP correction](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6225481669).
Runtime wiring remains **REPORT_ONLY** until the owning prerequisite PRs merge.
This allocation supersedes earlier record-only/source-allocation statements below
only for these pure leaves. CP retains readiness, wiring allocation and merge authority.

## Current published-head checkpoint — 2026-10-17, NOT_FULLY_QUALIFIED

Evidence head: `dc8da517bf2e8962d7dd73f05115e5f7a7d2df97`, the published
doc-only successor of `3a039e095c28856a4bdb5cbed8c4237e90e60386`.
The five SEC policy source files remain unchanged. This continuation records
newly completed CI, the concrete next-contract request and CP's D1326 no-grant
decision; it adds no runtime implementation or child allocation.

### Newly completed exact-head CI

Both Rust job logs were read directly. Each checks out and verifies the exact
`dc8da517bf2e8962d7dd73f05115e5f7a7d2df97` head, then selects only
`oteryn-game-server` (`mode=changed packages=1/31`). Both execute Rust 1.95.0
strict Clippy with `--locked --all-targets -- -D warnings` and the selected
crate's unit tests successfully. The observed counts are independent new output,
not an attribution of the earlier source candidate's PASS:

| Exact-head run / Rust job | Library unit result | Operations binary result | Aggregate |
| --- | --- | --- | --- |
| [39021803181 / 117365160027](https://github.com/Oteryn/Oteryn-Game/actions/runs/39021803181/job/117365160027) | 3150 passed, 0 failed, 29 ignored, 0 filtered | 6 passed, 0 failed, 0 ignored, 0 filtered | Merge gate and `game-gate` PASS |
| [39022072461 / 117365930745](https://github.com/Oteryn/Oteryn-Game/actions/runs/39022072461/job/117365930745) | 3150 passed, 0 failed, 29 ignored, 0 filtered | 6 passed, 0 failed, 0 ignored, 0 filtered | Merge gate and `game-gate` PASS |

The import-proficiencies, migrate and main binary test harnesses each report
zero tests in both jobs; they are not counted as exercised cases. Neither job
contains the SEC policy tests: the new security module remains unregistered.
These Cargo results do not qualify the leaves, cryptography or SEC runtime.
The separately recorded Rust 1.94.0 standalone **17/17** evidence below remains
bound to the unchanged five source blobs; no leaf rerun or new native build was
performed for this record update.

### CP request, D1326 decision and remaining owning boundaries

The lead posted and verified the [FAMILY-I request, SEC section](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6232808829)
at `2026-10-17T10:27:27Z`. CP replied at `2026-10-17T10:31:07Z` with
[D1326](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6232842478):
**no grant for SEC-TRUST-1 or SEC-REL-1**. CP will name the owners after reviewing
#2188 and #2202; runtime stays **REPORT_ONLY**. The request is answered, and its
following contract requirements remain unallocated:

- **SEC-TRUST-1 / D1031:** name the contract owner, exact accepted contract SHA
  and implementation paths for the complete trusted key state at the embedded
  floor, checkpoint seed, genesis/root-lineage transition and non-restored
  freshness input. A DB below the floor remains fail-closed; default revision 0,
  an ordinary record at the floor and clamped World time are not bootstrap.
- **SEC-REL-1 / RELEASE-MANIFEST-3:** assign one owner to close the signed schemas
  and key-id rules, full-state versus delta representation, decoder byte/count
  limits and final artifact/corpus inventory. JCS, uint64 decimal strings and
  detached Ed25519 are already settled; no replacement encoding is requested.
- **Allocation timing:** the request asked whether contract preparation could
  proceed before #2202 merges; D1326 gives no grant. Runtime/config/store/transport
  wiring and PROD-COMPAT/SEC-CHAL remain outside the request. Current durable
  trust/fetch/clock-latch provenance still requires its owning contract; the
  policy predicates supply none of it.

The fresh read still finds #2202 draft/open at
`4b8688fe03a39e1b25502c695fa34cfa1dd6ee38`, with no newer comments or reviews
on #2188 or #2202 since the SEC publication. No new source defect or child
allocation follows from the completed checks. D1326 leaves the draft
**NOT_FULLY_QUALIFIED**, with runtime wiring **REPORT_ONLY**, while CP reviews
the two PRs before naming the next owners.
Earlier source checks and prerequisite observations are retained below as history.

## Historical continuation checkpoint — 2026-10-17, NOT_FULLY_QUALIFIED

Reviewed source candidate: `3a039e095c28856a4bdb5cbed8c4237e90e60386`.
The complete five-file SEC leaf diff was independently reviewed in this
continuation, with no confirmed source defect. The source files are unchanged;
only this owned task record is being refreshed through the family lead.
This review does not qualify cryptography, current trust or runtime integration.

### Exact-source validation evidence

- The five downloaded source files match their independently retrieved Git blob
  SHAs. An isolated harness includes their existing modules and all 17 tests.
- Fresh execution on Molehill-PC / Ubuntu-24.04 used
  `rustc 1.94.0 (4a4ef493e 2026-03-02)` and
  `rustc +1.94.0 --edition 2024 --test -D warnings harness.rs`.
  Enumeration found **17 tests, 0 benchmarks**; execution with
  `--test-threads=1` passed **17/17, 0 failed, 0 ignored, 0 filtered**.
  `rustfmt +1.94.0 --edition 2024 --check harness.rs`, including the referenced
  leaf modules, passed. Root's process `8476` exited 0; the exact five GitHub
  source blob hashes were verified before execution.
  Harness, binary, `list.log` and `test.log` are retained at
  `/root/oteryn-pr-i-20261017-b2bde334/sec/` on that test host.
- Existing exact-head CI is now verified: [run 38973469765, job 117217584647](https://github.com/Oteryn/Oteryn-Game/actions/runs/38973469765/job/117217584647)
  selected and compiled `oteryn-game-server`; strict changed-crate Clippy passed.
  Library unit output is **3150 passed, 0 failed, 29 ignored**, plus **6 passed**
  in the ops binary. `game-gate` passed on that source candidate.
- Those Cargo logs contain none of the SEC policy tests: the new security module
  is intentionally unregistered. Crate CI PASS therefore does not qualify the
  isolated leaves or claim SEC runtime integration. The old crate NOT_RUN claim
  is superseded only by the bounded CI evidence above.
- `python3 tools/agents/validate_governance.py` passed (22 required policy
  documents, 9 project lanes); the governance unittest suite passed **124 tests,
  0 failures**. The owned-record `git diff --check` passed.

### Prerequisite fresh-read

| Prerequisite | Verified state and interface | Effect |
| --- | --- | --- |
| #2182 ITEM-LOOK-1 | Merged at `2026-10-17T06:29:02Z`; head `089f720792934f1f3943f658ad7aabaa92fa632c`, merge `b1e90b421692289db3aac78a915629251ba80481`. `interaction/look.rs::item_text` remains a content-description read. | This prerequisite is merged; it supplies no trust, build-admission or clock authority. |
| #2202 WEATHER-1 | Draft/open at `4b8688fe03a39e1b25502c695fa34cfa1dd6ee38`. `world_clock.rs::now_utc_ms` still reads wall time once and advances from monotonic elapsed time. | Runtime wiring remains REPORT_ONLY. SEC still needs separate raw wall observations; clamped World time cannot detect backward wall-clock steps. |

The [CP leaf allocation](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6225481669)
and [D1122 checkpoint conditions](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224930201)
remain the scope of this continuation. Closed signed schemas and bounds, trusted
node seed/root lineage, durable trust/fetch/clock-latch provenance and the
PROD-COMPAT admitted-build source remain unqualified owning-contract gaps.
The draft is not ready for runtime wiring or merge. Earlier checkpoints below
are retained as dated history, not current check or dependency status.

## Historical leaf checkpoint — 2026-10-16, NOT_FULLY_QUALIFIED

- `trust_policy` checks ordinary consecutive revision/predecessor/floor rules,
  checkpoint revision/time bounds, applied/checkpoint equality, raw-wall versus
  monotonic backward discrepancy, and strict post-step-fetch observation ordering.
  These are numeric predicates, never an authenticated checkpoint or VERIFIED store.
- `release_policy` checks signing-role separation and raw Ed25519 public-key
  membership/revocation. It selects no signed key-id schema. Inputs do not prove
  signatures or current authenticated key state. No crypto implementation is added.
- Neither leaf reads a clock, network, database or config, supplies embedded pins,
  seeds a restored store, emits challenges, admits builds or changes gameplay rights.
  No existing server module/Cargo file is edited or registered.

Accepted authority is SEC_CLIENT01 §3 (acceptance remains D1034); D1029 fixes
canonical JCS and detached Ed25519, but these leaves consume no signed schema or
serialized bytes and need no dependency or JCS rerun. Local `Duration` facts
select no wire timestamp units. The 24h and 5min constants are accepted limits,
not new resource allocations.

Targeted prerequisite interface evidence:

| Exact supplied PR head | Inspected interface | Consequence |
| --- | --- | --- |
| #2182 `1b9568e4df886d681a2319653d24a0f87a9b32d9` | `interaction/look.rs::item_text` | Read-only content description consumes qualified generation semantics; it supplies no trust/build admission authority. No speculative dependency is added. |
| #2202 `0106edef4ad12afcf70a835d1ede994a41ad4608` | `world_clock.rs::now_utc_ms`, `WorldClockSource`; `world_weather.rs::absolute_weather_slot` | World time is clamped from one wall observation and weather arithmetic supplies no current binding. SEC backward-step checks therefore require separate raw wall and monotonic observations. Runtime wiring remains deferred. |

Validation: isolated Rust 1.94.0 `rustc --edition 2024 --test -D warnings`
compiled the exact five source files outside the checkout; **17/17 tests PASS**
(11 trust-policy, 6 release-policy). Exact-file Rustfmt and `git diff --check`
PASS. This is pure-policy proof, not Game Server compilation, cryptographic
verification, signature/schema qualification, persistence/restart, freshness
provenance or runtime integration proof. No heavy build, PG, PC or JCS run.
Private harness/log and exact file hashes are retained under
`/workspace/oteryn-prep/sec-policy-b2c7/`; lead owns independent review/publication.

Still missing for consuming implementation: accepted closed trust/release/corpus
schemas and inventory/bounds, exact embedded node seed and root-lineage/bootstrap
contract, serialized durable trust owner with independently current retrieval and
clock-latch provenance, and PROD-COMPAT's exact admitted-build source. None can be
replaced by a caller boolean or this policy result. Existing detailed prerequisite
packet below retains those concrete gaps; no new approval or runtime grant is implied.

## Goal

Server-side integrity checks per SEC_CLIENT01 (CLIENT_INTEGRITY, INPUT_TELEMETRY). Pacing stays in LOOP-WORLD-1; 9c/6c3 retirement is separate.

## Historical dependency decisions — 2026-10-16

Authority: [CP D1029–D1034](https://github.com/Oteryn/Oteryn-Game/pull/2188#issuecomment-6216591759)
and the later [owner telemetry override](https://github.com/Oteryn/Oteryn-Game/pull/2188#issuecomment-6216811726).

| Dependency | Current decision / unresolved gate |
|---|---|
| CLOCK-1 residue | Historical evidence: MERGED #2114. |
| SEC_CLIENT01; ADMIT0 | **ACCEPTED** (D1034). The old CANDIDATE blockers are superseded; acceptance does not prove ADMIT-CAP-1 implementation merged. |
| ITEM-LOOK-1; WEATHER-1 | CP rechecks live completion before allocating SEC-TRUST-1 and SEC-REL-1; no merge status inferred here. |
| SEC-TRUST-1 | Owns embedded node floor and checkpoint seed (D1031); no floor bootstrap or genesis-0 workaround in SEC-REL-1. |
| SEC-REL-1 / RELEASE-MANIFEST-3 | Shared canonical format/signing owner; D1029 fixes JCS, uint64 decimal strings and detached Ed25519. Remaining schema/bounds/inventory and independent review stay open. |
| Release train | GitHub Releases + Velopack (free), alpha-ops §4 rulings 5/8/9/11/12 (D1032, owner B); reconcile CLIENT-INSTALLER-0 §2/§5 before SEC-REL-1 slot. |
| PROD-COMPAT admitted-build source | SEC never synthesizes it; SEC-CHAL-1 waits for its owning PROD-COMPAT packet (D1033), plus SEC-REL-1 and ADMIT-CAP-1. |
| ANL-01; ANL-03 | Required by their telemetry/detector children; implementation readiness remains unverified here. |
| DATA-PRIVACY-01 disclosure | Production collection gate remains. Later owner D enables telemetry during **TEST**; it supersedes only D1034's telemetry-off ruling. No telemetry is implemented or activated by this record. |

Historical 2026-10-15 dependency state at main@5742f30f3 had ITEM-LOOK-1 and
ADMIT-CAP-1 unmerged, and incorrectly treated SEC_CLIENT01/ADMIT0 acceptance as pending.
That table is superseded by the decisions above; original source observations below
remain dated evidence, not current implementation or merge claims.

## Owned paths (current pure-leaf allocation)

- The five new `apps/game-server/src/security/` files listed in YAML above.
- `docs/agents/tasks/SEC-1.md`.

## Historical child source-path plan — inactive in #2188

- apps/game-server/src/security/ (new module dir)
- apps/game-server/tests/security_*.rs (new)
- crates/protocol-oteryn/src/client_integrity*.rs (new)

## Historical slot-only path plan — no current allocation

- connection.rs
- mod.rs
- capabilities.rs
- crates/protocol-oteryn/src/lib.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- RESOURCE_LIMITS_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `connection.rs` `trait FreshAdmissionAuthority` (l.331): new authority method (pattern: `npc_trade` l.696, `fight_modes` l.804)
- `connection.rs` `serve_admitted_actor` (l.1396): command-type dispatch arm and delta/snapshot frame (pattern: `quest_log_delta` l.3574, `chat_line_delta` l.3595)
- `capabilities.rs` offered sets `ITEM_VIEW_OFFERED_CAPABILITIES` / `BUNDLE_WORLD_OFFERED_CAPABILITIES` (l.100-150) and `GATED` (l.165); `capabilities_tests.rs` keeps them equal to `PROTOCOL_OTERYN_V1_REGISTRY.json`
- `crates/protocol-oteryn/src/lib.rs`: `pub mod` + re-export of the new codec module; constants live in the module (pattern: `world_object.rs` `CAPABILITY_RUNE_USE_V1 = 22`; highest offered capability on main is 23, PRESENT-WIRE-1 #2124; numbers leased on #162 by CP)
- `gameplay_transport/input_gate.rs` (OPS-SAVE-2 input permit) as the per-command observation point

## Inseparable bundles

Each SEC-* child with Reg=yes remains an inseparable wire+registry+capability+handler+tests
bundle. D1030 requires **separate allocations, one child at a time**:
SEC-REL-1 → SEC-CHAL-1 → SEC-TELEM-1 → SEC-DETECT-1 → SEC-CHAL-2.
SEC-REL-1's future runtime/signing scope includes `tools/oteryn-release-sign/`.
Only the listed pure security leaves are now allocated to this PR. SEC-TRUST-1 is a
separate prerequisite owner, not an implicit expansion of SEC-REL-1.

## Acceptance criteria

- [ ] Only children whose brief is ACCEPTED and deps merged are implemented.
- [ ] Telemetry enabled during TEST per later owner D; production collection remains off until DATA-PRIVACY-01 disclosure. No runtime activation in this REPORT_ONLY PR.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Historical child validation plan (not required/executed for this record)

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

SEC_CLIENT01 decision; plan App. 7 SEC-1. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 6 SEC-1 (row 135); App. 7 SEC-1.

## Current builder instructions

- Implement only the listed new pure-leaf files and this record on `skeleton/sec-1`; runtime wiring remains REPORT_ONLY.
- Children need separate CP allocation, accepted owning contracts and their prerequisite gates.
- One author per branch; author only in AUTHORING, freeze the candidate before review,
  and explicitly return to AUTHORING through the lead before repairing a frozen candidate.
- The record-only writer returns the candidate to the family lead for publication and
  review coordination; CP retains readiness, child allocation and merge authority.

## Historical record-only context checkpoint (superseded by current leaf allocation)

```yaml
last_progress: "D1029-D1034 recorded; later owner TEST telemetry override applied (2026-10-16)"
status: prerequisite_design_prepared
mode: REPORT_ONLY
branch: skeleton/sec-1
pr: 2188
authoring_source: 87d1ee94a335831c9af3b799e89664b47f24ea27
writer: /root/sec_record
owned_paths: [docs/agents/tasks/SEC-1.md]
blockers:
  - "Separate child allocations after ITEM-LOOK-1/WEATHER-1 completion recheck"
  - "SEC-TRUST-1 embedded node floor/checkpoint seed, root lineage and non-restored freshness qualification"
  - "SEC-REL shared closed schema, upstream JCS qualification/golden vectors, bounds and final artifact/corpus inventory"
  - "CLIENT-INSTALLER-0 sections 2/5 reconciliation with alpha-ops release train before SEC-REL-1 slot"
  - "PROD-COMPAT-owned admitted-build source; ADMIT-CAP-1 merge evidence before SEC-CHAL-1"
  - "ANL-01/ANL-03 child prerequisites; DATA-PRIVACY-01 before production telemetry"
next_action: "Lead validates/freezes this record candidate; CP allocates prerequisite/child packets separately"
```

## SEC-REL prerequisite/design packet — 2026-10-15 (historical evidence, amended below)

This is **REPORT_ONLY**, on `skeleton/sec-1`, exact authoring source
`65e256f7b49ff1249a38fcc2409f1d8e883d36a1`, existing draft #2188.
The current allocation owns **only this card**, overriding the historical broad source
allocation above. No security implementation, trust pin, schema, registry entry, capability,
challenge, telemetry, publication or production authority is created here.

### 1. Owning requirements and residue

**Accepted:** `SEC_CLIENT01` is ACCEPTED on supplied acceptance main
`ac9f172836f00226b68088aa04db95ccf1ab994e` through ACCEPT-BATCH-2026-10-15.
Its §3 is byte-identical to this branch's historical CANDIDATE copy. That old status and
this card's original blocking table are historical residue, not a remaining rejection
of SEC_CLIENT01. Acceptance does not allocate implementation of every child.

`OTERYN_GAME_ARCH_ALPHA_OPERABILITY_2026-10-06.md` §4 is unchanged on that main;
its header says rulings/packets are accepted on merge while named amendments remain
pending their owning packets. Its ruling 9 already chooses **detached Ed25519 over
canonical bytes**, the offline signing role and a separate root release attestation.
RELEASE-MANIFEST-3 is explicitly SEC-REL-1's manifest half, **one owner**, not a second
competing trust format. Those earlier words alone did not specify or implement canonical
serialization. D1029 now selects the canonical encoding. Remaining proposed schema and binding
choices still require that owner and independent security review; they are not silently
accepted by selecting JCS.

| Accepted source | Binding consequence |
| --- | --- |
| SEC §3 | Offline release key signs native build manifest: build id/platform, every executable/library hash, corpus reference. Offline trust root is a separate key, signs trust records/checkpoints, never manifests. Root public key is pinned in the Game node build. |
| SEC §3 ordering/floor | uint64 revision advances exactly one; previous_revision equals current. Ordinary application rejects revision <= max(database highest-applied revision, embedded node floor). Trust log is append-only outside the restored Game DB. |
| SEC §3 freshness | Checkpoint root signature; age <=24h; future skew <=5min; revision >= highest checkpoint seen since start and embedded floor; issued_at >= embedded checkpoint time. Fetch at start/every1h; publish at every record/at least every6h. |
| SEC §3 failure | Store starts UNVERIFIED; only equal applied/checkpoint revision with still-valid freshness can be VERIFIED. Missing/gapped/regressed/unsigned/stale sources and a backward wall-clock step >5min against monotonic time invalidate it. After that step, recovery requires an accepted checkpoint fetched after the step. No challenges or outcomes while UNVERIFIED. |
| SEC §3 emergency | Revoke release key out of train; manifests signed only by revoked keys stop being accepted. Root compromise requires a Game-node release with a new pinned root and raised embedded floor. Compatibility admission stays PROD-COMPAT's decision. |
| SEC §4.0/§4.1 | Evidence proves release-byte possession only, never running-code integrity or gameplay/sanction authority. At most8 admitted corpora,64MiB each; at most8 retiring corpora, oldest eviction; restart drops retiring corpora. Missing/revoked/evicted corpus means no challenge/outcome and a non-personal OPS-1 pipeline event. |
| Alpha ops §4 ruling9 | Updater additionally needs root attestation {channel,version,package_sha256,package_size,manifest_sha256,trust_revision}. Root binds the exact package, so a revoked release key cannot invent an installable package behind a replayed checkpoint. Offline keys never enter CI. |

SEC §3 explicitly accepts a bounded restore/restart replay window: an attacker controlling
the log view may replay an otherwise acceptable old checkpoint, and clock rollback across
restart is bounded only by the embedded checkpoint/build age plus24h. Repeated restore/
restart can extend exposure until SEC-TRUST-1. This packet does not claim to eliminate
that accepted residual or add an unaccepted witness service.

### 2. Exact native interfaces: available facts, not replacement trust

| Source at authoring SHA | Available interface / boundary |
| --- | --- |
| apps/client/src/lib.rs:28–119 | RELEASE_ID, CLIENT_BUILD, is_valid_release_id/is_contract_release_id: current native identity is `<version>+<channel>.<route>.g<sha12>`, reported with `oteryn-client/`; channels dev/preproduction/stable, routes ci/rel/local, release id <=50 bytes. No ordering or signature/admission proof follows. |
| crates/protocol-oteryn/src/lib.rs:348,522,595 | ClientBootstrapView/ClientResumeView carry declared client_build_id; encode_client_bootstrap bounds it to128 bytes/nonempty. Foundation protocol.rs re-exports this wire seam. A string is not an admitted-build observation. Accepted-main whole-file difference is LOGOUT registration only; this bootstrap code is unchanged. |
| apps/client/src/bin/oteryn-launcher.rs | parse_pointer/active_client validate the installed directory name and launch exactly current.txt's executable. They do not verify a signed SEC manifest, checkpoint or key revocation. |
| apps/client/installer/ci-installer.ps1; installer/sign.ps1 | Real Inno unsigned install qualification, package/hash inventory and SHA256SUMS; signing hook explicitly signs nothing/exits0 today. These are useful build outputs, not authenticated release publication. |
| .github/workflows/merge-group-gate.yml:705–785 | Exact composed SHA derives dev.ci release identity; Windows job builds/tests and uploads an explicitly unsigned installer artifact. It does not publish a SEC trust log or a production signing train. |
| apps/game-server/src/foundation/fnd04_verifier.rs:95,469,2154,2489 | Actual admission-grant Ed25519 verification and Fresh/Recovery trust contexts have Platform grant purposes, current-source provenance and their own revocation/fences. Private canonical-base64url/bounded-JSON helpers are not a release-manifest serializer. Do not reuse these trust contexts, issuer keys or signed grants as SEC trust. qualification.rs::sign_grant is a fixture, not a release signer. |
| crates/ntp-clock/src/lib.rs:16–23 | read_synchronized returns raw synchronized kernel UTC microseconds plus error bound, or None; Linux only. It supplies a time fact, not SEC checkpoint freshness/rollback state. |
| apps/game-server/src/world_clock.rs:32; premium/clock.rs:25 | World clock clamps a once-anchored wall clock to monotonic elapsed time, hiding later wall-clock steps. Premium adds its own uncertainty/high-water rules. Neither implements SEC's5min backward-step latch and post-step-fetch recovery. Do not transplant their gameplay clock semantics. |
| crates/error-codes/src/build_source.rs; build.rs | Exact OTERYN_BUILD_SHA can produce diagnostic sha12; local/unknown fallback is explicitly possible. This proves no release signature, artifact bytes, official build membership or current compatibility policy. |
| node/config.rs; Game boot/admission lineage | Existing config/TLS roots, registered native-source descriptors, World/Channel ownership and Character session-generation fences remain separate. They do not pin the SEC offline root, sign the client release train or attest release-log freshness. No Game boot roster/checkpoint is substituted. |

Git-tree inspection, not sparse-file inference, finds no `crates/release-manifest`,
`tools/oteryn-release-sign`, client update.rs/trust_floor.rs or client-release.yml at this
source. No native SEC trust-record/checkpoint store or authoritative bounded admitted-build
reader was found in the inspected runtime. Node config has no client_build_floor; current
bootstrap codecs alone do not implement VERSION-FLOOR-1. Primitive ed25519-dalek2.2.0 and
SHA-256 availability is not implementation of SEC-REL.

**Concrete release-train reconciliation gate (resolved direction, pending contract reconciliation):** CLIENT-INSTALLER-0 remains labelled
CANDIDATE while its implemented identity/Inno/MQ slices and owner D854 are documented.
Its §5 proposes Platform hosting/pointer sequence and authorized lower-version rollback;
alpha-ops §4 rulings5/8 propose alpha/GitHub Releases, Velopack and higher-version rollback,
and ruling11 says Platform does not gate builds. Installer §2/§5 instead names Platform/
Gateway compatibility authority. D1032 and owner B now select alpha-ops §4 rulings
5/8/9/11/12, GitHub Releases and
Velopack (free). Keep the actual native release id as an opaque exact identity;
CLIENT-INSTALLER-0 §2/§5 must be reconciled before the SEC-REL-1 slot.
Do not silently replace the native installer, impose SemVer ordering on release ids,
activate an unsigned ci artifact, or assume a future Platform API/schema.

### 3. Canonical bytes: D1029 selected; remaining schema proposals

**D1029:** RFC 8785 JCS is the canonical encoding; uint64 values are decimal strings.
Use one mature upstream JCS implementation and cross-language golden vectors; GM-1
uses the same canonicalizer (D1040). Detached Ed25519 signs the exact file bytes, with
no BOM or trailing newline. CBOR and protobuf are excluded (ANL-01 §§2, 9).
`serde_json::to_vec` or BTreeMap ordering alone is not JCS. Upstream selection and
qualification remain implementation work for the separately allocated owner. The original
2026-10-15 proposal compared JCS with RFC 8949 deterministic CBOR; that alternative
is superseded by D1029, not an available implementation choice.

**Historical 2026-10-15 alternatives proposal — superseded by D1029.** The original
comparison is preserved verbatim below as evidence only; its RFC 8949 alternative
and request for an encoding choice are no longer operative.

| Alternative | Tradeoff / recommendation |
| --- | --- |
| RFC8785 JCS, closed typed JSON, uint64 values encoded as canonical decimal strings | Recommended proposal: inspectable operator artifacts and existing JSON ecosystem; string representation preserves all uint64 values without JSON/JavaScript number truncation. Needs explicit schema acceptance and a qualified upstream JCS implementation; serde_json::to_vec or BTreeMap order alone is not JCS. |
| RFC8949 core deterministic CBOR, typed uint64 and byte strings | Valid alternative with natural full-width integers and compact signatures; adds a binary contract/tooling and must freeze deterministic encoding/tag policy. Do not mix both encodings under one format version. |

The following envelope, field names and normalization details remain **proposals**;
D1029 settles the canonicalizer/integer/signature choices, not the complete schema.

The proposed signed file is exactly UTF-8 JCS of a closed
object `{kind,format_version,signer_key_id,payload}`; **no BOM, trailing newline or bytes**.
`kind` distinguishes manifest/trust-record/checkpoint/release-attestation and is inside
the signature. These names/version are proposed artifact-domain bytes, not allocated wire
or registry IDs. Fixed format1 uses detached Ed25519 over the entire exact file, raw64-byte
signature in a separate artifact; no negotiated algorithm/fallback. Root/release key
selection never comes from an untrusted supplied public key. A wrong role/purpose fails
before the result is usable. Do not serialize-and-resign downloaded noncanonical input.
Parse with duplicate/unknown-field rejection and require re-encoding to equal input bytes.

Proposed values/normalization: uint64 is `"0"` or `[1-9][0-9]*`, range-checked with checked
successor; no signs/exponents/floats/leading zeros. Hashes/public keys use fixed lowercase
hex (SHA-256:64 chars; Ed25519 public key:64 chars). Key id is proposed SHA-256(raw32-byte
public key), checked against that key; aliases/duplicate keys reject. Identity/platform/
channel/path syntax must be frozen by the owning release contract; no silent Unicode,
case, slash or filesystem normalization. File paths are relative inventory identifiers,
not fetch destinations; reject traversal, absolute/drive/UNC paths and case-insensitive
Windows collisions. Keys and files sort by their specified byte identities; arrays used
as sets are unique/canonical. Unknown algorithms/versions and oversized inputs fail closed.
Manifest/key/log entry-count and byte limits are **not fixed by SEC**; the owner must size
and register them before a decoder allocates. No guessed limits are promoted here.

| Proposed canonical payload | Existing required fields and owner decisions still needed |
| --- | --- |
| Manifest | Required build_id/platform, executable/library `{path,sha256}` inventory, corpus reference; alpha-ops adds channel/version, full-package SHA-256/size, asset-manifest SHA-256. Propose inventory sizes, full40-char Game commit and corpus digest/size as additional bindings. Build id must equal the final executable's actual CLIENT_BUILD; sha12 is not artifact authentication. Inventory the launcher and all distributed executables/libraries, not just packages.json's possibly empty presentation-assets list. Exact inventory scope for generated installer/uninstaller bytes needs release-owner qualification. |
| Trust record | Required revision/previous_revision/active_keys/revoked_keys. Propose active keys as full `{key_id,public_key}` set and revoked keys as cumulative id tombstones: unique, disjoint, no previously revoked key reactivated, root key never a release key. Full-set versus delta and tombstone representation must be accepted, not inferred from braces in §3. |
| Checkpoint | Required revision/issued_at. Propose issued_at as canonical nonnegative Unix-seconds decimal string, checked conversion to clock microseconds. No invented head digest or alternate sequence is added to the accepted record fields. Root log publisher must reject different bytes at an existing revision; revision alone is not a proof against root equivocation. |
| Release attestation | Required alpha-ops fields exactly: channel/version/package_sha256/package_size/manifest_sha256/trust_revision. Propose manifest_sha256 = SHA-256(exact canonical manifest file, excluding detached signature). Validate exact package/manifest binding, and trust_revision <= applied trust head; later trust-only records do not make an attested release uninstallable. This root object is additional to release-key verification, never a root-signed manifest. |

No sample private/public root, signature, genesis revision or hardcoded build roster is
created. D1029's selected byte/signature rules require golden cross-language vectors and the
owning security review. The additional proposed schema choices still need explicit
acceptance; no signed envelope is invented as an accepted contract here.

### 4. Publication, Game trust and corpus integration sequence

1. **One owner:** CP allocates SEC-REL/RELEASE-MANIFEST-3 together for shared bytes/verification
   and local signing CLI; client update/release packets consume that format. Owner holds
   distinct offline release and root keys. OS provider keys, Platform admission keys and TLS
   roots remain separate. The same human can hold both offline roles; the key material cannot
   acquire another role through a kid or configuration flag. Never place offline keys in CI. The root-key holder must also supply the accepted
   <=6h unchanged-head checkpoints; the publication operator serves those already signed
   bytes. No online-root service or CI signing workaround is implied by that cadence.
2. **Finalize actual artifacts:** exact protected checkout builds the client/launcher/libraries
   and actual per-build corpus. Any authorized OS signing happens before final package hashes.
   Hash the complete final package and exhaustive file inventory; bind corpus/index to those
   exact build bytes. A content/presentation packages.json or copied Reference binary is not
   a SEC corpus producer. Corpus regions/index extraction and checked offsets/lengths need
   their own admitted format/coverage; no guessed challenge selector or region table here.
3. **Local signing / immutable publication:** release signer signs manifest; root signs changed
   trust record, checkpoint and root release attestation with the bindings above. Publish
   trust log+checkpoint, then package/corpus objects, then manifest/signature+attestation, then
   feed pointer, then authorized compatibility floor/set raise (alpha-ops ruling12). A reader
   seeing a partial release refuses that release evidence and retries, never trusts a dangling
   reference. Append-only log/checkpoint storage must survive Game DB restore; ordinary GitHub
   artifact upload or an unsigned SHA256SUMS is not that authenticated head.
4. **Node bootstrap:** pinned root, embedded floor and embedded checkpoint come from the Game
   node release, independently of runtime config/DB/download. Start UNVERIFIED. Re-read the
   authenticated log/checkpoint, validate their role/bytes/age/floor, apply consecutive records
   under the Game trust owner's serialized transaction and persist highest-applied revision
   with the corresponding key state. No CharacterRevision or session authority is advanced.
5. **Floor bootstrap: SEC-TRUST-1 (D1031):** SEC §3 does not specify the Game node's complete trusted key state
   at its embedded floor; client alpha-ops ruling9 does specify a client snapshot, which is
   not automatically a node contract. A restored DB below the embedded floor cannot be repaired
   by treating an ordinary record at that floor as a fresh advance. D1031 assigns the
   embedded node floor and checkpoint seed to SEC-TRUST-1.
   Until that packet qualifies bootstrap, the node starts UNVERIFIED and a DB below
   the floor fails closed. Neither a node seed nor a state restore is implemented here. Genesis,
   root-lineage transition and uint64 exhaustion need that owner; never default to revision0
   or bypass the <=floor rejection. External non-restored freshness high-water belongs to
   SEC-TRUST-1, not a new pretend DB anchor or self-issued current-verified boolean.
6. **Current usability:** persist data/floor, not a reusable VERIFIED grant. At every challenge
   issue/outcome boundary, the future owner checks current applied==accepted-checkpoint
   revision, unexpired24h freshness, root/key revocations, corpus availability and clock latch.
   Poll at start/every1h but expire evidence at its exact boundary between polls too. Checked
   arithmetic must handle timestamp overflow/future facts. Raw realtime plus monotonic
   elapsed observations detect a backward discrepancy >5min; neither clamped World time nor
   Premium's unrelated5s policy substitutes. After the step, only a qualifying fetch that
   actually occurs after its local latch epoch may clear it; cached/pre-step prepared data
   cannot. Re-fetching otherwise admissible signed data is not a claim of an extra witness
   or stronger freshness than SEC's accepted replay bound.
7. **Compatibility/corpus source GAP:** obtain the current PROD-COMPAT-authorized exact build
   set/transport profile and its generation/owner provenance. No such native input exists
   here. A client declaration, signature-valid manifest, Platform admission grant, all SemVers
   >=a floor, or installed directory list cannot synthesize the bounded admitted set. The
   contract must distinguish new-session admission from retiring already-admitted sessions;
   authentication alone never selects gameplay rights or makes a build official. Recheck at
   actual fresh/reconnect/recovery boundaries under their native fences; no new callbacks,
   wire IDs or current-authority wrappers are introduced by this report.
8. **Resources/revocation:** verify local corpus bytes/index against accepted manifest digest
   before use, <=64MiB per build; <=8 admitted builds and <=8 retiring corpora. Keep retiring
   bytes through last session+terminal challenge unless oldest-evicted; restart drops them.
   Revoked-key-only manifests, absent bytes and eviction suppress challenges/outcomes and
   produce SEC OPS-1 events (node/build/revision only, no actor). SEC does not reject gameplay
   admission on this trust failure; compatibility owner decides admission independently.

### 5. Proposed qualification matrix — NOT EXECUTED

Each negative varies the named invariant against an otherwise valid independently produced
control. Production consumers must distinguish matching immutable prepared data from
freshly resolved current trust/build/session sources; no record-derived happy-path helper
may stand in for the negative current-authority cases.

| Boundary / operator | Required future result and evidence owner |
| --- | --- |
| Canonical bytes: BOM/newline/reordered keys/duplicate/unknown field, noncanonical integer/hex, wrong version/kind | Reject; golden UTF-8 byte/signature vectors agree in signing tool, Game and updater. RELEASE-MANIFEST-3. |
| Cryptography: each field bit-flipped, wrong root, unknown/revoked release key, release key signs trust/checkpoint, root signs manifest, reused key across roles | Reject, no role confusion or trust change. Real fixed key fixtures only; no current_verified bool. |
| Binding: same version different full release id/channel/route/commit/platform, substituted manifest/package/asset/corpus, omitted executable/library, path/case collision | Reject the affected release; exact real artifact inventory and package byte fixture. No inferred admitted roster. |
| Root attestation: revoked-key forged higher-version package, old attestation replay with substituted package, wrong channel/version/size/manifest or trust_revision >head | No activation; attested real release still installs after later unrelated trust record. Release/updater owners. |
| Revision/floor: duplicate r, r<=max(DB,embedded), wrong previous, gap/out-of-order, maxuint64 successor, concurrent different r+1 | Refuse ordinary advance; serialize one winner and preserve exact key state/floor. Real store transaction/reload, not only pure comparisons. |
| Revocation: tombstone removed, active/revoked overlap, reactivation, revocation between prepared check and use | Reject/stop evidence under the current head; no prepared artifact reacquires trust. Follow accepted final tombstone contract. |
| Checkpoint: unsigned/bad signature, lower revision, below embedded, issued_at before embedded; valid old prefix without checkpoint | UNVERIFIED; no challenge/outcome. Exact signed controls and full log retrieval boundary. |
| Time: exactly24h vs24h+1 unit, exactly5min future/backward vs+1 unit, overflow, unavailable clock | Accepted numeric boundaries only; fail closed when unusable. A wall/monotonic disagreement must be observable, not clamped away. |
| Post-step provenance: cached prior checkpoint vsactual fetch after latch; expiry between hourly polls | Only the accepted post-step fetch can restore current usability; expiration stops issue/outcomes immediately. |
| Restart/restore: process restart, DB restored before revocation, DB head behind checkpoint, DB below embedded floor, missing log records | Start/remain UNVERIFIED; replay only through qualified seed+ordered records; explicitly demonstrate accepted residual replay and SEC-TRUST-1's still-missing stronger anchor. |
| Root replacement: old root/checkpoint/DB key state against new pinned build/floor | Reject old lineage; no config-selected replacement root; qualify restart/bootstrap separately. |
| Build ownership/currentness: stale/unknown compatibility generation, fabricated declared id, prepared set after operator raise, fresh/reconnect/recovery | Existing compatibility refusal at owning points; no trust evidence grants admission or bypasses session-generation fences. VERSION-FLOOR/admitted-set owner. |
| Corpus:8/9 admitted,64MiB/max+1,8/9 retiring, wrong bytes/index, unknown build, last-session/challenge lifecycle, oldest eviction/restart | Enforce accepted resource/lifetime policy; missing evidence has only OPS-1 pipeline effect, never session security observation. SEC-REL/SEC-CHAL owners. |
| Publication/crash: record published butcheckpoint absent, missing package/manifest/sig, DB crash before/after floor+key-state commit, client floor fsync/rename interruption | No partial activation/trust grant or floor rollback; durable reload and fault-injection tests. Distinguish Game DB floor from client trust-floor file. |
| Trust/checkpoint replay over repeated restore/restart | Demonstrate baseline bounded-residual limits honestly; a pure signature or clamped clock test cannot claim non-restored freshness. SEC-TRUST-1 remains explicit. |

### 6. CP handoff and validation

D1029–D1034 and the later owner comment settle the canonical encoding, ownership
boundaries, release-train direction and test telemetry policy; they leave these gates:

- CP rechecks ITEM-LOOK-1/WEATHER-1 and separately allocates SEC-TRUST-1 and
  SEC-REL-1 with their child owners; this record remains REPORT_ONLY.
- SEC-TRUST-1 qualifies embedded node floor/checkpoint seed and the root-lineage/
  non-restored freshness contract. Start UNVERIFIED; DB below floor fails closed,
  with no genesis-0 or ordinary-record-at-floor bypass.
- SEC-REL/RELEASE-MANIFEST-3 has one shared format/verifier/signing owner, with future
  `tools/oteryn-release-sign/` and `apps/game-server/src/security/` scope. It must settle
  closed schema/key-id/bounds, upstream JCS qualification/golden vectors and exact final
  artifact/corpus inventory under independent security review.
- CLIENT-INSTALLER-0 §2/§5 must be reconciled with alpha-ops §4 rulings 5/8/9/11/12
  before that slot. Owner B confirms free GitHub Releases + Velopack; owner C holds
  distinct offline release/root keys. No key provisioning, CI signing or publication
  is authorized here; unchanged-head checkpoint cadence still needs the offline owner.
- PROD-COMPAT supplies the authoritative exact admitted-build set; SEC-CHAL-1 waits
  for its owner, SEC-REL-1 and ADMIT-CAP-1 implementation. SEC cannot fabricate a
  source from declarations, manifests, directories, Platform grants or SemVer floors.
- Children are separately allocated in D1030 order. ANL-01/ANL-03 remain prerequisite
  gates. Owner D's later ruling enables telemetry during TEST and supersedes the
  earlier D1034 telemetry-off direction; DATA-PRIVACY-01 disclosure remains mandatory
  before production collection. This record changes no capability or runtime setting.

Release workflow, pins, migrations, registries, current-source wiring and production
activation retain their owning packet/review/CP slot gates. Session-generation fences,
WorldId/ChannelId separation and possession-only evidence limitations remain intact.

Only governance and owned-document checks are appropriate to this packet. No Rust/Cargo,
crypto verification, trust-store, release publication, checkpoint freshness, restore,
challenge or telemetry test has been executed. The matrix above is future acceptance
work, not evidence of runtime completeness. Actual read commands/source hashes and final
governance output from the 2026-10-15 packet were retained under
`/workspace/oteryn-prep/sec-1-evidence/` (historical local evidence, not a current gate).
The 2026-10-16 record-only refresh runs governance validation, its unittest suite and
`git diff --check`; its exact results are returned to the lead for the candidate freeze.

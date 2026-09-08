# WP4 GameSession nonreuse persistence/reload allocation amendment

Coordinator: #162. Existing worker: #329 / PR #335. Programme: #364.
Preparation authority: #329 comment `5574793115`, implementing protected Decision A `FND-DUR-GAMESESSION-NONREUSE-V1`.

## Status and activation boundary

This is a prospective amendment of the existing admitted Durability Child B task,
not a new worker, new migration line, architecture decision, database deployment
or consumer activation. It is **NOT_ACTIVE** until protected WP2 semantic/API
implementation and the legitimate WP3 prerequisite are complete, followed by
explicit Work application after fresh custody and migration-release checks.

```yaml
amendment_id: OTV2-WP4-GAMESESSION-NONREUSE-PERSISTENCE-20260907
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
worker_issue: 329
worker_pr: 335
worker_task_id: OTV2-20260906-durable-fresh-admission-child-b-329
worker_branch: refs/heads/agent/durable-fresh-admission-child-b-329
original_admission_main_sha: b8ae4c965cc7f686b89b4d5c0ba2bc04af6e07fd
observed_worker_head_sha: 834db1d7118d751e31287715d3eaac7780a0c7b9
allocation_base_main_sha: 0f72ffeb6218981bbef29ce783a8c3efa7e10964
amendment_state: NOT_ACTIVE
worker_launch: EXISTING_WORKER_ONLY
consumer_activation: FORBIDDEN
new_production_paths: []
existing_wp4_paths:
  - apps/game-server/src/bin/oteryn-game-migrate.rs
  - apps/game-server/src/durability/fresh_admission.rs
  - apps/game-server/src/durability/admission_authority_guards.rs
  - apps/game-server/src/durability/admission_journal.rs
  - apps/game-server/src/durability/db.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/schema.rs
  - apps/game-server/migrations/0002_fresh_admission_authority.sql
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/tests/support/postgres.rs
  - apps/game-server/tests/support/authority_matrix.rs
  - apps/game-server/tests/support/authority_recovery.rs
  - docs/agents/tasks/active/OTV2-20260906-durable-fresh-admission-child-b-329.md
  - docs/superpowers/plans/2026-09-06-durable-fresh-admission-child-b.md
released_migration_0001: IMMUTABLE
forward_migration_0002_status_at_preparation: UNRELEASED_ABSENT_FROM_PROTECTED_MAIN
wp2_prerequisite: PROTECTED_IMPLEMENTATION_REQUIRED
wp3_prerequisite: PROTECTED_DRIVER_PREREQUISITE_REQUIRED
joint_qualification: FOUNDATION_PLUS_POSTGRESQL_17_6_REQUIRED
```

The amendment adds no path because protected architecture section 7 already
assigned the logical ledger to these existing WP4 surfaces. The coordinator
verified that `0002_fresh_admission_authority.sql` is absent from protected main
at preparation time. This fact MUST be re-read immediately before application.
If `0002` becomes protected/released first, this document cannot authorize an
edit to released history; Work must allocate a new forward migration.

## Protected authority consumed

Implement only the already accepted contracts:

- `FND-DUR-GAMESESSION-NONREUSE-V1` in
  `docs/architecture/reviews/OTERYN_GAME_SESSION_NONREUSE_AUTHORITY_DECISION_2026-09-07.md`;
- its FND-04C permanent lifetime-capacity error amendment;
- registry resource `FND04-GAMESESSION-USED-IDS-PER-CHARACTER = 65536`;
- protected WP2 allocation `OTV2_WP2_GAMESESSION_NONREUSE_AMENDMENT_20260907.md`.

No copied retired-ID history is introduced into Foundation. Durability remains
the single Game owner of the exact permanent membership authority; Foundation
later consumes sealed candidate-specific observations only.

## Required logical persistence contract

Within the existing serialized durability transaction model, WP4 must provide one
canonical durable logical `GameSessionUseLedgerV1` whose semantics are independent
of physical table/helper naming:

1. Exact globally unique committed `GameSessionId` membership permanently bound
   to the owning `CharacterId`.
2. Per-character checked membership revision/floor; no wrap, rollback or silent
   reset across restart/reload/migration.
3. Explicit completeness state. `COMPLETE` means all successful session-creation
   transitions represented by the accepted authority are covered. Partial history
   cannot be promoted to complete because initial/current rows happen to exist.
4. Initial fresh admission commits initial membership, immutable fresh receipt,
   canonical current session, owner-authored claim effects and cross-origin
   transport reservation atomically under the existing fresh transaction.
5. Every later accepted operation creating a new GameSession commits candidate
   membership and the corresponding current-session/claim/receipt transition in
   the same serialized durability decision. The predecessor membership remains.
6. Same-session reconnect consumes no additional membership.
7. Exact immutable operation replay is classified before interpreting the used
   candidate as a new request; reconciliation returns the prior committed result
   without insert/revision/capacity increment/re-aging. A different binding using
   the used candidate is rejected.
8. At 65,535 committed memberships, one unused candidate may become entry 65,536.
   At 65,536, a distinct new candidate fails before any new membership/session/
   claim/receipt/transport effect. Exact replay still reconciles.
9. Membership is never expired, evicted or removed to create capacity. Any later
   compaction must preserve exact membership, owner, completeness and revision
   semantics and requires separate review if it changes representation authority.

## Observation/reload boundary for WP2

WP4 must expose only the owner-sealed data required to construct
`GameSessionUseObservationV1` after asynchronous loading. The durable fact must be
sufficient to bind at least:

- CharacterId;
- candidate GameSessionId;
- expected current/predecessor or explicit no-current origin;
- registered source identity/version;
- checked membership revision;
- COMPLETE/INCOMPLETE status;
- exact candidate membership result;
- applicable current authority/fence binding.

The observation is eligibility evidence, not stored authorization escrow. Loading
must not create a public caller-forgeable constructor, copy the full membership
ledger into Foundation, or use receipt/current-ID convenience as completeness.

WP4 is responsible for persistence/reload and an owner-sealed observation source;
WP2 remains responsible for its Foundation semantic/API consumption. Any concrete
need to modify a Foundation/export/registry/Cargo path is a new allocation before
mutation.

## Restore and migration semantics

### New database under the contract

A versioned empty complete ledger is valid. Every successful new-session commit
must extend it atomically thereafter.

### Pre-contract or partially populated database

Known IDs may be reconstructed only from authoritative durable rows whose
identity and relationship are provable. Existing reconnect session/replacement
rows and accepted fresh receipts are evidence inputs, not automatic completeness.

If all historical creation transitions cannot be proven, preserve every known
membership but mark the affected ledger `INCOMPLETE` (or equivalent fail-closed
state). Never invent an intermediate S1 and never call initial/current-only S0/S2
history complete. While incomplete, all operations that would create a new
GameSession for that character remain fail-closed until authoritative
reconciliation establishes complete state.

Migration/reload must preserve full-u64 IDs/revisions and strict mirrors. A
corrupted, conflicting, rolled-back or overflowed revision/floor fails closed.
No migration seeds current authorization facts it cannot prove.

## Required PostgreSQL 17.6 qualification

After activation and implementation, the existing configured real PostgreSQL
harness must prove the ledger together with the Foundation semantics, not only in
isolation. Required families include:

- S0 -> S1 -> S2 -> reject retired S1 for Terminal replacement,
  CompleteReconnect EarlyTerminalReplacement and PostGrace new-session paths;
- initial fresh session membership and fresh family exhaustion code;
- 65,535 -> final valid slot and 65,536 -> exact terminal family code for every
  governed new-session family;
- exact replay at the ceiling, no extra membership/revision/effect;
- conflicting reused ID under a different operation rejected even below ceiling;
- transaction rollback after failure at every effect position leaves no orphan
  membership or authority mutation;
- concurrent candidates / same candidate / competing transition ordering under
  existing deterministic relation/row fences;
- restart/reload preserving exact membership, revision, completeness, family
  error progression and current authority;
- pre-contract partial reconstruction remaining INCOMPLETE;
- source/revision rollback, membership mirror corruption and full-u64 boundaries;
- no duplicate/nonterminal Account/Character or cross-origin transport regression;
- existing reconnect V1/V2, terminal replacement/release and claim-publication
  qualification remains green.

A source-inclusion compile, unconfigured SQL return or fixture that constructs
both expected and actual authority from the same row is not joint qualification.
Use independent current facts and one authority invariant per negative case.

## Sequencing and custody

The accepted order is strict:

`WP1 protected required-gate credibility -> WP2 semantic/API implementation with consumers inactive -> WP3 protected blocking-runtime/TLS/driver prerequisite -> explicit WP4 application -> WP4 persistence/reload implementation -> joint Foundation + PostgreSQL17.6 qualification -> consumer activation -> Work re-evaluates #247`.

This document may be protected before those product prerequisites only because it
is prospective `NOT_ACTIVE` allocation evidence. It cannot be used to merge B
first, alter #335 source, consume the shared PostgreSQL target while #351 owns it,
or activate a Foundation consumer early.

Before application, Work MUST freshly verify:

- this amendment is protected/read back;
- WP2 implementation (not merely its allocation) is protected or otherwise in
  the accepted integration state required by #162;
- WP3 prerequisite and its shared `durability_postgres.rs` lease are terminally
  released back to Work;
- canonical #335 branch/head/task still owns the listed paths and has one writer;
- `0002` remains unreleased/writable; otherwise allocate a new forward migration;
- no current path overlap or source authority conflict exists.

No reset/rebase/force-push/new worker is authorized. Continue the SAME #329/#335
branch/history and preserve prior evidence; old green runs do not qualify new
semantics.

## Review and terminal evidence

This allocation amendment itself is material authority for session persistence and
therefore requires one independent exact-head deep review, canonical repository
checks, normal protected Merge Queue and main readback. Runtime E2E is
`NOT_APPLICABLE` to this documentation-only allocation.

The eventual material #335 candidate requires focused RED/GREEN, whole-diff
adversarial self-review, independent exact-head high-risk review, exact hosted
PostgreSQL17.6 evidence, applicable selected/full CI and normal Merge Queue before
protected readback. Work archives/releases B only after the complete amended
persistence/reload delivery; docs allocation merge alone is not completion.

No production database/config/secrets/live-data deployment or external repository
write is granted by this document.

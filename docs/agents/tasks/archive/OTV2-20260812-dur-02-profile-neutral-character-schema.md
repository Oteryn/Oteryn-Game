# OTV2-20260812-dur-02-profile-neutral-character-schema — archived

```yaml
task_id: OTV2-20260812-dur-02-profile-neutral-character-schema
title: Prepare DUR-02 profile-neutral core Character schema architecture
mode: COORDINATE
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
delivery_branch: docs/OTV2-20260812-dur-02-profile-neutral-character-schema
delivery_pr: 195
base_sha: 2913201186d0e38cfc0bf0c9e2c5b83f981a61c6
final_head_sha: df50e020a46cd94e2ed6d742b27d69fc58667b99
delivery_merge_sha: ca0a7373104cf9908e347dcc9890f46893098928
lifecycle_closeout_pr: 196
owner: released
created_at: 2026-08-12T00:42:00+02:00
completed_at: 2026-08-12T01:10:00+02:00
execution_budget_minutes: 60
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
implementation_status: NOT_APPLICABLE
owned_paths:
  - docs/agents/tasks/active/OTV2-20260812-dur-02-profile-neutral-character-schema.md
  - docs/architecture/DUR-02_PROFILE_NEUTRAL_CHARACTER_SCHEMA_DECISION_PACKET.md
public_contracts: []
depends_on:
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
  - docs/architecture/DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md
  - docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md
  - docs/architecture/GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md
  - docs/architecture/GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
  - docs/architecture/ADR-0012-character-authority-and-platform-lifecycle-boundary.md
  - docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md
  - docs/architecture/ARCHITECTURE_STATUS_MODEL.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
blocks:
  - owner decision on profile-neutral Character persistence partial baseline
  - later full DUR-02 Persistence-v1 reconciliation
  - later separately authorized physical PostgreSQL Character persistence implementation
external_repositories: []
```

## Outcome

Delivered the nonbinding paper-only decision packet for the profile-neutral Character persistence sub-scope of stable gate `DUR-02 — Persistence v1`:

- `docs/architecture/DUR-02_PROFILE_NEUTRAL_CHARACTER_SCHEMA_DECISION_PACKET.md`.

The packet remains **PRE-DECISION ARCHITECTURE / NOT ACCEPTED**. It defines a safe persistence architecture for Character/FND-04 durability without emitting SQL DDL/migrations/runtime code, without encoding unresolved Reference behavior, and without falsely claiming the entire historical DUR-02 gate is closed.

No DUR-02 scope is owner-accepted by this task.

## Source of truth

- `PROVEN`: trusted task base was `main@2913201186d0e38cfc0bf0c9e2c5b83f981a61c6`.
- `PROVEN`: GAME-CHAR-01 was `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED`, which unblocked only paper-only profile-neutral Character persistence architecture.
- `PROVEN`: PostgreSQL is authoritative native game persistence with Platform/game DB ownership separation and no cross-database FK/shared-writer shortcut.
- `PROVEN`: UUIDv7 durable IDs use PostgreSQL `uuid`; persisted CommandId preserves full uint64 as `numeric(20,0)` in `(GameSessionId, CommandId)` scope.
- `PROVEN`: CharacterRevision is distinct from CharacterLease, GameSession/connection and RuntimeScopeAuthority generations.
- `PROVEN`: FND-04 fresh admission and reconnect/recovery require atomic final authority linearization; PREPARE is zero authority.
- `PROVEN`: mandatory durable audit evidence commits atomically with its owning mutation; publication is at-least-once and replay does not mutate gameplay.
- `PROVEN`: Character Authority is the Character semantic writer; Platform direct Character-table mutation remains forbidden.
- `PROVEN`: historical `FOUNDATION_DECISION_BACKLOG.md` defines stable DUR-02 Persistence v1 more broadly than this Character packet; overall DUR-02 cannot honestly become accepted from this package alone.
- `PROVEN`: PR #191 and PR #162 remained disjoint and untouched throughout delivery.

## Acceptance criteria

- [x] Compared wide-row, EAV/JSON, event-sourcing and normalized-current-state options and selected normalized current state + typed children as the recommendation.
- [x] Defined `character_root` and one global CharacterRevision without UUID/wall-clock authority leakage.
- [x] Defined global name registry with complete domain-generated canonical key, authoritative DB uniqueness and collision-safe naming-policy cutover.
- [x] Defined AccountId guard rows for every quota-affecting lifecycle/portfolio transition without second Account/count authority.
- [x] Defined typed build/progression/profile extension boundaries and prohibited a generic misc-state path.
- [x] Kept AccountPresenceClaim, CharacterLease, GameSession and actor-wide ControlLoss continuity separate from CharacterRevision and from one another.
- [x] Defined atomic fresh-admission authority commit with Character-root TOCTOU protection and no partial authority on failure.
- [x] Defined reconnect PREPARE as zero-authority typed candidate state and prohibited process-local socket handles as restart-stable authority.
- [x] Defined atomic reconnect/recovery COMMIT with predecessor fencing, successor generation/proof binding, stable attempt result and single-use protection entitlement under ControlLoss fencing.
- [x] Defined post-grace recovery with a new GameSessionId preserving the same actor.
- [x] Defined OperationId receipts plus optional durable `(GameSessionId, CommandId)` dedup only where a durable boundary requires it.
- [x] Defined lock ordering and conditional READ COMMITTED versus bounded SERIALIZABLE policy.
- [x] Defined retained immutable event semantics + mutable publication state with atomic mandatory evidence and separately governed privacy/retention lifecycle.
- [x] Defined normalized current-state/checkpoint and no-ack-before-commit behavior without a generic snapshot blob.
- [x] Defined migration/rollback/backup/PITR restore safety and no-authority-resurrection requirement.
- [x] Preserved retirement versus physical deletion versus privacy erasure and CharacterId non-reuse.
- [x] Kept TTL/RPO/RTO/retention/retry values and unresolved Reference behavior outside this packet.
- [x] Did not emit SQL DDL, ORM/migration files, runtime code or production configuration.
- [x] Did not update current status/register/horizon because no owner acceptance exists.
- [x] Corrected the owner-decision effect so future acceptance creates only a binding partial baseline while overall stable DUR-02 remains PROPOSED pending later whole-gate reconciliation.
- [x] Completed full exact-head self-review and repository-required documentation CI before merge.

## Recommended architecture summary

- `character_root` is the Character identity/lifecycle/owner/world/global-revision anchor.
- `account_character_guard` is game-owned portfolio serialization, not Account authority/count truth.
- `character_name_registry` is the lossless canonical-key authority with one active naming policy at a time.
- build/progression/profile state uses typed relations/child aggregates only; no JSON/KV/EAV escape hatch.
- CharacterRevision is independent of FND-04 authority generations.
- AccountPresenceClaim, CharacterLease, GameSession and actor-wide ControlLoss continuity are distinct persistence authorities.
- fresh admission is one atomic authority commit; failure leaves no partial presence/lease/session/nonce authority.
- reconnect PREPARE grants zero authority; COMMIT is the only binding switch; post-grace recovery creates a new GameSession.
- process-local physical transport handles cannot become restart-stable durable authority.
- OperationId is the durable retry identity for retryable Character Authority workflows; durable CommandId dedup is conditional and GameSession-scoped.
- READ COMMITTED requires explicit anomaly-closing locks/constraints; otherwise bounded SERIALIZABLE applies; advisory locks are never sole authority.
- retained event evidence and mutable publication state are separate; mandatory audit evidence commits atomically with the owning mutation.
- normalized current state is canonical; no redundant generic Character snapshot.
- PITR/disaster restore cannot resurrect rolled-back session/lease authority.
- migrations follow expand -> migrate/backfill -> validate -> cut over -> contract.
- future owner acceptance is partial Character-persistence scope only, not whole-DUR-02 acceptance.

## Review repairs

### Repair cycle 1 — concurrency, ownership, retention and status

Resolved:

1. account guard coverage for all quota-affecting lifecycle transitions;
2. duplicate actor-wide ControlLoss ownership risk in GameSession;
3. incomplete FND-04 recovery binding-revision set;
4. unsafe simultaneous naming-policy canonicalization universes;
5. audit-journal wording that could override accepted privacy/retention lifecycle;
6. non-normative decorated future DecisionStatus wording;
7. pseudonymous analytics wording that could incorrectly prohibit legitimate restricted player-linked audit.

### Repair cycle 2 — FND-04 authority linearization

Resolved the missing transaction-level guarantee by adding:

- atomic fresh-admission authority commit;
- PREPARE as zero-authority typed candidate/disposition state;
- atomic reconnect/recovery COMMIT;
- post-grace recovery using a new GameSessionId while preserving the existing actor.

### Repair cycle 3 — stable gate scope and final FND-04 race closure

Resolved:

1. whole-gate DUR-02 acceptance overclaim — future acceptance is partial Character persistence baseline only;
2. fresh-admission Character-root TOCTOU around concurrent ownership/world/lifecycle changes;
3. reconnect protection double-consumption risk by locking/fencing ControlLoss continuity when entitlement changes;
4. process-local socket-handle persistence as false restart-stable authority.

Repair budget used: **`3/3`**.

No material final-head finding remained after the third repair. Under the bounded repair policy, any new material finding would have blocked the task rather than triggered a fourth repair.

## Delivery validation

### Focused reconciliation

Result: **PASS after repair cycle 3** against:

- ADR-0004;
- DUR-01;
- ANL-01;
- accepted GAME-CHAR Stage A/B;
- FND-04A/B/C;
- ADR-0012 and Character Authority/Platform boundary;
- `FOUNDATION_DECISION_BACKLOG.md`;
- architecture status model and decision discipline.

### Mandatory exact-head self-review

- exact delivery head: `df50e020a46cd94e2ed6d742b27d69fc58667b99`;
- PR review id: `4911516338`;
- final material findings: `0`;
- verdict: **PASS**;
- unresolved review threads at merge: `0`.

### Exact-head CI

Final delivery head `df50e020a46cd94e2ed6d742b27d69fc58667b99`:

- Agent Governance `31545127063` / generation #885 — **success**;
- Dependency Review `31545127033` / generation #635 — **success**;
- CodeQL `31545127038` / generation #773 — **success**.

### Component/integration/runtime E2E

`NOT_APPLICABLE` — nonbinding paper-only persistence architecture analysis; no executable DB/runtime behavior changed.

### Independent review

- required: `NO` for the nonbinding packet under the trusted-base risk policy;
- reason: no executable persistence/security/production authority changed and final self-review had zero material findings;
- a later owner-accepted persistence baseline or implementation must reassess review risk.

## Delivery PR and boundaries

- delivery PR: #195;
- final changed files: exactly two declared documentation paths;
- branch behind main at merge: `0`;
- squash merge: `ca0a7373104cf9908e347dcc9890f46893098928`;
- lifecycle closeout PR: #196;
- current status/register/horizon unchanged;
- no DDL/runtime/Platform/production authority;
- PR #191 and PR #162 untouched;
- external repositories untouched.

## Downstream consequence

The next action is an **owner decision** on whether to accept the complete seventeen-rule profile-neutral Character persistence package as a binding **partial DUR-02 owner baseline**.

If accepted later:

```text
DUR-02 Character persistence sub-scope
Owner baseline        = OWNER_ACCEPTED PARTIAL BASELINE
ImplementationStatus  = NOT_STARTED
Runtime / DDL authority = NONE

DUR-02 overall
DecisionStatus        = PROPOSED
DeliveryStatus        = PLANNED after partial-baseline closeout
ImplementationStatus  = NOT_STARTED
```

A separate future full-DUR-02 reconciliation must identify the remaining historical Persistence-v1 scope and either close it or explicitly supersede/narrow it before overall DUR-02 may become ACCEPTED.

## Context checkpoint

```yaml
last_progress: Nonbinding profile-neutral Character persistence packet delivered by PR #195 with repair budget 3/3, exact-head self-review PASS and all required exact-head CI PASS; lifecycle closeout PR #196 archives the complete task and releases ownership.
status: completed
delivery_pr: 195
final_head_sha: df50e020a46cd94e2ed6d742b27d69fc58667b99
delivery_merge_sha: ca0a7373104cf9908e347dcc9890f46893098928
lifecycle_closeout_pr: 196
repair_cycles_for_current_gate: 3
ci_run_ids:
  - 31545127063
  - 31545127033
  - 31545127038
owner_action_required: true
blocker: explicit owner decision on the profile-neutral Character persistence partial baseline
next_action: NONE
```

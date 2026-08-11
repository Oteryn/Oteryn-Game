# OTV2-20260812-dur-02-profile-neutral-character-schema

```yaml
task_id: OTV2-20260812-dur-02-profile-neutral-character-schema
title: Prepare DUR-02 profile-neutral core Character schema architecture
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260812-dur-02-profile-neutral-character-schema
pr: 195
base_sha: 2913201186d0e38cfc0bf0c9e2c5b83f981a61c6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-12T00:42:00+02:00
updated_at: 2026-08-12T01:08:00+02:00
execution_budget_minutes: 60
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
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Prepare one nonbinding paper-only decision packet for the profile-neutral Character persistence sub-scope of stable gate `DUR-02 — Persistence v1`.

The packet defines relation ownership, Character/FND-04 fencing, fresh-admission and reconnect/recovery authority transactions, idempotency, locks/isolation, mandatory audit/publication atomicity, checkpoint/recovery, migration and disaster-restore safety without emitting SQL DDL/migrations/runtime code or encoding unresolved Reference behavior.

It deliberately does **not** claim to close the entire historical `DUR-02` gate.

## Source of truth

- `PROVEN`: trusted task base is `main@2913201186d0e38cfc0bf0c9e2c5b83f981a61c6`.
- `PROVEN`: GAME-CHAR-01 is `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED`; only paper-only profile-neutral Character persistence architecture is unblocked.
- `PROVEN`: PostgreSQL is the authoritative game relational target; Platform/game use separate logical databases, credentials and migration histories; no cross-DB FK/shared-writer shortcut is accepted.
- `PROVEN`: native UUIDv7 durable IDs use PostgreSQL `uuid`; persisted CommandId preserves full uint64 via `numeric(20,0)` in `(GameSessionId, CommandId)` scope.
- `PROVEN`: CharacterRevision is distinct from CharacterLease, GameSession/connection and runtime-owner generations.
- `PROVEN`: FND-04 keeps AccountPresenceClaim, CharacterLease, GameSession, TransportBinding, ControlLoss continuity and RuntimeScopeAuthority distinct; fresh admission and reconnect/recovery use atomic authority linearization boundaries.
- `PROVEN`: mandatory durable audit evidence commits atomically with its owning mutation; publication is at-least-once and replay never replays gameplay mutation.
- `PROVEN`: Character Authority alone owns native Character mutation; Platform direct Character-table writes are forbidden.
- `PROVEN`: profile-specific Character facts, unresolved Reference behavior and item/economy conservation remain outside this packet's neutral-core authority.
- `PROVEN`: historical `FOUNDATION_DECISION_BACKLOG.md` defines stable `DUR-02 — Persistence v1` more broadly than the Character persistence sub-scope in this packet; no accepted whole-gate narrowing/closure exists yet.
- `PROVEN`: PR #191 is a disjoint factual evidence-provenance correction and PR #162 is disjoint CI/governance; neither is owned by this task.
- `PROVEN`: the foundation programme checkpoint has `owned_paths: []` and is non-owning.

## Acceptance criteria

- [x] Compared wide-row, EAV/JSON, event-sourcing and normalized-current-state options; recommended normalized current state + typed children.
- [x] Defined `character_root` and one global CharacterRevision without UUID/wall-clock authority leakage.
- [x] Defined global name registry with domain-generated complete canonical key, DB uniqueness and collision-safe naming-policy cutover.
- [x] Defined AccountId guard rows for every quota-affecting lifecycle/portfolio transition without second Account/count authority.
- [x] Defined typed build/progression/profile extension boundaries; no generic misc-state path.
- [x] Kept AccountPresenceClaim, CharacterLease, GameSession and actor-wide ControlLoss continuity separate from CharacterRevision and from each other.
- [x] Defined atomic fresh-admission authority commit with Character-root TOCTOU protection and no partial authority on failure.
- [x] Defined reconnect PREPARE as zero-authority typed candidate state and prohibited persisting process-local socket handles as restart-stable authority.
- [x] Defined atomic reconnect/recovery COMMIT with predecessor fencing, successor generation, proof rotation, stable attempt result and single-use ControlLoss protection entitlement.
- [x] Defined post-grace recovery with a new GameSessionId while preserving the same actor.
- [x] Defined OperationId receipts plus optional durable `(GameSessionId, CommandId)` dedup only where needed.
- [x] Defined lock ordering and conditional READ COMMITTED versus bounded SERIALIZABLE policy.
- [x] Defined retained immutable audit semantics + mutable publication state with atomic mandatory evidence and separately governed privacy/retention lifecycle.
- [x] Defined normalized current-state/checkpoint and no-ack-before-commit behavior without a generic snapshot blob.
- [x] Defined migration/rollback/backup/PITR restore safety and no-authority-resurrection requirement.
- [x] Preserved retirement versus physical deletion versus privacy erasure and CharacterId non-reuse.
- [x] Kept exact TTL/RPO/RTO/retention/retry values and unresolved Reference behavior outside this packet.
- [x] Did not emit SQL DDL, ORM/migration files, runtime code or production config.
- [x] Did not update current status/register/horizon because no owner acceptance exists.
- [x] Corrected the owner-decision effect so acceptance creates only a binding partial baseline; overall stable DUR-02 remains `PROPOSED` until later whole-gate reconciliation.
- [ ] Perform final exact-head full-diff self-review and repository-required documentation CI before merge.

## Recommended architecture summary

- `character_root` = Character identity/lifecycle/owner/world/global-revision anchor.
- `account_character_guard` = game-owned portfolio serialization, not Account authority/count truth.
- `character_name_registry` = lossless domain-generated canonical-key authority with one active canonicalization policy.
- typed build/progression/profile relations only; no JSON/KV/EAV escape hatch.
- FND-04 authority relations separate from Character semantic persistence.
- fresh admission = one atomic authority commit; failed admission leaves no partial presence/lease/session/nonce state.
- reconnect PREPARE = zero authority; COMMIT = only binding switch; post-grace recovery = new GameSession.
- Character semantic operations use CharacterRevision; FND-04-only transitions do not advance it.
- OperationId is durable retry identity for retryable Character Authority workflows; durable CommandId dedup is conditional, scoped by GameSessionId.
- READ COMMITTED requires an explicit anomaly-closing lock/constraint proof; otherwise bounded SERIALIZABLE; advisory locks are never sole authority.
- retained durable event semantics and publication state are separate; mandatory evidence commits atomically with owning mutation.
- normalized current state is canonical; no duplicate generic Character snapshot.
- PITR/disaster restore cannot resurrect rolled-back session/lease authority.
- migration is expand -> migrate/backfill -> validate -> cut over -> contract.
- accepted scope is profile-neutral Character persistence only.

## Review repair history

### Repair cycle 1 — concurrency, ownership, retention and status

Findings:

1. account guard did not cover every quota-affecting lifecycle transition;
2. `game_session` risked duplicating actor-wide ControlLoss state;
3. FND-04 recovery binding revisions were incompletely enumerated;
4. naming-policy migration could have allowed two authoritative canonicalization universes;
5. immutable-journal wording could be read as overriding accepted privacy/retention deletion;
6. proposed future status used non-normative decorated DecisionStatus wording.

Repair:

- guard all quota-affecting transitions;
- single-own actor-wide ControlLoss in `control_loss_continuity`;
- add FND-04 protocol/transport/ruleset/content/map/world-policy/runtime-owner recovery bindings;
- require destination-name collision validation and one-authoritative-policy cutover;
- make event immutability apply while retained and preserve separate audited retention/privacy lifecycle;
- restore normative status vocabulary;
- narrow pseudonymous analytics no-raw-ID rule to pseudonymous families rather than legitimate restricted audit.

### Repair cycle 2 — FND-04 authority linearization

Finding:

- separate FND-04 relations alone did not freeze the accepted authority transition atomicity and could still allow split-control in an incorrect implementation.

Repair:

- added atomic fresh-admission commit including GrantNonce/presence/lease/new GameSession/connection generation;
- added PREPARE as zero-authority typed candidate/disposition persistence;
- added atomic reconnect/recovery COMMIT with predecessor fence, successor generation/proof binding, stable attempt result and one-time protection semantics;
- added post-grace recovery with a new GameSessionId preserving the existing actor.

### Repair cycle 3 — stable gate scope + final FND-04 race closure

Findings:

1. historical `DUR-02 — Persistence v1` is broader than this Character packet, so future owner acceptance could not honestly set overall DUR-02 DecisionStatus to `ACCEPTED`;
2. fresh-admission final revalidation needed an explicit Character-root lock/equivalent fence to close concurrent transfer/lifecycle TOCTOU;
3. reconnect COMMIT needed explicit ControlLoss-row locking when protection eligibility is consumed;
4. prepared physical transport identity must not be represented by a process-local handle across restart.

Repair:

- changed recommended owner effect to `OWNER_ACCEPTED PARTIAL BASELINE` for the Character persistence sub-scope while overall DUR-02 remains `PROPOSED / PLANNED / NOT_STARTED` after closeout;
- required later whole-gate reconciliation/narrowing before overall DUR-02 acceptance;
- added Character-root revalidation under the fresh-admission authority transaction without advancing CharacterRevision solely for FND-04 state;
- added ControlLoss continuity locking/fencing for single-use protection in reconnect/post-grace recovery;
- prohibited process-local socket handles as durable candidate authority.

Repair budget used: `3/3`.

**No further material final-head finding may be repaired inside this task. A new material finding blocks the package and requires a successor task/explicit owner authorization under the bounded repair policy.**

## Excluded scope

No PostgreSQL DDL/migration implementation, DB provisioning, Rust persistence adapter, item/currency/DUR-03 semantics, Platform writes, ruleset/content implementation, first PvP/world-profile extension, unresolved Reference behavior, production RPO/RTO, DB library/tool selection or remaining whole-gate DUR-02 subjects outside the Character sub-scope.

## Validation

### Focused

Result after repair cycle 3 before final freeze: **PASS** against ADR-0004, DUR-01, ANL-01, accepted GAME-CHAR, FND-04A/B, ADR-0012/Character Authority boundary, `FOUNDATION_DECISION_BACKLOG.md`, status model and architecture decision discipline.

### Component/integration/runtime E2E

`NOT_APPLICABLE` — nonbinding paper-only persistence architecture analysis; no executable DB/runtime behavior changed.

### Exact-head CI

Pending final immutable PR head.

## Self-review

Pending final immutable PR head.

## Independent review

- required: `NO` for this nonbinding pre-decision packet unless final review reveals unresolved material uncertainty/executable high-risk authority change; a future accepted baseline or implementation must reassess risk.

## PR and closeout

- delivery PR: #195
- expected changed files: exactly this task + decision packet
- current status/register/horizon changes: none
- runtime/DDL authority: none
- after merge, lifecycle closeout must archive the complete task and release ownership without accepting any DUR-02 scope.

## Context checkpoint

```yaml
last_progress: DUR-02 Character persistence decision packet is in draft PR #195; three bounded repair cycles resolved concurrency/FND-04/naming/retention/status issues, added fresh/reconnect authority linearization, and corrected the stable-gate effect to a partial baseline only.
status: validating
branch: docs/OTV2-20260812-dur-02-profile-neutral-character-schema
pr: 195
final_head_sha: null
final_head_frozen_at: null
ci_checks_for_current_head: 0
repair_cycles_for_current_gate: 3
owner_action_required: null
blocker: null
next_action: Recheck live main/open ownership, freeze final PR head, perform terminal full-diff self-review, run exact-head documentation CI, merge/archive only if all gates pass; any new material finding blocks this task instead of triggering a fourth repair.
```

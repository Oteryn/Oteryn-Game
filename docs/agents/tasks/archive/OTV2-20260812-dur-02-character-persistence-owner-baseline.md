# OTV2-20260812-dur-02-character-persistence-owner-baseline — archived

```yaml
task_id: OTV2-20260812-dur-02-character-persistence-owner-baseline
title: Persist owner-accepted DUR-02 Character persistence partial baseline
mode: COORDINATE
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
delivery_branch: docs/OTV2-20260812-dur-02-character-persistence-owner-baseline
delivery_pr: 197
base_sha: c98f463b26f22df99dd10ef3819086a59c25250b
final_head_sha: 3a0695bb659e5f052c2fd0dfe14f3e791ff5daf3
delivery_merge_sha: a88c15e6bf77fe4b775005011ec3cc38837f2a0a
lifecycle_closeout_pr: 198
owner: released_after_closeout
created_at: 2026-08-12T08:25:00+02:00
completed_at: 2026-08-12T08:48:00+02:00
execution_budget_minutes: 60
repair_cycles_for_delivery: 0
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
implementation_status: NOT_STARTED
runtime_authority: NONE
owned_paths:
  - docs/agents/tasks/active/OTV2-20260812-dur-02-character-persistence-owner-baseline.md
  - docs/architecture/DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
public_contracts: []
depends_on:
  - docs/architecture/DUR-02_PROFILE_NEUTRAL_CHARACTER_SCHEMA_DECISION_PACKET.md
  - docs/architecture/GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md
  - docs/architecture/GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md
  - docs/architecture/DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md
  - docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
  - docs/architecture/ADR-0012-character-authority-and-platform-lifecycle-boundary.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/ARCHITECTURE_STATUS_MODEL.md
blocks:
  - later full DUR-02 Persistence-v1 reconciliation
external_repositories: []
```

## Outcome

Recorded the owner's explicit acceptance of the complete seventeen-rule recommendation from `DUR-02_PROFILE_NEUTRAL_CHARACTER_SCHEMA_DECISION_PACKET.md` as the binding **profile-neutral Character persistence partial owner baseline**:

- `docs/architecture/DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md`.

The acceptance is intentionally scoped. It does **not** accept the full historical `DUR-02 — Persistence v1` gate.

Canonical post-closeout status is:

```text
DUR-02 profile-neutral Character persistence
Owner baseline         = OWNER_ACCEPTED PARTIAL BASELINE
DeliveryStatus         = LIFECYCLE_CLOSED
ImplementationStatus   = NOT_STARTED
Runtime / DDL authority = NONE

DUR-02 overall
DecisionStatus         = PROPOSED
DeliveryStatus         = PLANNED
ImplementationStatus   = NOT_STARTED
Runtime authority      = NONE
```

No PostgreSQL DDL/migration, Rust persistence runtime, database provisioning, Platform write or production action was performed or authorized.

## Owner source

`USER_SOURCE`: on 2026-08-12 at 08:25 +02:00, after the seventeen recommendations were explained as technical/safety persistence architecture rather than gameplay formulas, the owner instructed:

> wykonaj

The delivery records that instruction as explicit acceptance of all seventeen recommendations, the detailed semantics they summarize, their partial-scope effect, supersession/reopening discipline and explicit non-decisions.

## Binding accepted sub-scope

The owner-accepted partial baseline binds:

1. normalized current-state Character persistence instead of full event sourcing;
2. one global CharacterRevision per committed Character semantic transaction, distinct from FND-04 authority generations;
3. game-owned AccountId portfolio guards for every quota-affecting lifecycle/portfolio transition without second Account/count authority;
4. a global Character name registry using the complete domain-generated canonical key, authoritative database uniqueness and collision-safe naming-policy cutover;
5. separate persistence authorities for AccountPresenceClaim, CharacterLease, GameSession/connection generation and actor-wide ControlLoss continuity;
6. one atomic fresh-admission authority boundary with Character-root TOCTOU revalidation and no partial authority on failure;
7. reconnect/recovery PREPARE as zero-authority typed candidate/disposition state, never a process-local transport handle promoted to durable authority;
8. reconnect/recovery COMMIT as the sole binding switch, including predecessor fencing, strict successor generation, proof rotation, stable result reconciliation and single-use eligible protection under ControlLoss fencing;
9. post-grace recovery using a new GameSessionId while preserving the existing authoritative actor/gameplay state;
10. dedicated typed Character/profile extensions and prohibition of generic JSON/KV/EAV miscellaneous-state escape hatches;
11. durable OperationId receipts for retryable Character Authority operations and persisted `(GameSessionId, CommandId)` only when a real durable gameplay boundary requires it;
12. explicit anomaly-closing lock/isolation proof, with READ COMMITTED only when proven sufficient and bounded SERIALIZABLE otherwise; advisory locks never sole authority;
13. retained immutable ANL audit semantics separated from mutable publication state and committed atomically with mandatory audited mutations, subject to separately accepted privacy/retention lifecycle;
14. normalized current state as Character checkpoint authority, typed checkpoint manifests only, and no acknowledged authoritative success before commit;
15. fail-closed no-authority-resurrection after process restart/PITR/disaster restore, with a future non-rollback recovery fence/equivalent required before restored admission resumes;
16. expand -> migrate/backfill -> validate -> cut over -> contract migration discipline, with retirement, physical deletion and privacy erasure kept distinct and CharacterId never reused;
17. profile-neutral scope only: unresolved Reference/profile/operational values, PvP/world-profile state, item/economy persistence and implementation technology remain separately gated.

## Preserved boundaries

- overall stable `DUR-02 — Persistence v1` remains `PROPOSED` because the historical gate is broader than the Character persistence sub-scope;
- `GAME-ITEM-01` / `DUR-03` retain item/currency/economy conservation authority;
- unresolved GAME-CHAR Reference values/formulas remain hard per-behavior parity gates;
- FND-04 authority generations remain independent from CharacterRevision;
- Platform remains AccountId/Identity/commercial authority and cannot directly mutate native Character tables;
- no cross-database Platform/game foreign keys are introduced;
- no generic persistence bag, current-Global/OTS inference or database collation may invent unresolved gameplay semantics;
- no runtime/DDL authority follows from architecture acceptance.

## Delivery validation

### Changed scope

Final PR #197 changed exactly five declared documentation/coordination paths:

- active task record;
- `DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md`;
- `FOUNDATION_PROGRAMME_CURRENT_STATUS.md`;
- `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`;
- `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`.

`FOUNDATION_DECISION_BACKLOG.md`, PR #191, PR #162 and all external repositories remained untouched.

### Mandatory exact-head self-review

- exact head: `3a0695bb659e5f052c2fd0dfe14f3e791ff5daf3`;
- review id: `4913700786`;
- verdict: **PASS**;
- material findings: `0`;
- branch at final review: `behind_by=0`;
- unresolved review threads before merge: `0`.

The self-review revalidated partial-vs-whole DUR-02 status, CharacterRevision/FND-04 separation, fresh/reconnect atomicity, ControlLoss/protection semantics, audit atomicity, restore fencing, typed-state restrictions and the absence of DDL/runtime/production authority.

### Mandatory independent exact-head review

- independent reviewer: `chatgpt-codex-connector[bot]` / Codex Review;
- trigger comment: `5263293771`;
- independent result comment: `5263320234`;
- reviewed commit: `3a0695bb65` (exact final head prefix);
- result: **PASS — "Didn't find any major issues"**;
- no independent-review material findings or inline threads remained.

This independent review is separate from the implementing agent's self-review and satisfies the high-risk persistence/recovery/session-fencing architecture review requirement.

### Exact-head CI

For final delivery head `3a0695bb659e5f052c2fd0dfe14f3e791ff5daf3`:

- Agent Governance run `31570888666`, generation #891 — **success**;
- Dependency Review run `31570888613`, generation #639 — **success**;
- CodeQL run `31570888677`, generation #779 — **success**.

### Component / integration / runtime E2E

`NOT_APPLICABLE` — the delivery is paper-only owner architecture acceptance and changes no executable database/runtime behavior.

## Delivery result

- PR #197 final head: `3a0695bb659e5f052c2fd0dfe14f3e791ff5daf3`;
- squash merge: `a88c15e6bf77fe4b775005011ec3cc38837f2a0a`;
- final content review threads: `0`;
- overall DUR-02 remained `PROPOSED` throughout;
- no runtime, PostgreSQL DDL/migration or production state changed.

## Next architecture action

A separate bounded **whole-`DUR-02 — Persistence v1` paper-only reconciliation** is the next persistence architecture action.

It must classify each historical DUR-02 subject from `FOUNDATION_DECISION_BACKLOG.md` as:

- already satisfied by an accepted source;
- still owned by DUR-02;
- moved to another accepted/planned owner such as `GAME-ITEM-01`/`DUR-03`, OPS/PERF/privacy;
- or requiring a new owner decision.

It must not implement PostgreSQL DDL/migrations/runtime merely because the Character persistence partial baseline is accepted.

## Lifecycle closeout

- closeout branch: `docs/OTV2-20260812-dur-02-character-persistence-owner-baseline-closeout`;
- closeout PR: #198;
- closeout scope: active -> full archive plus current-status `DUR-02 DeliveryStatus OPEN -> PLANNED` and immutable delivery evidence only;
- owner release: effective when closeout merges;
- no owner semantic change permitted in closeout.

## Context checkpoint

```yaml
last_progress: Owner-accepted DUR-02 Character persistence partial baseline delivered by PR #197 with mandatory self-review, independent Codex review and all exact-head CI PASS; delivery squash-merged as a88c15e6bf77fe4b775005011ec3cc38837f2a0a.
status: completed
delivery_pr: 197
final_head_sha: 3a0695bb659e5f052c2fd0dfe14f3e791ff5daf3
delivery_merge_sha: a88c15e6bf77fe4b775005011ec3cc38837f2a0a
lifecycle_closeout_pr: 198
repair_cycles_for_delivery: 0
ci_run_ids:
  - 31570888666
  - 31570888613
  - 31570888677
self_review: 4913700786
independent_review: 5263320234
owner_action_required: false
blocker: null
next_action: lifecycle closeout only
```

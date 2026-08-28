# OTV2-20260812-game-item-01-architecture — archived

```yaml
task_id: OTV2-20260812-game-item-01-architecture
title: GAME-ITEM-01 item model and equipment architecture
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
delivery_branch: agent/otv2-20260812-game-item-01-architecture
delivery_pr: 205
base_sha: 93a49731ad91620748b87cdaba9525c9df70bc12
final_head_sha: 53d0189a114c99b4e7d44ca8d0db7a6bf5f3ea1a
delivery_merge_sha: 5c502d24557621efc798def87b68f137ba23fad8
lifecycle_closeout_branch: docs/OTV2-20260812-game-item-01-architecture-closeout
lifecycle_closeout_pr: 206
owner: released_after_closeout
created_at: 2026-08-12T11:37:00+02:00
completed_at: 2026-08-12T13:28:06+02:00
execution_budget_minutes: 60
large_budget_reason: null
repair_budget_extension: USER_SOURCE-2026-08-12-one-additional-cycle
implementation_status: NOT_STARTED
runtime_authority: NONE
postgresql_ddl_migration_authority: NONE
owned_paths:
  - docs/agents/tasks/archive/OTV2-20260812-game-item-01-architecture.md
  - docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_ANALYSIS.md
  - docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md
public_contracts:
  - GAME-ITEM-01
depends_on:
  - GAME-VISION-01
  - DUR-01
  - DUR-02
  - ANL-01
  - ADR-0005
blocks:
  - DUR-03
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Delivered one bounded, paper-only `GAME-ITEM-01` architecture package defining native item semantics, equipment/container legality and item-definition compatibility. Delivery PR #205 is merged; lifecycle closeout PR #206 promotes the canonical maintained status and releases GAME-ITEM path ownership only after its own merge.

No runtime, PostgreSQL DDL/migration, entitlement activation or production authority was granted.

## Owner repair-budget extension

The repository default is `max_repair_cycles_per_gate: 3`. Four material repair cycles had already occurred and the task correctly stopped `blocked` at commit `8b2fa27003202351e93e86dc20ff9c10418f4378`.

On 2026-08-12 the owner explicitly replied `wykonaj` to the requested authorization for **one additional GAME-ITEM-01 repair cycle**. This is `USER_SOURCE` authority for exactly one further material cycle. It did not waive any other governance, review, CI, merge, production or cross-repository rule.

The authorized additional cycle was used only to repair independent-review P1 `3765563501` and resume validation. No further material GAME-ITEM delivery repair cycle was authorized by that instruction.

## Binding sources and boundaries

- `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md`: first Reference target is production-observable Global Tibia after the 2026-07-28 server-save/maintenance boundary; unresolved exact behavior stays fail-closed.
- ADR-0005: stable namespaced content identity is canonical; compact numeric IDs are revision-scoped mappings.
- DUR-01: `ItemInstanceId` is strongly typed UUIDv7; DUR-03 owns split/merge/transform/create/destroy identity transitions.
- DUR-02: common persistence/transaction substrate only; item/currency/value conservation remains downstream.
- ANL-01: event/audit identity and durable evidence; analytics is never gameplay authority.
- `PROD-ENTITLEMENTS-01` remains `PROPOSED / PLANNED / NOT_STARTED`; GAME-ITEM may consume Platform-owned entitlement facts only through a separately accepted Oteryn-v2 consumer/enforcement contract.

## Accepted design

The accepted contract selects typed capability composition over stable ItemType definitions and separates:

```text
ItemType            -> immutable/versioned authored semantic definition
ItemInstance        -> concrete mutable lifecycle with DUR-01 ItemInstanceId
StaticItemPlacement -> authored world placement, not automatically a durable instance
```

It defines typed stack, charge, durability, temporal/decay, equipment, container, binding/restriction and upgrade/modifier capability state; rejects authoritative JSON/EAV/free-form script escape hatches; defines server-authoritative atomic equipment occupancy; requires bounded acyclic containment; and requires explicit definition compatibility/migration rather than silent reinterpretation.

DUR-03 retains atomic location, idempotency, retry/crash handling, stale-writer rejection, item/currency/value conservation and identity-transition rules.

## Acceptance criteria

- [x] Definition/instance/static-placement boundary frozen.
- [x] Typed bounded capability model with no generic authoritative data escape hatch.
- [x] Server-authoritative equipment occupancy and typed requirements.
- [x] Deterministic item-modifier contribution ordering requirement without capturing SIM formulas.
- [x] Bounded acyclic container legality while leaving atomic moves to DUR-03.
- [x] Explicit item-definition compatibility and migration classes.
- [x] World/binding/location/authorization concepts remain distinct.
- [x] Item versus non-item currency/value distinction.
- [x] Reference-sensitive exact behavior remains `PARITY_PENDING_EVIDENCE` unless separately proven.
- [x] Maintained programme/status/register/horizon/index overlays stayed live-main while delivery candidate PR #205 was open.
- [x] `PROD-ENTITLEMENTS-01` consumer/enforcement remains separately gated.
- [x] Independent-review P1 `3765563501` repaired: DUR-03 becomes eligible only after GAME-ITEM accepted merge **and** lifecycle closeout.
- [x] Task used repository-supported status vocabulary.
- [x] Owner explicitly authorized one additional repair cycle beyond the default budget.
- [x] Terminal exact-head self-review PASS on final delivery head.
- [x] Independent exact-head review completed with zero suggestions/material findings.
- [x] Required exact-head CI PASS.
- [x] Zero unresolved material review threads before delivery merge.
- [x] PR #205 squash-merged unchanged from frozen exact head.
- [ ] PR #206 lifecycle closeout promotes canonical overlays, archives this task and releases ownership; this item becomes true only when #206 merges.

## Repair history

### Cycle 1 — status-overlay evidence typo

A transient historical DUR-01 SHA typo was detected and corrected; the status overlay was later restored bit-for-bit to live `main`, so it was not in the delivery diff.

### Cycle 2 — PR metadata governance / invalid CI recovery

Agent Governance `31586112278` correctly failed the original PR title/body metadata. Metadata was repaired. A task-only commit was then incorrectly used solely to retrigger CI; Codex P2 `3765519022` correctly identified the process violation. It is recorded and was not repeated.

### Cycle 3 — premature acceptance/unblocking overlays

Codex P1 `3765519016` correctly identified pre-merge `ACCEPTED`/DUR-03-unblocked status in maintained overlays. All four maintained overlay/index files were restored to exact live-main blobs. PR #205 was narrowed to task + analysis + candidate contract.

### Cycle 4 — entitlement consumer boundary self-audit

Commit `f0d62331d71deedcb2b9a3b1fe0a1a32578200ed` removed wording that could imply an accepted Oteryn-v2 entitlement consumer and replaced pre-acceptance `accepts representation capability` wording with `defines representation capability`.

### Owner-authorized additional cycle — lifecycle-unblock repair

Independent Codex P1 `3765563501` remained applicable: the contract said accepted merge alone unblocked DUR-03 although lifecycle policy required accepted merge + closeout. Owner explicitly authorized one additional repair cycle. Commit `746f2a3e61632a1ff86da350c460a24887acc183` repaired the contract so DUR-03 remained blocked through the post-merge/pre-closeout interval.

P2 `3765563506` (unsupported `reviewing` task status) had already been fixed by the blocker checkpoint. The resumed record used valid status `validating`.

## Superseded validation history

The following evidence was valid for its exact head when recorded but became non-terminal after later material repairs. It is retained for complete task provenance and must not be read as final readiness evidence.

### Pre-owner-authorization head `f0d62331d71deedcb2b9a3b1fe0a1a32578200ed`

- implementing-agent exact-head self-review `4915520214`: **PASS** for the then-known content;
- Agent Governance `31587838775`: **success**;
- Dependency Review `31587838756`: **success**;
- CodeQL `31587838750`: **success**.

These results were superseded when the owner-authorized lifecycle-unblock repair moved the delivery head. They did not satisfy terminal delivery readiness after that change and are recorded only as historical validation evidence.

## Terminal delivery validation

Frozen delivery head: `53d0189a114c99b4e7d44ca8d0db7a6bf5f3ea1a`.

- implementing-agent exact-head self-review `4915880173`: **PASS**, material findings `0`;
- independent Codex exact-head review request `5266011485`: completed without suggestions; Codex recorded PR 👍 reaction `450215687` after reviewing the unchanged frozen head;
- Agent Governance `31591336315`: **success**;
- Dependency Review `31591336312`: **success**;
- CodeQL `31591336340`: **success**;
- unresolved review threads immediately before merge: `0`;
- changed delivery paths: exactly task + analysis + candidate contract;
- final compare to live main: `behind_by=0`;
- component/integration/runtime E2E: `NOT_APPLICABLE` — documentation-only architecture delivery.

PR #205 was squash-merged unchanged from the frozen head as `5c502d24557621efc798def87b68f137ba23fad8`.

## PR and closeout discipline

Closeout PR #206 is status/coordination-only. It may not change accepted GAME-ITEM semantics. It must:

1. complete active -> archive movement and retain this full task/evidence history;
2. promote GAME-ITEM-01 to `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` in maintained status/register/horizon/index sources;
3. expose DUR-03 as the next eligible **paper-only** architecture gate while leaving DUR-03 `PROPOSED / PLANNED / NOT_STARTED`;
4. preserve runtime/DDL/production authority as `NONE`/unauthorized;
5. preserve `PROD-ENTITLEMENTS-01` as separately unaccepted;
6. refresh the non-owning programme checkpoint and successor handoff;
7. release GAME-ITEM ownership only after closeout merge.

No closeout merge SHA or self-referential final head is invented before it exists.

## Context checkpoint

```yaml
last_progress: GAME-ITEM-01 delivery PR #205 passed exact-head self-review, independent Codex no-suggestion review and all required exact-head CI and squash-merged unchanged as 5c502d24557621efc798def87b68f137ba23fad8; lifecycle closeout is PR #206.
status: completed
delivery_pr: 205
final_head_sha: 53d0189a114c99b4e7d44ca8d0db7a6bf5f3ea1a
delivery_merge_sha: 5c502d24557621efc798def87b68f137ba23fad8
lifecycle_closeout_pr: 206
self_review: 4915880173
independent_review_request: 5266011485
independent_review_pr_reaction: 450215687
ci_run_ids:
  - 31591336315
  - 31591336312
  - 31591336340
owner_action_required: false
blocker: null
next_action: Complete PR #206 lifecycle closeout; only after its merge may a new bounded paper-only DUR-03 architecture task be opened.
```

# OTV2-20260805-foundation-preimplementation-contracts

```yaml
task_id: OTV2-20260805-foundation-preimplementation-contracts
title: Coordinate Oteryn v2 foundation contracts and staged implementation gates
mode: COORDINATE
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: null
pr: null
base_sha: 22b64e1b20cf2220828f5a3d47b30df29f9a60b6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-12T10:20:00+02:00
execution_budget_minutes: 120
large_budget_reason: Non-owning programme checkpoint spanning accepted native foundation architecture and the remaining gameplay/durability/vertical-slice gates; executable packages remain separately bounded.
owned_paths: []
public_contracts:
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
  - docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md
  - docs/architecture/README.md
continuation_prompt: docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
handover_report: docs/agents/reports/OTV2-20260812-foundation-handover.md
depends_on:
  - accepted ADR-0001 through ADR-0016 as applicable to their named scopes
  - FND-01 and VSL-02 accepted/applied
  - FND-ID-01, FND-02, FND-03, FND-04, DUR-01, ANL-01 and NET-TRANSPORT-01 accepted/lifecycle-closed
  - GAME-VISION-01 and GAME-CHAR-01 accepted/lifecycle-closed
  - DUR-02 profile-neutral Character persistence partial baseline accepted/lifecycle-closed
  - whole DUR-02 Persistence-v1 accepted/lifecycle-closed
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
```

## Outcome

Maintain a truthful **non-owning** programme checkpoint for Oteryn-v2. It coordinates accepted architecture and names exactly one next safe package without implementing gates, reserving their paths or treating architecture acceptance as runtime completion.

Every substantial architecture or implementation gate still requires its own bounded task, branch, PR, validation, review policy, merge and archive lifecycle.

## Canonical continuation order

Use these sources in this order:

1. `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` — current DecisionStatus/DeliveryStatus/ImplementationStatus truth;
2. accepted ADRs/contracts/owner baselines and exact registries — semantic authority;
3. `docs/architecture/FOUNDATION_DECISION_BACKLOG.md` — stable gate definitions/dependencies;
4. `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` — global staged horizon;
5. `docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md` — gameplay/product ownership and dependencies;
6. `docs/agents/reports/OTV2-20260812-foundation-handover.md` — current successor handoff;
7. this checkpoint — non-owning programme summary;
8. live GitHub branch/PR/CI/task ownership — execution truth.

Older prose that conflicts with the current-status overlay or later accepted baselines is historical, not executable instruction.

## PROVEN current foundation state

Trusted state at this refresh originated from `main@22b64e1b20cf2220828f5a3d47b30df29f9a60b6`.

Accepted/lifecycle-closed architecture includes:

- canonical 19-member Rust workspace and client migration/cutover;
- `FND-ID-01` foundation identifiers;
- `FND-02` `protocol-oteryn` architecture;
- `FND-03` runtime execution architecture;
- `FND-04` admission/GameSession/CharacterLease/reconnect architecture;
- `DUR-01` durable identifier representation;
- `ANL-01` event/audit foundation;
- TCP-default/future-QUIC architecture direction under ADR-0014..0016 while gameplay transport runtime remains unavailable;
- `GAME-VISION-01` minimum product direction and immutable first Reference target;
- `GAME-CHAR-01` Stage A + Stage B semantics;
- profile-neutral Character persistence partial baseline;
- whole `DUR-02 — Persistence v1` common architecture.

Whole DUR-02 is canonically:

```text
DecisionStatus       = ACCEPTED
DeliveryStatus       = LIFECYCLE_CLOSED
ImplementationStatus = NOT_STARTED
Runtime authority    = NONE
DDL/migrations       = NOT_AUTHORIZED
```

Recent binding evidence:

- GAME-CHAR delivery #193 / closeout #194;
- Character persistence partial baseline #197 / closeout #198;
- whole-DUR-02 reconciliation #199 / closeout #200;
- whole-DUR-02 owner acceptance #201 / merge `ec4b840b0742967370a4235d87094b29a802fe28`;
- whole-DUR-02 lifecycle closeout #202 / merge `22b64e1b20cf2220828f5a3d47b30df29f9a60b6`.

No complete production-ready Rust GameNode, gameplay transport listener path, persistence runtime, event/outbox runtime or native client-to-server gameplay E2E is implied by those architecture decisions.

## Accepted architecture — do not silently redesign

A successor must consume rather than reopen, unless a later explicit superseding decision is accepted:

- one native Rust client/server stack and one project-owned `protocol-oteryn`;
- `protocol-canary` reference-only and absent from production runtime/fallback/translation paths;
- one-process GameNode identity under ADR-0009 with evidence-driven internal decomposition;
- multichannel-first world model and one authoritative mutation owner per current channel/instance scope;
- Platform Identity/Gateway versus native-game authority separation;
- PostgreSQL native-game target with separate Platform/game ownership;
- FND-02/FND-03/FND-04 authority, ordering, fencing and recovery semantics;
- GAME-VISION first Reference target and evidence discipline;
- GAME-CHAR ownership/lifecycle/progression semantic envelope;
- Character persistence normalized/typed model and no generic miscellaneous JSON/KV/EAV state escape hatch;
- whole-DUR-02 migration, transaction, durable outbox, durable-ack, PITR/restore and schema-evolution rules;
- ANL-01 event/audit semantic ownership and read-only Game Intelligence boundaries.

## Binding DUR-02 ownership split

Whole DUR-02 no longer owns item/economy/social/house/reward gameplay semantics.

Preserve:

- `GAME-ITEM-01 -> DUR-03` for item/currency/value semantics and conservation/anti-duplication;
- `EXP-ECONOMY-01` for market/economy semantics;
- `EXP-SOCIAL-01` for guild/social semantics;
- `EXP-HOUSES-01` for houses;
- `GAME-META-01` for recurring/meta rewards;
- `EXP-EVENTS-01` for encounter/event rewards;
- `ANL-01` for event/audit semantics;
- PERF/implementation evidence for partitioning and exact Rust DB/migration-library choices unless correctness evidence requires an architecture decision.

## Implementation boundary

Architecture prerequisites now permit a **later separately authorized** server/persistence foundation implementation programme for accepted common scopes.

Accepted decomposition hypothesis for that future authorization:

```text
server bootstrap / GameNode shell
-> protocol-oteryn transport/runtime adapter
-> admission + GameSession + CharacterLease
-> PostgreSQL migration/persistence substrate
-> Character/FND-04 persistence
-> minimal ChannelRuntime
```

This checkpoint grants **no implementation authority**. Do not create server runtime code, DDL/migrations or production configuration unless a new explicit owner instruction authorizes an implementation package.

## Current ordered architecture work

Because no new implementation authority was granted at the 2026-08-12 handoff, the immediate safe paper-only sequence is:

1. `GAME-ITEM-01` — item model/equipment/container/transform semantics against the accepted Reference target where parity applies;
2. `DUR-03` — item/currency/value transfer, conservation, single-location, retry/crash and anti-duplication invariants after GAME-ITEM-01;
3. `GAME-CHANNEL-01`, Reference evidence/parity tooling, `DUR-04` minimum headless content path and `SIM-DETERMINISM-01` may proceed in parallel only under separate explicit path ownership;
4. real-boundary VSL implementation expands only after each owning architecture and implementation authorization is ready.

QUIC remains later evidence-gated work, not the current implementation priority.

## Concurrent work discipline

At this refresh, unrelated open PRs included #191 (GAME-CHAR factual provenance correction) and #162 (CI/governance aggregate merge gate). Do not mutate them from a new gameplay package unless live ownership explicitly requires coordination.

Long-lived lag/disconnect architecture checkpoint tasks also remain present and architecture-only. Do not absorb or delete them as cleanup.

Always re-check live state before acting because these facts may advance after this checkpoint refresh.

## Repository and production authority

Routine writes: `blakinio/Oteryn-v2` only.

External repositories remain read-only unless the owner separately authorizes an exact write task. This checkpoint grants no production deployment, protected-environment, secret, live account/session/data/database or asset-copying authority.

## Context checkpoint

```yaml
last_progress: GAME-VISION, GAME-CHAR, profile-neutral Character persistence and whole DUR-02 Persistence-v1 architecture are accepted/lifecycle-closed. The stale FND-01-era programme handoff was refreshed so a successor can continue from current repo truth without chat history.
status: ready
branch: null
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: From live main, create one bounded paper-only `GAME-ITEM-01` architecture task that consumes the accepted Reference target and preserves `DUR-03` as the item/currency/value conservation authority.
```

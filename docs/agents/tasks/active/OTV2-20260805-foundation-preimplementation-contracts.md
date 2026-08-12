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
base_sha: 54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-12T17:26:00+02:00
execution_budget_minutes: 120
large_budget_reason: Non-owning programme checkpoint spanning accepted native foundation architecture and the remaining gameplay/content/vertical-slice gates; executable packages remain separately bounded.
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
  - FND-ID-01, FND-02, FND-03, FND-04, DUR-01, DUR-02, DUR-03, ANL-01 and NET-TRANSPORT-01 accepted/lifecycle-closed after their recorded closeouts
  - GAME-VISION-01, GAME-CHANNEL-01, GAME-CHAR-01 and GAME-ITEM-01 accepted/lifecycle-closed after their recorded closeouts
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

This refresh is part of GAME-CHANNEL lifecycle closeout PR #210 and becomes canonical only when that closeout merges.

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

Older progress prose that conflicts with current status or later accepted contracts is historical, not executable instruction.

## PROVEN accepted/lifecycle-closed architecture

Consume rather than redesign:

- canonical native Rust workspace/client cutover and one project-owned `protocol-oteryn`;
- FND-ID-01 typed identity vocabulary;
- FND-02 protocol/CommandRef sequencing/reconciliation;
- FND-03 one-writer runtime ownership, generations and asynchronous external-work boundary;
- FND-04 GameSession/CharacterLease/admission/recovery fencing;
- DUR-01 durable identifier representation including non-reused UUIDv7 ItemInstanceId;
- DUR-02 common Persistence-v1 transaction/migration/outbox/durable-ack/PITR/schema-evolution architecture;
- ANL-01 event/audit identity, durable evidence, privacy and read-only replay/investigation boundary;
- GAME-VISION-01 minimum product direction and immutable first Reference target after the 2026-07-28 Global Tibia server-save/maintenance boundary;
- GAME-CHAR-01 Stage A + Stage B semantic closure with unresolved exact Reference behavior still fail-closed;
- GAME-ITEM-01 typed item definition/instance/equipment/container/revision semantics;
- DUR-03 item/currency/value transaction, conservation and anti-duplication semantics from PR #207 merge `63380bcba469027e90677aaf4db571fa941be2f4`;
- GAME-CHANNEL-01 channel product/lifecycle policy from delivery PR #209 exact final head `ca1112191ede7d316c874189f3053ad7f8247579`, squash merge `54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1`.

Do not restart these gates merely because older backlog/predecision prose describes an earlier state.

## Accepted GAME-CHANNEL boundary — consume, do not reopen

`GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md` freezes:

- canonical `ChannelRef=WorldId+ChannelId` versus non-authoritative display labels/ordinals;
- bounded eligible-set + non-authoritative recommendation + explicit eligible target with FND-04 retaining final admission authority;
- fail-closed stale/full/draining/recovering/incompatible targets and no silent grant retarget;
- optional bounded target-Channel pre-admission queue that is never GameSession/CharacterLease/value authority;
- privacy-bounded party/friend co-location hints with independent Character admissions and no party-owned Channel;
- same-Channel reconnect/recovery versus completed fresh-session Channel switch;
- durable GAME-CHANNEL/world-policy anti-hopping state scoped Character+World, including prior successful Channel semantics across logout/relog/restart;
- different-Channel destination admission + remembered Channel/guard update as one recovery-safe semantic outcome;
- no mandatory new ChannelSwitchId without later lifecycle evidence;
- fail-closed authored Channel-sensitive value-source multiplicity classes `CHANNEL_LOCAL_REPEATABLE`, `CHANNEL_LOCAL_SHARED_ELIGIBILITY`, `WORLD_SCOPED_UNIQUE`, `EXPLICIT_EVENT_POLICY_REQUIRED`;
- explicit separation of event simulation scope and durable reward eligibility scope;
- no hidden inverse spawn/loot tuning based on active Channel count;
- qualitative lifecycle triggers `DEMAND_PRESSURE`, `RECOVERY_PRESSURE`, `LOW_LOAD_CONSOLIDATION_CANDIDATE`, `CHANNEL_UNHEALTHY`;
- legitimate new semantic Channel creation only from sustained eligible demand or bounded recovery-capacity need with insufficient healthy eligible capacity/headroom, compatible product/event/reward multiplicity policy, safe resources and Ready-before-Selectable;
- party/private-copy preference, emptier-Channel preference, rare-spawn/loot farming, PvP avoidance or operator preference alone cannot create public Channel capacity;
- same-ChannelId recovery for affected failed actors; a new semantic Channel may supply general capacity but cannot silently continue failed Channel A actors as Channel B;
- low-load drain prerequisites, drain abort/hold conditions and terminal retirement settlement/fencing/stale-routing requirements;
- temporary stop/reactivation versus permanent semantic retirement; retired ChannelId never reused;
- local PvP execution with World/Character consequences surviving Channel transitions;
- one-World guild/economy/ranking/community identity while local speech/combat/position stays Channel-local;
- product-facing `SELECTABLE`, `CAPACITY_LIMITED`, `DRAINING`, `RECOVERING`, `UNAVAILABLE` semantics;
- existing `world_policy_revision` for Channel-policy compatibility and separate cross-World lifecycle.

GAME-CHANNEL intentionally leaves all numeric demand/load windows, utilization percentages, queue objectives, headroom, players/Channel thresholds, min/max Channels, cooldowns, hysteresis/timers and placement capacities to PERF/OPS evidence. It does not grant runtime/client/Platform/DDL/production/entitlement authority.

## Accepted DUR-03 boundary — consume, do not reopen

DUR-03 remains authoritative for durable item/currency/value location, ItemInstanceId lifecycle transitions, exact conservation/source-sink lineage, CommandRef/OperationId/TransactionId idempotency/retry/ambiguous-commit handling, current lease/runtime fencing, typed custody, bounded durable audit and restore-integrity/anti-duplication.

GAME-CHANNEL decides product multiplicity/eligibility/lifecycle; it cannot authorize value creation or weaken DUR-03.

## Implementation boundary

Architecture acceptance still grants **no executable authority**. No task may create or claim:

- Rust GameNode/Channel/item/content runtime;
- protocol gameplay payload implementation;
- PostgreSQL DDL/migration files or migration execution;
- Platform/Gateway/World Registry implementation changes;
- live item/currency/value or Channel mutation;
- production traffic/deployment/configuration;
- Premium/VIP or other entitlement activation;

without a separate explicit owner implementation authorization and its own bounded task/evidence.

A future authorized server/persistence programme may consume accepted contracts through bounded real-boundary slices. The safe decomposition hypothesis remains:

```text
server bootstrap / GameNode shell
-> protocol-oteryn transport/runtime adapter
-> admission + GameSession + CharacterLease
-> PostgreSQL migration/persistence substrate
-> Character/FND-04 persistence
-> minimal ChannelRuntime
-> later movement/combat/item/content slices
```

Channel product implementation additionally consumes GAME-CHANNEL and later PERF/OPS numeric/orchestration evidence. Item/value implementation additionally consumes GAME-ITEM/DUR-03 plus concrete ANL/resource-limit evidence.

## Current ordered architecture work

After GAME-CHANNEL lifecycle closeout, remaining pre-VSL paper-only architecture includes independent Reference evidence/parity tooling, `DUR-04` and `SIM-DETERMINISM-01`.

To preserve singular ownership, the selected next action is:

1. `DUR-04 — Content, World Detail and Scripting Contract` — bounded paper-only architecture for the minimum headless content path: source schema -> validator -> deterministic compiler -> World Bundle -> loader, including Content Registry package/version/dependency semantics, source/bundle evolution and compatibility, corruption/decompression/resource limits, migration/import classification, scripting capabilities/isolation/persistence access/hot-reload boundary and asset provenance. Do not build the full Studio or runtime implementation.

`SIM-DETERMINISM-01` and Reference evidence/parity tooling may proceed independently under separate ownership. Neither is implicitly accepted by choosing DUR-04 as the next action.

QUIC remains later profile/reconciliation/evidence work.

## Repository and production authority

Routine writes remain limited to `blakinio/Oteryn-v2`. External repositories remain read-only unless the owner separately authorizes an exact write task.

This checkpoint grants no production deployment, protected-environment approval, secrets, live account/session/data/database mutation, entitlement activation or proprietary asset copying.

## Context checkpoint

```yaml
last_progress: GAME-CHANNEL-01 delivery PR #209 passed exact-head self-review, required fresh independent no-suggestion Codex review and Agent Governance/Dependency Review/CodeQL after repair budget 3/3, then squash-merged unchanged as 54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1; lifecycle closeout #210 reconciles canonical status/handoff and releases GAME-CHANNEL ownership.
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
owner_action_required: false
blocker: null
next_action: From live main after GAME-CHANNEL lifecycle closeout, create one bounded paper-only `DUR-04` architecture task for the minimum headless content/world/scripting contract; do not implement runtime/Studio/DDL/production behavior.
```

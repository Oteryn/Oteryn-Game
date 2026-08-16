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
base_sha: c197ba12cc1b2ebbc4b27eab5d6054037720c48a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-16T16:00:00+02:00
execution_budget_minutes: 120
large_budget_reason: Non-owning programme checkpoint spanning accepted native foundation/game/content/determinism architecture, completed A-F proposal delivery and remaining owner-decision/vertical-slice gates; executable packages remain separately bounded.
owned_paths: []
public_contracts:
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
  - docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md
  - docs/architecture/OTERYN_V2_POST_WAVE_A_F_RECONCILIATION_20260816.md
  - docs/architecture/README.md
continuation_prompt: docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
handover_report: docs/agents/reports/OTV2-20260812-foundation-handover.md
depends_on:
  - accepted ADR-0001 through ADR-0016 as applicable to their named scopes
  - FND-01 and VSL-02 accepted/applied
  - FND-ID-01, FND-02, FND-03, FND-04, DUR-01, DUR-02, DUR-03, DUR-04, ANL-01, NET-TRANSPORT-01 and SIM-DETERMINISM-01 accepted/lifecycle-closed after recorded closeouts
  - GAME-VISION-01, GAME-CHANNEL-01, GAME-CHAR-01 and GAME-ITEM-01 accepted/lifecycle-closed after recorded closeouts
  - Reference evidence/parity manifest v1 accepted/pinned; manifest revision 3 contains the first representative ABILITY_COMBAT case set from PR #255
  - Agent A PR #271 completed target-continuity/provenance pass with 0/4 cases promoted
  - GAME-ABILITY PR #268, GAME-AI successor #276, GAME-INTERACTION successor #277, ALPHA-CLIENT #273 and ANL-02/03 #270 merged and lifecycle-closed as proposal/candidate deliveries without automatic owner acceptance
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
```

## Outcome

Maintain a truthful **non-owning** programme checkpoint. This record owns no architecture path and grants no implementation authority. Every substantial future gate still requires its own bounded task, branch, PR, validation and archive lifecycle.

## Canonical continuation order

Use, in order:

1. `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` for current DecisionStatus/DeliveryStatus/ImplementationStatus and selected successor;
2. accepted ADRs/contracts/owner baselines and machine-readable registries for semantic authority;
3. `docs/architecture/OTERYN_V2_POST_WAVE_A_F_RECONCILIATION_20260816.md` for the exact status/coverage reconciliation of the completed first A-F wave against older horizon/gap snapshots;
4. `docs/architecture/FOUNDATION_DECISION_BACKLOG.md` for stable gate definitions;
5. `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` and `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md` for global/gameplay staged ownership;
6. `docs/agents/reports/OTV2-20260812-foundation-handover.md` for the durable earlier successor packet;
7. live GitHub main/PR/task/CI state for execution truth.

Older progress prose that conflicts with a later accepted contract, the current-status overlay or the explicitly scoped 2026-08-16 reconciliation overlay is historical.

## Accepted/lifecycle-closed architecture to consume

Do not restart these gates merely because older backlog/predecision text describes an earlier state:

- native Rust workspace/client cutover and one project-owned `protocol-oteryn`;
- FND-ID-01 identity vocabulary;
- FND-02 protocol/CommandRef sequencing/reconciliation;
- FND-03 one-writer runtime ownership/generation/asynchronous-work boundary;
- FND-04 GameSession/CharacterLease/admission/recovery fencing;
- DUR-01 durable identifier representation;
- DUR-02 common Persistence-v1 migration/transaction/outbox/durable-ack/PITR/schema-evolution architecture;
- ANL-01 event/audit identity, durable evidence, privacy and read-only investigation boundary;
- GAME-VISION-01 product direction and immutable first Reference target after the 2026-07-28 Global Tibia server-save/maintenance boundary;
- GAME-CHAR-01 Stage A/B semantic closure with unresolved Reference behavior remaining fail-closed;
- GAME-ITEM-01 typed item definition/instance/equipment/container/revision semantics;
- DUR-03 item/currency/value location, transaction, conservation and anti-duplication semantics;
- GAME-CHANNEL-01 selection/queue/co-location/anti-hopping/multiplicity/qualitative lifecycle/community policy;
- DUR-04 typed semantic content graph, exact package locking, deterministic compilation, immutable bundle staging/activation/migration, bounded loader/provenance and capability-oriented deterministic scripting;
- SIM-DETERMINISM-01 deterministic arithmetic/RNG/order/replay/state-hash/supported-target architecture;
- Reference evidence/parity manifest v1 acceptance/pinning: schema v1 unchanged, immutable first target retained, `canonical_digest=null` until accepted tooling;
- first representative ABILITY_COMBAT evidence package from PR #255 / merge `d04f0939f0078cb677ca3ad66f5949e9f3dadc8d`: manifest revision 3, four Light Healing/Ice Strike cases, all target `UNKNOWN`, source/case/legal provenance `PENDING`, implementation `NOT_STARTED`, parity `PARITY_PENDING_EVIDENCE`.

## Completed first A-F architecture delivery wave

The work-allocation completion condition is satisfied. Delivery/lifecycle truth:

- Agent A #271 — merged/lifecycle-closed; **0/4 ABILITY_COMBAT cases promoted**; fail-closed evidence state preserved;
- GAME-ABILITY #268 — merged/lifecycle-closed `CANDIDATE`; implementation `NOT_STARTED`; no owner acceptance implied;
- GAME-AI successor #276 — merged/lifecycle-closed `PROPOSED`; predecessor #272/#261 superseded; implementation `NOT_STARTED`;
- GAME-INTERACTION successor #277 — merged/lifecycle-closed `PROPOSED`; predecessor #269/#262 superseded; implementation `NOT_STARTED`;
- ALPHA-CLIENT #273 — merged/lifecycle-closed `CANDIDATE`; implementation `NOT_STARTED`;
- ANL-02/ANL-03 #270 — merged/lifecycle-closed `CANDIDATE`; implementation `NOT_STARTED`, analytics authority remains read-only/observational.

The 2026-08-16 C/D/E/F repair-budget continuation and C/D successor delegation history is preserved in the archived worker tasks and successor-delegation record. Do not reset stable-gate repair history through a new task name.

## Current ordered paper-only architecture work

The versioned Reference manifest, first representative ABILITY_COMBAT case package and bounded Agent-A continuity/provenance pass are already delivered. **Do not rebuild or rerun them as the programme next action.**

The selected next bounded paper-only programme action is:

```text
Prepare a GAME-ABILITY-01 owner-decision package from the merged whole-gate
analysis/candidate and obtain an explicit owner disposition without runtime implementation.
```

The next task must:

- consume `GAME-ABILITY-01_WHOLE_GATE_GAP_ANALYSIS.md` and `GAME-ABILITY-01_WHOLE_GATE_CONTRACT_CANDIDATE.md` as merged candidate evidence, not as already accepted architecture;
- preserve all accepted GAME-ABILITY partial baselines unless an explicit supersession is justified;
- preserve Agent A's 0/4 result and keep target/provenance/legal/parity state fail-closed;
- apply `ARCHITECTURE_DECISION_DISCIPLINE.md` and answer the mandatory decision-timing test;
- preserve FND-03 timer/catch-up semantics and SIM deterministic order/revision constraints;
- preserve GAME-ITEM/DUR-03, GAME-INTERACTION, GAME-AI, client/protocol and ANL ownership boundaries;
- obtain an explicit owner disposition such as accept/rework/defer without pretending candidate merge already accepted the gate;
- not create runtime/client/server/protocol/content/DDL/Platform/production implementation;
- not claim `PARITY_CONFIRMED` without sufficient target evidence, cleared provenance/legal state, exact Oteryn implementation revision and passing bounded fixture/test evidence.

After that owner decision is canonically delivered and lifecycle-closed, re-read live main and re-evaluate the merged GAME-AI, GAME-INTERACTION, ALPHA-CLIENT and ANL-02/ANL-03 packages before selecting their owner-decision order.

## Implementation boundary

Architecture acceptance and evidence/candidate delivery grant **no executable authority**. A future task may not create or claim Rust GameNode/Channel/item/content/SIM runtime, combat/AI/progression/scripts, compiler/loader/Studio/WIT host, gameplay client/runtime, analytics detectors/warehouse, PostgreSQL DDL/migrations, Platform/Gateway/World Registry changes, broad legacy content import, production traffic/configuration or entitlement activation without separate explicit owner implementation authority and its own evidence.

A later executable sequence remains bounded real-boundary slices only after the owning contracts/evidence are ready: GameNode/bootstrap -> `protocol-oteryn` transport/runtime -> admission/GameSession/CharacterLease -> PostgreSQL substrate -> Character/FND-04 persistence -> minimal ChannelRuntime -> later movement/combat/persistence/recovery/multichannel vertical slices.

## Repository and production authority

Routine writes remain limited to `blakinio/Oteryn-v2`. External repositories remain read-only unless explicitly authorized for an exact write task.

No production deployment, protected-environment approval, secrets, live account/session/data/database mutation, entitlement activation or proprietary asset copying is authorized.

Owner-funded Codex/OpenAI use remains forbidden unless the owner explicitly authorizes the exact PR/task invocation. Historical authorizations do not carry forward.

## Context checkpoint

```yaml
last_progress: First A-F parallel architecture wave is delivery/lifecycle complete. Agent A #271 preserved 0/4 ABILITY_COMBAT promotions; merged GAME-ABILITY/GAME-AI/GAME-INTERACTION/ALPHA-CLIENT/ANL-02/03 packages remain candidate/proposed until explicit owner acceptance. Global post-wave reconciliation is issue #302 / PR #303.
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
next_action: From live main after post-wave reconciliation closeout, create one bounded paper-only GAME-ABILITY-01 owner-decision task using the merged whole-gate candidate; preserve Agent-A 0/4 fail-closed Reference truth and do not implement runtime/DDL/production behavior.
```
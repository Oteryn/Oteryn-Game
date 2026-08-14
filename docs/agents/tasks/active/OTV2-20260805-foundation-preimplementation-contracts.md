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
base_sha: d04f0939f0078cb677ca3ad66f5949e9f3dadc8d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-14T13:55:00+02:00
execution_budget_minutes: 120
large_budget_reason: Non-owning programme checkpoint spanning accepted native foundation/game/content/determinism architecture and remaining Reference/vertical-slice gates; executable packages remain separately bounded.
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
  - FND-ID-01, FND-02, FND-03, FND-04, DUR-01, DUR-02, DUR-03, DUR-04, ANL-01, NET-TRANSPORT-01 and SIM-DETERMINISM-01 accepted/lifecycle-closed after recorded closeouts
  - GAME-VISION-01, GAME-CHANNEL-01, GAME-CHAR-01 and GAME-ITEM-01 accepted/lifecycle-closed after recorded closeouts
  - Reference evidence/parity manifest v1 accepted/pinned; manifest revision 3 contains the first representative ABILITY_COMBAT case set from PR #255
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
3. `docs/architecture/FOUNDATION_DECISION_BACKLOG.md` for stable gate definitions;
4. `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` and `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md` for global/gameplay staged ownership;
5. `docs/agents/reports/OTV2-20260812-foundation-handover.md` for the durable successor packet;
6. live GitHub main/PR/task/CI state for execution truth.

Older progress prose that conflicts with a later accepted contract or the current-status overlay is historical.

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

## Current ordered paper-only architecture work

The versioned Reference manifest and its first representative ABILITY_COMBAT case package are already delivered. **Do not rebuild them.**

The selected next bounded paper-only programme action is:

```text
Obtain target-continuity + provenance-clearance evidence for the four registered
ABILITY_COMBAT cases (Light Healing and Ice Strike).
```

The next task must:

- preserve the immutable Reference target `global-tibia-observable-2026-07-28-post-server-save`;
- locate provenance-cleared, time-appropriate evidence that directly bridges or captures the 2026-07-28 target boundary;
- promote/reject each existing case only when the accepted manifest evidence rules permit it;
- keep `UNKNOWN/PENDING` fail-closed when continuity or provenance remains insufficient;
- never treat patch-note/search silence as continuity proof;
- not broaden the mechanic inventory or freeze a physical catalogue schema/fixture runner before the representative historical-evidence path is proven;
- not invent a new stable gate ID unless explicitly registered;
- not claim `PARITY_CONFIRMED` without sufficient target evidence, cleared provenance/legal state, exact Oteryn implementation revision and passing bounded fixture/test evidence.

## Implementation boundary

Architecture acceptance and evidence registration grant **no executable authority**. A future task may not create or claim Rust GameNode/Channel/item/content/SIM runtime, combat/AI/progression/scripts, compiler/loader/Studio/WIT host, PostgreSQL DDL/migrations, Platform/Gateway/World Registry changes, broad legacy content import, production traffic/configuration or entitlement activation without separate explicit owner implementation authority and its own evidence.

A later executable sequence remains bounded real-boundary slices only after the owning contracts/evidence are ready: GameNode/bootstrap -> `protocol-oteryn` transport/runtime -> admission/GameSession/CharacterLease -> PostgreSQL substrate -> Character/FND-04 persistence -> minimal ChannelRuntime -> later movement/combat/persistence/recovery/multichannel vertical slices.

## Repository and production authority

Routine writes remain limited to `blakinio/Oteryn-v2`. External repositories remain read-only unless explicitly authorized for an exact write task.

No production deployment, protected-environment approval, secrets, live account/session/data/database mutation, entitlement activation or proprietary asset copying is authorized.

Owner-funded Codex/OpenAI use remains forbidden unless the owner explicitly authorizes the exact PR/task invocation. Authorization for PR #255 does not carry forward.

## Context checkpoint

```yaml
last_progress: PR #255 exact final head 6744f655c6438eebeab70b30aae17d33b5bd2fa7 passed final self-review, owner-authorized Codex no-finding review and exact-head Agent governance/Merge authority/Merge gate, then squash-merged as d04f0939f0078cb677ca3ad66f5949e9f3dadc8d. Issue #254 is completed; lifecycle closeout is issue #256.
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
next_action: From live main after ABILITY_COMBAT evidence lifecycle closeout, create one bounded paper-only target-continuity + provenance-clearance evidence task for the four registered Light Healing/Ice Strike cases; do not broaden/freeze tooling first and do not implement runtime/DDL/production behavior.
```

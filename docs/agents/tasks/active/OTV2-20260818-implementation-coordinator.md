# OTV2-20260818-implementation-coordinator

```yaml
task_id: OTV2-20260818-implementation-coordinator
title: Coordinate first native implementation wave
mode: COORDINATE
status: next_wave_readiness_prepared_allocations_pending
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/next-wave-readiness-reconcile-20260824
pr: 92
base_sha: 91d5865b1a33598d24391e3692d588462faf7c18
head_sha: null
owner: chat-github-20260818-implementation-coordinator
created_at: 2026-08-18T16:10:00+02:00
updated_at: 2026-08-24T15:40:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260818-implementation-coordinator.md
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/tasks/active/OTV2-20260822-impl-vsl-content.md
  - docs/agents/tasks/archive/OTV2-20260824-content-evidence-activation-fence-repair.md
public_contracts:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
depends_on:
  - Oteryn-Game#58
  - Oteryn-Game#84
  - Oteryn-Game#86
  - Oteryn-Game#87
  - Oteryn-Game#89
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Outcome

Coordinate the active native implementation programme, serialize shared mutations and preserve exact merged, repaired and still-blocked state without inventing product decisions.

## Current proven state

- `PROVEN`: FOUNDATION, SIM, DOMAIN and the repaired non-production CONTENT evidence seam are merged; Content repair #87 and closeout #89 are terminal.
- `PROVEN`: Issue #54 remains separately owner-gated for production Content hard maxima/activation and does not block evidence-only downstream preparation.
- `PROVEN`: QA has an allocated branch-only evidence shell at checkpoint `58d64130cc0526001bd1c9a00a179e1c39ad6e51`, no PR, and real Tier 1/Tier 2 remain `NOT_EVALUATED`.
- `PROVEN`: DUR-02 selects PostgreSQL and one game-owned migration history, but current `main` has no Rust PostgreSQL dependency or migration ledger; exact driver/migration library/DDL remain implementation choices requiring an explicit allocation.
- `PROVEN`: accepted GAME-ABILITY, GAME-INTERACTION, GAME-AI and VSL-MOVE require finite resource bounds while deliberately deferring exact numeric maxima; current Resource Limits Registry has no owning lane-specific entries for those required dimensions.
- `PROVEN`: the game-server still exposes only `GameplayAvailability::UnavailableBootstrap`; there is no production gameplay listener/client-entry seam, so CLIENT remains blocked.
- `DERIVED`: immediate safe parallel preparation is QA reconciliation, DURABILITY topology/allocation, CONTENT-FORMAT-SPIKE allocation and a Wave-2 resource-limit decision packet.
- `DERIVED`: Ability/Interaction/AI are architecture-ready but not executable-acceptance-ready until applicable numeric hard maxima are accepted/registered or the allocated slice explicitly excludes those dimensions fail-closed.

## Coordinator decision

Do not release all dependency-ready lanes simultaneously. The next wave is split into preparation gates and implementation lanes:

1. QA keeps its existing primary-path allocation and must be reconciled onto current `main`, delivered as an evidence-shell PR and remain truthful that Tier 1/Tier 2 are not yet proven.
2. DURABILITY requires a coordinator-owned topology preflight/allocation before writes. Recommended first increment is one game-server-local durability module plus one game-owned migration ledger, avoiding a speculative new workspace crate; any DB dependency, migration directory and Cargo/shared mutations must be explicitly allocated.
3. CONTENT-FORMAT-SPIKE may receive an evidence-only allocation now; it cannot select the permanent format.
4. Wave-2 resource limits are an owner/evidence decision packet. The coordinator may register exact values only after accepted evidence/owner decision; workers may not invent them.
5. Ability/Interaction/AI primary implementation can run in parallel only after their applicable resource-limit gate is satisfied or their executable slice is explicitly narrower and fail-closed.
6. A separate production gameplay listener/client-entry seam must merge before CLIENT is released. Movement follows Interaction + Client + real QA; Combat follows Movement + Ability + Durability + the remaining accepted prerequisites.

No write authority is granted by this reconciliation.

## Validation / merge posture

This closeout is lifecycle/status documentation only. Runtime/component/E2E is `NOT_APPLICABLE`. Before merge require exact changed-path review, governance validation, `git diff --check`, whole-diff self-review, zero unresolved review threads and exact-head repository gates including `game-gate`.

Independent implementation review is `NOT_REQUIRED` because the executable repair itself already received the required independent exact-head review and this closeout introduces no runtime, parser, value, protocol, persistence, security or production authority.

## Context checkpoint

```yaml
last_progress: Content repair #87 and closeout #89 are terminal; next-wave audit verified QA branch-only status, missing Durability DB/migration topology, missing lane-specific Ability/Interaction/AI/Movement resource maxima, and missing production gameplay listener/client-entry seam
status: next_wave_readiness_prepared_allocations_pending
branch: coord/next-wave-readiness-reconcile-20260824
head_sha: pending_final_freeze
pr: null
blocker: next implementation allocations must respect the explicit topology/resource/server-seam gates; production Content Issue #54 remains separately owner-gated
owner_action_required: exact Wave-2 resource hard maxima require accepted evidence/owner decision; production Content limits/activation remain separately owner-gated
next_action: merge this readiness reconciliation, then reconcile QA and prepare separate Durability topology, Content Format Spike and resource-limit decision allocations before releasing generic engine workers
```

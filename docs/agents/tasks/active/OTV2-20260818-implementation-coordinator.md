# OTV2-20260818-implementation-coordinator

```yaml
task_id: OTV2-20260818-implementation-coordinator
title: Coordinate first native implementation wave
mode: COORDINATE
status: wave1_content_lease_transfer_pending_closeout_merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chore/domain-closeout-content-lease-20260824
pr: 82
base_sha: 0facd7f89edc1b0685e67c5531839e8e6f04c466
head_sha: null
owner: chat-github-20260818-implementation-coordinator
created_at: 2026-08-18T16:10:00+02:00
updated_at: 2026-08-24T11:55:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260818-implementation-coordinator.md
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/tasks/archive/OTV2-20260822-impl-domain-core.md
public_contracts:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
depends_on:
  - Oteryn-Game#45
  - Oteryn-Game#46
  - Oteryn-Game#59
  - Oteryn-Game#74
  - Oteryn-Game#76
  - Oteryn-Game#78
  - Oteryn-Game#81
  - Oteryn-Game#56
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Outcome

Coordinate the active native implementation programme, serialize shared composition/registry mutations, integrate only dependency-ready lanes and preserve exact lifecycle/review provenance.

## Current proven state

- `PROVEN`: Bootstrap and SIM are completed/lifecycle-closed.
- `PROVEN`: FOUNDATION implementation is merged and its post-merge independent audit is `PASS` with zero P0/P1/P2, while the historical pre-merge independent exact-head gate is correctly `NOT_PROVEN`; no retroactive compliance is claimed.
- `PROVEN`: DOMAIN PR #56 frozen head `a76c999a2b03c4271fda9b4395cc3d76c346987b` passed full local integration validation, exact-head Merge Gate #329, Agent Governance #373, Merge Authority #248, Ready-state Architecture Audit #273, whole-diff self-review and genuinely independent exact-head review, then squash-merged as `0facd7f89edc1b0685e67c5531839e8e6f04c466`.
- `PROVEN`: Issue #55 is closed/completed and the DOMAIN source branch is absent after merge.
- `PROVEN`: PR #82 adds the terminal DOMAIN archive record and removes its active task lock; those lifecycle changes become canonical only after PR #82 merges.
- `PROVEN`: CONTENT PR #58 remains Draft at `ec68df7a461a011a6480898c9a6d9ee60703189e`, 7 commits ahead / 12 behind DOMAIN-merged main and still changes only `apps/game-server/src/content/**` plus its task record.
- `PROVEN`: accepted DUR-04/VSL production hard maxima remain absent; CONTENT may only remain evidence-only/non-production until those values are separately accepted.
- `PROVEN`: QA still has only its evidence shell and no PR; real Tier 1/Tier 2 gameplay journeys remain `NOT_EVALUATED`.
- `PROVEN`: DURABILITY prerequisites FOUNDATION+DOMAIN now have concrete merged seams, but no DURABILITY write allocation is created by this closeout.

## Coordinator decision

PR #82 prepares the established serialized shared-path handoff from completed DOMAIN to CONTENT.

The transfer is conditional on PR #82 merging. Before merge, DOMAIN remains the canonical shared lease owner on `main`; after merge, CONTENT becomes the one-writer owner for the shared paths below:

```yaml
shared_paths:
  - apps/game-server/src/lib.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
next_owner_after_closeout_merge: OTV2-IMPL-CONTENT
```

CONTENT authority is intentionally narrow: reconcile current main and make only minimum evidence-only/fail-closed composition needed to compile the existing VSL semantic/compiler/loader seam through `apps/game-server`. It may not mutate contracts/registries/workflows/new crate topology, select a permanent world/bundle format, activate production VSL, import broad content or claim Reference parity.

DURABILITY is now dependency-ready in the DAG but remains `write_authority: none`; the coordinator will allocate it separately after this lifecycle transition rather than mixing a new high-risk persistence allocation into the DOMAIN closeout.

## Validation / merge posture

This closeout changes only agent lifecycle/allocation documentation. Runtime/component/E2E is `NOT_APPLICABLE`. Required before merge:

- exact changed-file/diff review;
- `python tools/agents/validate_governance.py` through repository governance gate;
- repository-required exact-head aggregate checks;
- mandatory whole-diff self-review;
- no unresolved review threads.

Independent review is `NOT_REQUIRED` for this closeout because it follows an already-established serialized lease order and does not grant production/security/protocol/durable-value authority or allocate DURABILITY.

## Context checkpoint

```yaml
last_progress: DOMAIN PR #56 merged as 0facd7f89edc1b0685e67c5531839e8e6f04c466 after exact-head CI, self-review and independent review; Issue #55 closed and source branch deleted; PR #82 archives DOMAIN, releases its active lock, corrects Foundation provenance and stages the existing shared-lease handoff to CONTENT.
status: wave1_content_lease_transfer_pending_closeout_merge
branch: chore/domain-closeout-content-lease-20260824
head_sha: null
pr: 82
blocker: PR #82 must pass exact-head repository checks and merge before CONTENT may mutate shared composition paths
owner_action_required: null
next_action: qualify and merge PR #82 if clean; then reconcile CONTENT PR #58 against current main and perform only the authorized evidence-only shared composition.
```

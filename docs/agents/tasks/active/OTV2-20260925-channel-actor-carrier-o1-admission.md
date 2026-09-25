# OTV2-20260925-channel-actor-carrier-o1-admission

```yaml
task_id: OTV2-20260925-channel-actor-carrier-o1-admission
title: Replace ChannelActorCarrier admission scans with O(1) free-list
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/channel-actor-carrier-o1-admission-20260925
issue: 530
parent_coordinator: 162
pr: 899
base_sha: d092fe979e0ff1bff50aab4aa589c3e06bfd21d8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: work-coordinator
created_at: 2026-09-25T00:00:00Z
updated_at: 2026-09-25T15:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - docs/agents/tasks/active/OTV2-20260925-channel-actor-carrier-o1-admission.md
  - docs/agents/evidence/OTV2-20260925-channel-actor-carrier-o1-admission.md
public_contracts: []
depends_on:
  - pr: 898
blocks:
  - issue: 530
cross_repository_coordination_id: null
external_repositories: []
```

- **Allocation authority:** #162 comment 5832603591 after protected integration of #898.
- **Objective:** replace admission-time O(M) vacant-slot selection and one-creature full-carrier scanning with bounded O(1) carrier-local state.
- **Preserve:** exact actor/generation fencing, stale-ref rejection, direct-index lookup, no eviction, owner continuity, #541 generation-exhaustion behavior, rollback semantics, and the current at-most-one-creature fixture.
- **Free-list policy:** deterministic initial 0..M-1 order; removal returns the exact slot to the head, so recycled holes are reused in LIFO removal order.
- **Excluded:** production M, registry mutation, activation, >1-creature support, RL-05 widening, Ability, Movement, Content, protocol, foundation/mod.rs, Cargo/workspace, SQL, workflows or production.
- **RED:** historical head `92a4eff554f94816985b336cd36405deb050260f` was executed through validation-only PR #901; the reuse-order test failed exactly against the old lowest-index scan. PR #901 is closed and must never integrate.
- **GREEN:** PR #899 head `d8409b56fc3303978bdfc1a25192ba4af0519fb2` passed Agent governance, Architecture semantic audit, full Merge Gate, strict Clippy, affected/workspace tests and aggregate `game-gate`. The evidence packet records exact run/job IDs.
- **Current state:** implementation and canonical CI are green. Fresh physical release-mode M=4,096/M=131,072 measurements plus exact post-change Slot-size recapture remain pending because the current execution surface exposes no authorized arbitrary release benchmark for this branch.
- **Integration:** keep PR #899 draft and do not enqueue until physical qualification is completed or #162/#530 explicitly revises that acceptance requirement.

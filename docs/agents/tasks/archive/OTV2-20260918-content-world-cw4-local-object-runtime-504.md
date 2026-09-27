> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #669 merged as `0c703570613db7dedbe9ccafc8c12e4a9c660411`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# CW4 local-object runtime — task evidence

```yaml
task_id: OTV2-20260918-content-world-cw4-local-object-runtime-504
title: Implement first CW4 scope-local object runtime component
mode: IMPLEMENT
status: active
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw4-local-object-runtime-504
issue: 162
pr: 669
base_sha: d251770f6757d1f5e87df9c39c71f91bae8343f9
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world runtime"
created_at: 2026-09-18
updated_at: 2026-09-18
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/world_runtime.rs
  - apps/game-server/src/lib.rs
  - docs/agents/tasks/active/OTV2-20260918-content-world-cw4-local-object-runtime-504.md
public_contracts:
  - OTV2-20260917-content-world-local-transition-contract-candidate.md
depends_on:
  - "#162 comment 5732276491"
  - "#162 comment 5736225915"
  - "#504 comment 5721079000"
  - "PR #667 protected merge/readback d251770f6757d1f5e87df9c39c71f91bae8343f9"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Allocation

- Control plane: `Oteryn/Oteryn-Game#162`.
- Task: `CONTENT_WORLD_CW4_LOCAL_OBJECT_RUNTIME_COMPONENT_504`.
- Worker alias: `Oteryn: content world runtime`.
- Protected base consumed at launch: `main@d251770f6757d1f5e87df9c39c71f91bae8343f9`.
- Branch: `agent/content-world-cw4-local-object-runtime-504`.
- Draft PR: #669.
- Production authority: **NONE**.

Writable custody is limited to:

1. `apps/game-server/src/world_runtime.rs`;
2. `apps/game-server/src/lib.rs`;
3. this task evidence file.

## Launch fence

Immediately before the first tracked write, the isolated worktree was clean and all of:

- worktree HEAD;
- `origin/main`;
- `origin/agent/content-world-cw4-local-object-runtime-504`

resolved to `d251770f6757d1f5e87df9c39c71f91bae8343f9`.

Fresh open-PR census: **29 open PRs, 0 overlaps** with the three allocated paths.

## Proof boundary

This increment is only a synthetic, in-process Rust composition proving the first ordinary,
non-value-bearing, scope-local two-state object seam.

It does **not** claim:

- Reference target parity;
- wire/protocol compatibility;
- native-client integration;
- production deployment authority;
- hot-reload/content-generation migration;
- persistence or durable world-object ownership.

Foundation command lifecycle/order remains Foundation-owned through `CommandIngress`.
CW4 adds no receipt/result store, queue, actor registry, owner service, persistence layer,
protocol ID, schema ID, Cargo/workspace mutation, workflow mutation, or production change.

## RED evidence

Focused CW4/Q3 tests were authored before the implementation in the authorized isolated
worktree.

Command:

`cargo test -p oteryn-game-server world_runtime::tests --no-fail-fast`

Expected RED was observed: compilation failed because the future
`LocalObjectRuntime`, `LocalObjectOperation`, and `LocalObjectCommand` types did not
exist yet.

The local Remote Desktop session later disconnected while an implementation write was in
flight. That uncommitted local partial write was not trusted or promoted. GitHub readback
confirmed the remote branch was still identical to protected main before the implementation
was reconstructed directly on the authorized branch.

## Implemented invariant surface

The crate-private component currently targets these bounded invariants:

- exact Reference-playable profile/capability binding;
- exact Content-generation semantic identity;
- PlacementKey + local-object definition + incarnation binding;
- current active GameSession, connection generation, runtime scope, and scope-ownership fences
  before Foundation ingress or gameplay mutation;
- Foundation-owned duplicate classification and cross-target GameSession terminal order;
- retained A/1 replay without re-executing against later current object state;
- channel-local overlay isolation for the same PlacementKey;
- coherent state/revision/collision contribution publication;
- OPEN removes only the object's own blocker contribution;
- CLOSE validates the whole collision footprint before mutation and rejects occupied multi-cell
  closure atomically;
- binding/revision/capacity/order failures leave gameplay and spatial state unchanged;
- unresolved policy guards fail closed in this first child.

## Validation state

Current hosted exact-head GREEN evidence: **PENDING**.

Required before worker handoff:

- formatter check;
- focused `world_runtime::tests`;
- `content_reference_playable` regression;
- strict game-server Clippy with `-D warnings`;
- exact changed-path readback;
- whole-diff adversarial self-review;
- normal PR CI;
- independent exact-head review.

Do not treat this task as READY_FOR_INTEGRATION until those items are recorded on the final
exact head.

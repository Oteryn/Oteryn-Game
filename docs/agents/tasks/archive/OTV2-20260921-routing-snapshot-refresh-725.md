> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #726 exact candidate `cce8697106e218ad7da4d5fdd18c26842f4cef3d` passed real Merge Queue run `35629569584` with aggregate `game-gate` job `106435015597` SUCCESS and integrated as `5a28b53108b5a9e6c570e6dd3dbb2e9bad0a9555`. Protected `main@5a28b53108b5a9e6c570e6dd3dbb2e9bad0a9555` contains the accepted cleanup chain. Any nonterminal/checkpoint wording below is historical provenance only.

# OTV2-20260921-routing-snapshot-refresh-725

```yaml
task_id: OTV2-20260921-routing-snapshot-refresh-725
title: Refresh reviewed routing snapshot after protected WP3 integration
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/routing-snapshot-refresh-725
issue: 725
pr: 726
base_sha: 04f46b16e3b33c2ebab39edcf7e693a0a72c6301
head_sha: cce8697106e218ad7da4d5fdd18c26842f4cef3d
final_head_sha: cce8697106e218ad7da4d5fdd18c26842f4cef3d
final_head_frozen_at: null
owner: ChatGPT
created_at: 2026-09-21T15:18:17Z
updated_at: 2026-09-21T15:18:17Z
execution_policy: continuous_progress
owned_paths:
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_pr_test_lanes.py
  - docs/agents/tasks/active/OTV2-20260921-routing-snapshot-refresh-725.md
public_contracts:
  - trusted non-server routing snapshot
depends_on:
  - pr:673
  - pr:718
blocks:
  - pr:724
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Repair the reviewed non-server input model after protected WP3 added path-patched dependencies outside workspace membership, then adopt the resulting reviewed fingerprint. Preserve every predecessor fingerprint as a stale fail-closed fixture.

## Proven input audit

- Protected admission: `main@04f46b16e3b33c2ebab39edcf7e693a0a72c6301`.
- Current declared snapshot: `962dfe6c3c9fbe102a08b1040e3880589b6ee4cda52a4910feae3103225099fe`.
- Pre-repair protected-tree digest from routing validator: `6e6f7a9dafe5020cbfda968d49b97c47072c7ee471668d3d9e0cdc491a4af2f4`.
- Independent review P1 proved that digest omitted root `[patch.*]` path trees outside `workspace_members`.
- Repair makes root-manifest patched dependency trees audited inputs; a focused regression proves `vendor/tokio-1.53.1/src/lib.rs` mutation changes the digest.
- Post-repair RED routing run `35619359313` computed the new exact digest `118afae45f3c8fd7692e2e61ffd286e4efc3d4bfc8290304b999f0bbc3ca29ba`.
- Protected PR #673 changed root audited build inputs:
  - `Cargo.toml`: SQLx TLS ring -> AWS-LC, exact path patches for vendored SQLx core/postgres and Tokio, vendor packages excluded from workspace membership;
  - `Cargo.lock`: protected path-patched resolution for those exact versions.
- #673 reached protected main through real Merge Queue before this refresh.
- PR #718 changed routing model/tests but did not itself create the non-server tree digest; it exposed the inherited stale snapshot in #724.

## Acceptance

- [x] P1 repaired: root `[patch.*]` path dependencies outside workspace membership are hashed as audited inputs.
- [x] Focused regression proves current vendored SQLx/Tokio patch roots are audited and vendor-tree mutation changes the digest.
- [x] Predecessor snapshots `962d...` and `6e6f...` remain stale/fail-closed fixtures.
- [x] No workflow/Cargo/runtime/protection/Merge Queue primitive change.
- [ ] Exact-head routing contract reports `ROUTING_CONTRACT_HEALTHY snapshot=118afae4...`.
- [ ] Classifier/post-merge/repository/governance regressions pass.
- [ ] Exact-head hosted Merge Gate and aggregate `game-gate` pass.
- [ ] Normal Merge Queue integration and protected-main readback complete.

## Excluded scope

No product source, dependency, manifest/lock, workflow, protection, queue primitive, prompt or lifecycle semantics change. Classifier behavior changes only to include root path-patched dependency trees in audited-input hashing.

## Context checkpoint

```yaml
last_progress: >-
  independent review P1 repaired by hashing root path-patched dependency trees; RED routing contract
  computed final reviewed digest 118afae45f3c8fd7692e2e61ffd286e4efc3d4bfc8290304b999f0bbc3ca29ba
status: validating
branch: ci/routing-snapshot-refresh-725
head_sha: null
pr: null
next_action: freeze this final repaired head and require fresh routing-health, full CI, and independent re-review
```


## Terminal closeout

- issue: #725
- delivery PR: #726
- frozen exact head: `cce8697106e218ad7da4d5fdd18c26842f4cef3d`
- real Merge Queue run: `35629569584`
- merge-group aggregate `game-gate`: job `106435015597` — SUCCESS
- integrated commit: `5a28b53108b5a9e6c570e6dd3dbb2e9bad0a9555`
- final protected-main readback for this cleanup chain: `5a28b53108b5a9e6c570e6dd3dbb2e9bad0a9555`
- ownership: released

# OTV2-20260921-routing-snapshot-refresh-725

```yaml
task_id: OTV2-20260921-routing-snapshot-refresh-725
title: Refresh reviewed routing snapshot after protected WP3 integration
mode: REPAIR
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/routing-snapshot-refresh-725
issue: 725
pr: null
base_sha: 04f46b16e3b33c2ebab39edcf7e693a0a72c6301
head_sha: null
final_head_sha: null
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

Refresh only the reviewed non-server input fingerprint after protected WP3 changed audited workspace/build inputs. Preserve the previous fingerprint as a stale fail-closed fixture. Do not change classifier behavior.

## Proven input audit

- Protected admission: `main@04f46b16e3b33c2ebab39edcf7e693a0a72c6301`.
- Current declared snapshot: `962dfe6c3c9fbe102a08b1040e3880589b6ee4cda52a4910feae3103225099fe`.
- Current protected-tree digest from routing validator: `6e6f7a9dafe5020cbfda968d49b97c47072c7ee471668d3d9e0cdc491a4af2f4`.
- Protected PR #673 changed root audited build inputs:
  - `Cargo.toml`: SQLx TLS ring -> AWS-LC, exact path patches for vendored SQLx core/postgres and Tokio, vendor packages excluded from workspace membership;
  - `Cargo.lock`: protected path-patched resolution for those exact versions.
- #673 reached protected main through real Merge Queue before this refresh.
- PR #718 changed routing model/tests but did not itself create the non-server tree digest; it exposed the inherited stale snapshot in #724.

## Acceptance

- [x] Scope is limited to current fingerprint, predecessor stale fixture and this task record.
- [x] No classifier algorithm/workflow/Cargo/runtime/protection change.
- [ ] Exact-head routing contract reports `ROUTING_CONTRACT_HEALTHY snapshot=6e6f7a9d...`.
- [ ] Classifier/post-merge/repository/governance regressions pass.
- [ ] Exact-head hosted Merge Gate and aggregate `game-gate` pass.
- [ ] Normal Merge Queue integration and protected-main readback complete.

## Excluded scope

No product source, dependency, manifest/lock, workflow, protection, queue primitive, prompt or lifecycle semantics change.

## Context checkpoint

```yaml
last_progress: protected-tree drift independently identified and bounded to a reviewed snapshot refresh
status: validating
branch: ci/routing-snapshot-refresh-725
head_sha: null
pr: null
next_action: publish exact three-path candidate and require fresh full routing/control-plane qualification
```

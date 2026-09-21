> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #689 merged as `c22eb6cb5e953b7fa59dea7bd5d50114a10e8e7d`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260920-content-world-routing-snapshot-refresh-504

```yaml
task_id: OTV2-20260920-content-world-routing-snapshot-refresh-504
title: Refresh the reviewed non-server routing snapshot after PR 687
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/content-world-routing-snapshot-refresh-504
issue: 283
pr: null
base_sha: 037bf7818122ad7d2bb10ad47f89fdd90d2a35dc
head_sha: pending
final_head_sha: pending
final_head_frozen_at: pending
owner: Codex
created_at: 2026-09-20T00:00:00Z
updated_at: 2026-09-20T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_pr_test_lanes.py
  - docs/agents/tasks/active/OTV2-20260920-content-world-routing-snapshot-refresh-504.md
public_contracts: []
depends_on:
  - "PR #687 / protected main 037bf7818122ad7d2bb10ad47f89fdd90d2a35dc"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Adopt the independently recomputed reviewed non-server input fingerprint after PR #687 added Linux-only filesystem dependencies to the game server. Preserve the preceding fingerprint as a stale regression fixture so future drift continues to fail closed.

## Input audit

- **PROVEN** — protected base `037bf7818122ad7d2bb10ad47f89fdd90d2a35dc` recomputes to `962dfe6c3c9fbe102a08b1040e3880589b6ee4cda52a4910feae3103225099fe` with pinned Cargo 1.94 metadata.
- **PROVEN** — the accepted PR #687 manifest delta adds `cap-fs-ext`, `cap-std` and `rustix` only to `oteryn-game-server` under `cfg(target_os = "linux")`.
- **PROVEN** — normalized metadata leaves `oteryn-client`, `oteryn-simulation-determinism` and `oteryn-synthetic-client-harness` dependency lists unchanged; the added local reverse closure contains only `oteryn-game-server` and has no Windows consumer intersection.
- **PROVEN** — predecessor `c04bf8e0e010366170d76f23baa34b0abb2f69a9114157b0a6c6ebb880294594` remains a stale fixture and must select FULL.
- **PROVEN** — PR #687 received independent review PASS in comment `5746162634` before protected integration.

## Acceptance criteria

- [x] Update only the reviewed `AUDITED_INPUT_SHA256` fingerprint.
- [x] Preserve the predecessor fingerprint as a stale regression input.
- [x] Keep classifier behavior, fail-closed fallbacks, workflow predicates and Merge Queue invariants unchanged.
- [x] Focused classifier and candidate routing-contract validation pass.
- [x] Governance and repository-policy validation pass.
- [ ] Exact-head hosted Merge Gate and aggregate `game-gate` pass.

## Excluded scope

No workflow, gate, queue, Cargo, lockfile, product source, authority, model or unrelated task changes.

## Validation

- `python -I tools/repository/test_classify_pr_test_lanes.py` — PASS.
- candidate-environment `python -I tools/repository/validate_pr_routing_contract.py` — PASS, `ROUTING_CONTRACT_HEALTHY` at `962dfe6c3c9fbe102a08b1040e3880589b6ee4cda52a4910feae3103225099fe`.
- `python tools/agents/validate_governance.py` — PASS, 26 required policy documents and 9 project lanes.
- `python tools/repository/validate_repository_policy.py` — PASS, 23 files and 18 workflows.
- `git diff --check` — PASS.
- exact-head hosted Merge Gate / `game-gate` — pending.

## Review and integration

- Author self-review: pending immutable head.
- Independent review: coordinator-owned; this author cannot independently review its own change.
- Integration: coordinator-owned through the normal protected path.

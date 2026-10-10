# OTV2-20261007-game-preprod-topology-1

```yaml
task_id: OTV2-20261007-game-preprod-topology-1
title: GAME-PREPROD-TOPOLOGY-1 manifest, runbook and deploy check
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/game-preprod-topology-1-20261007
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: GAME-PREPROD-TOPOLOGY-1 writer (allocated by control plane session_0114oBVR3osF1auvFMu6ksMH, issue 1622)
created_at: 2026-10-07T00:00:00Z
updated_at: 2026-10-07T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - deploy/synology-game/preprod-topology.toml
  - deploy/synology-game/README.md
  - deploy/synology-game/deploy-ops.sh
  - deploy/synology-game/test-preprod-topology.sh
  - docs/agents/tasks/OTV2-20261007-game-preprod-topology-1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The repository carries the non-secret preproduction topology manifest, the Game operator runbook and a fail-closed deploy check. No deploy and no topology issuance is performed.

## Architecture and source of truth

- `docs/architecture/reviews/OTERYN_GAME_ARCH_PLATFORM_PREPROD_TOPOLOGY_2026-10-06.md` sections 0, 1, 4, 6, 7, 8, 10, 12 (merged via #1893). PROVEN.
- `NodeConfig::parse` requires distinct readiness and platform `source_authority`; the manifest uses `oteryn:runtime:synology-preprod` and `platform`. DERIVED from the packet.
- The check lives in `deploy-ops.sh check-setup`, which the workflow already runs as root before any install or migration. The manifest is read from the root copy `ROOT_BASE/ops/preprod-topology.toml`; the bundle does not carry it. DERIVED.

## Acceptance criteria

- [x] The deploy refuses on any mismatch, a placeholder, epoch 0 and WorldId equal to ChannelId (`test-preprod-topology.sh`).
- [x] The manifest has no key, no certificate and no non-private address (placeholders and subjects only).

## Excluded scope

No workflow change (so the new test is not yet wired into the build job), no Platform change, no deploy, no secret, no topology issuance.

## Implementation / findings

- `preprod-topology.toml`: placeholders until rollout step 6; epoch starts at 1.
- `deploy-ops.sh`: `topology_check` compares scope.env, node.toml (`[scope]`, listener port, `[readiness]`, `[platform]`, runtime-status epoch) and report.toml (endpoint, peer, epoch, scope ids) with the manifest; sourcing the script defines the functions only, for the test.
- README: Topology manifest section, Game operator runbook, epoch raise, NAS values from the manifest.

## Validation

### Focused

- `bash deploy/synology-game/test-preprod-topology.sh`: pass
- `shellcheck deploy/synology-game/*.sh`: only the pre-existing info finding in `supervisor.sh` (SC2015); none in the changed files
- `git diff --check`: pass
- `python tools/agents/validate_governance.py`: pass

### Component/integration

- NOT_APPLICABLE: no deploy is run.

### E2E

- NOT_APPLICABLE: no deploy is run.

### Exact-head CI

- final head: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent
- verdict: pending

## Independent review

- required: pending (control plane decides)

## PR and closeout

- PR: opened by the control plane
- ownership release: pending

## Context checkpoint

```yaml
last_progress: manifest, wrapper check, test and README written
status: implementing
branch: claude/game-preprod-topology-1-20261007
head_sha: null
pr: null
final_head_sha: null
next_action: run validators, commit, push, report FREEZE to the control plane
```

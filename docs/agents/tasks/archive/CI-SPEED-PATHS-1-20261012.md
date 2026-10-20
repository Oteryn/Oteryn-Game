# CI-SPEED-PATHS-1-20261012

```yaml
task_id: CI-SPEED-PATHS-1
title: "CI-SPEED-PATHS-1: skip PR Rust lanes for Content-only and deploy-only changes"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ci-speed-paths-20261012
pr: 2022
head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
final_head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
owner: claude-code-session_01CwP6d84eCPvpgoEuyci8Tx (implementation worker)
created_at: 2026-10-12
updated_at: 2026-10-12
owned_paths:
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_content_routing.py
  - docs/agents/tasks/archive/CI-SPEED-PATHS-1-20261012.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

**Content deferral in the PR gate.** When the only product-consumed inputs of a pull request are Content data files under `content/`, the trusted-base classifier routes `rust=false` and `windows=false`, with reason `content-consumers-deferred-to-merge-queue`. The World bundle pin-check and Server qualification selection are computed separately and are unchanged.

The Merge Queue calls `classify()` without the deferral flag, so the exact composed candidate keeps its full qualification. The classifier never defers:
- Rust and build files
- workflows and repository controls
- canonical-control consumers
- unknown consumers
- mixed Rust changes

**`deploy/` routing.** `deploy/` becomes a reviewed auxiliary root. Unconsumed deploy files skip Rust. `preprod-topology.toml`, which `oteryn-world-bundle-compiler` consumes, still routes FULL.

No workflow file changes, so the merge-authority audit pins stay valid.

## Evidence

A Content-only PR spent about 20 min in the PR changed-crate Rust job. No crate owns `content/`, so the job ran workspace clippy and unit tests.

Routing on real workspace metadata:

| Change | Before | After |
|---|---|---|
| `content/world/pins/*` | FULL | Rust skipped in the PR, World bundle still runs |
| `content/items/definitions/*` | FULL | Rust skipped in the PR, World bundle still runs |
| `deploy/synology-game/deploy-ops.sh` | FULL | skipped |
| Content plus Rust, `Cargo.lock` or workflow | FULL | FULL |

## Validation

python tools/repository/validate_repository_policy.py: pass
python tools/repository/test_*.py (13 files, PG-simulation tests with local pwsh 7.4.6): pass
python tools/repository/validate_pr_routing_contract.py --protected-main (real metadata): pass
git diff --check: pass
python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK

## Review and closeout

Accepted risk: a Content change that breaks a game-server test is caught by the Merge Queue on the exact candidate, not in the PR. G4 qualify still runs the focused WorldProject server tests in PRs.

Merge result: squash merge of #2022, pending CI, review and Merge Queue at authoring.

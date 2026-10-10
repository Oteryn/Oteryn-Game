# SYNOLOGY-GAME-DEPLOY-1-20261006

```yaml
task_id: SYNOLOGY-GAME-DEPLOY-1
title: "SYNOLOGY-GAME-DEPLOY-1: deploy the native Game server to the Synology preproduction environment"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/synology-game-deploy-1
issue: 1622
pr: 1874
head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
final_head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
owner: claude-code implementation writer (control plane session_013KJX6mv8LQveCKKXYgAX94, decision D832)
created_at: 2026-10-06
updated_at: 2026-10-06
owned_paths:
  - .github/workflows/synology-game-runner-acceptance.yml
  - .github/workflows/synology-game-deploy.yml
  - deploy/synology-game/**
  - docs/agents/tasks/archive/SYNOLOGY-GAME-DEPLOY-1-20261006.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

`preproduction` is the only environment. The acceptance workflow no longer mentions Canary and keeps the runner identity and least-privilege checks (no docker.sock, no Docker control).

New `synology-game-deploy.yml`: `workflow_dispatch` only, refuses any ref but `main`, GitHub environment `preproduction` is the human gate. A GitHub-hosted job builds the three Linux x86_64 binaries (never on the NAS) and stages the hash-bound gameplay inputs. The `game-runners`/`oteryn-game` job verifies runner `oteryn-synology-game`, installs under `/volume1/oteryn/game-preprod/bin`, runs `oteryn-game-migrate` with a database URL read from a NAS file, issues the launch authorization with `--supersedes <previous node id>` as `node_boot/run.sh` does, restarts the one service through a user-level pid-file supervisor and health-checks it.

`deploy/synology-game/` holds the operator README, `ops.toml`/`node.toml` templates (placeholders, no secrets), the supervisor and the staging script.

## Excluded scope

No gate workflow, `merge-authority-audit.yml` or pinned gate blob was touched. No secrets or new hostnames. No Canary. The one-time operator setup (PostgreSQL container, first authorization, S2, assignment) stays manual. The deploy has not run against the real NAS.

## Validation

actionlint: not available in the environment; both workflows parse as YAML
bash -n deploy/synology-game/supervisor.sh: pass
python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK
git diff --check: pass

## Review and closeout

Repair after Codex review 5427111614: the bundle checksum now covers every file; `oteryn-game-ops` runs as root from `/volume1/oteryn/game-preprod-root` through one `sudo -n` rule, and the authorization is issued before the service is stopped. README gained the NAS values table and the first-start sequence.

Full automation (CP): later deploys assign the scope with `assignment replace` and wait for ready, with no manual root step. Both root actions go through the root-owned `oteryn-game-deploy-ops` wrapper (strict positional validation, fixed binary/config/scope) behind a two-subcommand sudoers rule. Not run on the real NAS.

Review is decided by the control plane on the frozen head. Merge result: squash merge of #1874, pending CI and Merge Queue at authoring.

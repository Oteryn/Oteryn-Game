# OTV2-20261011-deploy-stage-world-1

```yaml
task_id: OTV2-20261011-deploy-stage-world-1
title: DEPLOY-STAGE-WORLD-1 preprod deploy verifies and stages the pinned World Bundle
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/deploy-stage-world-1-20261011
pr: 1995
base_sha: 227401d4e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: DEPLOY-STAGE-WORLD-1 writer
created_at: 2026-10-11T00:00:00Z
updated_at: 2026-10-11T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - deploy/synology-game/deploy-ops.sh
  - deploy/synology-game/README.md
  - deploy/synology-game/test-preprod-topology.sh
  - .github/workflows/synology-game-deploy.yml
  - docs/agents/tasks/archive/OTV2-20261011-deploy-stage-world-1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The preprod deploy can stage the pinned World Bundle (opt-in, default off) and check-setup refuses an enabled World whose bundle file is missing or not the pinned bytes.

## Architecture and source of truth

- PROVEN: the node boots the pinned World only with `[world_bundle] enabled = true`; BASE is root-owned, so only a root wrapper subcommand can write `BASE/world/world.otbundle`.
- PROVEN: the bundle digest is `sha256(FORMAT, 0x00, body)` over all bytes but the 32-byte trailer (`crates/world-bundle/src/bundle.rs`), so the shell can verify it without the compiler.

## Acceptance criteria

- [x] `world_bundle_check` fails closed with `world_bundle_missing`, `world_bundle_wrong_owner_or_mode`, `world_bundle_digest_mismatch`.
- [x] `stage-world` copies the runner file once, verifies the copy against the root manifest digest, installs 1001/0644 atomically, refuses on mismatch.
- [x] Before any temp file, `stage-world` creates or verifies `BASE` and every directory down to the target as root-owned, not group/other writable, not a symlink and not writable by the runner (probed as uid 1001, covering ACLs); it refuses with `world_bundle_unsafe_dir` otherwise, reads the incoming file with `nofollow` and renames with `mv -T` (Codex P1 4243554362).
- [x] Workflow input `stage_world_bundle` default false; default deploy unchanged.
- [x] README and owner one-time ops (sudoers entry, wrapper reinstall); no secrets.
- [x] `[world_bundle] enabled`, the pin digest and topology digest/start fields untouched.

## Excluded scope

Flipping `enabled`, the pin and topology digest/start values (PRs #1983, #1993, MAP-START-1), Platform.

## Validation

- `bash deploy/synology-game/test-preprod-topology.sh`: pass
- `bash deploy/synology-game/test-supervisor-gap.sh`: pass
- `bash -n deploy/synology-game/deploy-ops.sh`: pass
- workflow YAML parse: pass
- `bash deploy/synology-game/test-preprod-topology.sh` as root and as an unprivileged user (runner-owned, group/world-writable, runner-ACL-writable, symlinked world dir, symlinked ancestor and BASE refusals): pass
- `shellcheck -x deploy-ops.sh test-preprod-topology.sh`: pass
- `actionlint`: NOT_APPLICABLE, not installed
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

## Self-review

Whole diff re-read; no secrets; the wrapper takes no path argument, verifies its own copy, and writes only inside directories the runner cannot change. Residual: directories above `BASE` are not checked by the wrapper; the workflow checks `BASE` itself.

## PR and closeout

- merge commit/result: squash merge of #1995 (pending)
- review: pending

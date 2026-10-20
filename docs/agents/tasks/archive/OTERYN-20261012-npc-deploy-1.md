# OTERYN-20261012-npc-deploy-1

```yaml
task_id: OTERYN-20261012-npc-deploy-1
title: NPC-DEPLOY-1 preprod deploy stages the pinned NPC catalogue and starts the node with it
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-deploy-1-20261012
pr: 2011
base_sha: e128fd6cc
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: NPC-DEPLOY-1 writer
created_at: 2026-10-12T00:00:00Z
updated_at: 2026-10-12T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - deploy/synology-game/
  - .github/workflows/synology-game-deploy.yml
  - docs/agents/tasks/archive/OTERYN-20261012-npc-deploy-1.md
public_contracts: []
depends_on:
  - 2010
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The Synology preproduction deploy installs the pinned NPC catalogue and `supervisor.sh` starts the node with `--npc-data-project` and `--npc-data-sha256`, so a World Bundle's NPCs (NPC-LOAD-1, #2010) become actors on preprod instead of `event=npc_actors state=skipped reason=no_catalogue`. Owner decision: option (a), the deploy packet carries the content.

## Architecture and source of truth

- PROVEN: `--npc-data-project` names the WorldProject root (`content/world`); the node captures only `project.json`, `manifest.json`, `content.lock.json` and the eight manifest documents, and hashes the project root's parent for `catalogue_sha256` over `npcs/definitions`, `dialogues/definitions` and `services` (`npc_catalogue.rs`, `world-bundle/src/npc.rs`, `serve/bundle_npcs.rs`).
- PROVEN: the digest in `npc-data.pin.json` equals the `npcs.catalogue_sha256` inside the published `world-bundle-5c32d7ac...` artifact (compared by reading the bundle's manifest).
- The pin is verified at build (`stage_npc_data.py`), at install (tree digest against `SHA256SUMS`-verified bundle) and by the node at every boot.

## Acceptance criteria

- [x] Release bundle carries `npc-data/` (project documents and catalogue only), covered by `SHA256SUMS`; the build fails on any difference from `npc-data.pin.json`.
- [x] Deploy installs it to `BASE/run/npc-data` (runner-owned; no sudoers or wrapper change) and verifies the installed tree; input `stage_npc_data` (default on) off removes it.
- [x] `supervisor.sh` passes both flags exactly when staged and refuses to start on a damaged staging directory or pin.
- [x] Deploy fails when a booted World Bundle does not log `event=npc_actors state=placed`.
- [x] Tests for staging, pin drift and the supervisor; README and `node.toml.template` note; no secrets.

## Excluded scope

Rust changes, the World Bundle compiler and pin, `deploy-ops.sh` and the sudoers rule (not needed), NPC talk and services activation, flipping `[world_bundle] enabled`.

## Validation

- `python3 deploy/synology-game/test_stage_npc_data.py`: pass (17 tests; the 4 repository-bound tests also run against a checkout of the exact main content)
- `bash deploy/synology-game/test-supervisor-npc.sh`: pass
- `bash deploy/synology-game/test-supervisor-gap.sh`: pass
- `bash -n` on `supervisor.sh` and the new test; workflow YAML parse: pass
- `shellcheck`, `actionlint`: NOT_APPLICABLE, not installed
- `python tools/agents/validate_governance.py`: pass; `unittest discover -s tools/agents/tests`: 59 of 60 (1 error is a missing `experiments/` path in this sparse checkout, unrelated)
- Real staging of the pinned tree: 27 files, 71 MB, pin matches

## Self-review

Whole diff re-read; no secrets. Residual: `BASE/run/npc-data` is runner-writable, unlike the root-staged World Bundle, so integrity rests on the node's pinned-digest check at boot (it exits on mismatch); a pin refresh is a one-line PR when content or the World pin changes.

## PR and closeout

- merge commit/result: pending
- review: pending

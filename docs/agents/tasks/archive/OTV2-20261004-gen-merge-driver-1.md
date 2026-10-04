# OTV2-20261004-gen-merge-driver-1

```yaml
task_id: OTV2-20261004-gen-merge-driver-1
title: Merge driver for generated files
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gen-merge-driver-1-20261004
pr: 1779
base_sha: ba8b8df
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn work control plane worker (D561 2a)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - .gitattributes
  - tools/merge-driver/**
  - docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md  # one paragraph after the derived-content paragraph
  - docs/agents/tasks/archive/OTV2-20261004-gen-merge-driver-1.md
public_contracts: []
depends_on: []
blocks: []
```

## Outcome

A local `git merge origin/main` no longer stops on derived content files; `regen.sh` then regenerates them. CI freshness checks are unchanged and still catch a stale file.

## Evidence

`DERIVED`: replaying the 40 newest merges on `codex/spells-import-r22-20261002` (PR #1534) with `git merge-tree` shows the most frequent conflicts in generated files: `content/{items,creatures/definitions,abilities/{definitions,effects,formulas},presentations/definitions}/index.json` 5 each, `content/manifest.json` 1. Hand-written Rust and tool sources conflicted too (`gameplay_transport/mod.rs` 6, `durability/mod.rs` 4) and are not covered. `rulesets/progression/wheel-of-destiny/index.json` and `content/interactions/index.json` are hand-maintained and excluded.

Covered: the `index.json` files listed in `content/manifest.json` `managed_files`, the content registry (`project.json`, `manifest.json`, `content.lock.json`) and the `content/world` documents pinned in `content_world_project_repository.rs`. The generated chunk files and the Rust pin test are not covered; `regenerate_content.py --resolve` already handles the pin test by masked merge.

## Design

- `.gitattributes`: `merge=oteryn-regen` on those paths.
- `tools/merge-driver/oteryn_regen_driver.py`: keeps a clean three-way merge; on conflict takes the incoming side and exits 0.
- `tools/merge-driver/install.sh`: registers the driver in local git config (once per clone).
- `tools/merge-driver/regen.sh`: wrapper for `regenerate_content.py --resolve`.
- Fail-closed: the driver asserts nothing about freshness; skipping `regen.sh` leaves a stale file that the CI freshness checks reject. GitHub server-side merges ignore the driver. No workflow changes.

## Validation

Scratch-clone replay of merge `5e118629` of PR #1534: without the driver `content/manifest.json` conflicts; with it no conflict remains, and `regen.sh` reproduces the committed merge's `content`, `rulesets` and `apps` trees byte-identically; every `--check` freshness tool passes. `item_key_references.py` reports 610 errors identically on the historical merge commit itself (pre-existing, unrelated).

python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: pass

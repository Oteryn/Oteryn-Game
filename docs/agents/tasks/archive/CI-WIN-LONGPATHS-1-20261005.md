# CI-WIN-LONGPATHS-1-20261005

```yaml
task_id: CI-WIN-LONGPATHS-1
title: "CI-WIN-LONGPATHS-1: flatten solo item recovery paths under Windows MAX_PATH (P0)"
mode: DATA
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ci-win-longpaths-1-20261005
issue: 1622
pr: 1854
head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
final_head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
owner: claude-code-session_01GycLXrnULeDDFoVtNJwq5f (implementation worker, control plane #1622)
created_at: 2026-10-05
updated_at: 2026-10-05
owned_paths:
  - imports/ots-native-admission/solo-source-item-recovery-20261005/**
  - tools/content-migration/verify_solo_item_source_recovery.py
  - tools/content-migration/test_solo_item_source_recovery.py
  - docs/agents/tasks/archive/CI-WIN-LONGPATHS-1-20261005.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

P0: after #1826 (main `aa30141f`) the Windows checkout failed with `Filename too long` (MQ run 37380576270). The first packet (enable `core.longpaths` in the Windows jobs) was blocked because `merge-group-gate.yml` and `merge-gate.yml` are blob-pinned by the protected merge-authority audit. The control plane chose option B: a data fix.

Recovery archives moved from `recovery/files/imports/ots-native-admission/<dir>/` to `recovery/files/<dir>/`. The redundant nesting carries no information because the manifest `path` field holds the logical path. That alone left 22 paths over 200 characters (max 229), so `local-source-observation-checkpoint-20261005` is stored as `recovery/files/checkpoint/`. The archives were moved with `git mv` and every sha256 is unchanged. The `.gz.gz` archives are gzips of `.gz` originals per the manifest and keep their names. Only the manifest `archive` fields, the inventory `path` fields and the inventory sha256/bytes row for `recovery-manifest.json` changed. The repository has no producer tool for these manifests, so they were rewritten by a deterministic script with the same `json.dumps(indent=2)` formatting. The verifier now rejects any inventory member whose repo path exceeds 200 characters.

The repo references to the moved paths were only the two package manifests. No file outside this package exceeds 200 characters.

## Validation

python tools/content-migration/verify_solo_item_source_recovery.py: pass
python -m unittest test_solo_item_source_recovery (in tools/content-migration): OK
git ls-files | awk 'length($0) > 200': empty (max 195)
ruff check and ruff format --check on both recovery scripts: pass
git diff --check: pass
python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK

## Review and closeout

Review is decided by the control plane on the frozen head. Merge result: squash merge of #1854, pending CI and Merge Queue at authoring.

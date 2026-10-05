# OTV2-20261005-item-fx-test-fix-1

```yaml
task_id: OTV2-20261005-item-fx-test-fix-1
title: "OTV2-20261005-item-fx-test-fix-1 ITEM-FX-TEST-FIX-1 derive parent batch counts in raw FX/audio reimport test"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/item-fx-test-fix-1-20261005
pr: PENDING
base_sha: 9cb66e3
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
control_plane: session_013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-05
updated_at: 2026-10-05
owned_paths:
  - tools/content-schema/item-authoring/test_raw_fx_audio_reimport.py
  - docs/agents/tasks/archive/OTV2-20261005-item-fx-test-fix-1.md
```

## Defect

`test_actual_parent_all_thirteen_batches_are_preserved` hardcoded the live
`content/world/provenance/imports.json` at 13 parent batches and 104 states
(14 and 400 merged). Later admitted imports grew the file, so the test failed
with `26 != 13` and turned every item-authoring PR red.

## Fix

The test is renamed `test_actual_parent_batches_are_preserved` and derives the
parent batch and state counts from the live parent. Kept invariants:
`append_batch` does not mutate the parent, every parent batch is preserved
byte-canonically, and the merged document has parent + 1 batches and parent
states + 296. Producer, `imports.json` and the workflow are unchanged.

# OTV2-20260922-content-world-item-classification-crosswalk-504

```yaml
task_id: OTV2-20260922-content-world-item-classification-crosswalk-504
title: D6-M1 Item Classification Crosswalk
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-item-classification-crosswalk-504
pr: pending
base_sha: 9ddd6e020837bd43ec66ce7d78d2af7742afeb39
head_sha: pending
owner: "Oteryn: item classification lead"
owned_paths:
  - tools/reference-world-corridor-census/item_classification_crosswalk.py
  - tools/reference-world-corridor-census/item_classification_crosswalk_self_test.py
  - docs/agents/evidence/OTV2-20260922-content-world-item-classification-crosswalk.json
  - docs/agents/tasks/active/OTV2-20260922-content-world-item-classification-crosswalk-504.md
```

## Scope

Build a deterministic, bounded crosswalk over the protected 38,157-Item native identity closure. Reuse the protected B1 catalogue, 64 binding map and family-scale registry without importing source XML or allocating a second identity family. Classify source observations separately from accepted Reference capabilities and preserve absence as `UNKNOWN`, with current-source status `NOT_EVALUATED` unless a pinned current source is explicitly supplied.

The crosswalk outcomes are `EXACT_ONE`, `ZERO_MATCH`, `AMBIGUOUS` and `CONFLICT`. Display names and source labels never resolve identity. This task publishes evidence only: it does not promote OTS gameplay values or change runtime, client, registry, protocol or persistence behavior.

Source-policy direction is inherited from Oteryn/Oteryn-Game#504 comments `5771546143`, `5771561550`, `5773340420`; no history rediscovery is performed.

## Validation

- Python self-test covers deterministic output, duplicate/missing/remap rejection, crosswalk partition states, orthogonal capability states and absent current-source handling.
- Full protected 38,157-record compile reproduces the family allocation digest, emits every native identity exactly once, preserves all 64 semantic bindings and produces byte-identical repeated output.
- Committed evidence is a compact manifest of canonical full-record output digests and counts; it does not hand-author or duplicate the 38,157-row corpus.
- Repository CI is required before qualification. No exact-head review, freeze, paid review or Merge Queue action is part of this implementation handoff.

## Status

Implementation and local validation pending. Draft PR is opened before candidate validation.

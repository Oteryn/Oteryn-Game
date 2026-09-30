# OTV2-20260930-item-sem-2a-fixup

```yaml
task_id: OTV2-20260930-item-sem-2a-fixup
title: ITEM-SEM-2a key rule and attribution for the TibiaWiki item stat snapshot (P0, broken main)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-sem-2a-fixup
issue: 162
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 9ca2bfd2
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
owner: "implementation worker allocated by the work coordinator (control plane session_01EfiFA9LMuUuzoNkizLfR2R)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - tools/content-census/item_wiki_stats_capture.py
  - tools/content-census/item_wiki_stats_capture_self_test.py
  - imports/tibiawiki/facts/items-stats.json
  - imports/tibiawiki/sources.json
  - imports/tibiawiki/batches.json
  - docs/agents/tasks/archive/OTV2-20260930-item-sem-2-item-stats.md   # moved from tasks/active
  - docs/agents/tasks/archive/OTV2-20260930-item-sem-2a-fixup.md
public_contracts: []
jira: null   # sync pending (coordinator batch)
```

## Why

#1324 (ITEM-SEM-2a) merged as `9ca2bfd2` before its review findings (#1324 comment 5908347499) were fixed. Every
snapshot record was keyed `oteryn:item.tibia.i<id>`, but 991 of the 13,826 ids have neither an Item record nor a Crystal
binding, so `item_key_references.py` reported 991 `DANGLING_KEY` errors and "G4 Item Crystal identity bindings" was red
on `main`.

## Outcome

- HIGH (key rule): a record is keyed `oteryn:item.tibia.i<id>` only when that key is an Item record or a Crystal
  binding target; the other 991 ids are keyed by the bare decimal Tibia id. No evidence is dropped. The committed
  snapshot was re-derived offline (`item_wiki_stats_capture.py --rekey`): every record and observation (page id,
  revision id, revision sha1, content digest) and `captured_at` are unchanged; only keys and the digest changed.
  `snapshot_sha256` `2c5d20b5…6db` -> `33a1bf7f…c6d`, re-pinned in `sources.json` and `batches.json` together with the
  capture tool digest (`mapper_sha256`, `mapper_revision` `item-wiki-stats-r2`).
- LOW (attribution): the snapshot `source.attribution` names the TibiaWiki contributors, CC BY-SA 3.0 and the per-page
  history URL `https://tibia.fandom.com/index.php?curid=<page_id>&oldid=<revision_id>`, as in the achievements README.
- Accepted limitation: there is no offline `--check` that regenerates the 9.8 MB snapshot from raw pages; the raw page
  fetch is not committed. The snapshot is verifiable through the pinned page and revision ids (re-capture with
  `--cache`); the self-test checks the digest, key rule, allowlist and attribution offline.
- Not touched: `item_key_references.py` and every checker, `.github/workflows/` (the #1324 workflow change is handled
  by the control plane with the owner).

## Validation

- `python tools/content-census/item_key_references.py`: PASS (0 errors).
- `item_wiki_stats_capture_self_test.py`: PASS, 13,826 records.
- Results of the remaining local checks are in the FREEZE_SHA packet on #162.

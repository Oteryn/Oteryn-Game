# ITEMS-HELMETS-HATS-20261113

```yaml
task_id: ITEMS-HELMETS-HATS-20261113
title: Ordered Helmets and Hats Basic128 evidence
mode: AUDIT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/items-helmets-hats-evidence-takeover-20261113
pr: 3160
base_sha: 5efd67da164ac9dc33c8c066434b228cebb86c2b
owner: /root/compare_items
created_at: 2026-11-13T08:00:00Z
updated_at: 2026-11-13T08:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/items-helmets-hats-20261113/**
  - docs/agents/tasks/ITEMS-HELMETS-HATS-20261113.md
public_contracts: []
```

## Outcome

PARTIAL:128 ordered Basic observations,268 BasicPNG,652 private card-proof references,899 non-narrative literals and28 private text receipts. Correction brass helmet7→3 is independently receipted. Canonical/selected source candidates are reproduced without client/serverID claims. Formal review pending with root.

Lead reopened AUTHORING before the first formal review solely for a whitelist current-client appearance receipt. It adds126 unique client-namespace candidates and two sets, server_id=null for all128 rows,56 verified sheets and272 unique known sprites/299 recomputed slot metrics. Raw assets, descriptions and source/collector code remain private. Historical authoring head58f1d1ef93a2be4e23c8a5c60ea634487a67c3d7 is superseded by the new externally recorded final freeze.

## Authority and boundaries

Allocation: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6846579722 .
High-risk authority/recovery and mechanic reference parity: NOT_APPLICABLE, evidence-only documentation; no runtime/content/protocol mutations.
Whole Items audit, NPC/trade/animation, runtime admission and qualified server identity remain incomplete. Existing frozen evidence PRs are unchanged. No merge or review trigger by this writer.

## Validation

- `python docs/agents/evidence/items-helmets-hats-20261113/validate.py`: pass,128 records/652 proofs/268 BasicPNG.
- `python docs/agents/evidence/items-helmets-hats-20261113/compare_sources.py --check`: pass, exact source reproduction.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: OK,129 tests.
- `git diff --check`: pass; staged blob bytes match every packet-integrity receipt.
- Private projection verification: pass,135 input hashes/8 decoder hashes+Git blobs/56 sheet hashes/299 recomputed metrics; strict nested whitelist and Description seals checked against the128 Basic observations. OpaqueRGB/static-icon coverage does not certify animation1:1 or server identity.

No game compilation or runtime/E2E test: evidence-only documentation. Final exact FREEZE_SHA is recorded externally after the final commit, never self-referentially here.

## Closeout

Draft #3160 published. Formal independent review and exact-head CI qualification remain pending with lead after freeze; no review trigger, merge or deployment by this writer. This active task remains waiting/PARTIAL rather than terminally archived. Final writer freeze ends authoring of this candidate, not the complete Items audit.

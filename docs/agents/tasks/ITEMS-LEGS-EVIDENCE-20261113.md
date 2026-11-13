# ITEMS-LEGS-EVIDENCE-20261113

```yaml
task_id: ITEMS-LEGS-EVIDENCE-20261113
title: Ordered Legs Basic66 evidence and bounded source/client candidates
mode: AUDIT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/items-legs-evidence-takeover-20261113
pr: 3161
base_sha: 9b7552e07f241a423a10d8304726c2de94489c74
owner: /root/compare_items
created_at: 2026-11-13T07:47:07Z
updated_at: 2026-11-13T07:50:18Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/items-legs-20261113/**
  - docs/agents/tasks/ITEMS-LEGS-EVIDENCE-20261113.md
public_contracts: []
```

## Outcome

PARTIAL:66 ordered Basic cards,153 BasicPNG,351 card proofs,507 non-narrative literals,6 Description and6 layout receipts. Four native-reread canonical name-candidate differences remain outside any runtime/content fix. Client appearance66/66 candidates retain server_id=null;31 sheets/139 unique sprites/142 independently recomputed slot metrics.

## Authority and boundaries

Allocation: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6847143575 .
High-risk authority/recovery and mechanic parity: NOT_APPLICABLE, evidence-only documentation with no runtime or production mutation.
Original private assets/text/config remain private. Frozen Helmets3160 and all other evidence PRs are unchanged. Whole static audit/NPC/trade/animation/runtime/server identity remain incomplete.

## Validation

- `python docs/agents/evidence/items-legs-20261113/validate.py`: pass,66 cards/153 BasicPNG/351 proofs,6 Description+6 layout seals,66 client-only candidates.
- `python docs/agents/evidence/items-legs-20261113/compare_sources.py --check`: pass, exact canonical/selected source reproduction.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: OK,129 tests.
- `git diff --check`: pass; all12 integrity receipts match staged Git bytes.
- Private source/appearance checks: pass, exact actual index/reader pathset;73 input hashes,8 decoder hashes/Git blobs,31 packed/decoded sheet hashes,142 independently recomputed sprite-slot metrics. Nested raw Description and serverID promotion negative cases rejected.

No game compilation or runtime/E2E tests: documentation evidence only. Exact FREEZE_SHA is external after the final commit, never self-referentially recorded here.

## Closeout

Draft #3161 published. Root is sole independent review dispatcher; formal review and exact-head CI qualification remain pending after freeze. No worker review trigger, merge or deployment. This active task remains waiting/PARTIAL rather than terminally archived; final writer freeze ends this candidate's authoring, not the complete Items audit.

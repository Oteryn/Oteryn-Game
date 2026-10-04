# OTV2-20261004-timed-content-2b

```yaml
task_id: OTV2-20261004-timed-content-2b
title: "TIMED-CONTENT-2b: wiki duration only from the reciprocal inactive form"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/timed-content-2b
pr: 1721
base_sha: 8588b1bb
owner: claude-code-session-01QFRdKvFbNsNiCzMyR75FrC (control-plane allocation)
created_at: 2026-10-04
updated_at: 2026-10-04
owned_paths:
  - apps/game-server/src/content/item_timed_promotion.rs
  - docs/agents/evidence/OTV2-20261003-timed-item-facts-v1.json
  - tools/content-schema/item-authoring/lower_timed_item_packet.py
  - tools/content-schema/item-authoring/test_lower_timed_item_packet.py
  - docs/agents/tasks/archive/OTV2-20261004-timed-content-2b.md
public_contracts: []
```

## Outcome

This fixes #1717 Codex P2 4176298665, deferred under D245. A timed form takes a TibiaWiki duration from an
inactive form only when the pair is reciprocal (`reciprocal(item_id, i)`, TIMED-ITEM-0B). A new test covers
two inactive forms that equip into one active form, where only the reciprocal one may lend its duration.
The committed packet changes only its compiler digest. Its rows, report, `show_count` and counts are
byte-identical, and `ITEM_TIMED_PROMOTION_V1_PACKET_SHA256` is re-pinned. Content is unchanged.

## Validation

- `test_lower_timed_item_packet.py`: 11/11, and the new test fails on the old filter.
  `lower_timed_item_packet.py --check`: pass.
- Item-authoring workflow suite, run locally in each step's working directory: pass. Ruff 0.16.1: clean.
- `cargo test --lib item_timed`: 14 passed. `content_world_project_repository`: pass. `cargo fmt --check`: clean.
- `reward_claim_variant_migration.py --check`: pass.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass.

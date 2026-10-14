# SPELL-ITEM-TEMPORAL-JOIN-FIX

```yaml
task_id: SPELL-ITEM-TEMPORAL-JOIN-FIX
title: "Fix ambiguous USING(item_instance_id) in the spell item and field temporal expiry sweeps"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: spell-items
base_branch: main
branch: claude/fix-spell-item-temporal-join
pr: 2118
base_sha: a94831d0
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01XYC239givrMonrYoBwwECV
created_at: 2026-10-14
updated_at: 2026-10-14
packet: "preprod PostgreSQL log: common column name item_instance_id appears more than once in left table"
owned_paths:
  - apps/game-server/src/durability/spell_item_temporal.rs
  - docs/agents/tasks/archive/SPELL-ITEM-TEMPORAL-JOIN-FIX.md
depends_on: []
public_contracts: []
external_repositories: []
```

## Outcome

Both temporal expiry sweep queries (item and field schedules) joined
`game_character_equipment_slots eq USING(item_instance_id)` after the `bag` ON-join, so the left
side carried two `item_instance_id` columns and PostgreSQL rejected the whole sweep. The `eq` join
is now `ON eq.item_instance_id=p.item_instance_id`. The other `USING` joins precede the `bag`
join and stay unambiguous. A source-level unit test forbids `USING(item_instance_id)` after the
`bag` ON-join in both queries. The PostgreSQL audit script (`spell_item_temporal_pg_cases.sql`)
is driven by `verify_house_privacy_pg.py`, not a Rust harness, and does not run the sweep query.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo clippy --locked -p oteryn-game-server --lib -- -D warnings`: pass. The `--all-targets` clippy and the new unit test could not compile locally (lib test build killed by memory); left to CI.
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass (124 tests)

## Review

Independent review on the frozen head, requested by the lead.

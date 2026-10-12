# EQUIP-WIRE-1

```yaml
task_id: EQUIP-WIRE-1
title: "EQUIP-WIRE-1: equip and unequip one item over command type 9 (ITEM-MOVE-2a slice)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/equip-wire-1-20261012
pr: 2009
base_sha: e128fd6cc
head_sha: "exact head in the PR"
final_head_sha: "exact head in the PR"
owner: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
control_plane: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-12
updated_at: 2026-10-12
migration_lease: "0082 (leased for ITEM-MOVE-2a; this slice adds no migration)"
owned_paths:
  - apps/game-server/src/durability/character_equipment.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/item_move.rs
  - apps/game-server/src/gameplay_transport/item_move_tests.rs
  - apps/game-server/src/gameplay_transport/item_view.rs
  - apps/game-server/src/gameplay_transport/item_view_tests.rs
  - apps/game-server/src/gameplay_transport/equipment_move.rs
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs
  - apps/game-server/src/gameplay_transport/item_ref_admission.rs
  - apps/game-server/src/gameplay_transport/container_view_tests.rs
  - apps/game-server/tests/item_ref_production.rs
  - apps/game-server/tests/support/equipment_move_postgres_cases.rs
  - docs/agents/tasks/active/EQUIP-WIRE-1.md
  - docs/agents/tasks/archive/EQUIP-WIRE-1.md
public_contracts: []
external_repositories: []
```

## Outcome

Command type 9 (`ItemMoveIntentV1`) moves one item between the Character's main backpack and an
equipment slot on the live Game node. The move persists through the existing equipment owner
(`apply_equipment_in_transaction`) in one fenced semantic transaction. Domain 9 shows the
equipment when the session selected capability 12.

## Architecture and source of truth

- PROVEN: ITEM-MOVE-WIRE-1 decision `docs/architecture/reviews/OTERYN_GAME_ITEM_MOVE_WIRE1_EQUIP_AND_DROP_DECISION_2026-09-30.md` section 4 (equip/unequip semantics, outcomes 10 to 12, no CharacterRevision advance).
- PROVEN: `docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md` section 2.8 (ITEM-MOVE-2a acceptance).
- PROVEN: the equipment owner `apps/game-server/src/durability/character_equipment.rs` and its guards (migrations 0035, 0042, 0045, 0075).
- DERIVED: movement speed, cast facts and condition snapshots read equipment per command or per step, so no cached copy needs invalidation.

## Acceptance criteria

- Equip of a direct main-backpack entry into a free compatible slot commits and answers `MOVED`, then the domain 9 delta.
- Unequip to the main backpack commits and answers `MOVED`; a full or missing backpack answers `NO_ROOM` or `NO_BACKPACK`.
- Incompatible slot answers `SLOT_MISMATCH`; level or vocation answers `REQUIREMENT_NOT_MET`.
- A resent CommandId that committed answers `MOVED` without a second write; an unknown outcome ends the connection.
- CharacterRevision does not advance.
- Ground stays `NOT_SUPPORTED`.
- Capability 12 stays `offered: false` in this slice (see Closeout).

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib --quiet`: 2810 pass, 1 fail (`content::item_admission::tests::pinned_cheese_packet_digest_and_evidence`: the tracked reference file `docs/reference/spells/r21-local-candidate/item-profiles/item-profiles.json` is absent from the local checkout; unrelated to this change)
- `OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test item_ref_production --quiet` on PostgreSQL 17.6: 1042 pass (includes `equipment_move_postgres_cases`)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

## Closeout

- Delivered: command type 9 equips a direct main-backpack entry into a free slot and unequips an
  equipped item into the main backpack through `apply_equipment_in_transaction`, under the spell
  lane, the session item fence and the Channel content pin, with a replay-first equipment receipt
  read by CommandRef. Domain 9 shows the occupied slots under capability 12. CharacterRevision
  never advances (Postgres test). No migration; 0082 stays leased for the rest of ITEM-MOVE-2a.
- Not offered: capability 12 stays `offered: false`. The ITEM-MOVE-2a offer condition (§2.8)
  also needs swap, the B3 merge on unequip, Premium at commit and the timed slot call sites.
- Follow-ups: swap and its guards (migration 0082); B3 stack merge on unequip; Premium check at
  commit; timed-host slot call sites; capability 12 offer with a production-path admission test;
  Ground drop (ITEM-MOVE-2b); quiver and EXTRA slot rules; the equipment receipt lookup adds one
  durable read per command 9.

# OTV2-20260929-d3-2-corpse-loot

```yaml
task_id: OTV2-20260929-d3-2-corpse-loot
title: D3-2 - corpse-first MINT and loot MINT into the corpse container
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
allocation_comment: "D3-2 row, OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md §6; scope widened by the #162 control plane to include the MINT-side proto/audit/durability path"
base_branch: main
branch: claude/d3-2-corpse-loot
base_sha: 6133bde
head_sha: adb3bed2af938da6997a7ef372bba99acd529ba8
owner: "Oteryn: impl combat" (Claude Code)
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/combat/death_reward.rs
  - apps/game-server/src/combat.rs
  - apps/game-server/src/durability/item_mint.rs
  - apps/game-server/src/durability/item_mint_audit.rs
  - apps/game-server/tests/support/combat_death_reward_postgres_cases.rs
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md   # one sentence, §39.4 MINT-widening ownership
  - docs/agents/tasks/active/OTV2-20260929-d3-2-corpse-loot.md
public_contracts: [DUR-03 §39.4, game-events/v1/native_one_item_transaction.proto]
depends_on: [D3-1 (#1213), D3-3 (#1215)]
blocks: [D3-6]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

`settle_creature_death_rewards` now mints the death's corpse first, then each loot entry into that
corpse's container instead of onto Ground (D3 §4.1):

- **Corpse first.** An ordinary Ground MINT (`freeze_item_mint` + `commit_corpse_mint`) with the
  `CORPSE_MATERIALIZATION` cause, `draw_ordinal = 0`, the caller-supplied corpse item definition
  (`CreatureDeathRewardInput::corpse_item`; Content binding of `i00005801` waits on D3-7) in place
  of a loot table, and the owner's top-damage `CharacterId` (D132, `top_damage_character`, read at
  the same moment as `(death, corpse)`).
- **Loot into the corpse.** New `freeze_corpse_loot_mint` / `commit_corpse_loot_mint`
  (`durability/item_mint.rs`): a fresh item, its `game_item_corpse_container_entries` row and a receipt
  carrying `destination_parent_item_instance_id` / `destination_ordinal` (migration 0013, unchanged)
  commit together. The parent must be the same death's live corpse (typed `InvalidInput` early; the
  0013 guard is the authority). Same D52 fence, idempotency and RL-08 budget as every MINT.
- **Whole-plan preflight.** `check_corpse_container_capacity` checks the plan's full accepted entry
  count against `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` (16) once, before the corpse or any entry
  is frozen. The per-entry 16 ceiling and the corpse-row lock stay the 0013 trigger.
- **Proto (strictly additive).** `OneItemMintV1.corpse_container_entry = 5`
  (`OneItemContainerEntryV1`, existing message): exactly one of `destination` (Ground) and this. The
  audit gate (`check_mint`/`encode_mint_event`/`decode_envelope`) admits the corpse-loot shape; the
  event World/Channel are the death's. The Ground MINT bytes and intent bindings are unchanged.
  `DECAY_RETIRE` (D3-6) and the TRANSFER source widening (D3-4) are separate additive changes.
- **D52 / cap.** A generation ending mid-plan leaves the corpse holding only the entries that
  committed (`CombatDeathRewardLootError::Mint`); at `COMBAT01-CORPSES-PER-SCOPE` the corpse MINT is
  refused (`Corpse(CapacityExceeded)`), so no corpse and no loot exist, and XP still settles.

## Decisions for review

- **Placement ordinal.** Entry `i` of the accepted plan occupies ordinal `i + 1`, assigned by the
  caller, not "highest live ordinal + 1" under the corpse-row lock. The ordinal is inside the frozen
  audit event and the cause's intent binding, both fixed at freeze, so it must be known before the
  commit takes any lock. Plan-index ordinals are deterministic (replay-stable), cannot collide
  (`UNIQUE (parent, placement_ordinal)`) and are bounded by the preflight; the corpse-row lock still
  serializes the DB-side 16 ceiling.
- **No tracked contributor.** `corpse_top_damage_character_id` is `NOT NULL` for a corpse receipt
  (0013), so a death with no tracked contributor (damage-free, or every hit from an untracked 17th
  attacker) names the death's single reward principal (`COMBAT01-REWARD-PRINCIPALS`) as the window
  winner instead of dropping the corpse. Flagged for the control plane; no migration touched.
- **Paths beyond the brief.** `combat.rs` re-exports and one DUR-03 contract sentence (D3-2, not
  D3-6, registers the MINT widening) were also touched. Deviations 1-3 accepted by the coordinator.

## D4 dependency note

The carrier fixture supports one committed hit per creature generation (see the note in
`damage_contributors_tests.rs`), so the **multi-hit** end-to-end test (several attackers, tie-break
resolved through the composition into the corpse receipt) cannot be written yet. It lands after
**D4-1** (multi-hit combat); until then the tie-break is proven by D3-3 against `DamageContributors`
directly and D3-2 proves the single-hit attribution seam into the corpse receipt.

## Validation

PG cases (CI runs them; no local PG): one death yields one corpse with the loot inside and nothing
loot on Ground; replay is idempotent; a D52 generation change before the corpse and mid-plan drops
the remainder; at the corpse cap the death settles with no corpse and no loot; damage-free death
names the reward principal; empty plan still materializes a corpse.

Local at 847d94d: `cargo test --lib` 1086 passed / 0 failed; PG test targets `--no-run` build clean;
`validate_governance.py` passed; `git diff --check` clean; fmt and clippy `-D warnings` clean.

## Context checkpoint

```yaml
last_progress: PR #1222 merged via Merge Queue as d579154; protected-main readback matched adb3bed; record archived
next_action: none for this task; follow-ups routed on #162
```

## Closeout

- merge commit/result: `d579154` on protected `main` (#1222); every file the PR changed is byte-identical to `adb3bed`
- ownership release: all leases released at merge

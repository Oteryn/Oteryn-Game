# QB-OUTFITS — outfit and addon quest build lane qb

```yaml
task_id: QB-OUTFITS
title: "QB: outfit and addon quests fully playable at Tibia global parity"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/happy-davinci-ijz3wy-qb
pr: null
merge_authority: CP_ONLY (lane opens, labels and merges no PR)
owned_paths:
  - docs/agents/tasks/QB-OUTFITS.md
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qb_*.rs
  - tools/world-bundle-compiler/tests/npcs/qb_*.rs
  - tools/content-schema/npc-authoring/test_qb_*.py
  - this lane's quests' entries in shared callback, placement and index files
```

Done bar (owner): the quest passes E2E **and** its rewards and mechanics match
Tibia global, with the TibiaWiki page cited. Where a Canary or Crystal donor
differs from global, global wins.

## QB-1 — outfit hand-in NPC placements

Gap: the reward callbacks for these quests existed and their stories passed in
the PostgreSQL harness. But the NPCs that serve them were missing from the
native World placement index, so no player could reach the conversations.

| Quest | TibiaWiki | NPCs placed (x, y, z) |
|---|---|---|
| Assassin Outfits | `Assassin_Outfits_Quest/Spoiler` | Vescu 32565,32651,7 (base); Erayo 32518,32911,8; Atrad 32077,32533,9 |
| Beggar Outfits | `Beggar_Outfits_Quest/Spoiler` | Hugo 32950,32102,5; Simon the Beggar 32186,32471,7 |
| Hunter Outfits | `Hunter_Outfits_Quest/Spoiler` | Legola 32298,31785,7 (Elane was already placed) |
| Mage / Summoner Outfits | `Mage_Outfits_Quest/Spoiler`, `Summoner_Outfits_Quest/Spoiler` | Angelina 32633,32402,10; Lynda 32331,32199,7; Myra 32580,32751,6; Sandra 33257,31839,7; Zoltan 33266,31847,4 |
| Wizard Outfits | `Wizard_Outfits_Quest/Spoiler` | Lugri 32389,32118,8; The Queen of the Banshees 32260,31866,14 |

- Every NPC is placed at its single agreeing Canary/Crystal promotion-candidate
  cell (`npc_placement_scope.json`, then `convert_placements.py`, then
  `pin-refresh`).
- Vescu's donor cell holds a BlockSolid item in our map. No adjacent cell is
  clear single-item native ground, so `placement_adjustments` cannot apply.
  Vescu is therefore a source-backed `block_solid_contact`, the same mechanism
  as Zarifan. The neighbouring border items do not block movement.
- `test_m38_zarifan.py` now pins the exact exception set
  {Zarifan, Vescu} instead of "Zarifan only".

Tests (red before the placement change, green after):

- `tools/world-bundle-compiler/tests/npcs/qb_outfit_npcs.rs`: each NPC is
  admitted by `realize_npcs` at its exact cell.
- `tools/content-schema/npc-authoring/test_qb_outfit_placements.py`: the
  converter writes each NPC exactly once at its cell.
- `postgres_tests/qb_outfit_placements.rs`: every reward-callback NPC of each
  quest is in the placement index, then the whole source story runs on
  PostgreSQL.
  - PostgreSQL legs: the harness requires 18.6. This container has 16, so the
    routed legs are left to CI.
  - The placement assertions run route-less.

Known global-parity gaps (later batches or the server lane):

- Sandra's lottery (100 empty vials for a random winning ticket) needs a
  random-outcome NPC exchange.
- The full-outfit achievement on owning both addons, from any source, has
  been sent to the server lane.

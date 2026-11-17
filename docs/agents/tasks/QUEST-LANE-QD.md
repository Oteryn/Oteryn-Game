# QUEST-LANE-QD: travel, door, lever, teleport and trigger quests

- Branch: `claude/happy-davinci-ijz3wy-qd` (from `origin/main` e2152d05); no PR is opened by this lane.
- Done-bar (owner): the quest passes a real-dispatch PostgreSQL case and its rewards and
  mechanics match Tibia global (TibiaWiki). Global wins over a Canary/Crystal donor.
- Owned paths: `content/quests/{npc-callbacks,npc-offers,authored-bindings}/qd-*.json`,
  `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qd_*.rs`, this record, and
  sorted insert-only hooks in the shared registries/scenario dispatchers.

## Batch 1

| Quest | Global source (TibiaWiki) | Route | Proof | State |
|---|---|---|---|---|
| Steal From Thieves | `Steal_From_Thieves_Quest` curid 20034 rev 842913; spoiler curid 22844 rev 1057227 | Ahmet `mission`/`yes` starts; the Darama oasis box (33131,32661,7; existing claim `reward_oasis`) gives Bag (Ahmet) i235 (400 oz); Ahmet `quest`/`yes` takes the bag and gives 100 Brown Mushrooms i3725 + 100 gold i3031, completes; premium | `qd_steal_from_thieves.rs` | done |
| Sam's Old Backpack | `Sam's_Old_Backpack_Quest` curid 5118 rev 788465; spoiler curid 5178 rev 891416 | Sam takes the backpack (existing `sam-backpack`, achievement Backpack Tourist); Kroox `Sam sent me` grants the mine door (action 40603 reads `sams_old_backpack_door`); the chest at 32455,31968,14 gives one Dwarven Armor i3397 and completes; level 35, free account | `qd_world.rs` (`qd-world-sams-old-backpack`) | done |
| Draconia | `Draconia_Quest` curid 3286 rev 683833; spoiler curid 5553 rev 855037 | Level 25, free; two chests: Ice Rapier + Serpent Sword, Stone Skin Amulet + Energy Ring. The existing claim set `kv/draconia1`/`kv/draconia2` and key-door bindings already match global | existing `m35_claim_sets.rs` (`m35_draconia_dispatch_requires_both_complete_bags_in_either_order`), `draconia_escape.rs` | done (no change) |

Notes:
- Sam route files: Kroox is an NPC reward callback (`npc-callbacks/qd-sams-old-backpack.json`,
  Sam's quest has no completion-plan TALK stage); the chest is the authored binding's `map_use`
  stage `s4` (`authored-bindings/qd-sams-old-backpack.json`).
- Sam's global dialogue is `backpack` -> `yes` -> `yes`; the existing shared Sam callback uses one
  `yes` (`shared-reward-callbacks.json`, Crystal donor). Value and state are the same; the
  second confirmation is a P3 dialogue gap for the P2/P3 backlog.

## Deferred / open

- Giant Smithhammer: the donor boxes (32774..32778,32253,8) give a Hammer of Mighty Cheating
  (i12510) where global (`Giant_Smithhammer_Quest` curid 3476 rev 756268; item page
  `Giant_Smithhammer` itemid 3208) gives the Giant Smithhammer i3208, and the chosen
  `questpc7-smith` cache at 32776,32254 grants a second Talon/hammer/100 gp. Global wins over
  the PR2770 Q5=A decision; fix in a later batch.

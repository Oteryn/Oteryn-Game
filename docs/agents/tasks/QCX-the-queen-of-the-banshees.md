# QCX76 · The Queen of the Banshees Quest

Quest `oteryn:quest.the_queen_of_the_banshees_quest`. Base `871c1dec7d9963420d455210e4cddc0c9df80d26`.

Global evidence is the fetched 2026-11-17 wikitext on qfacts at `460e1d8c2c0e88c438ae5e54910db00822dc0129`. The base page establishes requirements and rewards. The spoiler establishes the seal sequence, the level check at the kiss, and the final room and exit. [Quest page, curid 3216 and oldid 1038207](https://tibia.fandom.com/?curid=3216&oldid=1038207) · [Spoiler, curid 34100 and oldid 1057212](https://tibia.fandom.com/?curid=34100&oldid=1057212)

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` supplies numeric source bindings where the global pages give names. The retained review snapshot inventories 51 donor steps and preserves source file and line witnesses. [Donor snapshot](../evidence/quests-2-20261102/the_queen_of_the_banshees.json)

## Audit

| Part | Global evidence and comparison to the base |
| --- | --- |
| Requirements | No premium, level 60, and quest-log visibility already match the canonical definition. The recommendation of 80+ is advisory. The actual level check belongs at the kiss. Rope and pick are route tools, and blood and two pearls are puzzle inputs. They are absent from the non-runtime approximation as explicit inputs. |
| First, Hidden | Two wall switches and pressure-tile reset, poison-field stair switch, a pick hole, then a flame spawning two Ghosts and a Demon Skeleton for each player. The only active quest callback was the return teleport. The retained map lacked the donor action on the three return tiles. |
| Second, Plague | Pick access and one pearl of each color on their respective tiles, with either entrance teleport usable. The entrance transaction was absent. The active blue-flame completion callback already grants the two seal flags and rejects a repeated step. Its donor UID was missing from the native tile. |
| Third, Demonrage | Two Warlocks spawn once for each player who triggers the tiles, followed by the switch sequence and flame. The native quest callback registry contains no equivalent sequence or spawn path. |
| Fourth, Sacrifice | Blood from any container is valid on the marked tile before the flame. The native quest callback registry contains neither interaction. |
| Fifth, True Path | The tile sequence and its flame are absent. The Spectral Dress door is an Explorer Society dependency. |
| Sixth, Logic | The shared switch arrangement and blue-flame completion are absent. The spoiler allows one player to arrange the switches for the group. |
| Last Seal | Queen dialogue branches already check the six independent flags in the donor order. The final affirmative response is deferred. The level-60 refusal, three writes and teleport are missing. The Queen has an admitted definition and a donor placement candidate, but no row in the scoped native NPC placement package. The Wizard addon and the Spectral Dress lines belong to their own quests. |
| Final room access | The definition and door catalogue record the donor's per-character progress doors. The wiki describes access after the seventh seal, a one-way descent and no re-entry after the exit. The deferred kiss blocks a natural end-to-end proof of door access. The exit callback resets seven door flags and sets final-battle completion. |
| Final room encounter | The spoiler lists 6 Banshees, 2 Dragon Lords and 2 Giant Spiders. The native spawn data around the reward room includes 4 Banshees, 3 Dragon Lords and 2 Giant Spiders in the area bounded by x32205–32235, y31889–31919, z15. Exact Global creature placements are open. The spoiler describes clearing the room for safety and gives no durable kill-credit requirement. |
| Final exit | The donor sends the player to (32269, 31853, 15). The wiki says the forcefield goes to the Ghostlands surface at (32202, 31846, 7), decoded from `125.202 / 124.102 / 7` using the repository's 256-tile sectors. The native source forcefield lacked UID 35019. |
| Reward claims | All six source quantities already match. UIDs 6056–6061 were missing from the six retained chest stacks. The separate non-runtime approximation lists only Boots of Haste and platinum coins. The canonical definition references all six claims. [Crystal chest.lua:1000–1041](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1000-L1041) |
| Other rewards | The captured quest reward list gives no XP, achievements or outfit. The Queen's donor dialogue grants a Wizard addon and Warlock achievement on a separate quest branch. The quest route includes an Isle of the Kings connection via the pearl area and a monastery restriction warning. [Crystal Queen dialogue:151–168](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/the_queen_of_the_banshees.lua#L151-L168) |

The claim plan is 1 Boots of Haste (3079), 1 Giant Sword (3281), 1 Tower Shield (3428), 1 Stealth Ring (3049), 1 Stone Skin Amulet (3081), and 100 Platinum Coins (3035). The Stone Skin Amulet already has a native exact partial-charge projection despite its historical source-readiness hold. The two sources establish one amulet and the donor separately supplies the existing one-charge operand.

The active shared callback document had three Banshee callbacks and the inspected held callback directory had no Banshee row. The original door overlay binds actions 10074, 10076, 10078, 10081, 10083, 10086, 10088 and 10089 to the eight donor door tracks.

## Blockers and open facts

The existing callback engine has timed target resets and encounter effects. Its timed passage reset excludes reversible circuits, and the group encounter owner has different admission semantics from shared-room per-player ambushes. The NPC program catalogue supports level predicates and recoverable teleport, but the current classic transcript owner lacks the combat-lock-aware handoff for the final confirmation.

```text
BLOCKER Hidden Seal wall and trapdoor puzzle shared paired toggles and delayed reset composition data-global/scripts/quests/the_queen_of_the_banshees/action-1-first_seal_lever.lua:3-45
BLOCKER Hidden Seal flame per-player ambush spawn coupled to credit and teleport data-global/scripts/quests/the_queen_of_the_banshees/movement-1-first_seal_flame.lua:3
BLOCKER Plague Seal pearl entrance guarded dual-stack consumption and teleport without a progress mutation data-global/scripts/quests/the_queen_of_the_banshees/movement-2-second_seal_pearl.lua:3-33
BLOCKER Demonrage switch puzzle shared lever sequence and multi-object reset data-global/scripts/quests/the_queen_of_the_banshees/action-3-third_seal_lever.lua:3
BLOCKER Demonrage ambush per-player once-only two-Warlock spawn data-global/scripts/quests/the_queen_of_the_banshees/movement-3-third_seal_warlock_tile.lua:3
BLOCKER Sacrifice typed blood placement and consumption coupled to seal admission data-global/scripts/quests/the_queen_of_the_banshees/movement-4-fourth_seal_sacrifice.lua:3
BLOCKER Logic shared six-campfire puzzle and full-layout flame admission data-global/scripts/quests/the_queen_of_the_banshees/action-6-sixth_seal_lever.lua:3;movement-6-sixth_seal_flame.lua:3
BLOCKER Queen kiss classic conversation handoff to level/combat-guarded teleport with atomic seal and door writes data-global/npc/the_queen_of_the_banshees.lua:137-150
```

- OPEN-FACTS Exact Global switch and path layouts, pick-hole tiles, reset timings, pearl pedestal coordinates and combat-lock behavior. The text snapshot references images for several layouts.
- OPEN-FACTS Exact Global inner-seal and kiss destinations, level-refusal wording, monastery fee and entry-place selectors, and exact final-room spawn cells.
- OPEN-FACTS Queen placement. The retained candidate is (32260, 31866, 14). The repository’s BR cross-check records (32260, 31863, 14). The supplied Global quest text gives relative directions. [BR NPC capture, curid 11674 and oldid 421546](https://www.tibiawiki.com.br/index.php?curid=11674&oldid=421546)
- OPEN-FACTS Global Stone Skin Amulet charge count. The current donor-backed one-charge projection is preserved.

PARTIAL Hidden through Logic complete puzzle routes, Queen placement, kiss and final-room access from fresh progress, final-room spawn parity, Isle of the Kings routing and restriction, source approximation and quest-log completion.

## Corrections

The six final-room chests now bind through the existing native placement alias owner. The aliases join the donor chest rows and canonical claims to the decoded native B3 stack at ordinal 1. The claim identities, once-per-character behavior, item IDs and counts stay exact. The amulet uses the pre-existing one-charge plan.

The existing startup placement overlay restores action 25019 on the three Hidden Seal return tiles and UIDs 35014 and 35019 on the Plague Seal flame and final forcefield. The additions were extracted with the startup capture parser from its already pinned `TileAction` and `TeleportUnique` tables. The base conversion consumes these exact placement rows. The final forcefield now uses the Global surface destination and keeps the existing durable door resets. [Crystal tile.lua:885–894](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/tile.lua#L885-L894) · [Crystal teleport.lua:566–594](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/teleport.lua#L566-L594) · [Spoiler, curid 34100 and oldid 1057212](https://tibia.fandom.com/?curid=34100&oldid=1057212)

## Verification

PG via shard (pgsplit build files applied on `5f5f96a04d817c96e928b988190f67a3d6701601`) baseline executable, 0 passed and 3 failed in 67.32 s. The native reward binder first failed on the untagged Stone Skin Amulet chest. The final-room STEP completed in 20.87 s and exposed the underground donor target. The Hidden and Plague callbacks dispatched on the fixtures in 13.14 s and 14.32 s before the native placement check exposed the missing return action. The final regression also checks all three action tiles, both unique IDs and the six real USE plans.

The callback registry and SOURCE proof were regenerated with the existing authoring tools. The quest tree generator then refreshed only shared source-change provenance hashes in 28 generated definitions and their catalogue index. A structural comparison found no gameplay field changes in those records.

The generated definitions are whole-shard inputs to several other derived catalogues and two authored-prop packets. Their generators refreshed the transitive SHA-256 references. A structured diff confirmed that every JSON value changed in the transitive regeneration differed only by a SHA-256, including the World prop inputs. Recompiling the World after that refresh produced the same corrected bundle digest.

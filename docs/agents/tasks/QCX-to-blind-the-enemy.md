# QCX88 · To Blind the Enemy Quest

Base `2c1e9158c30b411de28ec064bc7198ea4f87dd44`. Quest `oteryn:quest.to_blind_the_enemy_quest`.

The audit uses the fetched TibiaWiki quest and spoiler revisions for Global behavior. Crystal `201189f871d23597b82caf88fd611454a2ddefb8` supplies donor implementation evidence. [Quest, curid 20031 oldid 1091434](https://tibia.fandom.com/?curid=20031&oldid=1091434) · [Spoiler, curid 22845 oldid 749434](https://tibia.fandom.com/?curid=22845&oldid=749434)

| Part | Global and repository comparison |
| --- | --- |
| Requirements | Base definition already has level 0 and free account eligibility. Level 30+ is a recommendation. The recipe already lists the Destroy Field Rune, shovel and rope. The mirror comes from Eroth. No named quest prerequisite is listed. |
| Eroth | The NPC definition already exists. The base has no placement or mission offer, acceptance or return callbacks. The spoiler supplies the greeting and `mission → yes` transcript. Crystal grants one mirror, item 3463, and sets the questline to 1. |
| Fortress access | The spoiler specifies the Destroy Field Rune at Elvenbane and travel up the central tower. The imported quest has no gate. The spoiler locator resolves to (32579, 31680, 7), where the retained map has walkable ground. The separate Elvenbane route crosses a field of appearance 2121 on floor 6. The Destroy Field Rune cast owner only removes durable Ground Item instances. The exact field tile and variant at the spoiler approach are open. |
| Scrying Ball | The chosen recipe names item 10186. The retained tower top has appearance 132 at (32592, 31645, 2) with no action attribute. Crystal checks action 4001 and appearance 132 with mirror 3463, consumes one mirror, sets questline 2, produces a magic effect and tries to restore the ball after 60 seconds. Neither world-callback directory contains an active or held runtime record for this quest. |
| Return | The recipe already names 10 platinum coins (3035) and one elven amulet (3082). The live quest had no Eroth delivery. Crystal delivers both items only after the ball and four chest claims. The Global spoiler directs players straight back to Eroth after the ball, so the hand-in predicate uses questline 2 alone. |
| Donor chest claims | The generated definition is `reward_only` with four donor claim references. UIDs 6253 and 6254 yield one each of item 3425 and item 3282. UIDs 6255 and 6256 yield one backpack (2853) each. The first contains 237 ×1, 3147 ×1 and 3059 ×1. The second contains 3028 ×2 and 3031 ×100. Global lists none of these as Reflect Quest rewards. The separate Elvenbane runtime and its reward tests use the same claims and physical bindings. |
| Other rewards | The pinned pages award no XP, achievements, outfits, or access entitlement. |
| Journal | Global groups the quest under Tibia Tales. The generated donor record is a hidden reward-only definition and contains no playable journal mission. |

Donor evidence [Eroth, lines 57–80](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/eroth.lua#L57-L80) · [Mirror, lines 3–31](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/to_blind_the_enemy_quest/mirror.lua#L3-L31) · [Chests, lines 2551–2580](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2551-L2580) · [NPC placement, lines 582–584](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/world/world-npc.xml#L582-L584)

## Changes

The NPC placement generator promotes Eroth at the retained donor cell (32662, 31682, 6). The offer and acceptance use the spoiler transcript and grant one mirror. The completion callback requires orb-complete phase 2, then grants the two Global rewards in one receipt and advances to phase 3. The phase guard blocks duplicate grants. [Spoiler](https://tibia.fandom.com/?curid=22845&oldid=749434) · [Quest](https://tibia.fandom.com/?curid=20031&oldid=1091434)

## Outstanding

BLOCKER Scrying Ball timed removal of the selected shared object, with exact restoration, requires shared world projection support. Timed passages reject the interaction object itself, timed transforms substitute an appearance, and selected target removal has no restore option. `data-global/scripts/quests/to_blind_the_enemy_quest/mirror.lua:10-21`

BLOCKER Fortress entry removal of a static bundle field via Destroy Field requires shared rune and map support. `data/scripts/runes/destroy_field_rune.lua:14-20`

BLOCKER Treasure claim attribution overlaps the existing Elvenbane quest completion and its four source claim contracts. Moving the mappings requires changes to another quest's records and callers. `data-global/startup/tables/chest.lua:2551-2580`

OPEN-FACTS Global numeric item and map IDs, exact Eroth tile, field tile and variant, mirror consumption and replacement policy, orb action and reset semantics, and exact Tibia Tales journal text and stage thresholds. Crystal's restoration function looks up the item it just removed, so its script does not establish successful respawn.

PARTIAL Orb callback and placement action, fortress field placement, Tibia Tales journal, and legacy chest attribution. The NPC return test supplies the persisted orb result as fixture state, so the evidence covers NPC dispatch only.

## Local verification

- The sharded PostgreSQL test ran against the baseline NPC callbacks with the promoted placement loaded. The greeting succeeded and the `mission` offer assertion failed as expected in 53.16 s.
- Quest authoring checks passed all 44 commands, including 269 schema cases. NPC placement generator check and workspace formatting check passed.

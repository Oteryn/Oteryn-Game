# WORLD-SRC seasonal continuation: fresh donor investigation

Base: `1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7`. Research only; no Cargo
build, live server or donor Lua VM execution is claimed here. Raw downloads and
census inputs remain under `/tmp/world-src-cont/seasonal/`. Exact request URLs,
response statuses, SHA-256 digests and Git blob verification are recorded in
`seasonal-continuation-sources.json`. Donor facts are `OTS_HYPOTHESIS_ONLY`.

## Routes and refusals

Both recursive Git trees returned HTTP 200 with `truncated=false`:

- `https://api.github.com/repos/zimbadev/crystalserver/git/trees/201189f871d23597b82caf88fd611454a2ddefb8?recursive=1`
- `https://api.github.com/repos/opentibiabr/canary/git/trees/47dfd51f45280a59a1d3e50ba7edd573d7234446?recursive=1`

The following permitted MediaWiki requests returned HTTP 402; neither supplied a
page revision or content. User-Agent was `OterynCompatibilityResearch/1.0`.
No HTML page, alternate wiki host, captcha bypass or CipSoft endpoint was used.

- `https://tibia.fandom.com/api.php?action=query&prop=revisions&rvprop=ids%7Ctimestamp%7Ccontent&rvslots=main&titles=Cake_Golem%7CHalloween%7CChristmas%7CSanta_Claus%7CThe_Halloween_Hare&format=json&formatversion=2`
- `https://tibia.fandom.com/api.php?action=parse&page=Cake_Golem&prop=wikitext&format=json`

The Canary counterpart Cake file returned HTTP 404:
`https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/monster/quests/a_piece_of_cake/cake_golem.lua`.
Its absence also agrees with the complete pinned Canary tree.

## Cake Golem: donor damage semantics resolved

All Crystal locators below use commit
`201189f871d23597b82caf88fd611454a2ddefb8`; all have exact raw URLs in the JSON.

| Established fact | Source file and lines |
| --- | --- |
| The second Cake attack omits `type`, but declares combat damage 0–40, interval 2000ms, chance 15, range 7 and cake/poison-area presentation. | `data-global/monster/quests/a_piece_of_cake/cake_golem.lua:76–81` |
| `readSpell` creates a MonsterSpell, sets combat type only when `incomingLua.type` exists, and still loads the declared combat values/effects. | `data/scripts/lib/register_monster_type.lua:860–862,886–896,938–953` |
| The MonsterSpell constructor defaults combat type to `COMBAT_UNDEFINEDDAMAGE`. | `src/lua/functions/creatures/monster/monster_spell_functions.cpp:51–55`; `src/creatures/monsters/monsters.hpp:244` |
| Undefined is an actual combat family: value 4, distinct from physical 0, earth 2 and `COMBAT_NONE=255`. | `src/creatures/creatures_definitions.hpp:800–822` |
| The monster combat branch passes that type unchanged. Only the physical branch enables armor blocking; armor/shield defaults are false. | `src/creatures/monsters/monsters.cpp:123–131`; `src/creatures/combat/combat.hpp:103–119` |
| Non-NONE combat executes the health-damage path; it is not a condition-only or zero-damage default. | `src/creatures/combat/combat.cpp:198–215,1505–1513` |
| Undefined still passes ordinary damage admission/blocking, indexed absorbs and mitigation. It is not direct `addHealth`/unmitigated damage. | `src/creatures/combat/combat.cpp:905–907`; `src/game/game.cpp:7212–7255,7283`; `src/creatures/creature.cpp:842–870,875–891,942–944`; `src/creatures/players/player.cpp:3886–3893,3913–3961` |
| Undefined specifically bypasses mana shield while other downstream HP consequences remain in the ordinary health owner. | `src/game/game.cpp:8142` |

Canary independently corroborates the default (`monsters.hpp:236`), distinct
Undefined family (`creatures_definitions.hpp:838`) and shield exclusion
(`game.cpp:8592`). No Earth/Physical inference is made from poison visuals.

### Native consumer constraint, distinct from missing source data

The real scheduled caller is
`crates/gameplay/dispatch/src/gameplay_transport/monster_profile_cycle/physical_spell.rs:255–269,309–336`.
The current source parser rejects an `undefined` family at
`crates/gameplay/spell/src/creature_damage_spell/mod.rs:758–780`, and the player
reader has a closed element match at `physical_spell.rs:688–700`.

Merely adding a private element string is insufficient: the existing HP owner
calls `stage_typed_hit_in_world` in protected
`crates/gameplay/dispatch/src/gameplay_transport/actor_spell.rs:1253–1282`
(via its typed attack receipt path at 2616–2672). Protected
`crates/gameplay/spell/src/spell/actor_conditions.rs:715–721,763,877–918`
always passes the typed hit through mana-shield absorption. With HP 100, MP 100,
a 100-point active shield and an otherwise unmodified 40-point Undefined hit,
the donor loses 40 HP with MP/shield unchanged; the current ordinary native path
absorbs the hit into mana/shield instead. This is a concrete required consumer
change, not an unresolved donor element.

The existing `stage_agony_hit` is not an alias: it also skips mitigation/final
reductions and Gift (`actor_conditions.rs:823–837,877–918`). A raw-health
callback or Earth/Physical substitution is likewise incorrect. A generated package manifest can carry additional profiles through the normal
pinned reader, as the now-withdrawn Ruprecht experiment demonstrated. That does not provide the missing
HP semantics. Current native admission still excludes Cake Golem
(`crates/gameplay/durability/src/durability/item_craft/soulpit.rs:697`). Existing
wire `DamageElement::Untyped` can present the damage without a new wire ID; it
does not supply the missing HP semantics.

Cake STOP (five lines):
1. Fresh donor code resolves the second attack to real Undefined damage, not an unknown Earth/Physical element.
2. Faithful behavior includes ordinary mitigation/absorption while bypassing mana shield.
3. The real native typed-hit consumer unconditionally absorbs into mana shield in user-protected `spell/actor_conditions.rs`.
4. That protected consumer remains exactly main; a package profile cannot replace its HP semantics, and no alternate HP writer or partial creature is introduced.
5. Stop native Cake admission under this file-ownership constraint; the donor damage fact itself is now resolved.

## Santa: permanent claim and reachable reward groups

Crystal `data-global/npc/santa_claus.lua:45–56,83–122` and Canary
`data-otservbr-global/npc/santa_claus.lua:47–58,85–124` agree on the following:

- Eligibility is `storage 840293 != 1`; successful delivery sets it to 1. No year,
  annual reset, Premium or calendar guard occurs in these callback bodies.
- A first draw in 1..90 selects `normalItems`; the later `elseif <=70/30/10`
  branches cannot run. A draw in 91..100 leaves an empty reward table.
- Thus no corrected rare probabilities or annual entitlement may be invented.
- The empty branch calls `math.random(0)` before returning a reward. The code
  cannot select a valid group from that empty table. Exact error timing versus
  an empty container mint is not proved by the allowed sources examined here.
  LuaJIT is a dependency (`vcpkg.json:13`) and standard libraries are opened
  (`src/lua/functions/lua_functions_loader.cpp:46`), but those facts alone do
  not rule out later Lua overrides. No retained LuaJIT/lua/lupa VM was available;
  no runtime experiment or unconditional pre-mint-error claim is made.
- The storage assignment follows the reward loop, not the failed selection.
  A native port must not silently convert an empty selection to a random gift.

The source initializes `count` once before iterating the chosen group. Encountering
3599 or2992 changes it to 10 for that and every later item, not just that item
(Crystal:112–119; Canary:114–121). Literal reachable groups are therefore:

| Normal group | Item IDs and resulting quantities |
| --- | --- |
| 1 | 7439×1,7440×1,7443×1 |
| 2 | 3599×10,6507×10 |
| 3 | 3599×10,6508×10 |
| 4 | 3599×10,6506×10 |
| 5 | 3599×10,2995×10 |
| 6 | 3599×10,2992×10 |
| 7 | 3051×1,3097×1,3098×1 |
| 8 | 10310×1 |
| 9 | 3039×1 |
| 10 | 3036×1 |

A source gift is placed in container 6510 before the success flag. These are
known donor components, not proof of a playable Santa producer.

## Placement and other Christmas components

The full trees identify eight NPC placement XMLs. Every one was fetched and
parsed by exact normalized NPC name: none contains `Santa Claus` or
`Messenger of Santa`. The complete census of the specified global-event, primary world-change,
startup and `data/libs/tables` Lua paths (75 Crystal and 45 Canary files), plus the fetched Lua
raid definitions, supplied no Santa activation/position. This is a bounded
source census, not a claim about arbitrary operator-created NPCs.

Do not confuse `Messenger of Santa` (its Lua internal name at line 1) with the
seven Heaven-related messenger entries in the primary XMLs: Crystal
`data-global/world/world-npc.xml:526,1334,1457,1805,2236,3138,3141` and Canary
`data-otservbr-global/world/otservbr-npc.xml:451,1177,1285,1606,1990,2837,2840`.
Those names are Messenger of Heaven, Lesser Messenger of Heaven and Heavenly
Messenger. Their positions do not establish the holiday merchant's placement.

Ruprecht has a source placement candidate at (31953,31718,z7), not an added
production placement in the retained patch: Crystal primary
NPC XML:1522–1523 and Canary primary NPC XML:1350–1351; both give center x/y
with leaf offsets 0,0 and absolute z7. Its 22 rows are 21 token redemptions plus
one present-bag exchange. Crystal `npc/ruprecht.lua:54–76,95–124` establishes
6496×1 ->6526×1 and, for each other row, the listed number of 6526 tokens
->one output item. The JSON records the exact 22 rows. These are complete source facts for a distinct merchant component, not an
annual Santa claim or seasonal raid. Its production integration is now STOPPED
because the latest open branch protects both real native NPC consumers; see
[ruprecht-native.md](ruprecht-native.md). No merchant package or parallel writer
is retained. `Messenger of Santa` has 12 known
shop offers (`npc/messenger_of_santa.lua:51–63`) but no placement in this census.

Ruprecht is Dialogue-backed in the native canonical definition; the missing
native entry behavior is its framework greeting/farewell, not a NULL dialogue.
The pinned Crystal `data/npclib/npc_system/npc_handler.lua:77–90` supplies talk
range 4, `Greetings, |PLAYERNAME|.`, `Good bye, |PLAYERNAME|.` and walkaway
`Good bye.`. `modules.lua:5–6,281–300` registers `hi`/`hello` and
`bye`/`farewell`; `data-global/npc/ruprecht.lua:182` enables that FocusModule.
The handler substitutes the current player name before speaking
(`npc_handler.lua:348–355,386–391,406–410`). Canary corroborates the strings and
range at `npc_handler.lua:84–97`, keyword registration at `modules.lua:5–6,284–303`
and name substitution at `npc_handler.lua:355–362,393–398,413–417`.
These establish exact Ruprecht entry behavior; they do not authorize a broad
fallback for NPCs without dialogue. All eight newly fetched NPC framework files
have exact raw URLs, SHA-256 digests and matching pinned Git blob IDs in the JSON.

Christmas producer STOP (five lines):
1. The donor's permanent Santa flag and reachable reward groups are known; annual reset is not assumed or required for that source behavior.
2. No Santa actor coordinate/activation was recovered from eight NPC XMLs or the activation-script census; an unbound helper would not be a real producer.
3. Exact empty-selection error timing remains unverified; the source does not authorize inventing corrected rare rewards.
4. The Grynch creature prose mentions three waves but supplies no executable seasonal raid geometry/counts or lifecycle in the inspected registries.
5. Stop Santa/seasonal-raid activation pending those specific facts; retain Ruprecht separately as a source placement candidate (its native producer is blocked by the latest protected consumers), and do not alias Heaven messengers.

## Halloween producer

Crystal `data-global/scripts/globalevents/others/raids_schedule.lua:21–22,43–48`
and Canary's same relative script request Halloween Hare on October 31 at 16:00,
then mark the process-local occurrence without checking the call's success.
Neither complete XML raid registry nor any of the fetched Lua raid system/definition
files (4 Crystal, 22 Canary) defines Halloween Hare, Mutated Pumpkin or Grynch.
Their creature files establish creatures, not spawn coordinates or a world event.
Ordinary goblin XMLs cannot substitute: Crystal
`raids/edron/goblins.xml:6–9` names Goblin/Scavenger/Assassin, and
`raids/kazordoon/goblins.xml:8–13` adds Goblin Leader, not Grynch Clan Goblin.

Halloween STOP (five lines):
1. The pinned October 31 16:00 Hare request is recovered exactly.
2. The pinned XML/Lua raid registries supply no definition matching that request.
3. No executable Hare/Pumpkin spawn geometry/counts, portal lifecycle or participation entitlement was recovered.
4. Both authorized MediaWiki query/wikitext-parse routes returned 402, so no fresh page revision resolves those missing facts.
5. Stop that producer; do not create guessed waves, positions, aliases or health-only actors.

# M07 native trial evidence

Assigned: Lion's Rock, Machinery of War, Dreamer's Challenge, Too Hot to Handle, Realm of Dreams. Engine slices do not qualify whole quests.

Crystal source revision: `201189f871d23597b82caf88fd611454a2ddefb8`.

- [Lion trial source, lines13–79](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/lions_rock/actions_lions_rock.lua#L13-L79): consume tongue9634, flower21389 or water21466 only while its flag is below1 and pagoda2074 exists. The pagoda becomes2075 for15seconds. Existing donor storage belongs to TibiaTales; standalone bindings must preserve that ownership.
- Repository `world_region_codec.decode_region` plus the placements index establishes these native floor-8 targets: `(33137,32291)` ordinal1 source item21390; `(33138,32369)` ordinal3 source item2110; `(33154,32279)` ordinal2 source item21391. Pagodas at `(33134,32289)`, `(33136,32369)`, `(33156,32279)` each have ordinal1 source item2074, in `content/world/placements/region-z08-x129-y126.b3`.
- [Dreamer walls](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/dreamers_challenge_quest/actions_walls.lua#L1-L71): UID2246–2249 operate distinct literal wall sets; walls return after7seconds, levers after8seconds. An arbitrary lever command is not evidence of solving the puzzle.
- [Realm lever](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/chayenne_realm/actions_lever.lua): action55021 requires key14682, removes2129 at33075,32591,z3 for60seconds and has a5minute shared cooldown. [Entrance](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/chayenne_realm/movements_enter_realm.lua): action55022 requires level40 and retains the key. Neither operation alone proves the selected recipe's area-clear stage.
- [Bricklayer tool](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/tools/bricklayer_kit.lua) is action40027's bridge repair using wood5901/953 and kit7785. It does not establish the selected Machinery route's44-kit/outpost/siege facts; do not transplant it as that route.

Area-clear decision: use the current Channel carrier and exact registered creature keys, require a complete bounded census and previously started positive stage progress, and refuse unknown definitions or failed reads before inputs or quest state mutate. This predicate does not create an encounter or invent kill history. Source-specific stage bindings remain separate work.

Dreamer control mapping decision: captured `content/world/placements/region-z14-x127-y126.b3` has plain2772 levers at32761,32289;32763,32289;32761,32291;32763,32291 (all z14, ordinal1) and no UID tags. Oteryn explicitly binds those controls to north/east/west/south respectively. This is an authored control mapping, not a claim that the lost donor UID mapping was recovered. The literal source wall sets and7s/8s deadlines are retained. Source lookup is conditional: both1027 and1272 occur in the north configuration at32760,32288, but the captured map contains only1272; an absent1027 is skipped and never invented or recreated.

Strength wiring retains the established native TibiaTales Questline key (see existing inner-sanctum callback). The new Strength flag uses the Crystal namespace, since no corresponding Canary flag alias exists in the checked lowered tracks. Its default-1 source gate becomes1 only with one consumed9634 and an unlit native pagoda; Questline uses the shared checked max(old+1,2) operation. Premium admission follows the selected Oteryn recipe. Native B3source tuples are statue[19271,21390],pagoda[499,2074],player start[499].

### Lion Tears input qualification

`imports/canary/items-xml/items.xml:42634-42637` at pinned revision
`47dfd51f45280a59a1d3e50ba7edd573d7234446` (SHA-256
`1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2`)
defines item21466, “ewer with holy water”, with weight1750 and no duration,
charges or decay attributes. The pinned client appearance also passes the existing
non-fluid portable-item admission checks. Qualify this exact existing identity
through the shared source importer; do not invent a fluid subtype or evaporation
timer from descriptive prose. This input qualification alone does not implement
the well producer or the Tears offering.

### Lion Tears native offering

Crystal201189 `actions_lions_rock.lua:30-79` consumes one21466 at
33154,32279,z8, sets LionsTears, advances the existing questline by
`max(old+1,2)`, and transforms the pagoda at33156,32279 from2074 to2075
for15 seconds. Current B3 `region-z08-x129-y126.b3` preserves pedestal
21391 at ordinal2 above19272/6481, and pagoda2074 at ordinal1 above499.
The PG crop starts on actual ground499 at33155,32278; no donor UID is
invented. A fresh Crystal-namespaced Tears flag follows the Strength
namespace decision; the original TibiaTales questline remains its owner.

### Repeatable sacred-well producer

Crystal201189 `actions_lions_rock.lua:96-111` grants one21466 on every use
at33137,32351,z6, raises Questline to at least1, and sets TibiaTales.DefaultStart.
Current B3 region-z06-x129-y126 has4409/21394 there (well ordinal1) and
walkable4407 at33136,32351. The new explicit native per-use reward mode
uses a command/session/character/source scoped claim in the existing mint
transaction. A repeated command returns its historical receipt; a new admitted
USE can grant again. It neither creates a daily epoch nor weakens once-only
or preserved-epoch reward modes. Original DefaultStart is not a proven lowered
alias, so the callback uses its separate Crystal namespace as for the trial flags.

### Chosen Beauty input: donor duration and absolute deadline

QMP2D applies the owner's donor-first parity decision: Crystal
`201189f871d23597b82caf88fd611454a2ddefb8` `data/items/items.xml:42586-42590`
sets item21389 duration3600 and weight100, without a decay target. Both native
profile producers therefore publish duration3600000 with
`DURABLE_ABSOLUTE_DEADLINE` and no replacement item. This supersedes the earlier
inactive-consumption annotation; the owner explicitly selects a deadline when
the donor declares a duration. The finite qualifier still rejects changed
source duration/weight and any new instance mechanism.

Crystal `data-global/scripts/quests/lions_rock/actions_lions_rock.lua:13-79`
consumes each trial input once, increments `max(old questline, 1) + 1`, and resets
the shared pagoda after15seconds. Lines80-94 grant the flower and reset its plant
after60seconds; that plant timer is separate from the carried flower's lifetime.
The canonical selectors are the three `m07/lion-*` offerings; the native-rule and
M44 registrations are retired. Their dispatch tests use those canonical
selectors, retaining progress, replay, uncertain-commit and timed-reset coverage.

### Beauty offering and both source flower pickers

Crystal201189 `actions_lions_rock.lua:22-79` consumes21389 at33138,32369,z8
and lights pagoda33136,32369 for15 seconds. Current B3z08-x129-y126 has
[19272,6477,6374,2110] at the brazier, so the exact native target is ordinal3,
appearance2110; the pagoda is ordinal1/2074. No UID/AID is invented.

The source picker at Lua80-94 grants21389, raisesQL>=1 and DefaultStart,
transforms21388 to21935, then restores it after60 seconds. B3z06-x129-y126
has two actual plants:33133,32359 [4409,6373,21388] and33157,32364
[4403,6373,21388], both ordinal2. Their callbacks explicitly require the
current blooming appearance21388, use the admitted native per-use reward
mode, and reuse the existing timed source transform owner. Legacy item
rewards retain the world-requirement restriction; only the explicit admitted
native per-use mode can combine a fresh-value grant with this physical guard.

### Realm guardian footprint and retained-key controls

Current `content/world/spawns/spawns-*.json` declares Dragon/Dragon Lord guards
around33082..33096,32582..32599 on floors6/5/4, and seven Fire Elementals
around33076..33086,32584..32587 on floor3. A single-floor census would miss
required species. `ClearArea.additional_floors` therefore opts into at most
three unique extra floors of the same bounded footprint; omitted fields retain
the old single-floor wire and behavior. This is a current-world census, not a
client claim of kill credit.

Selected Realm identity is `oteryn:quest.chayenne_realm` (recipe record28),
with level40 and premium requirements. Pinned Crystal201189
`chayenne_realm/actions_lever.lua:1-29` retains key14682, removes2129 at
33075,32591,z3 for60 seconds and has a five-minute shared cooldown.
`movements_enter_realm.lua:10-24` supplies the level40/key entry facts.
Current B3z03-x129-y127 has an untagged2772 at33080,32582 ordinal3,
fire2124 at33080,32583 ordinal1, and wall2129 at33075,32591 ordinal1.
No recovered AID55021 is claimed.

Chosen binding decision: the existing lever starts the personal trial with a
retained key and live level40 guard. The adjacent fire is the finite programme's
hidden control: one admitted use after clearance reveals and operates it
atomically (2124->2773), opens the exact wall60s, and restores the original
fire/control after300s. The explicit clear footprint is33074..33096 by
32580..32601 on floors3,4,5,6. Only a started trial may clear it; the personal
phase prevents repeated credit after shared reset. The PG case seeds an actual
native Dragon on floor5 and checks that it blocks a floor3 control.

Key14682 weighs100 and bricklayers kit7785 weighs1800 in the shared pinned
Canary XML. Both are ordinary portable tools with no duration/charge mechanism.
Their admission supports the assigned Realm and Machinery families; the kit
profile alone is not a Machinery repair-route qualification.
# M07 native trial evidence

Assigned: Lion's Rock, Machinery of War, Dreamer's Challenge, Too Hot to Handle, Realm of Dreams. Engine slices do not qualify whole quests.

Crystal source revision: `201189f871d23597b82caf88fd611454a2ddefb8`.

- [Lion trial source, lines13–79](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/lions_rock/actions_lions_rock.lua#L13-L79): consume tongue9634, flower21389 or water21466 only while its flag is below1 and pagoda2074 exists. The pagoda becomes2075 for15seconds. Existing donor storage belongs to TibiaTales; standalone bindings must preserve that ownership.
- Repository `world_region_codec.decode_region` plus the placements index establishes these native floor-8 targets: `(33137,32291)` ordinal1 source item21390; `(33138,32369)` ordinal3 source item2110; `(33154,32279)` ordinal2 source item21391. Pagodas at `(33134,32289)`, `(33136,32369)`, `(33156,32279)` each have ordinal1 source item2074, in `content/world/placements/region-z08-x129-y126.b3`.
- [Dreamer walls](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/dreamers_challenge_quest/actions_walls.lua#L1-L71): UID2246–2249 operate distinct literal wall sets; walls return after7seconds, levers after8seconds. An arbitrary lever command is not evidence of solving the puzzle.
- [Realm lever](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/chayenne_realm/actions_lever.lua): action55021 requires key14682, removes2129 at33075,32591,z3 for60seconds and has a5minute shared cooldown. [Entrance](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/chayenne_realm/movements_enter_realm.lua): action55022 requires level40 and retains the key. Neither operation alone proves the selected recipe's area-clear stage.
- [Bricklayer tool](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/tools/bricklayer_kit.lua) is action40027's bridge repair using wood5901/953 and kit7785. It does not establish the selected Machinery route's44-kit/outpost/siege facts; do not transplant it as that route.

Area-clear decision: use the current Channel carrier and exact registered creature keys, require a complete bounded census and previously started positive stage progress, and refuse unknown definitions or failed reads before inputs or quest state mutate. This predicate does not create an encounter or invent kill history. Source-specific stage bindings remain separate work.

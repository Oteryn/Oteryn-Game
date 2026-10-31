# W02 initial activity template qualification

Status: **ContentUnavailable for the current production content**. The existing
Content activation owner now has a bounded activity-template qualification and
runtime factory path. No production boss catalogue row is enabled by this
change, and no source map or bespoke boss script is silently approximated.

This is the explicitly accepted outcome in
[ANSWER 6543189919](https://github.com/Oteryn/Oteryn-Game/pull/2769#issuecomment-6543189919)
when no complete source configuration qualifies. The shared runtime and durable
handoff implementation is independently exercised by the real activation fixture
in the PostgreSQL transport test; that fixture is not a claim that the production
Crystal room is loaded.

## Source and selection evidence

The donor is `zimbadev/crystalserver` at
`201189f871d23597b82caf88fd611454a2ddefb8`. The candidate inventory is produced by
`tools/content-schema/encounters/import_boss_levers.py`, without executing Lua,
into `content/encounters/bosses/crystal-lever-candidates.json`. Every record carries
its exact donor path, first configuration line and SHA-256 of the source bytes.
The artifact retains unsupported expressions, extra monster closures and custom
callbacks as explicit blockers. It supplies no admission, assignment or gameplay
catalogue authority.

The source search located 57 BossLever-related files. The comparison from the
pinned revision to the indexed revision
`82978a12cf8f5e2a53d66eaa8e68c0c41c9bb00c` contained 42 changed paths. All 19
changed or removed Lua files were also read at the donor pin; none supplied a
missing BossLever configuration. The capture contains 54 constructor-bearing
files, 39 complete literal layouts, and 37 of those with entirely literal/default
policy values. All 39 complete layouts are outside the current qualified source
map. Configurations with dynamic policies or script-generated placements remain
unqualified, even if a creature profile with the same name exists.

Lexicographic selection is by exact donor path. The first complete literal room
is [faceless_bane.lua:1–24](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/bosses_levers/faceless_bane.lua#L1-L24).
Its source SHA-256 is
`5e70a82af03c116923d5780d0cfa11310461df8456599eceb214884dc0e7d0f0`.

| Source fact | Exact captured value |
| --- | --- |
| Boss | Faceless Bane, `(33617,32561,13)` |
| Ordered player pads | `(33638..33642,32562,13)` |
| Ordered entry cells | All five are `(33617,32567,13)` |
| Room bounds | `(33607,32553,13)` through `(33627,32570,13)` |
| Exit | `(33618,32523,15)` |
| Minimum level / participants | 250 / 1 |
| Fight limit | 900 seconds |
| Default cooldown | 72,000 seconds |
| Default time after death | 60 seconds |

Defaults come from
[boss_lever.lua:63–76](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/libs/functions/boss_lever.lua#L63-L76)
and [config.lua.dist:658–659](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/config.lua.dist#L658-L659).
Repeated entry cells are preserved. The native group-transfer owner supports
their source-authored stacking; this implementation never invents neighboring
landing cells.

## Concrete missing map qualification

At the implementation base `80dca5fd1578d029f953f05690c6246a0236fd16`, the activated
source-world artifact is `imports/spells/r25/source-world.json`, selected by
`content/spells.manifest.json`. Its qualified source is Canary Thalom, revision
`99902524e052f37574194466c2949c576e4ab269`, with 2,833 tiles and source bounds
`[5840,5283,5871,5314,5,8]`. The admitted projection is origin `(5854,5298)` and
`native_floor=-source_z`, yielding native bounds `[-14,-15,17,16,-8,-5]`.

These are enforced qualifications, not an incidental small sample: the existing
[native_spell_world.rs:286–307](https://github.com/Oteryn/Oteryn-Game/blob/80dca5fd1578d029f953f05690c6246a0236fd16/crates/gameplay/content/src/content/project/native_spell_world.rs#L286-L307)
requires that exact source revision, bounds, temple, frame and tile count.
Faceless Bane and every other complete captured BossLever room lie outside them.
Moving a room to Thalom, cloning the origin map pin, or using an open-world raid
spawn would change the source and is refused.

## Concrete missing boss behavior qualification

Faceless Bane's monster registers `dreamCourtsDeath` and `facelessThink` in
[the donor monster:23–25](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/monster/quests/the_dream_courts/bosses/faceless_bane.lua#L23-L25).
The [quest callback source:4–53](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_dream_courts/creaturescripts_faceless_bane.lua#L4-L53)
implements the think transition, registers `facelessHealth`, and changes the
health/phase behavior. The currently compiled generic profile is not a complete
implementation of these hooks. A matching creature name or maximum HP cannot
qualify this boss. Unknown callback closures on later candidates also remain
explicitly unavailable.

## Runtime boundary implemented here

`ContentActivationController` compares the exact captured candidate, the active
artifact pair, current qualified map and source profile closure before issuing a
sealed `QualifiedActivityRoomTemplate`. A bounded map view removes cells and
mutable placements outside the authored room and retains the original source
frame and map identity. It does not copy an ordinary spawn schedule. The template
issues its runtime pin only while its parent activation sequence and digests
still match.

The production gameplay seam registers these sealed templates at boot. Command
178 reaches the existing activity factory, which checks the catalogue's complete
ordered definition, the origin's current content issuance and map/frame pin, and
an independently committed, ready, unconsumed Instance allocation. It creates
the scoped carrier, installs the real compiled creature policies and realizes
one qualified boss. It never issues an allocation, fabricates readiness, or
aliases the origin Channel. Its manual-runtime fixture escape exists only in
`cfg(test)`.

## Verification and Left out

The source capture tool is reproducible from the exact donor bytes and the
compiled creature-profile document. The content tests assert the exact Faceless
landing order and defaults, refusal of forged relocated source facts, the
current map's refusal of every fully literal candidate, and isolation of a
bounded map view. The shared PostgreSQL transport fixture separately exercises a
real activated Instance and the domain-99 scope barrier.

Left out of production activation: the exact Crystal room map import and
qualification, the complete bespoke boss callback closure, and consequently an
enabled production boss catalogue row. This change does not import a whole
world, select a substitute boss, or claim a playable production Faceless Bane
encounter. The existing World, Channel and Instance ownership distinctions are
preserved.

## Personal reward and physical chest qualification

The unchanged source inputs contain387 profiles with reward_boss=true, but none of their corpse Items is materializable in the pinned loot section. The existing CreatureRewardTable admission returns CorpseItemInadmissible; this task does not loosen that admission. Evidence: content/creatures/definitions/spell-native-profiles.json blob f23e6281c5a5418ca6b2186786834eb200c080d8 and tools/content-schema/native-gameplay/loot-tables.json blob aa55acfd71d1f85cfe02f06d4ccf3c5a65011ed8 at the task baseline.

The exact reward chest Item i19250 resolves in ItemDefinitionIndex, but content/items/definitions/spell-native-profiles.json has no native physical-policy record for it (789-record input; blob 8014ea99827859eb07c8110b00be7cfdaa0d14dd). Crystal 201189f data/items/items.xml:39545-39552 defines the rewardchest as non-movable, non-pickupable with 32 container slots. A complete production activation must qualify its physical policy and placement through the existing content owner.

Native reward PG fixtures classify the already admitted rat as a reward boss before compilation/activation, preserving its source HP, XP, corpse, loot table, probabilities and Item definitions. The fixture-preservation test also checks that the original source reward bosses still refuse. The physical Channel-wire fixture separately authors a static chest with the real indexed identity and source non-pickupable fact. Neither fixture creates a production fallback.

The actual Instance native lethal/settlement/custody scenario and the separate physical Channel browse/take scenario are intended to verify their respective native and durable paths; the task record retains their actual execution results. They do not establish a source-qualified production boss, or a combined physical Instance chest traversal in the source-room fixture. No active boss template is registered until its complete map, monster, callback and Item closure qualifies.

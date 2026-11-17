# Native monster callback source contract v1

SRX-3 W01c qualifies immutable callback descriptions from the current admitted
native Creature, Behavior and parent Ability bodies, the real full canonical
source-definition SHA-256 and the reviewed typed source specifications. The
bounded witness is rule data, not a reconstructed ProjectV2Draft and not scope,
actor, map, teleport, spawn or HP authority. Runtime consumers must separately
prove their current owner, exact actor incarnation, content/frame/map pin and
source occurrence. This document does not mark an unmounted consumer DONE.

## Callback child monster defaults

Pinned donor: Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`.
These are the sixteen concrete children named by the reviewed source spawn
callbacks. The current type's native HP must equal the donor or already accepted
wiki rule below; source constructors must not substitute a caller's HP or a
name-derived policy. No monster production profile is changed by this work.

| Child | Current default HP | Donor file:line | Difference |
| --- | ---: | --- | --- |
| `oteryn:creature.corrupted_soul` | 1100 | `data-otservbr-global/monster/quests/forgotten_knowledge/corrupted_soul.lua:16` (1100) | Donor match |
| `oteryn:creature.damned_soul` | 800 | `data-otservbr-global/monster/quests/ferumbras_ascension/damned_soul.lua:16` (800) | Donor match |
| `oteryn:creature.demon` | 8200 | `data-otservbr-global/monster/demons/demon.lua:29` (8200) | Donor match |
| `oteryn:creature.disgusting_ooze` | 3650 | `data-otservbr-global/monster/quests/ferumbras_ascension/disgusting_ooze.lua:16` (3650) | Donor match |
| `oteryn:creature.disruption` | 12000 | `data-otservbr-global/monster/quests/heart_of_destruction/disruption.lua:10` (12000) | Donor match |
| `oteryn:creature.energy_pulse` | 5000 | `data-otservbr-global/monster/quests/hero_of_rathleton/energy_pulse.lua:16` (5000) | Donor match |
| `oteryn:creature.frenzy` | 8210 | `data-otservbr-global/monster/quests/heart_of_destruction/frenzy.lua:16` (8210) | Donor match |
| `oteryn:creature.glooth_generator` | 12000 | `data-otservbr-global/monster/quests/hero_of_rathleton/glooth-generator.lua:10` (12000) | Donor match |
| `oteryn:creature.greed` | 1000 | `data-otservbr-global/monster/quests/heart_of_destruction/greed.lua:16` (1000) | Donor match |
| `oteryn:creature.lost_time` | 4000 | `data-otservbr-global/monster/quests/forgotten_knowledge/lost_time.lua:10` (2000) | Accepted wiki receipt below |
| `oteryn:creature.minion_of_gaz_haragoth` | 5500 | `data-otservbr-global/monster/quests/roshamuul/minion_of_gaz'haragoth.lua:16` (5500) | Donor match |
| `oteryn:creature.rage_of_mazoran` | 4500 | `data-otservbr-global/monster/quests/ferumbras_ascension/rage_of_mazoran.lua:16` (4500) | Donor match |
| `oteryn:creature.shadow_fiend` | 3200 | `data-otservbr-global/monster/quests/forgotten_knowledge/shadow_fiend.lua:16` (3200) | Donor match |
| `oteryn:creature.sin_devourer` | 2700 | `data-otservbr-global/monster/quests/ferumbras_ascension/bosses/sin_devourer.lua:16` (2700) | Donor match |
| `oteryn:creature.thorn_minion` | 3600 | `data-otservbr-global/monster/quests/forgotten_knowledge/thorn_minion.lua:16` (3500) | Accepted wiki receipt below |
| `oteryn:creature.time_waster` | 1000 | `data-otservbr-global/monster/quests/forgotten_knowledge/time_waster.lua:10` (1000) | Donor match |

- `oteryn:creature.lost_time`: Existing accepted Fandom capture: `tools/content-schema/monster-authoring/samples/wiki-population-2026-09-27.json`, Lost Time row, wiki line6, revision1083574, content SHA `ca833a17987fe3500536f43a8b533290534146c36632507bcaa969b368046f77`.
- `oteryn:creature.thorn_minion`: Existing accepted BR fill: `tools/content-schema/monster-authoring/samples/wiki-br-fill-2026-09-27.json`, Thorn Minion row, BR line3, revision400467, content SHA `338ab397bb756845ecf789845e362d7362938fc8c34eee378cc33a50babb0514`. Fandom health is unknown.

Fresh Crystal `201189f871d23597b82caf88fd611454a2ddefb8`
`data-global/monster/quests/forgotten_knowledge/lost_time.lua:10` also says2000;
`thorn_minion.lua:16` also says3500. Those donor values do not override the
existing accepted current-game wiki rules. New live wiki checks are wiki-pending.

## Top creature and source frame

Canary `src/items/tile.cpp:336-340` returns the first creature, while
`:1131-1143` and `:1764-1774` insert at the front. Therefore the top creature is
selected by actual physical tile entry order, including Player and NPC subclasses,
never ActorId order. A turn or same-cell write retains that order;
`src/game/game.cpp:3569-3571` rejects same-position teleport before map movement.
`src/creatures/creature.cpp:720-724` removes a dead monster after its death callbacks;
a retained corpse-settlement slot is not a live tile actor in the next native turn.
The canonical position owner keeps the ordering metadata in the existing boxed
actor state, with no second position map and no increase to the184-byte Slot.

Crystal `data-global/scripts/spells/monster/heal_brain_head.lua:2,5-15`
reads literal global tile `(31954,32325,10)`, emits its fixed source cues and heals
only its top creature named Brain Head by300..500. Matching XYZ on an unrelated
bundle does not prove that source anchor. The Brain Head capture and sealed room
role are documented in `NATIVE_ACTIVITY_SOURCE_ROOMS_V1.md`; generic bundle views
must not retain a source role. HP writes still use the existing current native
Creature heal owner and exact source occurrence replay memo.

## Retained legacy slots and their production owners

The native artifact additionally retains 33 schedule references without an
executable Ability body: Rat (one), Skeleton (two), and the five familiars (30).
These are a separate census from the 36 canonical callback slots above.
Namespace spelling alone does not exclude a slot from production.

Rat is reachable from `apps/game-server/src/qualification_wild_spawn.rs:103`.
Its admitted `monster_melee` entry retains the exact legacy schedule identity,
interval and chance. `monster_ai_cycle.rs` selects that existing physical-melee
owner when a native Ability body is absent; this is an explicit fallback, not an
implemented Lua callback. Ordinary world spawn definitions contain none of these
33 legacy references.

Skeleton is the Animate Dead acquisition exported by
`tools/content-schema/native-gameplay/build_skeleton_profile.py`.
`native_companion_item_cast.rs` calls the typed `prepare_animate_dead` owner in
`companion_lifecycle.rs`; installation binds the actual player master. The
native-profile scheduler accepts Ordinary and Intrinsic roles, whereas a
player-owned Skeleton is PlayerCompanion. This does not claim that every
player-companion spell mechanic is complete.

The five familiar profiles carry `summoning.is_familiar = true`. Their actual
acquisition installs a player master in `runtime_actor_companion.rs`, and
`current_source_creature_role` in `monster_summon.rs` classifies Familiar before
ordinary scheduling. Their defense effects have a distinct production consumer
in `familiar_defense.rs` using `CompiledFamiliarDefenses`. Familiar spell and
companion lifecycle work belongs to its existing owner; the generic boss callback
adapter must not grant these schedules Ordinary authority to bypass that owner.

This reachability receipt records the existing paths and scope boundary; it does
not turn the retained missing Ability bodies into implemented canonical spells.

## Fresh callback source inventory

All Canary paths below were fetched read-only at `47dfd51f45280a59a1d3e50ba7edd573d7234446`. The complete stated line interval is retained for rule review; this inventory is evidence, not a completion claim.

| Ability | Source file and lines | SHA-256 |
|---|---|---|
| `gaz_functions` | `data-otservbr-global/scripts/spells/monster/gaz_functions.lua:1-4` | `b0cd02bc4f06f549d23e8ec97e3c42906d6aaee5c03eb7bdb84240c5dd746f09` |
| `glooth_fairy_healing` | `data-otservbr-global/scripts/spells/monster/glooth_fairy_healing.lua:1-25` | `7781f1c8ea2d0879fd308adf0a5b7b324b2eed1b44c64f193c8564aa38a8aa81` |
| `lisa_heal` | `data-otservbr-global/scripts/spells/monster/lisa_heal.lua:1-27` | `69cc197d06ae863a01d449ea0692ec861f94bb09f686464e03db382ca7fa77ba` |
| `oteryn:ability.creature.foam_stalker.field-fill-foamsplash` | `data-otservbr-global/scripts/spells/monster/foam_splash.lua:1-57` | `349f2ee70d96cd3c77fae52ed2df251c824ab9f4673dcdf31d9c60244aaaf591` |
| `oteryn:ability.spell.destruction_summon` | `data-otservbr-global/scripts/spells/monster/destruction_summon.lua:1-34` | `90a7c07c79fb790c9a6293d43109d59b7daf60c152440b9ff2e9cb14e2a4c217` |
| `oteryn:ability.spell.devourer_summon` | `data-otservbr-global/scripts/spells/monster/devourer_summon.lua:1-26` | `7c2f2a85b1ac52a5ff58fd203eb920e993947b4c0832c1309e603ecb2d126e9d` |
| `oteryn:ability.spell.energy_pulse_explosion` | `data-otservbr-global/scripts/spells/monster/energy_pulse_explosion.lua:1-35` | `50fb41cf56f8e5f3cc7cb07720c85dbbae8c62375cce0a30ab5f497970cad5a9` |
| `oteryn:ability.spell.eruption_of_destruction_explosion` | `data-otservbr-global/scripts/spells/monster/eruption_of_destruction_explosion.lua:1-95` | `187faff452a85845e92026b8c4c7c1ea32cfc40ea9fe35f378d42c0947247edb` |
| `oteryn:ability.spell.gaz_haragoth_death` | `data-otservbr-global/scripts/spells/monster/gaz'haragoth_death.lua:1-90` | `6ef18d921d93baf9f4ac43266dae5817d4d4a22d02d7388423c1dce787d10d46` |
| `oteryn:ability.spell.gaz_haragoth_summon` | `data-otservbr-global/scripts/spells/monster/gaz'haragoth_summon.lua:1-49` | `db93fab130fdf02dd5d1697f2eb81b2058296e0ac0c8e09e8304a3349961f693` |
| `oteryn:ability.spell.generator` | `data-otservbr-global/scripts/spells/monster/generator.lua:1-22` | `5117addccc505a0a15b57aacf130dcee123102bb0a6f3c46ecc159c5efdac765` |
| `oteryn:ability.spell.glooth_generator_summon` | `data-otservbr-global/scripts/spells/monster/glooth-generator_summon.lua:1-22` | `e4027d8e41c9d6b036e21bd420430030b79ff6331e3aff38b5f28f2fbbde0223` |
| `oteryn:ability.spell.hunger_summon` | `data-otservbr-global/scripts/spells/monster/hunger_summon.lua:1-34` | `2a00ccf5a083a34776038a504726f4f33f54d82e864353e6bb140c8fcc6b9ce2` |
| `oteryn:ability.spell.mazoran_fire` | `data-otservbr-global/scripts/spells/monster/mazoran_fire.lua:1-967` | `4f8c27f8f13b59caae7355c600170e8f896a2e213637c5fa02e26595b730d9ea` |
| `oteryn:ability.spell.outburst_explode` | `data-otservbr-global/scripts/spells/monster/outburst_explode.lua:1-71` | `8fa2a802b7260b143d0a783bbee7be0fc197b01fa98f578a97d4de55451f9023` |
| `oteryn:ability.spell.plagirath_bog` | `data-otservbr-global/scripts/spells/monster/plagirath_bog.lua:1-67` | `f3b2b9b09f250270597faaeb15687c835f5cb872899c59706799c7ab86c17e2d` |
| `oteryn:ability.spell.plagirath_summon` | `data-otservbr-global/scripts/spells/monster/plagirath_summon.lua:1-23` | `4001f565d6c0e235f47fd3572d5fd92ae79737c894347be9b306ecef5f9b7f56` |
| `oteryn:ability.spell.rage_summon` | `data-otservbr-global/scripts/spells/monster/rage_summon.lua:1-23` | `39ce94ec2e36690b5c2359fb9cd4344fddcc25783001ef86b665ce6b1ff9d24f` |
| `oteryn:ability.spell.razzagorn_summon` | `data-otservbr-global/scripts/spells/monster/razzagorn_summon.lua:1-18` | `335894d9f4351b56c03323510e768f7718ee99f1f0020ed823e966d7a951ab89` |
| `oteryn:ability.spell.rotthingshaper` | `data-otservbr-global/scripts/spells/monster/rotthing_shaper.lua:1-55` | `c0f9b25d66219034bc027b822b8fdbeeace81aae707bef382dd72d72e84afdf0` |
| `oteryn:ability.spell.rotthingwave` | `data-otservbr-global/scripts/spells/monster/rotthing_wave.lua:1-50` | `c554ad43f68b7b68f60d11ca995fb37687f3322477a048a69ca9207575236084` |
| `oteryn:ability.spell.shulgrax_summon` | `data-otservbr-global/scripts/spells/monster/shulgrax_summon.lua:1-28` | `0285160685f876223064b173d1a7bd8abbefdde6a5071c8754cb70e900c9c5a6` |
| `oteryn:ability.spell.soulcatcher_summon` | `data-otservbr-global/scripts/spells/monster/soulcatcher_summon.lua:1-20` | `340e3dd0dfdaca35670a900b5fd0c2d539da0c02827b0d53492ace1758ed4b0c` |
| `oteryn:ability.spell.tenebris_summon` | `data-otservbr-global/scripts/spells/monster/tenebris_summon.lua:1-21` | `2667cf4d2994a00badd75da99d5ed2c02a6f80d24a8a9ff8f1f61c490f08da0e` |
| `oteryn:ability.spell.tenebris_ultimate` | `data-otservbr-global/scripts/spells/monster/tenebris_ultimate.lua:1-102` | `43fc74c1713787877c68415c6402b4d6de7298d2123687da18ee24e2d24d3949` |
| `oteryn:ability.spell.thorn_summon` | `data-otservbr-global/scripts/spells/monster/thorn_summon.lua:1-21` | `e507265f181812f28713a64fe787ca068944b860f1403cfe5492e23c24613028` |
| `oteryn:ability.spell.time_guardian_lost_time` | `data-otservbr-global/scripts/spells/monster/time_guardian_lost_time.lua:1-22` | `8b202aa6f12e9d72411fe5f4e20da29dfe210d7ee609c33286d6110970506d80` |
| `tyrn_heal` | `data-otservbr-global/scripts/spells/monster/tyrn_heal.lua:1-23` | `d7dc7e8a032c9e4471ace81b34fc799e0d86fdda88463b54086e30ca8b991755` |

The seven map/phase callback scripts were freshly fetched from Crystal `201189f871d23597b82caf88fd611454a2ddefb8`; their bytes match the existing reviewed Seven fixture despite its older metadata label.

| Source file and lines | SHA-256 |
|---|---|
| `data-global/scripts/spells/monster/heal_brain_head.lua:1-27` | `f43dead6738ef01a961778164a342b7f82664e5c0b0fdecd989cba5c18411eef` |
| `data-global/scripts/spells/monster/charge_vortex.lua:1-44` | `3e529d22f9c4f78960c45744b0caa4c98700e73d573c58425fa2c20845966db8` |
| `data-global/scripts/spells/monster/maxxen_teleport.lua:1-15` | `f77ac8c34f92afe1f7e0bb2b27e0e73a83efe92448ec23341475e02fdd2492fe` |
| `data-global/scripts/spells/monster/ragiaz_transform.lua:1-22` | `759d3885326cce17039788192093c6fcaee5a1672a68645dc75da19ace9d1587` |
| `data-global/scripts/spells/monster/time_guardian.lua:1-59` | `a426a55ae830a65f3aec0258d2630773998ad64229d7d64ca152ebd3b18d99aa` |
| `data-global/scripts/spells/monster/time_guardiann.lua:1-102` | `11837a0c512e8a0ec26279763c1d58d1ad5885df193a7e81672879f51017a08a` |
| `data-global/scripts/spells/monster/zamulosh_tp.lua:1-15` | `d4759dd2e9fafe0d3f85f465e41423fc7d1f333c8683124c282ea4dc1c1642cb` |

## Source rules that the runtime composition must preserve

Canary `foam_splash.lua:39-51` retains the original `var` in each of the three
1/2/3-second closures. Crystal `src/creatures/combat/spells.cpp:1293-1328`
creates VARIANT_POSITION for this non-targeted spell;
`src/lua/functions/creatures/combat/combat_functions.cpp:198-215` executes that
saved position. Later caster movement must not move the rings. The source-bound
position token retains geometry and exact cast identity; it grants no map facts,
HP authority or independent scheduling clock.

Canary `plagirath_bog.lua:33-40,48-58,65` retains the original target ID, applies
lookType299 for10seconds immediately, and at10seconds executes only while caster
and original target exist, caster is alive, and their current distance is below20.
Its VARIANT_NUMBER resolves the target's current position. Substituting the caster
as the delayed target changes the source rule.

Canary `data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_heart_minion_death.lua:7-15`
(SHA-256 `8b47831ca92d0d8ed2804c422994cf3529b90b6944cac997ec6171490a1e2de3`)
decrements Rage and Devourer counters on Frenzy death, and Destruction and
Devourer counters on Disruption/Charged Disruption/Overcharged Disruption death.
The final lever initializes the four counters at encounter entry. Source summon
callbacks increment their counter even when createMonster fails. A per-parent
successful-spawn counter cannot substitute for this shared encounter lifecycle.

### Shared regeneration marker and omitted Omrafir source slots

The marker is a real source `CONDITION_REGENERATION`, default condition ID,
sub-ID **88888**. A private threshold-owner timer is not equivalent to this
shared condition. Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446` uses it in
`data-otservbr-global/scripts/spells/monster/lisa_heal.lua:1-5` (6 s),
`glooth_fairy_healing.lua:1-5` (30 s), `tyrn_heal.lua:1-5` (900 s), and
`omrafir_beam.lua:1-5,105-108` (5 s). These four consumers are directly checked;
this list does not assert that unrelated donor scripts cannot use the same ID.
`src/lua/functions/creatures/combat/condition_functions.cpp:165-180` passes the
parameter through `getNumber<int32_t>`; `src/lua/functions/lua_functions_loader.hpp:215-232`
converts the Lua number with an integral `static_cast<T>`. The scripted `0.01`
health gain therefore becomes zero. The scripted callback, rather than the
regeneration condition, supplies the actual heal.

The current native Omrafir vector omits two registered source spell slots:
`data-otservbr-global/monster/quests/roshamuul/omrafir.lua:114` is the third attack,
`omrafir beam` (2 s, 15%); line 128 is the third defense, `omrafir healing 2`
(2 s, 20%). `build_srx3_omrafir_attacks.py` retains the exact original native
Creature/Behavior/attack closure and its canonical source binding. Its source
witness inserts only these two slots and preserves every original slot's order.
This is descriptive data; each actual callback still requires current actor,
content, owner, occurrence and map proofs.

`omrafir_beam.lua:105-129` adds the shared 5 s marker, then after 4 s re-reads the
same creature ID, current direction and current position. Its four literal
11-by-11 matrices contain 8, 8, 9 and 8 hit cells respectively. They must not be
replaced by a symmetric guessed beam. Crystal
`201189f871d23597b82caf88fd611454a2ddefb8`
`src/creatures/combat/combat.cpp:2456-2471,2518-2535` selects SOUTH for equal caster
and target positions and rotates the authored matrix by 180 degrees. The delayed
cast's source position variant is its current position, so this rotation matters.
`omrafir_healing_2.lua:3-16` draws 7500–9000 before checking HP below 99.99%,
requires an actual current tile and one of source Items 1487, 1492 or 1500, then
heals and emits Magic Blue plus the source monster speech. All new wiki checks
remain **wiki-pending**.

### Complete pinned marker-consumer census

The repository tool `build_regeneration_marker_audit.py` verifies every Lua path
and every C++/header path under `src/` against both complete pinned donor trees,
checks each Git blob identity and SHA-256, and refuses missing paths or unreviewed
uses. The generated receipt is
[`srx3-regeneration-marker-audit.json`](../../tools/content-schema/native-gameplay/srx3-regeneration-marker-audit.json).
It covers 13,566 Lua paths (8,626 unique blobs) and 933 C++/header paths
(929 unique blobs). Sixteen distinct Lua scripts construct or consume the shared
regeneration marker; no C++ source hardcodes 88888. Three other Lua uses belong
to Bakragore essence storage or a rum-barrel damage bound and are separate namespaces.
There are no outstanding fetch refusals.

All paths below use Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`,
relative to `data-otservbr-global/scripts/`; the receipt retains each exact
Crystal alias, blob, SHA-256 and source line. All markers have integer health gain zero.

| Source path and marker line | Lifetime |
| --- | ---: |
| `creaturescripts/monster/gaz_haragoth_heal.lua:2` | 7 s |
| `creaturescripts/monster/hirintror_heal.lua:2` | 900 s |
| `creaturescripts/monster/omrafir_explode.lua:2` | 5 s |
| `creaturescripts/monster/white_pale_heal.lua:2` | 900 s |
| `spells/monster/furyosa_heal.lua:2` | 900 s |
| `spells/monster/glooth_bomb_walkthrough.lua:2` | 10 s |
| `spells/monster/glooth_fairy_healing.lua:2` | 30 s |
| `spells/monster/hirintror_heal.lua:2` | 900 s |
| `spells/monster/lisa_heal.lua:2` | 6 s |
| `spells/monster/mawhawk_ue.lua:9` | 600 s |
| `spells/monster/minotaur_wallbreaker_walkthrough.lua:2` | 10 s |
| `spells/monster/omrafir_beam.lua:2` | 5 s |
| `spells/monster/terofar_curse.lua:3` | 900 s |
| `spells/monster/tyrn_heal.lua:2` | 900 s |
| `spells/monster/white_pale_heal.lua:2` | 900 s |
| `spells/monster/zushuka_heal.lua:2` | 900 s |

Omrafir is a direct same-actor interoperability case: its beam spell and
`creaturescripts/monster/omrafir_explode.lua` both check and install this same
five-second condition. Their guards must read the carrier's existing canonical
committed actor-condition store. Writing a threshold-private cooldown or the
legacy companion condition field does not satisfy this rule. A marker admission
does not itself qualify the omitted callback, its native schedule, its event
registration or any HP/map mutation.

### Gaz and generator source creation outcomes

Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`
`data-otservbr-global/scripts/spells/monster/gaz'haragoth_summon.lua:6-17`
counts every creature named `Minion of Gaz'haragoth` in the same-floor spectator
rectangle, 25 tiles each way. It does not count only the caster's owned summons.
`gaz_functions.lua:1-3` initializes the desired count to two and maximum to seven;
`gaz'haragoth_summon.lua:20-38` refills to the desired count, or uses the exact integer draw
`math.random(0, 100) < 25` and increments the desired count even if creation fails.
The source speech is inside each creation attempt, before its success test.

The calls at `gaz'haragoth_summon.lua:26,35` are on the newly created minion and
pass the unbound variable `sum`; the created local is named `monster`.
`data/libs/functions/creature.lua:105-108` returns false for `setSummon(nil)`.
The complete pinned Lua corpus has no nonlocal assignment establishing `sum`;
other executable assignments are lexical locals, including
`data-otservbr-global/monster/quests/the_order_of_lion/usurper_commander.lua:116-118`.
The C++ string literals `sum` are SQL column names, not Lua global registrations.
The minions therefore remain masterless under the checked source program. A
native adapter must not silently repair this into `caster:setSummon(new_minion)`.
The normal extended, non-forced source placement still needs actual current map
and occupancy facts.

`data-otservbr-global/scripts/spells/monster/glooth-generator_summon.lua:9-13`
shows that a charged Glooth Generator's 14-second closure always attempts Energy Pulse
creation, speaks, and removes the generator, including a failed creation result.
Successful-child-only speech/removal or a counter that advances only after
successful creation is not a source-equivalent outcome. The focused source-owner
regressions cover these distinctions before production composition is admitted.

The native source owner retains Gaz's desired count across exact boss
incarnations in the same physical runtime scope. Private Instances keep separate
counters under the existing accepted D26 `instance_per_party` decision
(`docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md:231`) and the explicit
isolation of local creatures, objectives and timers
(`docs/architecture/FND-03_RUNTIME_LIFECYCLE_FAILURE_AND_REPLAY_ANALYSIS_BASELINE.md:162`).
This is the recorded architectural difference from the donor process-global
Lua table, not a per-actor reset or a new cross-Instance state owner. No
production caller that transfers Gaz between runtime scopes has been identified;
no hypothetical cross-scope counter service is introduced.

### Marker spell reachability versus event counterparts

The generated audit v2 also scans every pinned Canary and Crystal monster file
for literal spell table references, retaining each Git blob, SHA-256, file and
line. It finds source attack/defense callers for only `lisa heal`, `tyrn heal`,
`glooth fairy healing` (Professor Maxxen) and `omrafir beam`. These correspond to
the existing three native threshold slots and the source-qualified Omrafir
restoration. Spell registration alone is not a native monster attack caller.

The other eight registered spell scripts have no such source monster vector
caller in either full pinned tree: `furyosa heal`, `glooth bomb walkthrough`,
`hirintror heal`, `mawhawk ue`, `minotaur wallbreaker walkthrough`, `terofar curse`,
`white pale heal` and `zushuka heal`. Their native attack/defense profiles do not
reference those spell names. They are not silently inserted into production
schedules or counted as implemented callbacks. The receipt is explicitly a
literal-vector census, not a claim that arbitrary dynamic Lua calls were proven
impossible. The native production scheduler consumes the qualified immutable
Behavior vectors, not a Lua spell-registration table.

This distinction does not exclude the independently registered Hirintror and
White Pale **prepare-death events**, or Gaz and Omrafir **think events**. In
particular, `omrafir_explode.lua` declares the regeneration condition but never
calls `addCondition`: its guard reads the marker installed by Omrafir Beam on the
same actor. Installing another private or shadow-store marker would change that
interaction.

### Native source creation options

The sealed callback spawn token preserves these exact Canary
`47dfd51f45280a59a1d3e50ba7edd573d7234446` calls. Paths below are relative to
`data-otservbr-global/scripts/spells/monster/`. Omitted Lua arguments mean
`extended=false, force=false`; they do not authorize forced placement.

| Parent callback | Donor file:line | Extended | Force |
| --- | --- | --- | --- |
| Gaz'haragoth | `gaz'haragoth_summon.lua:23,32` | true | false |
| Glooth Generator | `glooth-generator_summon.lua:9` | true | true |
| Lady Tenebris | `tenebris_summon.lua:8` | false | false |
| Mazoran | `mazoran_fire.lua:919` | true | true |
| Mounted Thorn Knight | `thorn_summon.lua:9` | true | false |
| Plagirath | `plagirath_summon.lua:8` | true | true |
| Professor Maxxen | `generator.lua:12` | true | true |
| Razzagorn | `razzagorn_summon.lua:5` | true | true |
| Shulgrax, both children | `shulgrax_summon.lua:7,14` | true | true |
| Soulcatcher | `soulcatcher_summon.lua:7` | true | true |
| Blazing Time Guardian | `time_guardian_lost_time.lua:11` | false | false |
| Freezing Time Guardian | `time_guardian_lost_time.lua:9` | false | false |
| The Destruction | `destruction_summon.lua:19` | false | true |
| The Hunger | `hunger_summon.lua:19` | false | true |
| The Rage | `rage_summon.lua:12` | false | true |
| World Devourer, three children | `devourer_summon.lua:15` | false | true |

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` `src/map.cpp:234–313`
first probes the requested tile. Force bypasses its query-add refusal only when
that tile actually exists. If the requested tile is absent, candidate fallback
still applies ordinary query-add checks; the force option grants no arbitrary
occupied fallback tile. The normal eight candidates, or the extended twelve
candidates with their separate first-four shuffle, retain the source ordering.
Fallback rejects teleports and leaving a protection zone when the requested tile
was protected; it may enter a protection zone from an unprotected requested tile.

### Registered prepare-death healing

Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`
`data-otservbr-global/scripts/creaturescripts/monster/hirintror_heal.lua:8–18`
and `white_pale_heal.lua:8–18` register real prepare-death events, independently
of the unused similarly named instant spells. Without the shared regeneration
sub-ID 88888 they install the 900-second marker, heal 1000 or 400, and return Lua
nil. With the marker they return true. The nominal 0.01 health gain is converted
to integral zero by the source condition API; it creates no regeneration ticks.

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` `src/game.cpp:8284`
invokes prepare-death before draining HP. `src/creatureevent.cpp:269` executes the
callback through the Lua boolean return path; nil is false. Consequently the
first fatal hit is canceled in full, and healing uses the original pre-hit HP,
clamped to the current native maximum. It is not a heal after subtracting the
fatal hit and does not fabricate damage credit. Marker admission, HP, and the
completed hit receipt must publish together under the existing canonical HP
owner. Canceled damage preserves canonical invisibility. A repeated occurrence
cannot heal again or refresh the marker, and a subsequent fatal hit while the
marker is present follows ordinary death processing.

The canonical marker retains the explicit donor health interval: 900000 ms for
the two prepare-death events and Tyrn, 6000 ms for Lisa, 30000 ms for Maxxen's
Glooth Fairy Healing. Zero gains still generate no tick debt. Omitted mana
parameters use canonical zero-gain defaults; no mana regeneration is invented.

Condition presence uses the same owner clock as canonical condition expiry.
Crystal `src/creatures/creature.cpp:1473–1492` and Canary
`src/creatures/creature.cpp:1585–1606` test the absolute end time when answering
`hasCondition`; they do not require a later condition-execution pass before a
marker becomes inactive. The existing canonical Oteryn condition contract uses
half-open microsecond lifetimes, expiring at the deadline, whereas the donor
integer-millisecond query includes equality at that boundary. Callback owners
reuse this existing condition representation and never extend the lifetime by
one native think or introduce a second marker clock.

Delayed callback preparation uses `OwnerTimerLane::drain_due_matching` under
the same current scope fence. A stage without current map/combat facts remains
pending with its original deadline, scheduling stamp and sequence. It is not
rescheduled and does not redraw source RNG. Stale target incarnations are
purged independently of readiness. For a skip-to-latest family only its latest
due occurrence can run; an unprepared latest occurrence cannot be replaced by
an older ready one. The ordinary drain delegates to this mechanism with every
stage ready, preserving its previous ordering and catch-up behavior.

### Outburst's registered phase events

Canary `monster/quests/heart_of_destruction/outburst.lua:60–63` registers
`OutburstCharge`, and `charging_outburst.lua:55–57` registers `ChargingOutDeath`.
The script paths below are under
`data-otservbr-global/scripts/quests/heart_of_destruction/` at the same pinned
Canary revision. `creaturescripts_outburst_charge.lua:19–33` snapshots the
original Outburst's current HP before its 80/60/40/20 percent phase transition.
It removes that actor, attempts four Spark births and a distinct Charging
Outburst, then advances the stage and resets the charging-killed flag to -1
(lines 1–9). Sparks retain their actual admitted native maximum of 3900.

`actions_outburst.lua:23–25` initializes stage 0 and stored HP 290000; it does
not reset the killed flag. `outburst_explode.lua:52–62` sets that flag to 1,
schedules the Charging actor's removal after one second, and immediately
creates Outburst at `(32234,31284,14)`. Its restored HP comes from the shared
stored number, normalized to zero if nonpositive. It never comes from the
Charging actor's HP. `creaturescripts_charging_out_death.lua:4–9` instead
restores Outburst at `(32234,31285,14)` only when the killed flag is below 1.
The two actor identities and the original HP sample remain distinct.

The bounded witness retains the actual native Outburst and Spark bodies,
behaviors and ability closures, plus exact event-script SHA-256 evidence.
Registered events have their own sealed source descriptions; they are not
invented Ability slots. The source number is stored beside a retained typed
native HP sample in the same scope-bound callback owner. Restoring HP delegates
to the existing canonical NULL-attacker HP owner, with no player contribution.

### Registered flags and the AI proposal target

`build_srx3_callback_registered_flags.py` records all 36 native callback slots
against verified donor Git blobs in
`docs/agents/evidence/srx3-native-callback-registered-flags.json`. The only
metadata difference is Plagirath: its opaque native marker says
`needs_target=false`, while the registered Lua spell explicitly calls
`needTarget(true)` at `plagirath_bog.lua:65`. Rotthing Wave and both Time
Guardian Lost Time slots already retain their registered direction flag; the
remaining flags match explicit setters or the InstantSpell defaults.

This difference does not require an AI metadata rewrite. The real native
scheduler (`ai_think/profile_schedule.rs`, attack recipient selection) always
retains the independently selected combat target in an Attack proposal; a
Defense proposal targets its issuer. The actual native Plagirath regression
passes before any projection change. Its callback retains the original Player
target for the delayed distance check and target variant. Non-targeted callback
stages normalize their VAR semantics inside the qualified callback owner, while
retaining the original proposal for validation and replay. No alternate target
is assigned to a supposedly original native proposal.

Opaque child-stage descriptions reuse the exact existing callback fixture,
including its full source-body SHA-256, and require the actual qualified parent
and exact Encounter body. They expose typed immutable damage, geometry, timing
and parent schedule data; they do not accept mutable SpellSource numbers as
source authority and do not manufacture a composed-attack profile.

### Prepare-death healing does not invent a revive effect

Canary `47dfd51f` `scripts/creaturescripts/monster/hirintror_heal.lua:9–15`
and `white_pale_heal.lua:9–15` install regeneration/subID88888, add 1000/400
HP and return nil. Neither script emits a magic effect or speech. Crystal
`201189f8` `src/lua/functions/creatures/creature_functions.cpp:591–609`
forwards positive `addHealth` through `Game::combatChangeHealth` with a NULL
attacker and healing damage. Its positive branch at
`src/game/game.cpp:7809–7918` publishes healing/HP messages; the sole magic
FX branch there requires `damage.critical`, which these callbacks do not set.
A fabricated blue revive effect would therefore change the donor rule. The
joint canonical HP/condition commit and its normal observation remain the
presentation source.

### Omrafir source frame and raid caller

The existing active `raid:roshamuul.omrafir` catalogue is the production birth
caller, rather than a static World spawn. Both pinned donors' raid XML places
Omrafir at 33587,32378,12 (`raids/roshamuul/omrafir.xml:6`, SHA-256
`63d8800d966c0b2eb6f9b800871995e5c10a9169a9d4ebbabd0285a05ff0c156`).
The published whole-World codec contains the actual Roshamuul geometry,
including all 441 tiles of the 21×21 neighborhood of 33586,32379,12. Its
qualified published-World frame projects literal source coordinates; no
synthetic activity scene or guessed global frame is required.

The two omitted source spell slots use a separate sealed Omrafir descriptor.
They have no invented canonical Encounter parent. The beam's retained callback
reads CURRENT direction and position after 4000 ms and selects its literal Lua
matrix once; the geometry owner must not rotate that chosen matrix again.
`omrafir_healing_2.lua:4–12` checks actual current source item IDs 1487/1492/1500,
not every field of the fire element. These are source stack predicates, not a
blanket portable Item materialization grant.

`creaturescripts/monster/omrafir_explode.lua:23–35` uses shared signed storage
112416 (missing value −1), a 10 ms boom and 26764 ms parent-independent reformation.
Both deadlines originate in the same actual onThink; a delayed boom does not
rebase the reform deadline. The native owner captures closed child-birth proofs
while the original actor exists, attempts all nine literal creations, and
removes that original after each attempt. The normal source storage updates
survive successive incarnations inside the same physical runtime scope.
Its terminal branch requests the literal missing monster type `Omrafir2`; both
pinned source trees contain no such registered monster definition. Preserve the
known nil creation followed by the script's nil-receiver error, explicitly,
rather than inventing a replacement profile or silently substituting Omrafir.

The Omrafir predicate retains literal source IDs, despite the donor naming: pinned
Canary `data/items/items.xml:3429–3434` declares 1487–1498 as stone wall and 1499–1500 as
stone wall window (SHA-256 `1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2`).
`omrafir_healing_2.lua:4–12` checks 1487/1492/1500, so ordinary fire never
qualifies. This records the donor mismatch; it grants neither portable wall
creation nor a substituted fire condition. Metadata uses `exact_ids`.

The reproducible final schedule/body census is
[`srx3-monster-callback-reachability.json`](../agents/evidence/srx3-monster-callback-reachability.json),
produced and checked by
`python3 tools/content-schema/native-gameplay/build_srx3_callback_reachability.py [--check]`.
It retains each of the 33 legacy schedule references and pinned current Canary
source file/line evidence. The current secondary authoring audit has 661
unavailable rows; that status alone neither blocks nor completes runtime callbacks.
All present native schedule bodies retain effects, variants or an Encounter.
The actual current Familiar/PlayerCompanion role and its dedicated defense owner
are traced separately from ordinary monster AI. The twenty-five familiar attack
slots and two Skeleton companion attack slots remain W01-relevant until a real
controlled-companion attack caller executes them. C13's DONE lifecycle/owner-end
receipt does not prove offense. S17 casting logic remains outside this change.

Native activity preserves source perception separately from an attack target.
Crystal `src/creatures/monsters/monster.cpp:648–690` keeps the visible target list
from multifloor spectators, while `:975–982` rejects a different-floor attack
target. `src/creatures/creature.cpp:47–65` permits underground spectators within
two floors, including the source floor offset in x/y. Parked Ragiaz forms may
therefore keep their actual onThink cadence without an eligible attack target;
absence of a same-floor target alone does not authorize freezing them.

The additional reproducible transitive CreatureEvent census is
[`srx3-creature-event-audit.json`](../agents/evidence/srx3-creature-event-audit.json),
produced by `build_srx3_creature_event_audit.py` from the two pinned Git trees and
Git-blob-verified Lua. It follows actual literal birth/transform targets and
source-defined dynamic alternatives, not unrelated names in a shared progression
table. Its implementation fields remain pending until a real production caller
is proved; source declarations alone are not completion evidence.

The event census separately marks `OmrafirExplode`, `HirintrorHeal` and
`WhitePaleHeal` as declarations without literal monster attachment. The pinned
monster Lua has no corresponding `monster.events`; the event names occur only
at their declaration scripts in the complete cached donor Lua. Crystal
`src/creatures/monsters/monster.cpp:63–69` attaches the MonsterType's actual
`scripts` list, and Canary `data/scripts/lib/register_monster_type.lua:332–337`
only adds the supplied `mask.events`. Generic `functions.lua:455–467` and
`encounters.lua:213–226` registration requires an explicit caller argument.
A restored native event engine is not proof that the original donor attached
it. An attachment override needs a separately recorded accepted source fix;
this audit does not invent one.

### Attached health events: origin and deferred death boundary

The attached `HealthForgotten`, `RuptureHeal` and `SoulcatcherSummon`
handlers are normal Combat health events. They must cover both signed
negative damage and positive healing; direct Lua `Creature:addHealth`
with origin NONE is a separate raw HP operation and does not call these
handlers. A damage multiplier belongs after blocking and before Wheel
adjustments. Applying it again in the final HP keeper would duplicate it.
The canonical staged Combat heal branch applies its event before clamping.

Canary47 `creaturescripts_healthchange_forgotten.lua:4-21` checks all
same-floor spectators in the inclusive seven-tile square, by lowercase
name only. It has no `isMonster` or health predicate. Shadow Tentacle
protects Lady Tenebris; Possessed Tree protects the three Thorn Knight
forms. Without that named spectator, only the primary component doubles.
A zero-HP creature is still a source spectator until its queued onDeath
completes; its continued physical slot while SQL drop materialization
settles must not extend that logical registration after completion.

Canary47 `creaturescripts_rupture_heal.lua:4-10` reads the actual shared
Rupture resonance flag. With flag one and an actual Player attacker, it
adds 5000..10000 HP before returning the original components unchanged.
It does not restrict the incoming component to negative damage.
`creaturescripts_soulcatcher_summon.lua:3-11` checks pre-event HP at most
49999, then rolls 15 percent and attempts the literal forced/extended
Corrupted Soul birth. An incoming lethal hit from HP50000 does not meet
that pre-event threshold.

These facts describe source requirements, not a completion claim. The
attached-event implementation and native production tests are tracked
separately. Ordered death notices must be reserved before canonical HP
publication, preserve the actual lethal seal, and begin before ordinary
reward qualification. At that beginning the CPP drop profile is frozen
before ordered onDeath callbacks; completing callbacks ends logical
source registration. Administrative disappearance is never a lethal proof.

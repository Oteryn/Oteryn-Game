# SRV7 cosmetic source population

Scope: S06 only. Current Tibia is the target; pinned donor rules are compatibility evidence,
not a claim that an OTS probability is independently proven for Global. Donor rule evidence
has source class `OTS_HYPOTHESIS_ONLY`. Existing structured TibiaWiki observations retain
that source class and their dated revision. No Store purchase or OT-only random doll is
introduced by the population.

The canonical admission/provisioning foundation is the relevant NX-5 slice from
`claude/happy-davinci-ijz3wy-srvpkg4` at
`5e60c19701fe3acdda0a3d1331da1239440a9d1e`. That branch has 14 outfits, eight mounts and
49 grant mappings; it does not populate the 42 distinct taming mounts. The imported
provisioning reader/materializer and descriptor-contract test are identified in the diff.
Other NX-1/SRX changes from that branch are not part of this import.

| Rule | Evidence and selection |
| --- | --- |
| Both sexes of all 136 current outfit identities | `docs/reference/content-prep/outfits-content-1/admission-evidence/evidence-00000-00135.json`, per-record exact male/female IDs; `imports/cipsoft-appearances/outfits/manifest.json` and its pinned source-record digests. `populate_srv7.py` binds each look to its exact source observation. |
| Starter outfits | Crystal `201189f871d23597b82caf88fd611454a2ddefb8:data/XML/outfits.xml`, each explicit `type`, `looktype`, `unlocked` flag. An admitted premium starter still requires the existing Premium owner for selection. No addon starts unlocked. |
| No-addon outfits | The same structured outfit evidence has empty addon requirements for retro/Jersey/newly-wed outfits and explicit unavailable addon facts for Monk. These admit zero addons rather than inventing grants. |
| Taming mount looks/speed/Premium | Crystal `201189f8:data/XML/mounts.xml`; each native mount's source citation contains its exact line and the complete source hash. Speed is the existing reference profile's 10. |
| Normal creature taming and Music Box | Existing `content/cosmetics/taming.json` retains its per-pair Canary/Crystal file:line citations, chance, failure actions and equipment prerequisites. Every pair gets an explicit canonical mapping with `mount-taming-v1` provenance. |
| Foxmouse refuses a cursed tamer | Crystal `201189f8:data-global/scripts/actions/mounts/mounts.lua:182-185`. Both the spell-owned and runtime-owned current curse stores are checked before spending anything. |
| Tin Key and Golden Can of Oil | Crystal `201189f8:data-global/scripts/actions/mounts/mounts.lua:40-41,235-254`; actual machine IDs are `data/items/items.xml:30010,31157`. `rand` is 1..100 and failure is `rand < 40`, so the executable donor has 61 successful rolls. This follows executable code rather than silently replacing it with the table's apparent intended 40%. |
| Glooth Glider construction | Crystal `201189f8:data-global/scripts/actions/mounts/mounts.lua:20-36,111-140`: blueprint 21897 plus covering 21898, gear wheel 21906, crank 21902, tubes/wires 21899, hinge 21905 and casing 21901, all consumed atomically. |
| Wind-up Loco | Crystal `201189f8:data-global/scripts/actions/mounts/mounts.lua:70,240-242` removes the actual 37398 target and consumes the 37397 source. Mount 194 is the existing canonical Gloothomotive, confirmed by `data/XML/mounts.xml:223`; the previous undefined `wind_up_loco` mount identity is corrected. |
| Native physical operands | `mount_kit_item_profiles.py` captures exactly pinned Crystal `data/items/items.xml` and `data/items/appearances.dat` through the existing source profile reader and independent binding qualifier. The two inoperative machines remain immovable Ground-only objects. All 43 distinct taming source items, the Gryphon Mask and the reachable addon-NPC costs (including Silver Token 22516) use the same source reader. Crystal `src/items/items.cpp:211-213` applies the mask's exact appearance clothing slot; `src/items/items_definitions.hpp:425` defines its HEAD bit. The active native admission is also carried into the death-loot provider for these exact inventory operands; 23 named taming drops were previously unmaterializable even though their loot entries existed. |
| Reachable NPC grants | `tools/content-schema/cosmetics-authoring/srv7-npc-grants.json` records six exact addon mappings, donor citations, source hashes and provenance. The NPC adapter owns costs, quest gates, duplicates and atomic grant execution. |

`tools/content-schema/cosmetics-authoring/srv7-grant-sources.json` binds the separate
item, quest, taming and reachable-NPC source tables. Their provenance remains independent.
The native authoring and descriptor are generated with the repository tools; source
observations and merged migrations are not edited.

Wiki check: `wiki-pending` — the MediaWiki API refused
`https://tibia.fandom.com/api.php?action=parse&page=Tin_Lizzard&prop=wikitext&format=json`
with HTTP 402. No bypass was attempted. The pinned GitHub donor fetches succeeded.

Validation evidence and exact PostgreSQL filters are reported with the final SRV7 diff;
this source parity record alone is not a DONE claim.

## Production acquisition closure

The populated catalogue has 136 outfits, 51 mounts and 99 explicit grant mappings.
The production grant writers are item cosmetics, creature/music-box taming, the six
reachable NPC addon rules, the two native direct quest rewards, and Tibiadrome.
The task-board entitlement writer is outside this population because the owner excludes
Prey Hunting Tasks. No generic quest callback or Store grant is enabled by this change.

All 46 official item-action rules resolve their 39 unique granted identities and eight
required base-outfit identities. The physical source overlay contains 106 distinct item
profiles. The 33 previously unmaterializable named official drops are now qualified:
12551, 12552, 12553, 12554, 12555, 12556, 23684, 23685, 27655, 27656, 27657,
32629, 32630, 32631, 34072, 34073, 34074, 34075, 34076, 35595, 35695, 36938,
37604, 37605, 39544, 39545, 43899, 43900, 44048, 50060, 50061, 50064, 50067.
The real death/mint caller rejected these source operands before population; the focused
regression exercises 12551, 44048 and 50064 without replacing production item facts.
The eleven official operands without named death loot remain dependent on their actual
quest/NPC/reward sources; no loot source is fabricated for them.

Spiritual Horseshoe 44048 has a closed candidate identity packet generated from nine
pinned donor files. Both XMLs omit its explicit item entry, but the pinned appearances
record names it and supplies its exact movable/takeable, nonstackable and light facts.
Crystal's item factory admits that appearance-backed ID; its default weight/charges/decay
are zero. The action consumes four horseshoes for mount 217. The consumer checks the
complete captured profile and packet SHA rather than adding a general numeric fallback.
TibiaWiki checks for Spiritual_Horseshoe returned 402 (Fandom) and 403 (tibiawiki.com.br):
wiki-pending. These donor defaults are recorded evidence, not a measured Global weight.

Phant uses Crystal `data/XML/mounts.xml:211`, mount 182/look 1417. The existing real
`durability/tibiadrome.rs` reward caller already asks for that look; its missing native
identity and explicit Drome grant mapping are now populated, alongside Lion of War.
No BOSS2 Rust owner is changed.

The two OT-only random doll actions (6579 and 37746) already fail closed: the existing
qualifier resolves the complete choice set before drawing, and their unsupported choices
have no production grant authority. Their real caller regression passed before any policy
change; this slice is ALREADY-DONE, with no extra OT rule introduced.

The permanent-NPC donor census covers 63 grant scripts among 1,156 pinned scripts.
Its intersection with the 50 static and 21 timed current-world placements is Amber,
Norma, Tom, Cledwyn and daytime Kesar: six addon rules. None of the 17 new Summer NPCs
adds a permanent cosmetic grant. Required base outfits remain required; Cledwyn does not
silently grant the Rift Warrior base.

The native direct quest callers grant Revenant in `quest_state/soul_war.rs` and Decaying
Defender in `quest_state/rotten_blood.rs`; both explicit base mappings already existed.
The quest-PC branch `6c09e0365110db6683f938dd08d5d584744cbf37` has the same callers.
Its `samples/native-binding-preflight/all352.json` explicitly reports zero native dispatch
and reward-delivery bindings and 128 base/addon policies requiring explicit native policy.
That authoring evidence does not establish implementation of those other quests. Held
world callbacks, including In Service of Yalahar, remain runtime-disabled; their separate
quest owner must activate their prerequisites and rewards. This population does not claim
that an unplaced NPC or a held quest became reachable through catalogue admission.

Focused official-source PostgreSQL proof: `srv7_official_cosmetic_sources_are_reachable_through_real_death_loot_mint` failed before population in142.425s (`loot_item_inadmissible`) and passed after population in122.494s (repeat114.920s inPG9b).

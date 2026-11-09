# questpc-n3 source completion contract

The frozen pc2 branch remains owned by the wpkg3 collector. This continuation
starts from main and reuses QREADY1's native source objects and existing
claim-obligation writer. No earlier branch is changed.

The first five bindings cover Adorned UH Rune, Barbarian Axe, Battle Axe, Blood
Herb and Captain Iglues Treasure. Each canonical definition is a standalone,
definition-ready reward-only quest with one personal once-claim identity.
Conservative Oteryn rule: its successful native claim starts and completes
that quest. Alternative placements of one claim retain their shared once
identity. The transition writes an exact0-to1 claimed flag and completes;
reward custody, obligation recovery and replay retain their existing owners.

The original source selectors, item identities, quantities and startup hashes
are retained in tools/content-schema/world-authoring/samples/quest-chest-bindings/qready1.json
and the original reward-use tests. These bindings add Oteryn lifecycle metadata,
not a new source reward or an invented NPC/monster requirement.

Quests with multiple required/alternative claims need a declared aggregate
completion policy. A fixed final transition with unmet prerequisites is unsafe:
quest_state.rs documents that refused obligations become terminal, rather than
waiting for another chest. Those quests remain WAITING-ENGINE with QENGINE2.

Full authored routes require the engine-owned Bindings::attach API and typed
NPC/USE/STEP_IN/kill trigger delivery. They exist on QENGINE2 source925d8480,
but are absent from main3b4696be; that branch also records pending post-merge
engine validation. Retain those delivery dependencies without duplicating the
shared engine or promoting historical reward-only tests to full-path proof.

Father's unchanged source acceptance is one eleven-track write. Current-main
real-PG reproduction stops at missing native material definition admission.
The NPC programme writer also hardcodes completes=false; source completion
needs the ENGNPC owner. The reproduction stays local until those dependencies
land. No incomplete Father test is published or counted as DONE.

M01b Elvenbane s1 chooses the canonical Area entry (32590,31645,floor -7),
not a guessed basement rectangle. The pinned Area source is
https://tibia.fandom.com/index.php?curid=4498&oldid=1113998 ; the selected
quest programme separately retains its spoiler source
https://tibia.fandom.com/index.php?curid=8007&oldid=265159 . Native B3 decoding
confirms ground436 and clear east/west approaches,house0. This callback only
records the first selected entrance visit. It does not claim s2's poison-floor
crossing, dragon defeat, reward delivery or whole-quest completion.

M01b Premium entrance choices: Edron Goblin s1 uses the canonical cave entry
(33108,31850,floor -7,ground355); Lion's Rock s1 uses the canonical trial
entrance (33138,32358,floor -7,ground4399). Retained B3 confirms clear east
approaches and house0. Both keep Current Premium admission, once-only visit
flags, no teleport/items/XP and no whole-quest completion effect. Sources:
https://tibia.fandom.com/index.php?curid=27120&oldid=748545
https://tibia.fandom.com/index.php?curid=69447&oldid=1192599

Koshei s1 counts four distinct chosen source-placement approaches,not four visits to one fragment. The pinned claims are in samples/chests/claims.json under canary:reward-claim/quest/u8_1/koshei_the_deathless/koshei_amulet_part1..4; sourcehttps://github.com/opentibiabr/canary/tree/04b83b51 . Each approach is Manhattan-adjacent on the same source floor and verified against retained B3. Native callbacks grant no fragment item or teleport; existing claim/assembly owners retain those steps. Current Premium and durablelevel40 remain required. The PGfixture reuses QENGINE's server positioning between disconnected floors;actual credits require nativeSTEP.

FireAxe s1 uses the exact Edron Dragon Lair Area point33098,31701,floor-7 (ground4409),sourcehttps://tibia.fandom.com/index.php?curid=19176&oldid=1114456 . The southern approach33098,31702 retainsground4407+decoration1855. Canonical Terrain declares4407/4409 walkable;WorldObject1855 explicitly does not block movement. Level60 andCurrent Premium remain required. No items,XP,teleport or whole-quest completion effect is introduced.

Supersedes FireAxe's previous entry geometry:the real native case failed StartNotEnterable55.015954915s. The revised chosen approach is33098,31703,-7,ground4408,entered fromWest33097,31703,-7,ground4406+border1103. Both terrain records are explicitly walkable and1103 is not an unwalkable border. The original Area marker33098,31701 remains unchanged;only this explicitly selected point has declared distance2. No native Item collision fact is weakened or invented. The callback still requiresCurrent Premium+level60 and grants only one entry visit. Reference evidence:docs/reference/content-prep/questpc-m01b-entry-approaches.md. DragonTower andBerserkPotion choose explicit distance1 Shadowthorn/Helheim approaches;these callbacks do not imply deeper-route or whole-quest completion.

Behemoth s1 uses theactual retainedBehemothfloor12,notCyclopolis surfaceUIpoint:sourcecontent/world/spawns/spawns-42000-43999.json#/records/110/declaration/points/3,creatureoteryn:creature.behemoth,cell33234,31695,z12. Chosen adjacententry33235,31695,-12 andEaststart33236,31695 areclear499ground inregion-z12-x129-y123.b3. The producer pins sourceframe,creature,origin,adjacency andquestownership. Entry stillrequireslevel80+Current Premium;no items,XP,teleport orwholecompletion.

FrostDragon s1 uses the clearSouthapproach32146,31431,-8 to its pinnedcorpse32146,31430,z8,which actuallyretainsappearance2473. StartEast32147,31431 isclear4404ground;entryground4407. This reaches thequest's actualfloor8lair ratherthan blocked surfaceUI/debris. Itscanonicalsource_refs explicitlyidentifycrystalserver:quest/frost_dragon_quest anddeclare the selectedCanary-keyedclaim;ownership ischecked againstthoseexactrefs,not guessedfromnamespace. Thecallbackonlyrecordsentry,retainsCurrentPremium,andgrantsno corpsecontents.

Fourstaticclaimapproaches select actualquestlocations without claimingrewards:IronHelmet32770,32225,-7,4516 besidecampcorpse;GoblinTemple31973,32210,-12,502 southofchest;Katana32177,32132,-9,355 besideKey4603cache BEFORElockedrewardroom;MinotaurHell32131,32066,-12,351 besideretainedcorpse. Starts areoneclear tileEast. Sourceclaims andexactcanonicalsource_refs verified;no key/item isgranted byexploration. Goblin retainsCurrentPremium. Nativecases assert0wholequestcompletions.

FishingBox/Berserker chosen first-cycle EXPLORE anchors use clear adjacent native ground beside their own declared claims, preserving all source placements. FishingBox is the Krimhorn cache route;Berserker is floor11 before subsequent kill credit. Entry does not invoke either reward claim. Existing retained claim JSON pointers and upstream source URLs are emitted by the generator;no reward/obstacle/collision policy changes.

DragonCorpse first EXPLORE uses the clear northern approach to canonical Rookgaard Bear Cave,not its later corpse/cache beyond shovel/scythe steps. The declared map origin remains unchanged. Ferumbras inactive source approach is evidenced by https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/data-otservbr-global/scripts/quests/ferumbras_ascension/movements_entrance.lua : uncompleted portal destination33275,32390,z8 and minimum150. Native east neighbour33276,32390,z8 appearance410 is retained;existing teleports/boss-access policies are untouched.

Ferumbras/Heart source approaches now active. Resolver rejects foreign owners,unprefixed aliases and changed canonical source revisions. The pinned script references are reviewable evidence for destinations;the chosen adjacent native grounds are explicit first-cycle entry approximations. Source teleports,access storage and boss completion remain separate.

GraveDanger chosen12visit stage counts7source sanctification-grave approaches plus5pre-boss antechamber approaches. Distinct source aliases yield distinct once-only visit flags and one counter bounded0..12. Entry grants no sanctification,boss kill or later-stage credit. Kilmaresh alone uses explicit source distance4 on clear outer ground469,keeping retained pillars/tomb objects and collision policy intact. Canonical level250+CurrentPremium retained;sourceURLtable in docs/reference/content-prep/questpc-m01b-entry-approaches.md.

Cookies s1 binds the clear north approach to the Key3800 chest,not the final cookie cache behind the subsequent chamber. No key/reward claim occurs on STEP. Canonical definition reuse retains duplicate-owner rejection and exact source membership checks;provided foreign identities are rejected. Generation output for all37pre-cookie callbacks is byte-equivalent under canonical JSON hashing.

World-entry binding reuses the admitted owner-catalogue key and existing fenced callback.world_change predicate. It refuses missing,held or ambiguous World-owner declarations;it never promotes an event or invents activation. Grimvale s1 requires the current committed full-moon decision plus CurrentPremium and an exact native island visit. It does not create Feroxa or replace the encounter-owned portal. The selected clear eastern approach avoids retained tree objects without changing collision rules.

A qualified World owner must have a nonempty exact string key. Null or empty keys are authoring errors,never an instruction to omit the World predicate. Native binding proofs also cannot substitute another event key for the admitted moon key. Current valid generated callbacks remain unchanged;realGrimvale PG proof61.823016566s covers the runtime condition.

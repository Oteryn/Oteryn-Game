# QMECH missing-mechanics index

**65 unique mechanics: 3 DONE, 39 CLAIMED, 22 OPEN, 1 OWNER. OPEN rows block 66 distinct named quests. Four additional NPC interaction groups have unmapped quest identities; all four are CLAIMED and are excluded from quest counts.**

Indexed all **32** `MISSING-*.md` reports and `CLAIMS.md` against qmech `84cbdfb9f6c6f7f1a68ade136a53d4b93cbb4cd3`. Counts describe reported stage/route blockers, not whole quests newly playable after a fix. Quest names are unioned across OPEN rows, never summed. Multiple engine capabilities can block one quest.

Deduplication and status decisions:

- Coordinator ASSIGN.md IDs are preserved. M01 merges its TALK/EXPLORE/USE-COLLECT-KILL work slices because they share the authored-binding capability; claim lists retain actual scoped owners. Suffixes split distinct capabilities inside combined assignment packages (M10, M29, M30, M38, M39, M41). New independently reported boundaries use M44–M57. Assignment-only work stays OPEN until explicitly claimed.
- ASSIGN M04 includes the five NPC finale/completion consumers; M07 is the shared offering/trial owner while generic route registration stays M01. Rock Steady/Mistrock content registration also stays M01 rather than inventing another grant/progress engine. ASSIGN M38 physical/presence/travel boundaries are split, retaining unknown quest mappings.
- Ownership overlap is retained in CLAIMS: engclaim still has an older achievement claim, while coordinator ASSIGN M41 and the newer explicit claim name wpkg. The index follows the coordinator-selected wpkg owner; the older claim is not deleted or silently rewritten. Other assignment-only suggestions are not new claims.

- A row represents a reusable capability/owner boundary, not a lane, quest, source spelling or occurrence count. Generic authored route/completion registration is grouped; concrete source-NPC terminal receipts, child completion, unordered claim sets and event-scoped progress remain distinct. Data/binding work reported as a blocker stays visible without claiming its underlying primitive is absent.
- `claim` follows explicit active claims in `CLAIMS.md`; IN PROGRESS is CLAIMED. Assignment categories, queue suggestions and historical lane work alone do not claim a mechanic; the explicit ASSIGN M28 override resolves the retained ENGENC2 OWNER row into that lane’s assigned active work. Claims apply to the scoped work described in that file, not automatically every additional consumer named here.
- SRVWLD and SEASON reported the same annual date-window capability: one M17 row, including current-year projection per ASSIGN; SEASON’s released duplicate remains preserved in CLAIMS. qready1’s text claim and qworld’s remove-only claim are RELEASED and confer no ownership. The source-corrected fungus transform is supported and is not another missing engine.
- Literal reward text is DONE because implementation `68d538460` is reachable in qmech (`git log -1 68d538460` plus ancestor check); stale questpc5 claim remains historical in CLAIMS. Native authored leaf/container admission is DONE at 2d88d5c14, but exact placements/quest wiring remain M01 and independent outputs remain M09. Teammate refusal fidelity is DONE at ed8f218de; the separate Instance corpse lifecycle stays CLAIMED. A shared encounter foundation, an uncommitted local PG success or an external lane head does not make the required production closure DONE.
- OWNER denotes the retained explicit Cake bridge48227 decision. ASSIGN M28 supersedes the historical Fugue OWNER label: ENGENC2 must source/implement it, so it is CLAIMED, not DONE or OWNER. NPCFIX’s four unmapped interaction groups retain their NPC names but contribute no invented quest names/counts. Existing/no-missing findings and environment prerequisites are excluded.

| id | mechanic | quests blocked (count + names) | reported by (lanes) | claim (lane or none) | state |
|---|---|---|---|---|---|
| M01 | Canonical authored stage/route binding and terminal callback registration (NPC, USE, STEP, kills and completion; includes content wiring) | 115 — A Piece of Cake; A Pirate's Death to Me Quest; A Pirate's Tail Quest; Adventures of Galthen Quest; Alawar's Vault Quest; An Interest In Botany Quest; An Uneasy Alliance Quest; Ancient Aucar Outfits Quest; Annual Autumn Vintage Quest; Asura Palace Quest; Awash World Change; Bank Robbery Mini World Change; Barbarian Arena Quest; Barbarian Test Quest; Bear Room Quest; Behemoth Quest; Berserk Potion Quest; Berserker Treasure Quest; Between the Lines Quest; Bewitched; Bigfoot's Burden Quest; Black Knight Quest; Blood Brothers Quest; Bloody Tusks Quest; Bored Mini World Change; Cartography 101 Quest; Cave Explorer Outfits Quest; Chakoya Iceberg Mini World Change; Child of Destiny Quest; Children of the Revolution Quest; Chyllfroest Mini World Change; Citizen Outfits Quest; Cults of Tibia Quest; Dangerous Depths Quest; Dead Archer Quest; Decaying Defender Outfits Quest; Deeplings World Change; Demon Wars World Change; Demon's Lullaby; Devovorga's Essence Mini World Change; Down the Drain Mini World Change; Draccoon Herald Outfits Quest; Draconia Quest; Dragon Corpse Quest; Dragon Slayer Outfits Quest; Dragon Tower Quest; Dreamer's Challenge Quest; Edron Goblin Quest; Elemental Spheres Quest; Elvenbane Quest; Emperor's Cookies Quest; Falconer Outfits Quest; Fanfare Quest; Feaster of Souls Quest; Feral Trappers Quest; Ferumbras' Ascension Quest; Fire Axe Quest; Fire from the Earth Mini World Change; Fishing Box Quest; Forgotten Knowledge Quest; Frost Dragon Quest; Fury Gates Mini World Change; Geomancer Quest; Ghoul Room Quest; Goblin Merchant Quest; Goblin Temple Quest; Grave Danger Quest; Grimvale Mini World Change; Grimvale Quest; Heart of Destruction Quest; Hero of Rathleton Quest; Hive Born World Change; Hive Outpost Mini World Change; Horse Station World Change; Illuminated Warrior Outfits Quest; Illuminator Outfit Quest; Illuminator Outfits Quest; In Service of Yalahar Quest; Insectoid Invasion World Change; Insectoid Outfits Quest; Inukaya Quest; Iron Helmet Quest; Jungle Camp Mini World Change; Katana Quest; Kilmaresh Quest; Kingdom Of Kormarak Quest; Kingsday Mini World Change; Kissing a Pig Quest; Koshei the Deathless Quest; Krailos Quest; Last Creep Standing; Lion Trophy Quest; Lion's Rock Quest; Liquid Black Quest; Longsword Quest; Lumberjack Mini World Change; Machinery of War Quest; Mad Mage Room Quest; Magic Sword Quest; Make Believe Quest; Mastermind Potion Quest; Measuring Tibia Quest; Medusa Shield Quest; Minotaur Hell Quest; Mystery Of The Valley Quest; Outfit and Addon Quests — Deepling weapon addon; Realm of Dreams Quest; Small Ruby Quest; The Ancient Tombs Quest; The Cradle of Monsters Quest; The Hidden City of Beregar Quest; The Order of the Stag Quest; The Rookie Guard Quest; Too Hot to Handle Quest; Treasure Hunt Quest | [engclaim](MISSING-engclaim.md), [engworldchg](MISSING-engworldchg.md), [npcfix](MISSING-npcfix.md), [questpc-pc2](MISSING-questpc-pc2.md), [questpc4](MISSING-questpc4.md), [SEASON](MISSING-SEASON.md), [srvgap2](MISSING-srvgap2.md) | qengine2 / questpc / questpc3 / npcfix / engoutfit | CLAIMED |
| M02 | Exact mutable Ground stack/custody/quantity/order prerequisites | 1 — Nomads Land Quest | [qengine2](MISSING-qengine2.md), [questpc5](MISSING-questpc5.md) | ENGNPC2+SRV7 | CLAIMED |
| M03 | Fenced live blessing/promotion/Premium world callback/STEP admission predicates | 2 — 25 Years of Tibia Quest; The Pits of Inferno Quest | [qengine2](MISSING-qengine2.md) | srvenc | CLAIMED |
| M04 | Canonical terminal completion in the native source-NPC receipt | 5 — A Father's Burden Quest; Djinn War Efreet Quest; Djinn War Marid Quest; Marlin Trophy Quest; White Raven Monastery Quest | [questpc-pc2](MISSING-questpc-pc2.md), [WPKG](MISSING-WPKG.md) | engnpc | CLAIMED |
| M05 | One source-NPC item/points/StorageTime/SetNowPlus receipt | 1 — Dangerous Depths Quest | [ENGNPC2](MISSING-ENGNPC2.md), [questpc2](MISSING-questpc2.md) | none | OPEN |
| M06 | Atomic child-quest completion participant on parent World reward receipt; preserve parent completion | 1 — Waterskin of Mead Quest | [questpc4](MISSING-questpc4.md) | questpc4 | CLAIMED |
| M07 | Shared entrance/trial offering and reset owner | 2 — Dreamer's Challenge Quest; Lion's Rock Quest | [questpc4](MISSING-questpc4.md) | none | OPEN |
| M08 | Fenced live level/vocation/Premium eligibility in source NPC item programmes | 3 — Elemental Spheres Quest; Hot Cuisine Quest; The Hidden City of Beregar Quest | [ENGNPC](MISSING-ENGNPC.md), [ENGNPC2](MISSING-ENGNPC2.md), [questpc2](MISSING-questpc2.md), [questpc4](MISSING-questpc4.md), [questpc7](MISSING-questpc7.md) | questpc2 | CLAIMED |
| M09 | Atomic independent root Item rewards, including source-NPC multi-grant planning/bridge (no invented bag) | 5 — An Interest In Botany Quest; Bigfoot's Burden Quest; The Rookie Guard Quest; Tower Defence Quest; Treasure Hunt Quest | [ENGNPC2](MISSING-ENGNPC2.md), [questpc-pc2](MISSING-questpc-pc2.md), [questpc2](MISSING-questpc2.md), [questpc4](MISSING-questpc4.md) | qready2 | CLAIMED |
| M10a | Atomic source-NPC carried/bank payment plus item hand-in/reward and quest progress | 4 — In Service of Yalahar Quest; Koshei the Deathless Quest; The Demon Oak Quest; The Scatterbrained Sorcerer Quest | [ENGNPC2](MISSING-ENGNPC2.md), [QNPCFX](MISSING-QNPCFX.md), [questpc2](MISSING-questpc2.md), [questpc4](MISSING-questpc4.md) | qnpcfx | CLAIMED |
| M10b | Fenced source-NPC magic-effect/visual receipt | 2 — The Demon Oak Quest; The Inquisition Quest | [ENGNPC2](MISSING-ENGNPC2.md), [questpc2](MISSING-questpc2.md) | qnpcfx | CLAIMED |
| M11 | Live current-Character-name comparison in fenced source NPC dialogue | 1 — The Dream Courts Quest | [ENGNPC2](MISSING-ENGNPC2.md), [questpc2](MISSING-questpc2.md) | ENGNPC2 | CLAIMED |
| M12 | Repeatable World Item MINT scoped to an admitted preserved quest-cycle epoch | 1 — Oramond Quest | [ENGITEM](MISSING-ENGITEM.md) | engitem | CLAIMED |
| M13 | Exact live-creature / fresh native Ground-corpse quest-tool sample, paint, capture and transformation with occurrence credit | 3 — Formorgar Mines Quest; Goblin Merchant Quest; The Ice Islands Quest | [ENGNPC2](MISSING-ENGNPC2.md), [qengine2](MISSING-qengine2.md), [questpc7](MISSING-questpc7.md) | none | OPEN |
| M14 | Quest-guarded exact map-object taming with atomic Item consumption, mount entitlement and progress | 1 — Targuna Quest | [ENGITEM](MISSING-ENGITEM.md), [questpc5](MISSING-questpc5.md), [SRV7](MISSING-SRV7.md) | SRV7 | CLAIMED |
| M15 | Literal readable text through NPC/World quest reward MINT | 1 — Nomads Land Quest | [qready1](MISSING-qready1.md), [questpc5](MISSING-questpc5.md), [SRVWLD](MISSING-SRVWLD.md) | questpc5 | DONE 68d538460 |
| M16 | Committed personal NPC quest map marker coupled to progress and client delivery | 1 — Targuna Quest | [qengine2](MISSING-qengine2.md), [questpc5](MISSING-questpc5.md) | none | OPEN |
| M17 | Transaction-clock UTC calendar context: inclusive annual NPC date windows and current-year bonus projection | 2 — Hot Cuisine Quest; The First Dragon Quest | [questpc-pc2](MISSING-questpc-pc2.md), [questpc3](MISSING-questpc3.md), [SEASON](MISSING-SEASON.md), [SRVWLD](MISSING-SRVWLD.md) | SRVWLD | CLAIMED |
| M18 | Player-admitting timed encounter, personal cooldown and cleanup | 1 — The First Dragon Quest | [questpc3](MISSING-questpc3.md) | SEASON | CLAIMED |
| M19 | Transaction-clock world-callback elapsed-time / SetNow operations | 2 — Bewitched; Threatened Dreams Quest | [qengine2](MISSING-qengine2.md), [questpc3](MISSING-questpc3.md) | qready1 | CLAIMED |
| M20 | Quest inventory-item/world-tool multi-ingredient and multi-output crafting / collection-container consequences | 3 — The Cradle of Monsters Quest; The Cursed Crystal Quest; The Scatterbrained Sorcerer Quest | [questpc4](MISSING-questpc4.md) | none | OPEN |
| M21 | Native nested map-container pickup/reset with exact numbered-key custody | 3 — Skeleton Decoration Quest; Small Ruby Quest; The Outlaw Camp Quest | [questpc4](MISSING-questpc4.md) | npcfix2 | CLAIMED |
| M22 | Capture whole target-stack quantity in carried transformations | 1 — The Tainted Souls Quest | [questpc3](MISSING-questpc3.md) | none | OPEN |
| M23 | Quest-selected temporary item-look disguise, chosen portal predicate and exact exit restoration | 2 — Last Creep Standing; Wrath of the Emperor Quest | [questpc5](MISSING-questpc5.md), [questpc8](MISSING-questpc8.md) | srvgap1108 | CLAIMED |
| M24 | Source-bound native encounter death objectives with frozen personal/room/defender recipients and replay-safe progress | 6 — A Piece of Cake; Heart of Destruction Quest; The Ancient Tombs Quest; The Order of the Stag Quest; The Primal Ordeal Quest; Tower Defence Quest | [questpc4](MISSING-questpc4.md), [SEASON](MISSING-SEASON.md), [SRVENC](MISSING-SRVENC.md) | SRVENC | CLAIMED |
| M25 | Qualified BossLever configured exit teleporter and named native callback closure | 1 — Adventures of Galthen Quest | [srvgap1108](MISSING-srvgap1108.md), [SRX3-BOSS2](MISSING-SRX3-BOSS2.md) | boss2 (SRX3-BOSS2) | CLAIMED |
| M26 | Qualified multi-constructor boss rooms, zone/extra-monster closures and computed source cooldowns | 2 — Rotten Blood Quest; Soul War Quest | [srvgap2](MISSING-srvgap2.md) | srvgap2 | CLAIMED |
| M27 | Production phase-bound boss admission/death routing and durable participant settlement (foundation alone is insufficient) | 3 — Cults of Tibia Quest; In Service of Yalahar Quest; The Cradle of Monsters Quest | [ENGENC2](MISSING-ENGENC2.md), [questpc4](MISSING-questpc4.md) | ENGENC2 | CLAIMED |
| M28 | Authoritative native Fugue combat profile (and dependent encounter settlement) | 1 — The Order of the Lion Quest | [ENGENC2](MISSING-ENGENC2.md) | ENGENC2 | CLAIMED |
| M29a | Instance Ground corpse temporal stage/deadline receipts and draining | 1 — The Primal Ordeal Quest | [ENCX](MISSING-ENCX.md) | encx | CLAIMED |
| M29b | Preserve teammate boss-entry prerequisite refusal in the player command result | 1 — The Primal Ordeal Quest | [ENCX](MISSING-ENCX.md) | encx | DONE ed8f218de |
| M30a | Successful spear-hit / magic-missile-hit quest credit from ordinary combat | 1 — Child of Destiny Quest | [ENGENC](MISSING-ENGENC.md) | engenc | CLAIMED |
| M30b | Participant-scoped admitted defender deaths open the corresponding chamber door | 1 — Asura Palace Quest | [ENGENC](MISSING-ENGENC.md) | engenc | CLAIMED |
| M31 | Admitted world-change activation/variant/spawn membership and source escape/raid/stable/report/reset lifecycle | 6 — Hive Outpost Mini World Change; Horse Station World Change; Insectoid Invasion World Change; Jungle Camp Mini World Change; Kingsday Mini World Change; Lumberjack Mini World Change | [engworldchg](MISSING-engworldchg.md) | engworldchg | CLAIMED |
| M32 | World-change predicates on personal native-death counter registration/admission | 3 — Fury Gates Mini World Change; Grimvale Mini World Change; Their Master's Voice World Change | [qworld](MISSING-qworld.md) | qworld | CLAIMED |
| M33 | Phase-discriminated world STEP callback selection for a shared physical selector | 1 — Feaster of Souls Quest | [questpc7](MISSING-questpc7.md) | none | OPEN |
| M34 | Bounded native_authored_v1 RewardClaim placement admission and explicit leaf/container allocation (multi-root outputs are M09) | 4 — Kingdom Of Kormarak Quest; Magic Sword Quest; Mastermind Potion Quest; Mystery Of The Valley Quest | [engclaim](MISSING-engclaim.md) | engclaim | DONE 2d88d5c14 |
| M35 | Unordered required/alternative RewardClaim-set completion; do not finish on the first chest | 26 — Alawar's Vault Quest; Bear Room Quest; Behemoth Quest; Black Knight Quest; Circle Room Quest; Crystal Wand Quest; Demona Ring Quest; Double Hero Quest; Draconia Quest; Dragon Tower Quest; Dreamer's Challenge Quest; Edron Goblin Quest; Elvenbane Quest; Emperor's Cookies Quest; Fanfare Quest; Ferumbras' Ascension Quest; Fire Axe Quest; Forgotten Knowledge Quest; Ghoul Room Quest; Goblin Temple Quest; Griffin Shield Quest; Katana Quest; Koshei the Deathless Quest; Longsword Quest; Mad Mage Room Quest; Minotaur Hell Quest | [questpc-pc2](MISSING-questpc-pc2.md) | none | OPEN |
| M36 | Source-NPC outfit/addon delivery and authoritative outfit snapshot (generic callback grants already exist) | 1 — In Service of Yalahar Quest | [engoutfit](MISSING-engoutfit.md), [questpc4](MISSING-questpc4.md) | engoutfit | CLAIMED |
| M37 | Atomic source-NPC teleport with fenced dialogue progress, admitted destination and command replay | 2 — In Service of Yalahar Quest; The Paradox Tower Quest | [PGSPEED](MISSING-PGSPEED.md), [questpc4](MISSING-questpc4.md) | PGSPEED | CLAIMED |
| M38a | Conditional event NPC presence and admitted return binding | unknown — quest mapping pending; Rascalio; Ghost Captain; Hoaxette; Minzy — NPC event availability/return | [npcfix](MISSING-npcfix.md) | npcfix | CLAIMED |
| M38b | Source-qualified stationary/water NPC cell admission without relaxing ordinary collision | unknown — quest mapping pending; Yselda; Zarifan; A Swan; Eustacio's Spy Rat; ... — exact-cell interactions | [npcfix](MISSING-npcfix.md) | npcfix | CLAIMED |
| M38c | Fenced conditional passage permission/fine/ticket and last-port return state | unknown — quest mapping pending; Captain Jack; Petros — conditional passages | [npcfix](MISSING-npcfix.md) | npcfix | CLAIMED |
| M38d | Admitted Anna tutorial continuation to Targuna | unknown — quest mapping pending; Anna — tutorial continuation | [npcfix](MISSING-npcfix.md) | npcfix | CLAIMED |
| M39a | NPC reward offers admitted independently of current topic (any_topic) | 1 — Tomes of Knowledge Quest | [questl2](MISSING-questl2.md) | questl2 | CLAIMED |
| M39b | NPC reward inventory quantity-range/absence eligibility and explicit refusal dialogue | 1 — Into The Bone Pit Quest | [questl2](MISSING-questl2.md) | questl2 | CLAIMED |
| M40 | Native diagonal STEP command support across server and client | 1 — Unnatural Selection Quest | [questl2](MISSING-questl2.md) | none | OPEN |
| M41a | Achievement delivery composed with source-NPC progress/item rewards | 2 — Kilmaresh Quest; The Order of the Stag Quest | [engclaim](MISSING-engclaim.md), [QNPCFX](MISSING-QNPCFX.md), [questpc4](MISSING-questpc4.md) | wpkg | CLAIMED |
| M41b | Source-provenance Item hand-in/pickup credit composed with personal quest completion | 2 — The Ancient Tombs Quest; The Order of the Stag Quest | [questpc4](MISSING-questpc4.md) | wpkg | CLAIMED |
| M42 | Dispatch existing UTC cooking planner with typed ingredient exchange and terminal completion | 1 — Hot Cuisine Quest | [questpc7](MISSING-questpc7.md) | questpc7 | CLAIMED |
| M43 | Selected carried USE against another live player’s fenced quest state | 1 — The Colours of Magic | [questpc8](MISSING-questpc8.md) | questpc8 | CLAIMED |
| M44 | Revision-fenced source world-object transform/reset/regrowth lifecycle | 7 — An Uneasy Alliance Quest; Children of the Revolution Quest; Dreamer's Challenge Quest; Kissing a Pig Quest; Lion's Rock Quest; Machinery of War Quest; The Cursed Crystal Quest | [questpc-pc2](MISSING-questpc-pc2.md), [questpc4](MISSING-questpc4.md) | none | OPEN |
| M45 | Conditional multi-cell lever readset/toggles | 1 — Children of the Revolution Quest | [questpc-pc2](MISSING-questpc-pc2.md) | none | OPEN |
| M46 | Source-authorized public player speech from a quest action | 1 — Children of the Revolution Quest | [questpc-pc2](MISSING-questpc-pc2.md) | none | OPEN |
| M47 | Replay-bound random quest-item outcomes | 1 — Barbarian Test Quest | [questpc-pc2](MISSING-questpc-pc2.md) | none | OPEN |
| M48 | Live drunk-condition mammoth/quest action admission | 1 — Barbarian Test Quest | [questpc-pc2](MISSING-questpc-pc2.md) | none | OPEN |
| M49 | Item-arrival-triggered multi-altar readset, corpse consumption and boss summon in one fence | 1 — Elemental Spheres Quest | [questpc-pc2](MISSING-questpc-pc2.md) | none | OPEN |
| M50 | Cross-floor elevator batch Item movement and displacement of every platform occupant | 1 — Goblin Merchant Quest | [questpc-pc2](MISSING-questpc-pc2.md) | none | OPEN |
| M51 | Credited arena deaths compose source pillar removal/next-pit portal with exclusive pit lease and kick/reset | 1 — Barbarian Arena Quest | [questpc-pc2](MISSING-questpc-pc2.md) | none | OPEN |
| M52 | Source Feroxa occurrence selection/RNG and boss lifecycle (full-moon gate alone is insufficient) | 1 — Grimvale Mini World Change | [questpc-pc2](MISSING-questpc-pc2.md) | none | OPEN |
| M53 | Locked live necklace-slot prerequisite in source NPC dialogue | 1 — Blood Brothers Quest | [questpc-pc2](MISSING-questpc-pc2.md) | none | OPEN |
| M54 | Source-saga completion scoped to mainland/Rook/source aliases under a shared canonical quest | 1 — Citizen Outfits Quest | [questpc-pc2](MISSING-questpc-pc2.md) | none | OPEN |
| M55 | Occurrence-scoped personal quest progress/completion for repeatable world events (distinct from reward-MINT epochs) | 18 — Awash World Change; Bank Robbery Mini World Change; Bored Mini World Change; Chakoya Iceberg Mini World Change; Chyllfroest Mini World Change; Deeplings World Change; Demon Wars World Change; Devovorga's Essence Mini World Change; Down the Drain Mini World Change; Fire from the Earth Mini World Change; Fury Gates Mini World Change; Hive Born World Change; Hive Outpost Mini World Change; Horse Station World Change; Insectoid Invasion World Change; Jungle Camp Mini World Change; Kingsday Mini World Change; Lumberjack Mini World Change | [questpc-pc2](MISSING-questpc-pc2.md) | none | OPEN |
| M56 | Exact self-target quest Item consumption and protective condition consequence | 1 — The Cursed Crystal Quest | [questpc4](MISSING-questpc4.md) | none | OPEN |
| M57 | Owner-approved Cake bridge identity 48227 | 1 — A Piece of Cake | [questpc-pc2](MISSING-questpc-pc2.md) | none | OWNER |

## Source-row traceability

Every positive report row maps below; a compound row can contribute to more than one capability. The original reports retain their exact quest-step and code evidence. No original lane report was edited.

- M01: `MISSING-SEASON.md:7`; `MISSING-engclaim.md:11,12,13,14`; `MISSING-engworldchg.md:9,10,17,18`; `MISSING-npcfix.md:7`; `MISSING-questpc-pc2.md:11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,42,44,45,46,47,48,49,50,52,54,55,56,57,58,59,60,61,62,63,64,65,66,67,68,69,70,71,72,73,74,75,76,77,78,79,80,82,83,84,85,86,87,88,90,91,92,93,94,95,96,97,98,99,100,101,102,103,104,105,106,107,108,109,110,111,112,113,115,116,117,118`; `MISSING-questpc4.md:8,11,12,13,16,18,19,21,22,23,25`; `MISSING-srvgap2.md:5`.
- M02: `MISSING-qengine2.md:15`; `MISSING-questpc5.md:8`.
- M03: `MISSING-qengine2.md:11,12`.
- M04: `MISSING-WPKG.md:11,12,13`; `MISSING-questpc-pc2.md:9,114`.
- M05: `MISSING-ENGNPC2.md:29`; `MISSING-questpc2.md:7`.
- M06: `MISSING-questpc4.md:20`.
- M07: `MISSING-questpc4.md:21,22`.
- M08: `MISSING-ENGNPC.md:9`; `MISSING-ENGNPC2.md:28`; `MISSING-questpc2.md:8`; `MISSING-questpc4.md:7`; `MISSING-questpc7.md:7`.
- M09: `MISSING-ENGNPC2.md:25`; `MISSING-questpc-pc2.md:15,30`; `MISSING-questpc2.md:9`; `MISSING-questpc4.md:16,19`.
- M10a: `MISSING-ENGNPC2.md:27`; `MISSING-QNPCFX.md:9`; `MISSING-questpc2.md:10`; `MISSING-questpc4.md:10,17`.
- M10b: `MISSING-ENGNPC2.md:27`; `MISSING-questpc2.md:10`.
- M11: `MISSING-ENGNPC2.md:26`; `MISSING-questpc2.md:11`.
- M12: `MISSING-ENGITEM.md:10`.
- M13: `MISSING-ENGNPC2.md:21,22,23,24`; `MISSING-qengine2.md:14`; `MISSING-questpc7.md:10,11`.
- M14: `MISSING-ENGITEM.md:9`; `MISSING-SRV7.md:7`; `MISSING-questpc5.md:10`.
- M15: `MISSING-SRVWLD.md:7`; `MISSING-questpc5.md:7`.
- M16: `MISSING-qengine2.md:16`; `MISSING-questpc5.md:9`.
- M17: `MISSING-SRVWLD.md:29`; `MISSING-questpc-pc2.md:89`; `MISSING-questpc3.md:9`.
- M18: `MISSING-questpc3.md:10`.
- M19: `MISSING-qengine2.md:13`; `MISSING-questpc3.md:12`.
- M20: `MISSING-questpc4.md:13,14,17`.
- M21: `MISSING-questpc4.md:15,24,25`.
- M22: `MISSING-questpc3.md:11`.
- M23: `MISSING-questpc5.md:11`; `MISSING-questpc8.md:8`.
- M24: `MISSING-SEASON.md:7`; `MISSING-SRVENC.md:9,10,11,12`; `MISSING-questpc4.md:12`.
- M25: `MISSING-SRX3-BOSS2.md:7`; `MISSING-srvgap1108.md:8,9`.
- M26: `MISSING-srvgap2.md:6,7`.
- M27: `MISSING-ENGENC2.md:5`; `MISSING-questpc4.md:10,13`.
- M28: `MISSING-ENGENC2.md:6`.
- M29a: `MISSING-ENCX.md:7`.
- M29b: `MISSING-ENCX.md:8`.
- M30a: `MISSING-ENGENC.md:8`.
- M30b: `MISSING-ENGENC.md:7`.
- M31: `MISSING-engworldchg.md:11,12,13,14,15,16`.
- M32: `MISSING-qworld.md:7`.
- M33: `MISSING-questpc7.md:9`.
- M34: `MISSING-engclaim.md:11,12,13,14`.
- M35: `MISSING-questpc-pc2.md:14,24,25,31,41,43,51,53,56,59,60,61,63,64,66,69,70,73,77,79,81,99,103,109,112,117`.
- M36: `MISSING-engoutfit.md:5`; `MISSING-questpc4.md:10`.
- M37: `MISSING-questpc4.md:9,10`.
- M38a: `MISSING-npcfix.md:17`.
- M38b: `MISSING-npcfix.md:18`.
- M38c: `MISSING-npcfix.md:19`.
- M38d: `MISSING-npcfix.md:20`.
- M39a: `MISSING-questl2.md:11`.
- M39b: `MISSING-questl2.md:12`.
- M40: `MISSING-questl2.md:13`.
- M41a: `MISSING-QNPCFX.md:10`; `MISSING-engclaim.md:46,47`; `MISSING-questpc4.md:8`.
- M41b: `MISSING-questpc4.md:8,12`.
- M42: `MISSING-questpc7.md:8`.
- M43: `MISSING-questpc8.md:7`.
- M44: `MISSING-questpc-pc2.md:16,39,102,107`; `MISSING-questpc4.md:14,21,22,23`.
- M45: `MISSING-questpc-pc2.md:39`.
- M46: `MISSING-questpc-pc2.md:39`.
- M47: `MISSING-questpc-pc2.md:23`.
- M48: `MISSING-questpc-pc2.md:23`.
- M49: `MISSING-questpc-pc2.md:62`.
- M50: `MISSING-questpc-pc2.md:78`.
- M51: `MISSING-questpc-pc2.md:22`.
- M52: `MISSING-questpc-pc2.md:82`.
- M53: `MISSING-questpc-pc2.md:32`.
- M54: `MISSING-questpc-pc2.md:42`.
- M55: `MISSING-questpc-pc2.md:20,21,34,37,40,48,49,52,54,71,75,86,87,88,94,98,101,110`.
- M56: `MISSING-questpc4.md:14`.
- M57: `MISSING-questpc-pc2.md:10`.

## Existing paths and exclusions

- 24 explicit “none / no missing generic grant” source rows are not missing mechanics: PGSPEED’s three inspected reward slices, qready1’s 17 chest quests, qready2’s 18-title packet row and engoutfit’s three existing generic callback-grant findings. They are not full-quest completion certifications.
- Existing equipped-head predicates, sixteen-track capacity, native arena advancement, private boss loot and source ItemOnTarget reusable/target_replacement support are not re-added as missing mechanics merely because reports mention their finished transfer. The missing actor/corpse/room-specific adapters above remain separate.
- NPCFIX’s missing exact Platform checkout is an environment prerequisite, not a missing engine. Unresolved recipe reward/form/book identities mentioned as data caveats are not converted into additional guessed engine capabilities.

DONE implementation ancestry verified for this index:

- M15: `68d5384607d90afd6851e097c917e28bc9eae898 ENGITEM: literal_readable_reward_text (needed by Nomads Land Quest)`; implementation file list checked with `git show --stat`; this is a code commit on qmech, not a report-only or external branch commit. This documentation task does not claim a fresh runtime/PG execution.
- M29b: `ed8f218dec2a056f886bb0e418c5666cdaf034ed ENCX: transfer tested endgame party refusal fidelity`; implementation file list checked with `git show --stat`; this is a code commit on qmech, not a report-only or external branch commit. This documentation task does not claim a fresh runtime/PG execution.
- M34: `2d88d5c14b022fd8d92e918b5b0274ab44508b89 feat(engclaim): bind native authored once reward allocations`; implementation file list checked with `git show --stat`; this is a code commit on qmech, not a report-only or external branch commit. This documentation task does not claim a fresh runtime/PG execution.

# QMECH mechanic claims

| mechanic | lane | migrations | quests it unblocks | state |
|---|---|---|---|---|
| Atomic child-quest completion participant in a native world-reward receipt; preserve parent quest ownership/completion | questpc4 | 2060 (coordinator ASSIGN range 2060-2069; replaces unshipped 1259 draft) | Waterskin of Mead Quest (final personal completion) | CLAIMED |
| Fenced live-player quest predicate for selected carried USE | questpc8 | none planned; own range remains 1300–1309 | The Colours of Magic | CLAIMED |
| Atomic multiple independent Item reward outputs in one once-only quest receipt (no invented reward container) | qready2 | 2100; coordinator M09 range 2100–2109 supersedes old packet | The Rookie Guard Quest; Treasure Hunt Quest; Tower Defence Quest (M09 reward-output boundaries) | CLAIMED |
| cults_boss_phase_closures: wire shared native phase owner into production admission/deaths and durable settlement | ENGENC2 | none; reserved range 1800–1819 | Cults of Tibia | CLAIMED |
| Fugue native combat profile source missing from checked native catalogue | ENGENC2 | none | The Order of the Lion | OWNER: need an authoritative creature profile; do not invent combat data |
| M15 — Literal readable text on NPC/World quest Item rewards through the existing MINT owner | questpc5 | none; reuse ENGITEM migration 1641 already on qmech | Nomads Land Quest (readable reward boundary only; other route gaps remain) | DONE 68d538460; fresh PG 51.497609265s; see M15-questpc5.md |
| M16 — Personal NPC dialogue map marker, durable server receipt and native client projection | questpc5 | 2070 reserved; owner-assigned range 2070–2079 only | Targuna Quest (Sterling return/treasure marker) | CLAIMED |
| Literal readable text on quest-reward root Items through the existing fenced mint owner (NPC/world bridge) | qready1 | none | Nomads Land Quest | RELEASED: questpc5 claimed first during push race; no code written |
| M10: NPC carried/bank payment + item hand-in + magic effect (reuse existing value owners) | qnpcfx | none planned; reserved 2120-2129 only | Koshei the Deathless Quest; The Demon Oak Quest; The Inquisition Quest | CLAIMED |
| Source NPC quest teleport composed with fenced dialogue progress, admitted destination and command replay | PGSPEED | 2220 reserved; own QMECH range 2220–2229 | The Paradox Tower Quest (Riddler); In Service of Yalahar Quest (NPC teleport steps only; other blockers remain) | CLAIMED |
| Native authored RewardClaim placement/allocation admission through shared binder | engclaim | none; 1830–1839 reserved if required | Kormarak, Magic Sword, Mastermind Potion, Mystery of the Valley (engine prerequisite; exact native bindings remain) | PARTIAL: native placement + leaf/container allocations verified; independent outputs depend on qready2; quest bindings remain |
| Quest-guarded object taming with atomic quest progress | SRV7 | none planned; retained own range 0992–0993 only | Targuna Quest (Reliable Ram step; preceding stages remain separate) | CLAIMED |
| M24 — source-bound native boss/monster death quest progress, exact room recipients and personal kill counters | SRVENC | none used; coordinator-assigned QMECH range 2180–2189 only (prior 1108 reservation released) | Heart of Destruction; The Order of the Stag; Tower Defence; The Primal Ordeal (Magma Bubble); A Piece of Cake (20 Cake Golems); Ancient Tombs (seven pharaoh stages) | CLAIMED |
| Quest-cycle-scoped repeatable World Item MINT using a preserved, fenced local epoch (no command-created cycles or fabricated price) | engitem | none for M12; new assignment range2250–2259 | Oramond Quest (repeatable item reward boundary) | CLAIMED — engine slice01b524d4d GREEN (PG40.462554022s); production content wiring continuing |
| Annual UTC month/day window for source NPC dialogue, derived from the transaction clock | SRVWLD | none planned; retained range 1110–1119 | The First Dragon Quest (Tamoril Jan 14–Feb 12 opening only; other encounter/binding gaps remain) | CLAIMED |
| M23: item-look disguise and return through the existing condition/appearance owner | srvgap1108 | none planned; coordinator QMECH range 2260–2269 only | Wrath of the Emperor; Last Creep Standing | CLAIMED |
| Inclusive annual NPC dialogue date-window predicate evaluated from the authoritative transaction clock | SEASON | none planned; retained range 1160–1169 only | The First Dragon Quest (Tamoril seasonal dialogue admission; encounter remains separate) | RELEASED: SRVWLD published the same claim first in3e73a1cf7; local alternative remains unpublished |
| M11: live current-Character-name matcher in fenced source NPC dialogue | ENGNPC2 | none planned; binding QMECH range 2290–2299 only | The Dream Courts (Stricken Soul access branch; later stages remain separate) | CLAIMED |
| Remove-only native map-target consequence for reusable carried-tool quest USE, atomic with progress and replay | qworld | none planned; retained range 1320–1329 | Their Master's Voice World Change (fungus-cleaning stage) | RELEASED: pinned donor Gobbler_onUse transforms target to12065; existing target_replacement supports it, so remove-only is unnecessary |
| M32: world-change conditions on the existing personal native-death counter adapter, with authored Fury/Werewolf registrations | qworld | none planned; coordinator QMECH range2110–2119 | Fury Gates Mini World Change (10 Fury); Grimvale Mini World Change (10 Werewolf) | DONE 40eeb6445919f2c396d5661f1ec406154c3c6a87; preceding personal stages remain M31 bindings |
| Canonical-key achievement grant participant in fenced source NPC progress/item receipts | engclaim | none planned; retain own range1830–1839 | The Order of the Stag (Eldoran news reward); Kilmaresh (Sculptor Apprentice finale, programme binding also needed) | RELEASED: coordinator ASSIGN.md assigns M41 to wpkg; engclaim executes M34 then M35 |
| Selected personal Cake Golem kill objective binding with Biff-start admission through the existing native kill/QuestState owners | SEASON | none planned; retained range1160–1169 only | A Piece of Cake (s1 admission and s2 personal20-kill progress only; later stages remain separate) | RELEASED: coordinator ASSIGN transfers Cake Golem to SRVENC/M24; no Cake code written |
| Rock Steady Deepling collection source: persisted NPC stages, exact hand-ins and weapon-addon grant | srvgap2 | none planned; retained range 1760–1779 | Outfit and Addon Quests — Deepling weapon addon source chain | RELEASED: coordinator assigns this source to ENGOUTFIT/M36; no implementation written |
| M36 — NPC outfit/addon grants and real outfit snapshot | engoutfit | none planned; 1820–1829 reserved | In Service of Yalahar; Rock Steady / Deepling | CLAIMED |
| M19 — authoritative transaction clock in world callbacks (reuse existing quest clock) | qready1 | none planned; QMECH range 2090–2099 only | Bewitched; Threatened Dreams | CLAIMED |
| M29 — Primal Ordeal Instance corpse stage/deadline and party prerequisite refusal | encx | 2200; coordinator-assigned 2200–2209 only | The Primal Ordeal | DONE 7ed4a83d688452aff1519757b115864e9a898ad2; physical Instance deadline PG 15.890s, four corpse PG cases 32.91s, Channel takeover PG 5.39s; party refusal regression transferred in ed8f218de |
| M39: NPC any_topic offers and inventory quantity/absence predicates with refusal dialogue | questl2 | none planned; owner allocation 2020–2029 reserved | Tomes of Knowledge; Into The Bone Pit | CLAIMED |
| M01b: imported-route EXPLORE native stage binding | questpc | none planned; only 2000–2009 if required by this assignment | 46 explore routes in MISSING-questpc-pc2.md, per ASSIGN.md | CLAIMED |
| M08 — fenced live level/vocation/Premium predicates in source NPC programs | questpc2 | none planned; owner-assigned QMECH range 2050–2059 only | Elemental Spheres; Hot Cuisine; The Hidden City of Beregar | CLAIMED |
| M01c imported-route: native USE / COLLECT / KILL stage bindings | questpc3 | 2030–2039 reserved; none allocated yet | Ancient Aucar Outfits Quest; Black Knight Quest; Dead Archer Quest; Decaying Defender Outfits Quest; Draconia Quest; Dreamer's Challenge Quest; Falconer Outfits Quest; Fanfare Quest; Feral Trappers Quest; Grimvale Quest; Illuminator Outfits Quest | CLAIMED |
| M31 — native world-change activation, thresholds, raid bindings and journal/recipe lowering | engworldchg | 1840 reserved; explicit task allocation1840–1859 retained | All10 quests in MISSING-engworldchg.md | CLAIMED |
| M30 — native spear/magic-missile hit events and participant-scoped defender death/door progression | engenc | 2240 reserved; only 2240–2249 for this assignment | Child of Destiny; Asura Palace | CLAIMED |
| M21 — nested map containers: pickup/reset and captured key provenance | npcfix2 | 2150 reserved if required; assigned range2150–2159 only | Outlaw Camp; Skeleton Decoration; Small Ruby | CLAIMED |
| M25 — BossLever Ahau configured exit teleporter and Mitmah Vanguard callback closure | boss2 (SRX3-BOSS2) | none planned; 2171–2172 reserved within 2170–2179; 2170 is the existing boss2 cleanup | Adventures of Galthen (s5 Ahau; s8 Mitmah Vanguard) | CLAIMED |
| M18: timed player-admitting encounter with personal cooldown and cleanup | SEASON | 2210 reserved if schema work is needed; coordinator range2210–2219 | The First Dragon Quest (selected final encounter) | CLAIMED: source audit in docs/reference/content-prep/qmech-season/M18.md; reuse existing Activity owner where applicable |
| M38: conditional NPC presence, stationary/water admission, returns/tickets and Mistrock binding | npcfix | 2140–2149 reserved by coordinator; none allocated yet | Five checked rows in MISSING-npcfix.md, including The Hidden City of Beregar — Way to Beregar | CLAIMED |
| M42: dispatch existing UTC cooking planner and typed ingredient operations | questpc7 | 2080 reserved for typed cooking input/repeat-reward proof; M42 range 2080–2089 | Hot Cuisine | CLAIMED |
| M41 — achievement in the NPC transaction and corpse-provenance-bound hand-ins | wpkg | 2130 reserved; only 2130–2139 permitted | Kilmaresh; The Order of the Stag finale | CLAIMED |
| M26: qualified Soul War and Rotten Blood boss rooms with source-faithful Bakragore cooldown | srvgap2 | none allocated yet; reserve 2270–2279 only | Soul War Quest; Rotten Blood Quest | CLAIMED |
| M01a — imported-route native TALK stage bindings through preflight and server consumer | qengine2 | none planned; assigned QMECH range 2040–2049 | 44 TALK quests listed in MISSING-questpc-pc2.md; ASSIGN.md M01a | CLAIMED |
| M04 — canonical quest completion in final native NPC hand-in receipt | engnpc | 2230 reserved if required; own QMECH range 2230–2239 | A Father's Burden; Marlin Trophy; Djinn War Efreet; Djinn War Marid; White Raven Monastery | CLAIMED |

<!-- QMECH original layout history; every displaced structural line is retained verbatim below.

# QMECH mechanic claims

| mechanic | lane tag | migration numbers | quests it unblocks | state |
|---|---|---|---|---|
# QMECH mechanic claims

| mechanic | lane tag | migration numbers | quests it unblocks | state |
|---|---|---|---|---|
# QMECH claims

| mechanic | lane | migrations | quests it unblocks | state |
|---|---|---|---|---|
























-->

M38 progress: first working server slice `8cc2aaf7e` supplies exact-occurrence conditional NPC projection with real PG RED233.289ms/GREEN355.733ms. Parent M38 remains CLAIMED until event/content wiring and the other assigned rows are complete.

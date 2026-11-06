# NPCFIX report

Branch: `claude/happy-davinci-ijz3wy-npcfix`; base `ed788c4bb1e2b420c8bfbd7a9bd68e36ea8ebfeb`. AUTHORING, local changes only. No staging, commits, pushes, resets, checkout, stash, credential access, dependency downloads or external publication performed.

Audit inventory: 320 rows (288 DIFF, 32 MISSING). Historical/test-server references excluded only where the audit says so; Dawnport/W14/P23/X01-X06 are outside the selected audit rows. Current result counts: 24 REMAINING, 270 AUTHORED; QUALIFICATION PENDING, 24 EXCLUDED, 2 SUPERSEDED.

External WIP snapshots were observed during this continuation; current HEAD is `c40970106f519b3742c0d594ea772b601f6435c1`. This worker invoked no Git mutation. All package diffs/file inventories use the original task base above.

This is a partial package while any eligible row below remains unresolved. Authored source presence is distinguished from a running production actor; no live deployment was performed.

## Per-NPC results

| npc | town | audit status | fix | file:line | test | result |
| --- | --- | --- | --- | --- | --- | --- |
| A Sweaty Cyclops | Ab'Dendriel | DIFF | Retail placement authored. Placement is repaired. The audit leaves exact retail smithing eligibility/effects unresolved; verified quest exchanges must be routed to the excluded quest owner. | content/world/npc-placements/placements-00000-00339.json:1#/records/12 | test_npcfix_retail_population_has_each_audited_cell | REMAINING |
| Alkestios | Ab'Dendriel | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/21 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Elvith | Ab'Dendriel | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/95 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Olrik | Ab'Dendriel | DIFF | Retail placement authored.  Ordinary Letter8/Parcel15; completed Postman5/10 through the current-rank trade writer. | content/world/npc-placements/placements-00000-00339.json:1#/records/248; content/services/trade/trade-00000-00326.json:1#/records/221 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Ahmet | Ankrahmun | DIFF | Retail placement authored. Oil-filled Vial20 is not executable: fluid mint provenance and subtype trade support need the existing fluid owner contract; this package adds placement only. | content/world/npc-placements/placements-00000-00339.json:1#/records/14 | test_npcfix_retail_population_has_each_audited_cell | REMAINING |
| Alesar | Ankrahmun | DIFF | Retail placement authored. 29 retail offers, including a full-state Ice Rapier purchase; existing Efreet mission03=3 gate enforced in window and durable trade. | content/world/npc-placements/placements-00000-00339.json:1#/records/19; content/services/trade/trade-00000-00326.json:1#/records/6 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npcfix_postgres_dispatch_services_and_replay | AUTHORED; QUALIFICATION PENDING |
| Brutus | Ankrahmun | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-brutus.md:1 | Source review | EXCLUDED |
| Fitzgerald | Ankrahmun | MISSING | Retail placement authored. Add exact displayed identity. | content/world/npc-placements/placements-00000-00339.json:1#/records/116; content/npcs/definitions/npcs-00000-00499.json:1#/records/440 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Hercule | Ankrahmun | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-hercule.md:1 | Source review | EXCLUDED |
| Malor | Ankrahmun | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/210 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Enpa Rudra | Blue Valley | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/104 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| A Helpful Fairy | Boss Lairs | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-a-helpful-fairy.md:1 | Source review | EXCLUDED |
| Rascalio | Boss Lairs | DIFF | Nimmersatt event-conditioned Banor's Eye transport needs an admitted event selector and return route. | .npcaudit/research/rascalio.md:1 | Source review | REMAINING |
| Yselda | Boss Lairs | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/325 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Captain Jack | Carlin | DIFF | Retail placement authored. Tibia return depends on Costello permission and monastery fines; no qualified native predicate. Quest-lane repair required. | content/world/npc-placements/placements-00000-00339.json:1#/records/49 | test_npcfix_retail_population_has_each_audited_cell | REMAINING |
| Cerdras | Carlin | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/58 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Liane | Carlin | DIFF | Retail placement authored. Ordinary Letter8/Parcel15; completed Postman5/10, with current durable rank validation. | content/world/npc-placements/placements-00000-00339.json:1#/records/192; content/services/trade/trade-00000-00326.json:1#/records/181 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npcfix_postgres_dispatch_services_and_replay | AUTHORED; QUALIFICATION PENDING |
| Padreia | Carlin | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/260 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Altar | Darashia | DIFF | Retail placement authored. Crimson Offering/Bakragore eligibility and reward transaction require the excluded quest lane. | content/world/npc-placements/placements-00000-00339.json:1#/records/23 | test_npcfix_retail_population_has_each_audited_cell | REMAINING |
| Chemar | Darashia | DIFF | Retail placement authored.  Ordinary Letter8/Parcel15; completed Postman5/10 through the current-rank trade writer. | content/world/npc-placements/placements-00000-00339.json:1#/records/64; content/services/trade/trade-00000-00326.json:1#/records/60 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Elyen Ravenlock | Darashia | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/96 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Kasmir | Darashia | DIFF | Retail placement authored. Heal to40 and cure fire/poison/energy through the canonical vitals/condition owner. | content/world/npc-placements/placements-00000-00339.json:1#/records/182; crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs:760 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npcfix_postgres_dispatch_services_and_replay | AUTHORED; QUALIFICATION PENDING |
| Muzir | Darashia | DIFF | Retail placement authored. Connect existing native bank service and source dialogue. | content/world/npc-placements/placements-00000-00339.json:1#/records/226 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Nah'Bob | Darashia | DIFF | Retail placement authored. 32 retail offers including default-state Ice Rapier buyback; existing Marid mission03=3 gate enforced in window and durable sale. | content/world/npc-placements/placements-00000-00339.json:1#/records/229; content/services/trade/trade-00000-00326.json:1#/records/207 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npcfix_postgres_dispatch_services_and_replay | AUTHORED; QUALIFICATION PENDING |
| Oblivion | Darashia | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/244 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Omrabas | Darashia | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/250 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Petros | Darashia | DIFF | Retail placement authored. Wiki fares and destinations for eight ordinary routes; carried scarab-cheese guard. Travora source row retained held. Eight ordinary destinations repaired. Travora remains held: daily ticket and Captain Tiberius last-port return owner are absent. | content/world/npc-placements/placements-00000-00339.json:1#/records/269; crates/gameplay/content/src/content/npc_catalogue/travel_routes.json:2179; content/services/travel/travel-00000-00058.json:1#/records/44 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals | REMAINING |
| Razan | Darashia | DIFF | Paid spell teaching removed in retail15.22 (2026-01-27); preserve existing automatic spell acquisition. | crates/gameplay/dispatch/src/gameplay_transport/automatic_spell_progression.rs:1 | dispatch_planner_gates_level_vocation_and_explicit_exclusions | SUPERSEDED |
| Shalmar | Darashia | DIFF | Paid spell teaching removed in retail15.22 (2026-01-27); preserve existing automatic spell acquisition. | crates/gameplay/dispatch/src/gameplay_transport/automatic_spell_progression.rs:1 | dispatch_planner_gates_level_vocation_and_explicit_exclusions | SUPERSEDED |
| Zarifan | Darashia | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/327 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| A Dark Priestess | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/2 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| A Majestic Warwolf | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/4 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| A Swan | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/11 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Albert | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/16 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Alexander | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/20 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Amanda | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/25 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| An Ominous Bat | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/29 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Anaztassja Moroia | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/30 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Artemyth | Edron | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-artemyth.md:1 | Source review | EXCLUDED |
| Beatrice | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/39 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Captain Seahorse | Edron | DIFF | Retail placement authored. Enable Venore40 with carried scarab-cheese guard. | content/world/npc-placements/placements-00000-00339.json:1#/records/54; crates/gameplay/content/src/content/npc_catalogue/travel_routes.json:811 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals | AUTHORED; QUALIFICATION PENDING |
| Chief Grarkharok | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/65 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Chrystal | Edron | DIFF | Retail placement authored.  Ordinary Letter8/Parcel15; completed Postman5/10 through the current-rank trade writer. | content/world/npc-placements/placements-00000-00339.json:1#/records/68; content/services/trade/trade-00000-00326.json:1#/records/64 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Eliza | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/92 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Emberwing | Edron | DIFF | Book World scene shares its exact cell with Grovebeast; no admitted mutually exclusive scene selector. | .npcaudit/research/emberwing.md:1 | Source review | REMAINING |
| Eremo | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/105 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Grovebeast Npc | Edron | DIFF | Book World scene shares its exact cell with Emberwing; no admitted mutually exclusive scene selector. | .npcaudit/research/grovebeast-npc.md:1 | Source review | REMAINING |
| Gundralph | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/151 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Gunther | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/152 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Hawkhurst | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/160 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Iwan | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/169 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Lai | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/188 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Lailene | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/189 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Lesser Messenger of Heaven | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/190 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Luna | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/205 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Maeryn | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/208 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Milos | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/223 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Mirabell | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/224 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Myzzi | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/227 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Ortelio | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/253 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Owin | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/259 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Pemaret | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/265 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Pino | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/270 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Puffels | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/273 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Rudolph | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/290 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Sane Mage | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/291 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Shadowpunch | Edron | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-shadowpunch.md:1 | Source review | EXCLUDED |
| Telas Golem | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/302 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Thomas | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/306 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Ursula | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/311 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Willard | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/316 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Wyrdin | Edron | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/319 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| A Fading Memory | Farmine | DIFF | Ghost's Tear reward is a Blood Brothers quest transaction; excluded quest-lane repair required. | .npcaudit/research/a-fading-memory.md:1 | Source review | REMAINING |
| A Sleeping Dragon | Farmine | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/7 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| A Strange Chalice | Farmine | DIFF | Retail placement authored. Forgotten Knowledge quest topic/actions need the excluded quest lane. | content/world/npc-placements/placements-00000-00339.json:1#/records/9 | test_npcfix_retail_population_has_each_audited_cell | REMAINING |
| A Wandering Soul | Farmine | DIFF | Blood Crystal enchantment is a Blood Brothers quest transaction; excluded quest-lane repair required. | .npcaudit/research/a-wandering-soul.md:1 | Source review | REMAINING |
| Chartan | Farmine | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/62 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Chrak | Farmine | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/67 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Cranky Lizard Crone | Farmine | DIFF | Affirmative cape/hat exchanges require Wayfarer quest transitions; excluded quest-lane repair required. | .npcaudit/research/cranky-lizard-crone.md:1 | Source review | REMAINING |
| Esrik | Farmine | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/107 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Harlow (Vengoth) | Farmine | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/158 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Lizard Prisoner | Farmine | MISSING | Retail placement authored. Add exact displayed identity. | content/world/npc-placements/placements-00000-00339.json:1#/records/198; content/npcs/definitions/npcs-00500-00999.json:1#/records/240; content/dialogues/definitions/dialogues-00000-00499.json:1#/records/487 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Lizard Tunnel Guard | Farmine | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/199 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Murim | Farmine | DIFF | Retail placement authored. Connect native bank service alongside postal shop; conditional postage. Ordinary Letter8/Parcel15; completed Postman5/10 through the current-rank trade writer. | content/world/npc-placements/placements-00000-00339.json:1#/records/225; content/services/trade/trade-00000-00326.json:1#/records/206 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Ongulf | Farmine | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/252 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Pompan | Farmine | DIFF | Retail placement authored. Add ten Wiki-confirmed ammunition, weapon and flag offers. | content/world/npc-placements/placements-00000-00339.json:1#/records/272; content/services/trade/trade-00000-00326.json:1#/records/232 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Zalamon | Farmine | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/326 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Zizzle | Farmine | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/333 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Zlak | Farmine | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/334 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Ztiss | Farmine | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/337 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Zumtah | Farmine | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/338 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Zurak | Farmine | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/339 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Maelyrra | Feyrist | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/207 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Toffee | Feyrist | DIFF | Retail placement authored. Add seven candy buyback offers, including Pastry Dragon95000. | content/world/npc-placements/placements-00000-00339.json:1#/records/308; content/services/trade/trade-00000-00326.json:1#/records/291 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Alyxo | Issavi | DIFF | Retail placement authored. Kilmaresh quest dialogue/progression needs the excluded quest lane. | content/world/npc-placements/placements-00000-00339.json:1#/records/24 | test_npcfix_retail_population_has_each_audited_cell | REMAINING |
| Captain Jack Rat | Issavi | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/50 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Eshaya | Issavi | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/106 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Eustacio's Spy Rat | Issavi | MISSING | Retail placement authored. Add exact displayed identity. | content/world/npc-placements/placements-00000-00339.json:1#/records/110; content/npcs/definitions/npcs-00000-00499.json:1#/records/410 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Taya | Issavi | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/301 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Yonan | Issavi | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/324 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Brom | Kazordoon | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-brom.md:1 | Source review | EXCLUDED |
| Burt | Kazordoon | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-burt.md:1 | Source review | EXCLUDED |
| Etzel | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/108 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Gnomission | Kazordoon | DIFF | Retail placement authored. Rank4 Warzone access needs Bigfoot quest predicates and action owner; excluded quest-lane repair required. | content/world/npc-placements/placements-00000-00339.json:1#/records/137 | test_npcfix_retail_population_has_each_audited_cell | REMAINING |
| Gnops Member | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/139 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Hemor, the guard | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/163 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Humnog, the guard | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/166 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Lokur | Kazordoon | DIFF | Retail placement authored.  Ordinary Letter8/Parcel15; completed Postman5/10 through the current-rank trade writer. Connect the native postal service (Letter, Parcel and Label). | content/world/npc-placements/placements-00000-00339.json:1#/records/201; content/services/trade/trade-00000-00326.json:1#/records/184 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Lukosch | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/204 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Malech | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/209 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Maryza | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/220 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Melfar | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/222 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Nezil | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/235 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Pukosch | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/275 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Pydar | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/276 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Riddler | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/285 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Shortsighted Dwarf | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/295 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Sigurd | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/296 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Tezila | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/303 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| The Orc King | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/305 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Tulf | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/310 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Wally | Kazordoon | DIFF | Retail placement authored.  Ordinary Letter8/Parcel15; completed Postman5/10 through the current-rank trade writer. | content/world/npc-placements/placements-00000-00339.json:1#/records/313; content/services/trade/trade-00000-00326.json:1#/records/308 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Xelvar | Kazordoon | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/320 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Captain Max | Liberty Bay | DIFF | Retail placement authored. Yalahar400 route with started-Yalahar quest gate; correct scene identities; no Postman discount. | content/world/npc-placements/placements-00000-00339.json:1#/records/52; crates/gameplay/content/src/content/npc_catalogue/travel_routes.json:625; content/services/travel/travel-00000-00058.json:1#/records/12 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals | AUTHORED; QUALIFICATION PENDING |
| Captain Waverider | Liberty Bay | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/55 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Chantalle | Liberty Bay | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/59 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Charlotta | Liberty Bay | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/61 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Chondur | Liberty Bay | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/66 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Duncan | Liberty Bay | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/79 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Eleonore | Liberty Bay | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/91 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| John The Carpenter | Liberty Bay | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/176 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Marina | Liberty Bay | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/212 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Ocelus | Liberty Bay | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/245 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| One-Eyed Joe | Liberty Bay | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/251 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Polly | Liberty Bay | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/271 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Amra | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/28 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Asphota | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/36 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Candra | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/44 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Captain Corsarah | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/46 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Dhira | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/75 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| G Ezkho | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/123 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Giri | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/129 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Goldro | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/143 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Haani | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/154 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Isika | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/168 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Jata | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/172 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Javala | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/173 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Kaumudi | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/183 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Nagging Naga | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/228 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Nekaret | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/231 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Nihaka | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/237 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Nilavarna | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/238 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Nipuna | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/239 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Omar | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/249 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Pescadu | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/267 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Ziyad | Marapur | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/332 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Maris (Mistrock) | Mistrock (Wiki city Yalahar; scene association verified by transcript) | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/214 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Charles | Port Hope | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/60 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Ray | Port Hope | DIFF | Retail placement authored. Ordinary Letter8/Parcel15; completed Postman5/10, with current durable rank validation. | content/world/npc-placements/placements-00000-00339.json:1#/records/284; content/services/trade/trade-00000-00326.json:1#/records/242 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npcfix_postgres_dispatch_services_and_replay | AUTHORED; QUALIFICATION PENDING |
| A Beggar | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/0 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Chavis | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/63 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Eathar | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/81 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Edmund | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/85 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Elbert | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/89 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Elliott | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/93 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Emily | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/101 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Fabiana | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/111 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Fayla | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/112 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Fillias | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/115 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Flint | Rathleton | DIFF | Retail placement authored. Add three exercise-wrap offers and buy Sword25; qualify full default charge purchases. | content/world/npc-placements/placements-00000-00339.json:1#/records/117; content/services/trade/trade-00000-00326.json:1#/records/103 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npcfix_postgres_dispatch_services_and_replay | AUTHORED; QUALIFICATION PENDING |
| Fyodor | Rathleton | DIFF | Retail placement authored.  Ordinary Letter8/Parcel15; completed Postman5/10 through the current-rank trade writer. | content/world/npc-placements/placements-00000-00339.json:1#/records/122; content/services/trade/trade-00000-00326.json:1#/records/111 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Georgia Rainbird | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/128 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Graham | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/145 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Guardian of the Deep Seas | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/147 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Guide Kunibert | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/149 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Helsi Van Ore | Rathleton | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-helsi-van-ore.md:1 | Source review | EXCLUDED |
| Hexel | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/165 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Jossel | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/177 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Klesar | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/186 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Kromrek | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/187 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Maro | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/217 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Marvin | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/219 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Nicholas | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/236 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Normod | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/242 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Olm | Rathleton | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-olm.md:1 | Source review | EXCLUDED |
| Peremin | Rathleton | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/266 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Roughington | Rathleton | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-roughington.md:1 | Source review | EXCLUDED |
| Al Dee | Rookgaard | DIFF | Retail placement authored. Pick Quest affirmative exchange requires the excluded quest lane. | content/world/npc-placements/placements-00000-00339.json:1#/records/15 | test_npcfix_retail_population_has_each_audited_cell | REMAINING |
| Dixi | Rookgaard | DIFF | Retail placement authored. Buy Chain Helmet from player for17 instead of12. | content/world/npc-placements/placements-00000-00339.json:1#/records/76; content/services/trade/trade-00000-00326.json:1#/records/78 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| The Gatekeeper | Rookgaard | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-the-gatekeeper.md:1 | Source review | EXCLUDED |
| ... | Roshamuul | MISSING | Retail placement authored. Add exact displayed identity. | content/world/npc-placements/placements-00000-00339.json:1#/records/94; content/npcs/definitions/npcs-00000-00499.json:1#/records/385 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Captain Marie-Denise Banner | Roshamuul | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/51 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Dream Butterfly | Roshamuul | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/78 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Eldoran the Ambitious | Roshamuul | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/90 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Glimpse of Allusion | Roshamuul | MISSING | Retail placement authored. Add exact displayed identity. | content/world/npc-placements/placements-00000-00339.json:1#/records/130; content/npcs/definitions/npcs-00000-00499.json:1#/records/489 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Lieutenant Harlan | Roshamuul | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/193 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| WOX | Roshamuul | MISSING | Retail placement authored. Add exact displayed identity. | content/world/npc-placements/placements-00000-00339.json:1#/records/318; content/npcs/definitions/npcs-01000-01289.json:1#/records/241 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Yielothax | Roshamuul | MISSING | Retail placement authored. Add exact displayed identity. | content/world/npc-placements/placements-00000-00339.json:1#/records/323; content/npcs/definitions/npcs-01000-01289.json:1#/records/260 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| A Restless Soul | Svargrond | DIFF | Retail placement authored. Spirit-language greeting and story progression require the Ice Islands quest owner; do not grant speech unconditionally. | content/world/npc-placements/placements-00000-00339.json:1#/records/5 | test_npcfix_retail_population_has_each_audited_cell | REMAINING |
| A Starving Dog | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/8 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Baltim | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/38 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Bertha | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/41 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Captain Breezelda | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/45 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Captain Haba | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/48 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Dankwart | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/73 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Eirik | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/87 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Freezhild | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/120 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Fynn | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/121 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Hawkyr | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/161 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Janz | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/171 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Jessica | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/175 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Kjesse | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/185 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Lurik | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/206 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Nelly | Svargrond | DIFF | Retail placement authored.  Ordinary Letter8/Parcel15; completed Postman5/10 through the current-rank trade writer. | content/world/npc-placements/placements-00000-00339.json:1#/records/233; content/services/trade/trade-00000-00326.json:1#/records/209 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Nor | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/240 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Rashid | Svargrond | DIFF | Retail placement authored. 155 retail offers including seven default-state amulet buybacks; mission07 gate and seven weekday placements matching the existing runtime selector. | content/world/npc-placements/placements-00000-00339.json:1#/records/277; content/services/trade/trade-00000-00326.json:1#/records/240 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npcfix_postgres_dispatch_services_and_replay | AUTHORED; QUALIFICATION PENDING |
| Robert | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/286 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Romir | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/289 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Sinatuki | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/297 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Sirik | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/298 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Thorwulf | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/307 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Zora | Svargrond | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/336 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Akananto | Thais | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-akananto-npc.md:1 | Source review | EXCLUDED |
| Albinius | Thais | DIFF | Retail placement authored. Buy Heavy Old Tome30; compose its existing canonical policy into the active provider, preserving the gated blank-scroll purchase. | content/world/npc-placements/placements-00000-00339.json:1#/records/17; content/services/trade/trade-00000-00326.json:1#/records/4 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npcfix_postgres_dispatch_services_and_replay | AUTHORED; QUALIFICATION PENDING |
| Anna | Thais | DIFF | Retail placement authored. Retail Targuna continuation conflicts with the existing owner-selected landing and needs tutorial/quest transition and destination admission; coordinator required. | content/world/npc-placements/placements-00000-00339.json:1#/records/35 | test_npcfix_retail_population_has_each_audited_cell | REMAINING |
| Benjamin | Thais | DIFF | Retail placement authored. Ordinary Letter8/Parcel15; completed Postman5/10, with current durable rank validation. | content/world/npc-placements/placements-00000-00339.json:1#/records/40; content/services/trade/trade-00000-00326.json:1#/records/33 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npcfix_postgres_dispatch_services_and_replay | AUTHORED; QUALIFICATION PENDING |
| Deseus | Thais | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-deseus.md:1 | Source review | EXCLUDED |
| Dorian | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/77 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Eclesius | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/82 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Eddy | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/83 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Edgar-Ellen | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/84 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Edowir | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/86 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Elane | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/88 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Emiliana | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/98 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Emilie | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/100 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Excited Tibian | Thais | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-excited-tibian.md:1 | Source review | EXCLUDED |
| Ferryman Kamil | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/113 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Ferryman Kamil (Meluna) | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/114 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Festival Assistant | Thais | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-festival-assistant.md:1 | Source review | EXCLUDED |
| Gaberiel | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/124 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Galuna | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/125 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Gamel | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/126 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Ghost Captain | Thais | DIFF | Conditional Ghost Ship scene has no admitted event selector; unconditional placement would invent availability. | .npcaudit/research/ghost-captain.md:1 | Source review | REMAINING |
| Gorn | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/144 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Grof, The Guard | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/146 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Guide Luke | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/150 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Hanna | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/156 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Hans Gender | Thais | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-hans-gender.md:1 | Source review | EXCLUDED |
| Hardek | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/157 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Harsky | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/159 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Hector The Mentor | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/162 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Henricus | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/164 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Hoaxette | Thais | DIFF | April event availability has no admitted selector; unconditional placement would invent availability. | .npcaudit/research/hoaxette.md:1 | Source review | REMAINING |
| Junkar (Mines) | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/179 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Junkar (Robsons) | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/180 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Junkar (Thais) | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/181 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Lizzie | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/200 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Lubo | Thais | DIFF | Retail placement authored. Sell Torch for2. | content/world/npc-placements/placements-00000-00339.json:1#/records/203; content/services/trade/trade-00000-00326.json:1#/records/185 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Markwin | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/216 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Marvik | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/218 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Mercutios Drum | Thais | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-mercutios-drum.md:1 | Source review | EXCLUDED |
| Mercutios Harp | Thais | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-mercutios-harp.md:1 | Source review | EXCLUDED |
| Mercutios Piano | Thais | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-mercutios-piano.md:1 | Source review | EXCLUDED |
| Partos | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/262 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Paulette | Thais | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/263 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Robson | Thais | DIFF | Retail placement authored. Parcel15 and Label1 service; parcel/label keyword aliases; no Postman discount. | content/world/npc-placements/placements-00000-00339.json:1#/records/287; content/dialogues/definitions/dialogues-00500-00838.json:1#/records/148; content/services/trade/trade-00000-00326.json:1#/records/247 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npcfix_postgres_dispatch_services_and_replay | AUTHORED; QUALIFICATION PENDING |
| Stone Temple Parrot | Thais | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-stone-temple-parrot.md:1 | Source review | EXCLUDED |
| Victor | Thais | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-victor.md:1 | Source review | EXCLUDED |
| Doctor Marrow | Thalassara | MISSING | Add exact displayed identity. Identity added; D17 map-owner conflict remains between Wiki and reviewed map coordinates. Placement is held, not guessed. | content/npcs/definitions/npcs-00000-00499.json:1#/records/330 | npcfix_retail_services_and_greeting_match_the_audited_repairs | REMAINING |
| Testserver Assistant | Varies | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-testserver-assistant.md:1 | Source review | EXCLUDED |
| A Behemoth | Venore | DIFF | Retail placement authored. Spike quest-conditioned greetings require the excluded quest lane. | content/world/npc-placements/placements-00000-00339.json:1#/records/1 | test_npcfix_retail_population_has_each_audited_cell | REMAINING |
| A Ghostly Guardian | Venore | DIFF | Retail placement authored. Retail greeting/name substitution, farewell and eight keyword aliases; preserve all multipart lines through command and ordinary chat. | content/world/npc-placements/placements-00000-00339.json:1#/records/3; content/dialogues/definitions/dialogues-00000-00499.json:1#/records/13 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npcfix_postgres_dispatch_services_and_replay; npcfix_guardian_chat_preserves_name_and_multipart_source_order | AUTHORED; QUALIFICATION PENDING |
| A Strange Fellow | Venore | DIFF | Retail placement authored. Postman progression dialogue/actions require the excluded quest lane. | content/world/npc-placements/placements-00000-00339.json:1#/records/10 | test_npcfix_retail_population_has_each_audited_cell | REMAINING |
| A Weakened Forest Fury | Venore | DIFF | Retail placement authored. Forgotten Knowledge guarded interaction requires the excluded quest lane. | content/world/npc-placements/placements-00000-00339.json:1#/records/13 | test_npcfix_retail_population_has_each_audited_cell | REMAINING |
| Allen | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/22 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Captain Fearless | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/47 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Demonguard | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/74 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Dustrunner | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/80 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Eustacio | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/109 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Frank The Plank | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/118 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Frans | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/119 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| George The Boyscout | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/127 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Gnome Trooper | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/131 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Gnommander | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/138 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Guide Elena | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/148 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| H.L. | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/153 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Hagor | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/155 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Jack The Worker | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/170 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Jean Claude | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/174 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Julian | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/178 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Leopold | Venore | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-leopold.md:1 | Source review | EXCLUDED |
| Lightfoot | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/194 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Livielle | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/197 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Lorbas | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/202 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Maria | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/211 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Minzy | Venore | DIFF | Bewitched event NPC; no admitted availability selector. | .npcaudit/research/minzy.md:1 | Source review | REMAINING |
| Nathaniel | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/230 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Nelliem | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/232 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Nemal | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/234 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Oldrak | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/246 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Romella | Venore | DIFF | Retail placement authored. Add three exercise-wrap offers; qualify full default charge purchases. | content/world/npc-placements/placements-00000-00339.json:1#/records/288; content/services/trade/trade-00000-00326.json:1#/records/250 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npcfix_postgres_dispatch_services_and_replay | AUTHORED; QUALIFICATION PENDING |
| Shiantis | Venore | DIFF | Retail placement authored. Add sixteen Wiki-confirmed pillow/pick offers. | content/world/npc-placements/placements-00000-00339.json:1#/records/294; content/services/trade/trade-00000-00326.json:1#/records/265 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Solkrin | Venore | MISSING | Historical/test-server-only reference; no current-playable change. Deseus is independently listed in current Deprecated NPCs. | .npcaudit/research/reference-solkrin-npc.md:1 | Source review | EXCLUDED |
| Zebron | Venore | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/328 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Chuckles | Yalahar | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/69 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Emperor Rehal | Yalahar | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/102 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Lisander | Yalahar | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/196 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Maris | Yalahar | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/213 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Maritima | Yalahar | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/215 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Oliver | Yalahar | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/247 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Ortheus | Yalahar | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/258 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Palimuth | Yalahar | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/261 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Peter | Yalahar | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/268 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Pugwah | Yalahar | DIFF | Retail placement authored. Sell empty Mug4 using the existing default empty state. | content/world/npc-placements/placements-00000-00339.json:1#/records/274; content/services/trade/trade-00000-00326.json:1#/records/233 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npcfix_postgres_dispatch_services_and_replay | AUTHORED; QUALIFICATION PENDING |
| Tarak (Inner) | Yalahar | DIFF | Retail placement authored. Monument Tower50 route; passage/trip keyword aliases; displayed name Tarak. | content/world/npc-placements/placements-00000-00339.json:1#/records/299; crates/gameplay/content/src/content/npc_catalogue/travel_routes.json:2596; content/services/travel/travel-00000-00058.json:1#/records/50 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals | AUTHORED; QUALIFICATION PENDING |
| Tarak (Sunken) | Yalahar | DIFF | Retail placement authored. Free return passage; displayed name Tarak. | content/world/npc-placements/placements-00000-00339.json:1#/records/300; crates/gameplay/content/src/content/npc_catalogue/travel_routes.json:2609; content/services/travel/travel-00000-00058.json:1#/records/51 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs; npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals | AUTHORED; QUALIFICATION PENDING |
| Xorlosh | Yalahar | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/321 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Yalahari | Yalahar | DIFF | Retail placement authored. | content/world/npc-placements/placements-00000-00339.json:1#/records/322 | test_npcfix_retail_population_has_each_audited_cell | AUTHORED; QUALIFICATION PENDING |
| Zirkon | Yalahar | DIFF | Retail placement authored. Add the recorded sacred-place greeting. | content/world/npc-placements/placements-00000-00339.json:1#/records/331; content/dialogues/definitions/dialogues-00500-00838.json:1#/records/330 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |
| Znozel | Yalahar | DIFF | Retail placement authored. Connect existing native bank service and source dialogue. | content/world/npc-placements/placements-00000-00339.json:1#/records/335 | test_npcfix_retail_population_has_each_audited_cell; npcfix_retail_services_and_greeting_match_the_audited_repairs | AUTHORED; QUALIFICATION PENDING |

## Unfinished

- **A Sweaty Cyclops (Ab'Dendriel)**: Placement is repaired. The audit leaves exact retail smithing eligibility/effects unresolved; verified quest exchanges must be routed to the excluded quest owner.
- **Ahmet (Ankrahmun)**: Oil-filled Vial20 is not executable: fluid mint provenance and subtype trade support need the existing fluid owner contract; this package adds placement only.
- **Rascalio (Boss Lairs)**: Nimmersatt event-conditioned Banor's Eye transport needs an admitted event selector and return route.
- **Captain Jack (Carlin)**: Tibia return depends on Costello permission and monastery fines; no qualified native predicate. Quest-lane repair required.
- **Altar (Darashia)**: Crimson Offering/Bakragore eligibility and reward transaction require the excluded quest lane.
- **Petros (Darashia)**: Eight ordinary destinations repaired. Travora remains held: daily ticket and Captain Tiberius last-port return owner are absent.
- **Emberwing (Edron)**: Book World scene shares its exact cell with Grovebeast; no admitted mutually exclusive scene selector.
- **Grovebeast Npc (Edron)**: Book World scene shares its exact cell with Emberwing; no admitted mutually exclusive scene selector.
- **A Fading Memory (Farmine)**: Ghost's Tear reward is a Blood Brothers quest transaction; excluded quest-lane repair required.
- **A Strange Chalice (Farmine)**: Forgotten Knowledge quest topic/actions need the excluded quest lane.
- **A Wandering Soul (Farmine)**: Blood Crystal enchantment is a Blood Brothers quest transaction; excluded quest-lane repair required.
- **Cranky Lizard Crone (Farmine)**: Affirmative cape/hat exchanges require Wayfarer quest transitions; excluded quest-lane repair required.
- **Alyxo (Issavi)**: Kilmaresh quest dialogue/progression needs the excluded quest lane.
- **Gnomission (Kazordoon)**: Rank4 Warzone access needs Bigfoot quest predicates and action owner; excluded quest-lane repair required.
- **Al Dee (Rookgaard)**: Pick Quest affirmative exchange requires the excluded quest lane.
- **A Restless Soul (Svargrond)**: Spirit-language greeting and story progression require the Ice Islands quest owner; do not grant speech unconditionally.
- **Anna (Thais)**: Retail Targuna continuation conflicts with the existing owner-selected landing and needs tutorial/quest transition and destination admission; coordinator required.
- **Ghost Captain (Thais)**: Conditional Ghost Ship scene has no admitted event selector; unconditional placement would invent availability.
- **Hoaxette (Thais)**: April event availability has no admitted selector; unconditional placement would invent availability.
- **Doctor Marrow (Thalassara)**: Identity added; D17 map-owner conflict remains between Wiki and reviewed map coordinates. Placement is held, not guessed.
- **A Behemoth (Venore)**: Spike quest-conditioned greetings require the excluded quest lane.
- **A Strange Fellow (Venore)**: Postman progression dialogue/actions require the excluded quest lane.
- **A Weakened Forest Fury (Venore)**: Forgotten Knowledge guarded interaction requires the excluded quest lane.
- **Minzy (Venore)**: Bewitched event NPC; no admitted availability selector.

No quest-lane module or `content/quests/**` was edited. The coordinator must route the explicit quest dependencies above; event, map and ticket ownership gaps are not treated as fixed by adding unconditional actors.

## Checks

| check | result | command | log |
| --- | --- | --- | --- |
| regeneration | FAIL | `python3 tools/content-migration/regenerate_content.py --resolve --no-stage` | .npcfix/logs/qualified-regeneration.log |
| fmt | PASS | `/home/mole/oteryn-cache/codex-bin/cargo fmt --all` | .npcfix/logs/qualified-fmt.log |
| fmt-mounted | PASS | `rustfmt --edition 2024 crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/npc_postage.rs` | .npcfix/logs/qualified-fmt-mounted.log |
| check | PASS | `/home/mole/oteryn-cache/codex-bin/cargo check --offline --locked -p oteryn-gameplay-content -p oteryn-gameplay-npc -p oteryn-gameplay-dispatch -p oteryn-gameplay-durability -p oteryn-game-server --all-targets --all-features --quiet` | .npcfix/logs/qualified-check.log |
| clippy | PASS | `/home/mole/oteryn-cache/codex-bin/cargo clippy --offline --locked -p oteryn-gameplay-content -p oteryn-gameplay-npc -p oteryn-gameplay-dispatch -p oteryn-gameplay-durability -p oteryn-game-server --all-targets --all-features --quiet -- -D warnings` | .npcfix/logs/qualified-clippy.log |
| content-unit-npc | PASS | `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-gameplay-content --lib npc --quiet` | .npcfix/logs/qualified-content-unit-npc.log |
| native-content-unit | NOT RUN | See `.npcfix/run_checks.py` |  |
| npc-item-provider | PASS | `python3 -m unittest discover -s tools/content-migration -p test_npc_trade_item_profiles.py` | .npcfix/logs/qualified-npc-item-provider.log |
| npc-unit | PASS | `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-gameplay-npc --lib --quiet` | .npcfix/logs/qualified-npc-unit.log |
| durability-npc-unit | PASS | `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-gameplay-durability --lib npc_trade --quiet` | .npcfix/logs/qualified-durability-npc-unit.log |
| npc-materializer | PASS | `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-game-server --example materialize_content_world_project_v2 npcfix --quiet` | .npcfix/logs/qualified-npc-materializer.log |
| content-npc-catalogue | PASS | `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-game-server --test content_npc_catalogue --quiet` | .npcfix/logs/qualified-content-npc-catalogue.log |
| content-npc-service-model | PASS | `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-game-server --test content_npc_service_model -- --nocapture` | .npcfix/logs/qualified-content-npc-service-model.log |
| content-world-repository | PASS | `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-game-server --test content_world_project_repository --quiet` | .npcfix/logs/qualified-content-world-repository.log |
| content-world-v2 | PASS | `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-game-server --test content_world_project_v2 --quiet` | .npcfix/logs/qualified-content-world-v2.log |
| integration-content | PASS | `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-game-server --test integration_rest content_ --quiet` | .npcfix/logs/qualified-integration-content.log |
| dispatch-service-unit | NOT RUN | See `.npcfix/run_checks.py` |  |
| dispatch-npc-unit | PASS | `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-gameplay-dispatch --all-features --lib npc_command --quiet` | .npcfix/logs/qualified-dispatch-npc-unit.log |
| dispatch-guardian-chat | PASS | `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-gameplay-dispatch --all-features --lib npcfix_guardian_chat --quiet` | .npcfix/logs/qualified-dispatch-guardian-chat.log |
| postgres-npcfix-services | PASS | `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-gameplay-dispatch --all-features --lib npcfix_postgres_dispatch_services_and_replay -- --nocapture` | .npcfix/logs/qualified-postgres-npcfix-services.log |
| postgres-paid-travel | PASS | `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-gameplay-dispatch --all-features --lib npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals -- --nocapture` | .npcfix/logs/qualified-postgres-paid-travel.log |
| placements | PASS | `python3 -m unittest discover -s tools/content-schema/npc-authoring -p test_convert_placements.py` | .npcfix/logs/qualified-placements.log |
| architecture | NOT RUN | See `.npcfix/run_checks.py` |  |
| pin-check | NOT RUN | See `.npcfix/run_checks.py` |  |
| world-npc-admission | NOT RUN | See `.npcfix/run_checks.py` |  |
| governance | NOT RUN | See `.npcfix/run_checks.py` |  |
| governance-unit | NOT RUN | See `.npcfix/run_checks.py` |  |
| fmt-check | NOT RUN | See `.npcfix/run_checks.py` |  |
| fmt-mounted-check | NOT RUN | See `.npcfix/run_checks.py` |  |
| diff-check | NOT RUN | See `.npcfix/run_checks.py` |  |

Regression RED evidence: `.npcfix/logs/placement-red.log`, `.npcfix/logs/services-red.log`, `.npcfix/logs/albinius-red.log`; intermediate failed build/regeneration logs are retained and are not final qualification. PostgreSQL qualifications require the absolute wrapper plus a `POSTGRES_STARTED` marker; early-return cases do not count.

## Source reconciliation

Validation scope correction: the earlier broad `npc_command` unit filter also ran `gameplay_transport::monk_save::postgres_tests::classic_quests::classic_quests_npc_commands_commit_exact_start_branches_and_fence_topics` (passed). The final unit selector is `gameplay_transport::npc_command`; no quest module was changed and this unrelated PG case is not repeated.

- Razan/Shalmar: current retail notes remove paid spell teaching in15.22: [Razan](https://www.tibiawiki.com.br/wiki/Razan), [Shalmar](https://www.tibiawiki.com.br/wiki/Shalmar). Older audit pages describe superseded behavior.
- Every placement retains the exact Wiki revision; all49 initial and3 follow-up native repairs retain audit research references and an exact before-state. Canary/Crystal hints do not override retail prices.
- Default-state charged buybacks retain the existing durable refusal for instances with mutable timed state. Ahmet filled-fluid support remains explicitly unfinished.
- Original source holds/provenance survive. Generated prey valuations change because newly admitted buyback offers change loot resale values; the repository generator owns those reports and pins.

## Self-review

Checklist: `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. Field/constructor/consumer search: `.npcfix/constructor-sites.txt`. Publication and any independently required review belong to the coordinator; this worker did not trigger owner-funded review. Whole-diff review evidence is recorded in `.npcfix/SELF-REVIEW.md`.

## Changed files

- `.claude/skills/oteryn-dev/references/content-world.md`
- `.npcfix/REPORT.md`
- `.npcfix/RESUME.md`
- `.npcfix/SELF-REVIEW.md`
- `.npcfix/audit-scope.json`
- `.npcfix/checks.jsonl`
- `.npcfix/constructor-sites.txt`
- `.npcfix/default-state-red.json`
- `.npcfix/final_checks.py`
- `.npcfix/logs/before-default-state-npc_talk.rs.txt`
- `.npcfix/logs/before-default-state-service.rs.txt`
- `.npcfix/placement-evidence.json`
- `.npcfix/prove_default_state_red.py`
- `.npcfix/retail-evidence.json`
- `.npcfix/run_checks.py`
- `.npcfix/semantic-review.json`
- `.npcfix/write_report.py`
- `apps/game-server/examples/materialize_content_world_project_v2.rs`
- `apps/game-server/examples/npc_materializer/npcfix.rs`
- `apps/game-server/tests/content_npc_catalogue.rs`
- `apps/game-server/tests/content_npc_service_model.rs`
- `apps/game-server/tests/content_world_project_repository.rs`
- `apps/game-server/tests/content_world_project_v2.rs`
- `apps/game-server/tests/integration_rest/content_world_project_v2_npc_admission.rs`
- `content/content.lock.json`
- `content/cosmetics/mounts/index.json`
- `content/creatures/definitions/index.json`
- `content/creatures/definitions/spell-native-profiles.json`
- `content/creatures/prey-slot-values.json`
- `content/dialogues/definitions/dialogues-00000-00499.json`
- `content/dialogues/definitions/dialogues-00500-00835.json`
- `content/dialogues/definitions/dialogues-00500-00838.json`
- `content/dialogues/definitions/index.json`
- `content/documents/index.json`
- `content/encounters/definitions/index.json`
- `content/items/definitions/spell-native-profiles.json`
- `content/items/index.json`
- `content/manifest.json`
- `content/npcs/definitions/index.json`
- `content/npcs/definitions/npcs-00000-00499.json`
- `content/npcs/definitions/npcs-00500-00999.json`
- `content/npcs/definitions/npcs-01000-01281.json`
- `content/npcs/definitions/npcs-01000-01289.json`
- `content/services/bank/index.json`
- `content/services/blessings/index.json`
- `content/services/promotion/index.json`
- `content/services/trade/index.json`
- `content/services/trade/trade-00000-00323.json`
- `content/services/trade/trade-00000-00326.json`
- `content/services/travel/index.json`
- `content/services/travel/travel-00000-00058.json`
- `content/spells.manifest.json`
- `content/world/content.lock.json`
- `content/world/definitions/declarations.json`
- `content/world/manifest.json`
- `content/world/npc-placements/held.json`
- `content/world/npc-placements/index.json`
- `content/world/npc-placements/placements-00000-00032.json`
- `content/world/npc-placements/placements-00000-00339.json`
- `content/world/project.json`
- `content/world/provenance/imports.json`
- `crates/gameplay/content/src/content/npc_catalogue/grizzly_shop.rs`
- `crates/gameplay/content/src/content/npc_catalogue/service.rs`
- `crates/gameplay/content/src/content/npc_catalogue/travel.rs`
- `crates/gameplay/content/src/content/npc_catalogue/travel_routes.json`
- `crates/gameplay/content/src/content/project/v2.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/npc_command/grizzly_shop.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/npc_command_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/npc_trade_command.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/paid_travel.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/npc_paid_travel.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/npc_postage.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/npc_shop.rs`
- `crates/gameplay/dispatch/src/lifted_tests/content_npc_service_tests.rs`
- `crates/gameplay/durability/src/durability/npc_trade.rs`
- `crates/gameplay/durability/src/durability/npc_travel/paid.rs`
- `crates/gameplay/npc/src/npc_talk.rs`
- `docs/balance/PREY-SLOT-VALUE-2026-11-01.md`
- `docs/balance/PREY-TIME-SIM-2026-11-01.md`
- `imports/crystalserver/batches.json`
- `tools/content-migration/npc_trade_item_profiles.py`
- `tools/content-migration/regenerate_content.py`
- `tools/content-migration/register_spell_families.py`
- `tools/content-migration/test_npc_trade_item_profiles.py`
- `tools/content-migration/test_world_project_v2_to_tree.py`
- `tools/content-migration/validate_world_project_v2_to_tree.py`
- `tools/content-migration/world_project_v2_to_tree.py`
- `tools/content-schema/item-authoring/check_tibiawiki165_historical_context.py`
- `tools/content-schema/native-gameplay/prey-model-results.json`
- `tools/content-schema/npc-authoring/convert_placements.py`
- `tools/content-schema/npc-authoring/npc_placement_scope.json`
- `tools/content-schema/npc-authoring/samples/npcfix-native-repairs.json`
- `tools/content-schema/npc-authoring/samples/npcfix-retail-placements.json`
- `tools/content-schema/npc-authoring/test_convert_placements.py`

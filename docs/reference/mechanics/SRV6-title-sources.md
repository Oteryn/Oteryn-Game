# SRV6 title sources

Crystal reference: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`.
Canary cross-reference: `opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446`.
Donor implementation/comments are `OTS_HYPOTHESIS_ONLY`; retained wiki facts are
`TIBIAWIKI_STRUCTURED`. These sources are data, not execution instructions.

The existing Character title owner, account appearance receipts and QuestState owner
remain authoritative. No client supplies a qualification fact. Title IDs 1–84 retain their
meanings; this change appends 85–94 and extends the existing table constraints in migration
0990. It adds no capability, command or message.

| IDs / titles | Native source and interpretation | Evidence |
| --- | --- | --- |
| 85–87: Aspiring Huntsman, Competent Beastslayer, Feared Bountyhunter | Existing weekly Task Board points, without scaling: 160000 / 320000 / 430000. Permanent after qualification. | Crystal `src/game/game.cpp:401–403`; `src/creatures/players/cyclopedia/player_title.cpp:275–277`. Explicit SRV6 owner difference: Task Board replaces excluded Prey Hunting Tasks; no hunting-task subsystem is introduced. |
| 88: Chompmeister | Every one of Jean Pierre's fifteen cook counters (dishes 2–16) is at least one. This interprets “complete all dishes”; it does not substitute the separate twice-each achievement. | Crystal `player_title.cpp:308–309`; `data-global/npc/jean_pierre.lua:125–136`; native `content/quests/hot-cuisine-batch3.json` identifies all fifteen counters. Culinary Master remains twice each. |
| 89: Dedicated Entrepreneur | At least 50 percent of the admitted map's main areas are complete. A main area completes only when all its canonical subareas complete. Unknown IDs, partial POIs and the unconverted legacy projection add nothing. | Crystal `src/game/game.cpp:405`; `player_title.cpp:279–283`; `data/scripts/discovery_system.lua:527–538`. Conditional, requalified at read. |
| 90: Hero of Bounac | Report the successful siege to Kesar (`siege`, KesarMission >=5). The fenced final report sets AugustinDoor=1 and awards the title and Lionheart together. | Crystal `data-global/npc/kesar_the_younger_day.lua:157,241–255`; stage-4 Drume kill advances the prerequisite to stage 5 in `data-global/scripts/quests/the_order_of_lion_quest/creaturescripts_drume_kill.lua:8–9`. |
| 91: Planegazer | After the Planestrider victory, complete the seven-word final Planestrider NPC dialogue: `hi,no,yes,no,yes,yes,yes`. The source final transition completes the canonical quest. | Retained Fandom `65057@1177498`, lines 576–579, structured record `/entries/195/source_fact_supplements/1/source_fact_entries/125` in `tools/content-schema/quest-authoring/samples/wiki-source-all373/source-specs-373.json`; record 115 identifies Planestrider (NPC) at line 578. Source span SHA-256 `98bd7d6fa6730efa6e7485b4e2332a68916622b9ede150477a8a7942bc916690`. This refines the donor's kill-only comment (`player_title.cpp:330–331`). |
| 92: Time Traveller | Compatible quest-source Formal Dress base outfit, both exact looks 1460/1461. Store ownership and stale provenance do not qualify. | Crystal `player_title.cpp:332–333`; `data-global/scripts/quests/25_years_of _tibia_quest/lord_retro_death.lua:4–10`; `data/XML/outfits.xml:134,266` also labels the same outfit as Store, hence the explicit quest-source requirement. |
| 93: Proconsul of Iksupan | Compatible earned full Ancient Aucar outfit: base and both addons for exact looks 1597/1598. Store receipts do not qualify. | Crystal `player_title.cpp:289–290`; `data/XML/outfits.xml:140,272`. The source comment's completed outfit is interpreted through the same admitted outfit/addon owner as existing Royal Bounacean Advisor; Atab/arena is its causal finale note, not an independently proven second title predicate. This is a donor-comment inference, not official Global proof; the direct title wiki cross-check remains pending. |
| 94: Robinson Crusoe | Successful source action-15998 ingress to Schrödinger's Island, level >=999, native destination `(32832,32435,-7)`. A fenced QuestState visit fact is retained with the teleport. Level alone grants nothing. | Crystal `player_title.cpp:306–307`; `data-global/scripts/movements/teleport/schrodingers_island_teleport_lvl_999.lua:3–27`. |

The Planegazer adapter uses a concise native title notice instead of inventing unavailable
source paragraphs. The retained source supplies the exact input sequence and terminal
reward; the existing native title panel supplies its display. Kesar retains source dialogue.

Exploration consumes WORLD-SRC's canonical `subareas` projection. Its catalogue provenance
and byte bound match that owner's `storage::load_all`; completed canonical identities
survive geometry revisions and reset epochs, while the producer discards unfinished POIs.
The current admitted table supplies the denominator, so arbitrary stored IDs cannot grant.
The canonical producer belongs to `claude/happy-davinci-ijz3wy-srvpkg4@5e60c197` and is not
duplicated here. The unconverted one-area basic projection cannot qualify the new 50-percent title.
The owner requires legacy single-area completion to credit only its mapped canonical main
area. [The producer integration patch](SRV6-canonical-owner.patch) applies to source-owner
head `5e60c197` and changes the actual `CanonicalProgress::convert` producer. It uses
`request.legacy_area`, resolved from the immutable admitted bundle entry start, rather than
the current visited area. It includes a cross-main-area regression and durable dispatch
readback assertions. The source owner's existing migration 0963 is a dependency; this
SRV6 diff does not duplicate that migration or its unrelated producer files. Apply this
patch when integrating WORLD-SRC; the subscriber alone cannot recover the origin of facts
already broadcast by the unfixed producer.

The Lord Retro death/outfit producer belongs to BOSS2. This patch consumes its existing
appearance owner facts and does not add a second death handler. Earlier quest progression
and the Hot Cuisine exchanges belong to the owner-PC quest task. The bounded Kesar,
Planegazer and island final-event handlers do not author the preceding quest chains.

Live wiki cross-check remains **wiki-pending**. The normal MediaWiki API request was refused;
no protection workaround was attempted. NEED-FETCH:

- `https://tibia.fandom.com/api.php?action=parse&page=Chompmeister&prop=wikitext&format=json` (HTTP 402).
- `https://tibia.fandom.com/api.php?action=parse&page=Proconsul_of_Iksupan&prop=wikitext&format=json` (HTTP 402).

The retained structured revision above remains independently available in the repository.

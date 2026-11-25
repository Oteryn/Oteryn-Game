# CNPC2 regional NPC placements

Base `3e09916423181c795c829eca5e5414dde3f29a3e`. The region is the named city and its documented NPC subareas, including Stonehome, Cormaya, the djinn fortresses, Treasure Island, Bounac, and the Svargrond islands. The regional join uses the identity's NPC location and qualifies each occurrence by position. Example maps and occurrences belonging to other regions were excluded.

| Audit row | Admitted | Evidence |
| --- | ---: | --- |
| Crystal XML name matches | 236 identities | `npc_placement_scope.json` records the XML line for each selected occurrence. Spawn center plus child x/y offset and parent floor determines the anchor. Explicit facing takes precedence over the donor default. |
| Retained Wiki and regional occurrences absent from Crystal XML | 7 identities | Barney, Eranth, Gnomfurry, Gnominer, Maeryn and Paolo use the retained promotion-candidate Wiki coordinates. Lesser Messenger of Heaven uses the Edron cell from TibiaWiki BR revision 385247. |
| Aliases resolved from Crystal XML | 2 identities | John (Bounac) is the Bounac occurrence of John at XML line 145. Gnomish Operative (Catching) is registered from `gnomish_operative_cathing.lua` and placed at XML line 3327. |

Crystal placement and script evidence is `OTS_HYPOTHESIS_ONLY` with the XML digest pinned in the authoring scope. Retained MediaWiki position facts are `TIBIAWIKI_STRUCTURED`. Crystal controls its qualified occurrences. For example, the form of Telas at his workshop and the one beneath his house both have XML rows. Anaztassja Moroia Init is the typed XML occurrence at Edron, while the similarly named vampire-lair candidate's shared Wiki cell refers to a different encounter. [Crystal 201189f8](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8)

The scope records one-cell moves from obstructed or groundless anchors to clear ground on the same floor. The two John occurrences stay at their respective Bounac and Treasure Island sites. Captain Max stands one tile beside the Calassa ferry's Liberty Bay arrival. Source-backed solid-object contacts without clear adjacency use the existing NPC admission flag. World walkability for players is still determined by map terrain. The placement compiler verifies every admitted cell, facing, and per-identity occurrence count through `regional_npc_contacts_are_admitted_at_their_qualified_cells`.

## Already served on main

The existing day/night producer owns Ghostly Wolf, Evrard, Evrard the Miller, Innkeeper Alphonse, Fitzmaurice, the Wes, Onfroi, Dal, Jehan, Fral and Kesar variants, and Kesar's valets. Use the production identifiers from `apps/game-server/src/node/serve/bundle_npcs.rs` and the Crystal schedule in `data-global/scripts/world_changes/spawns_npc_by_time.lua`. The base also has placements for Buddel at Svargrond, Asima, Habdel, Ishina, Kasmir, Miraia, Razan, Alexander, Amanda, Avar Tar, Dedoras, Eremo, Guide Jonathan, Spectulus, Willard, Zoltan, Brewster, Charles, Gail, Grizzly Adams, Guide Tiko, Myra, Ariella, Captain Waverider, Duncan, Marina, Morgan, Raymond Striker, Tyrias, Hjaern, Rashid and Sinatuki. Where the catalogue already serves dialogue, trade, travel, or quest offers, the new actors use those owners, including the four Djinn shops and Scrutinon's free passages.

The display names that differ from a key-derived name come from the registered `npcConfig.name` in the corresponding Crystal `data-global/npc/*.lua` files. The NPC service-model test verifies the complete set. Captain Max's Calassa passage uses the 200-gold price, Postman discount and Explorer Society prerequisite from `captain_max.lua:59–78`. The same source sets `premium = false`.

## Remaining source and map qualification

- Messenger of Santa is a Christmas-window shop. The retained position list repeats Santa Claus's cells in all six cities, and the Wiki describes the messenger as near Santa. A distinct actor anchor at each site still needs qualification.
- Ortelio's retained Wiki cells are in Azzilon on floor 14. The referenced cells have no native tile in the current world map. The old candidate at `33874,32407,8` does not establish the pinned source's Azzilon occurrence.
- Om'Wake Naha resolves to the XML occurrence in Zao. Corym Worker (1) through (5) resolve to the Venore Corym Dungeons. These occurrences are outside the requested slice.

## SERVER fragments

- Captain Haba's open-sea variant needs the Sea Serpent encounter lifecycle and actor hand-off. The dock occurrence is available through the existing stationary actor loader.
- The Buddel variant passages need arrival selection with the donor's random misrouting and the ring-paid guarantee. Static actors keep the catalogue's existing conversation and offers.
- Chemar's carpet confirmations need a carried-scarab-cheese refusal before travel, and Captain Max's Yalahar route needs its source quest prerequisite mapped to a native track.
- Halvar's fight conversation needs a TALK-to-arena entry trigger. Main already owns the arena through its dedicated commands. Frosty's Percht Skull trade needs a mount-grant exchange effect.
- Stateful donor callbacks require their specific native quest bindings and effects. The loaded catalogue supplies the admitted dialogue and service bindings.

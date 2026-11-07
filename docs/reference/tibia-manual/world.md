# Tibia manual notes: world

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=world>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `3d1e52541a38e60c22b8a796d2565d71ee5b77dabb02736220a5381c0282d062`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Server mechanics
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §world <heading>, capture 2026-09-28`.

## §world 5.2.1 Creatures, NPCs and Characters

- Three population categories on the map: creatures (hostile spawns), NPCs (permanent non-hostile inhabitants), characters (player avatars). [server]
- a) Creatures: broad category spanning renegade humanoids, undead, wildlife, monsters; includes rare/unique spawns tied to a single location. [server]
  - Killing creatures grants experience points — core progression loop. [server]
  - Respawn is suppressed while a character remains near the spawn point; creatures repopulate once the area clears. [server]
  - "Monster raids": scripted/ad-hoc surprise group spawns that can assault travelers or even a city. [server]
- b) NPCs: cannot be attacked, always present (no despawn), mostly humanoid, can appear "anywhere," including NPC-type monsters (non-attackable monster-looking NPCs). [server]
  - NPCs can be purchased via the Store as hireling-type furniture for houses; have job-based abilities, e.g. cook, banker, merchant. [server][platform]
  - Talkable NPCs show a speech-bubble indicator (togglable client setting); special bubble variants flag trade/banking function. [client]
  - Some NPCs can mark the minimap with points of interest on request. [client]
  - NPC dialogue opens a dedicated chat window and is mirrored to an "NPC" channel. [client]
- c) Characters: player avatars; can talk/trade/explore/fight each other; conduct governed by Tibia Rules (out of scope here — see rules section). [both]

## §world 5.2.2 Geography

- a) Newhaven and Rookgaard — two alternative starting areas:
  - Newhaven: mandatory first stop for brand-new characters; PvP/player-killing disabled below level 8 regardless of world PvP type. [server]
  - Newhaven offers an optional tutorial (skippable at character creation by leaving a checkbox unchecked) granting starter gear/XP and teaching mechanics. [server][client]
  - Movement from Newhaven onward is one-way: characters cannot return to Newhaven once they leave for the main continent. [server]
  - Rookgaard: legacy beginner island, offered as an alternative to characters who already have a main-continent character on the account; reached via an NPC "seagull" transport right after the tutorial. [server]
  - Rookgaard also one-way (no return once moved to main continent); monk vocation is unavailable to characters who choose Rookgaard. [server]
  - Rookgaard: no PvP permitted regardless of world type; south-western part of the island restricted to Premium accounts. [server][platform]
- b) Main Continent: single largest landmass; mixed biomes (forest, farmland, river, mountains, swamp, desert) implying varied terrain/travel-speed zones; contains ruins/dungeons from past eras. [server]
- c) Premium Areas: reachable only by boat or flying carpet — gates them to Premium accounts. [server][platform]
  - Named premium regions/cities called out: Edron (island, newest settlement), Darama (continent-sized; Darashia and Ankrahmun as rival capitals), Tiquanda jungle with Port Hope colony, Shattered Isles (12 islands, Vandura/Liberty Bay), Ice Islands (Hrodmir/Svargrond, has a boss arena), Zao (Zzaion lizard city, Farmine dwarf outpost), Quirefang (The Hive insect settlement, Gray Beach "Rock Boys"), Fiehonja (Deepling realm). [server]
  - Premium-time expiry while inside a Premium area: on next login the character is auto-relocated to the temple of Thais if citizen of a Premium city (e.g., Edron, Ankrahmun), or to the home-town temple if citizen of a non-Premium city (e.g., Venore, Carlin). [server][platform]

## §world 5.2.3 Cities

- a) 12 chartered cities listed: Ab'Dendriel, Ankrahmun, Carlin, Darashia, Edron, Kazordoon, Liberty Bay, Port Hope, Svargrond, Thais, Venore, Yalahar — each with lore flavor only (no mechanical rules beyond citizenship/geography noted elsewhere). [server]
  - Kazordoon and Svargrond have no public port, so cannot be chosen as citizenship-teleport destination directly from the starting-city selection (see Portals of Citizenship below). [server]
- b) What to Find in Cities — recurring city building types and their rules:
  - **Temples**: PvP disabled (protection zone-like for combat); central location; death respawn point; priests can heal on request (paid or free — manual doesn't state price here). [server]
  - **Depots**: protection zones (no PvP); house the personal locker with three sub-containers plus Market access:
    - Depot Chest: private per character; contains 17 depot boxes; capacity 2,000 items free account / 15,000 items Premium; accessible from any city's depot. [server][platform]
    - Stash: unlimited-count storage for most Market-tradeable items; excludes Store-purchased items, partially-used items (e.g., time-limited rings), and altered items (e.g., imbued gear); has search/sort/filter (incl. "sellable to a given NPC" filter); items moved in via context-menu "Stow" / "Stow all items of this type" or drag-and-drop; same stow mechanism works from backpacks, depot chest, and inbox. [server][client]
    - Inbox: destination for received parcels/letters, items recovered from house loss, and Market-purchase deliveries. [server]
    - Market: entry point to the trading system (detailed in another manual section). [server][platform]
  - **Post Offices**: sell parcels/letters via "Royal Tibian Mail"; delivery targets any depot in the game; sender writes recipient name on label (must be exact spelling) and must place label inside a parcel; posting is done by placing item on a mailbox; invalid address causes the parcel/letter to reappear on the mailbox instead of vanishing, must be retrieved and corrected. [server][client]
    - Editable texts (letters/labels) record last editor name + edit timestamp, visible to recipient. [server]
  - **Shops**: prices vary by city — implies a per-shop/per-city price table needed; large furniture is sold as unpack-in-place construction kits (right-click unwrap), only usable inside rentable houses, cannot be re-packed once unwrapped; beds require separate headboard + footboard kits unwrapped and combined in a rentable house. [server][client]
    - Player-run shops exist inside owned houses (see houses section). [server]
  - **Magic Shops**: sell spellbooks, runes (incl. blank runes), potions. [server]
    - Potion flasks carry a 5 gp deposit; returning an empty flask to any magic shop refunds the deposit via "trade"/deposit request. [server]
  - **Banks**: every character has one bank account, usable in any city and from any city after leaving the starter island. [server][platform]
    - Balance check also surfaces reserved house-bid + first-rent amount while an auction bid is active. [server]
    - Deposit: `deposit <amount> gold` or `deposit all` (confirmation required for "deposit all"). [server]
    - Withdraw: `withdraw <amount> gold`; cannot overdraw. [server]
    - Transfer: `transfer <amount> gold to <name>` — sends money to another character directly. [server]
    - Currency exchange: 1 platinum = 100 gold; 1 crystal = 100 platinum = 10,000 gold; bank can convert gold↔platinum and platinum↔crystal, but NOT gold↔crystal directly (must go through platinum). [server]
    - Newhaven / Rookgaard / Island of Destiny characters hold **junior bank accounts**: cannot rent a house, cannot sell on Market, cannot send/receive bank transfers. [server][platform]
  - **Portals of Citizenship**: every non-starter character must have a home-city citizenship; chosen at the end of the starting-island sequence via captain NPC teleport (only ported cities selectable — excludes Kazordoon/Svargrond at that step). [server]
    - Citizenship is changeable later by walking into any city's "portal of citizenship" (blue shimmering field); re-registers home city; visible on the character info page. [server][client]
    - Death respawn always occurs at the home city's temple once citizenship is held. [server]
  - **Houses**: private rentable buildings; entry restricted to owner/invitees (full rules in houses section). [server]

## §world 5.2.4 Exploring the World

- a) Wilderness terrain types and movement/interaction rules:
  - Blueberry Bush: right-click → "Use" to harvest berries. [server][client]
  - Grass: passable, reasonably fast, slower than roads. [server]
  - Jungle Grass: impassable until cleared with a machete. [server]
  - Loose Stone Piles: signal a hidden dungeon entrance; opened with a shovel used on the pile. [server][client]
  - Roads: fastest movement terrain. [server]
  - Sand: fairly fast to traverse (desert regions). [server]
  - Snow: slows movement (no other stated penalty). [server]
  - Swamp: impassable terrain, must be routed around (eastern regions). [server]
  - Trees: impassable, cannot be chopped down. [server]
  - Water: impassable to walk on (except a distinct underwater zone); items dropped into water are lost permanently; fishing done via fishing rod used on a water tile, may fail on certain tiles or right after another catch there (implies a per-tile fish-availability/cooldown state). [server][client]
  - Wheat: harvestable with a scythe once ripe; harvested wheat bundles can be used to bake bread. [server][client]
- b) Dungeons:
  - Muddy dungeon floors slow movement below normal walking speed. [server]
  - Secret floor holes are detected using a pick axe on the floor tile. [server][client]
  - Descending a hole: simply move onto it. [server]
  - Ascending: requires a rope used on a rope-spot tile adjacent to the hole; a character above can also rope up a character positioned below the hole. [server][client]
- c) Transportation:
  - Boat captains charge a fare; if carried gold is insufficient, the fare is auto-deducted from the character's bank account (implies boat travel checks bank balance as fallback payment source). [server]
  - On crowded ships, captains can "kick" a passenger to a random nearby location outside the ship on request. [server]
  - Flying carpet service: faster, but available only at limited locations. [server]
  - Rumored underground ("dwarven") transport network — unconfirmed in-universe, flavor only. [server]
  - Kazordoon has a distinct ore-wagon transit system requiring a recurring weekly fare payment, then free use of wagons to fixed in-city stops. [server]

## Open questions for Oteryn

- Exact depot capacity enforcement point (soft-cap warning vs hard block) and whether 2,000/15,000 item counts include stacked-item stack counts or slot counts.
- Precise stash exclusion rule set (which "altered"/"partial-use" flags block stashing) needs a concrete server-side item-flag list.
- Fishing tile cooldown/failure mechanic (duration, respawn chance) is unspecified — needs a numeric model.
- Terrain speed modifiers (road/grass/sand/snow/mud) are only qualitatively ranked — need actual speed multipliers for server movement calc.
- Boat fare amounts, and whether "kick" is available identically at every captain, are unspecified.
- Kazordoon ore-wagon fare amount/frequency (exact gold cost, reset day) is unspecified.
- Whether junior bank accounts (Newhaven/Rookgaard/Island of Destiny) have any other restriction beyond house rental, Market selling, and transfers.
- Full 12-city list needs cross-check against current live game city list (Yalahar/Port Hope status, any newer cities) since manual text may lag actual world content.

# Cyclopedia player-operation parity

| Module | Operations | PRESENT | PARTIAL | MISSING | NOT-APPLICABLE |
|---|---:|---:|---:|---:|---:|
| game_cyclopedia | 94 | 48 | 19 | 27 | 0 |
| Total | 94 | 48 | 19 | 27 | 0 |

## game_cyclopedia

OTClient paths are relative to `modules/game_cyclopedia/` at `4955a3fa22d5f5456ec6c658187fd2901545b750`. Oteryn paths refer to `a4ad4164`. Short widget paths are relative to `apps/client/src/bevy_shell/game_ui/`. The audit follows reachable controls across the native shell, feature implementations and client crates. Equivalent actions in separate windows count when they accomplish the same task. Rows that specify a launch point also assess that shortcut. The reference Character Titles pane contains only a scrollbar. Its house rent-sort branch is empty. Neither contributes an operation.

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Browse creature categories and their entries | PRESENT | `tab/bestiary/bestiary.lua:150` | `apps/client/src/bevy_shell/game_ui/bestiary/widgets.rs:84` → `apps/client/src/features/0067-bestiary-charm-progression/input/catalogue.rs:60` |
| Page through the creature catalogue | PARTIAL | `tab/bestiary/bestiary.lua:765` | bestiary/widgets.rs:81 offers first and next. There is no previous-page action. |
| Inspect a creature’s progress, stats and loot | PRESENT | `tab/bestiary/bestiary.lua:451` | `apps/client/src/bevy_shell/game_ui/bestiary/widgets.rs:85` → `apps/client/src/features/0067-bestiary-charm-progression/input/catalogue.rs:40` |
| Search creatures by name | MISSING | `tab/bestiary/bestiary.lua:573` | Searched Bestiary catalogue controls and wire query fields. They accept category, cursor and race ID. |
| Edit the quick-loot list from a creature’s drops | PARTIAL | `tab/bestiary/bestiary.lua:1335` | Quick Loot manages the list separately. Bestiary loot rows have no add/remove action. |
| Track or stop tracking creature kills | PRESENT | `tab/bestiary/bestiary.lua:82` | `apps/client/src/bevy_shell/game_ui/cyclopedia_tracker.rs:522` → `apps/client/src/features/0197-cyclopedia-tracker/input.rs:28` |
| Open the kill tracker to monitor progress | PRESENT | `game_cyclopedia.otui:335` | `apps/client/src/bevy_shell/game_ui/cyclopedia_tracker.rs:267` → `apps/client/src/features/0197-cyclopedia-tracker/input.rs:12` |
| Go back from a creature detail or search result | PARTIAL | `tab/bestiary/bestiary.lua:744` | Bestiary can reload categories or the first page. No saved search or previous results page to restore. |
| Sort tracked creatures by name | PRESENT | `cyclopedia_widgets.otui:1331` | `apps/client/src/bevy_shell/game_ui/cyclopedia_tracker.rs:372` → `apps/client/src/features/0197-cyclopedia-tracker/input.rs:50` |
| Sort trackers by completion percentage | MISSING | `cyclopedia_widgets.otui:1335` | Searched TrackerSort and tracker configuration. Available keys are name, kills and remaining. |
| Sort trackers by remaining kills | PRESENT | `cyclopedia_widgets.otui:1339` | `apps/client/src/bevy_shell/game_ui/cyclopedia_tracker.rs:374` → `apps/client/src/features/0197-cyclopedia-tracker/input.rs:50` |
| Reverse the tracker sort direction | MISSING | `cyclopedia_widgets.otui:1345` | Searched tracker controls and configuration for ascending, descending and reverse. |
| Inspect boss progress and stage thresholds | PRESENT | `tab/bosstiary/bosstiary.otui:74` | `apps/client/src/bevy_shell/game_ui/bosstiary.rs:127` → `apps/client/src/features/0117-boss-encounter/input.rs:38` |
| Page through boss entries | PARTIAL | `tab/bosstiary/bosstiary.otui:260` | Refresh and Next page at bosstiary.rs:171. No Previous page control. |
| Filter bosses by rarity category | MISSING | `tab/bosstiary/bosstiary.otui:116` | The live selector calls boss_encounter_browse with BossCategory::All at bosstiary.rs:368. |
| Filter bosses by kill stage | MISSING | `tab/bosstiary/bosstiary.otui:161` | Searched Bosstiary controls and catalogue query for stage and kill filters. |
| Search bosses by name | MISSING | `tab/bosstiary/bosstiary.otui:242` | The live caller at bosstiary.rs:368 supplies an empty search string. |
| Track or stop tracking boss kills | PRESENT | `tab/bosstiary/bosstiary.lua:306` | `apps/client/src/bevy_shell/game_ui/bosstiary.rs:276` → `apps/client/src/features/0197-cyclopedia-tracker/input.rs:28` |
| Inspect boss points, occupied slots and their bonuses | PRESENT | `tab/boss_slots/boss_slots.lua:7` | `apps/client/src/bevy_shell/game_ui/bosstiary.rs:143` → `apps/client/src/features/0117-boss-encounter/input.rs:38` |
| Assign an eligible boss to a slot | PRESENT | `tab/boss_slots/boss_slots.lua:236` | `apps/client/src/bevy_shell/game_ui/bosstiary.rs:185` → `apps/client/src/features/0117-boss-encounter/input.rs:38` |
| Remove an assigned boss | PRESENT | `tab/boss_slots/boss_slots.lua:301` | `apps/client/src/bevy_shell/game_ui/bosstiary.rs:186` → `apps/client/src/features/0117-boss-encounter/input.rs:38` |
| Identify today’s boosted boss | PRESENT | `tab/boss_slots/boss_slots.otui:80` | `apps/client/src/bevy_shell/game_ui/bosstiary.rs:143` → `crates/client-ui/src/features/0117-boss-encounter/input/slots.rs:124` |
| Search eligible slot bosses by name | MISSING | `cyclopedia_widgets.otui:435` | Searched slot controls and slot candidate selection. Only previous/next boss is exposed. |
| Inspect charms, costs, stages and point balances | PRESENT | `tab/charms/charms1410.otui:275` | `apps/client/src/bevy_shell/game_ui/bestiary/widgets.rs:211` → `apps/client/src/features/0067-bestiary-charm-progression/input.rs:49` |
| Unlock or upgrade a charm | PARTIAL | `tab/charms/charms.lua:781` | bestiary/widgets.rs:290 offers the next stage. bestiary.rs:239 submits immediately, omitting the reference cost confirmation. |
| Assign a charm to a creature | PARTIAL | `tab/charms/charms1410.otui:202` | bestiary/widgets.rs:295 uses the selected creature. bestiary.rs:239 sends the assignment without the reference review step. |
| Remove an assigned charm | PRESENT | `tab/charms/charms1410.otui:94` | `apps/client/src/bevy_shell/game_ui/bestiary/widgets.rs:300` → `apps/client/src/features/0067-bestiary-charm-progression/input.rs:49` |
| Reset all charms | PRESENT | `tab/charms/charms1410.otui:232` | `apps/client/src/bevy_shell/game_ui/bestiary/widgets.rs:305` → `apps/client/src/features/0067-bestiary-charm-progression/input.rs:49` |
| Find an eligible charm target by name | MISSING | `tab/charms/charms1410.otui:148` | Searched charm selector, catalogue widgets and assignment input for a creature-name editor. |
| Inspect character profile and equipment | PRESENT | `tab/character/character.otui:68` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:97` → `apps/client/src/features/0199-cyclopedia-character/input.rs:7` |
| Read general stats and skills | PRESENT | `tab/character/character.otui:247` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:5` → `apps/client/src/features/0199-cyclopedia-character/input.rs:7` |
| Inspect earned and locked badges | PRESENT | `tab/character/character.otui:587` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:80` → `apps/client/src/features/0199-cyclopedia-character/input.rs:7` |
| Review combat statistics | PRESENT | `tab/character/character.otui:648` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:9` → `apps/client/src/features/0199-cyclopedia-character/input.rs:7` |
| Review recent deaths | PRESENT | `tab/character/character.otui:880` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:21` → `apps/client/src/features/0199-cyclopedia-character/input.rs:7` |
| Review recent PvP kills | PRESENT | `tab/character/character.otui:990` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:21` → `apps/client/src/features/0199-cyclopedia-character/input.rs:7` |
| Inspect achievement grades, points and earned dates | PRESENT | `tab/character/character.otui:1079` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:30` → `apps/client/src/features/0199-cyclopedia-character/input.rs:7` |
| Review item holdings by storage location | PARTIAL | `tab/character/character.otui:1337` | cyclopedia/rows.rs:52 shows inventory, depot, stash and inbox quantities. It omits separate Store Inbox quantities. |
| Browse character appearances | PARTIAL | `tab/character/character.otui:1490` | Gallery rows at cyclopedia/rows.rs:66 show catalogue entries. Ownership and source-specific outfit filters are absent. |
| Filter outfits by acquisition source | MISSING | `tab/character/character.otui:1516` | Searched gallery_entries and Appearance controls for standard, quest and store filters. |
| Open the Wheel from character stats | PARTIAL | `tab/character/character.otui:182` | Wheel has a dedicated window. Character widgets lack the reference shortcut. |
| Open the XP boost shop from general stats | PARTIAL | `tab/character/character.otui:280` | Store is accessible independently. No XP gain rate action in Character. |
| Inspect blessings from combat stats | PARTIAL | `tab/character/character.otui:738` | The HUD opens Blessings. The Character combat page renders text without a blessings action. |
| Review Store summary and active benefits | PRESENT | `game_cyclopedia.lua:110` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:70` → `apps/client/src/features/0199-cyclopedia-character/input.rs:7` |
| Review offensive modifiers | PRESENT | `game_cyclopedia.lua:112` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:9` → `apps/client/src/features/0199-cyclopedia-character/input.rs:7` |
| Review defensive modifiers | PRESENT | `game_cyclopedia.lua:113` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:9` → `apps/client/src/features/0199-cyclopedia-character/input.rs:7` |
| Review miscellaneous character modifiers | PRESENT | `game_cyclopedia.lua:114` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:9` → `apps/client/src/features/0199-cyclopedia-character/input.rs:7` |
| Browse the character’s familiars | PARTIAL | `tab/character/character.lua:226` | cyclopedia/widgets.rs:277 identifies a content catalogue. Character ownership is unavailable. |
| Sort achievements by name, grade or unlock date | PRESENT | `tab/character/character.lua:377` | `apps/client/src/features/local-account-achievements/view.rs:93` → `apps/client/src/features/local-account-achievements/view.rs:126` |
| Filter achievements by completion | PARTIAL | `tab/character/character.otui:1162` | The achievement form at apps/client/src/features/local-account-achievements/view.rs:115 filters regular and secret entries. It lacks the locked/accomplished selector. |
| Search character holdings by item name | PRESENT | `tab/character/character.lua:232` | `apps/client/src/bevy_shell/game_ui/cyclopedia/widgets.rs:172` → `apps/client/src/features/0199-cyclopedia-character/view.rs:5` |
| Browse houses in a town | PRESENT | `tab/house/house.lua:317` | `apps/client/src/bevy_shell/game_ui/houses.rs:522` → `apps/client/src/features/0049-house/input.rs:193` |
| Set an auction bid limit | PRESENT | `tab/house/house.lua:972` | `apps/client/src/bevy_shell/game_ui/houses.rs:524` → `apps/client/src/features/0049-house/input.rs:193` |
| Schedule a move-out | PRESENT | `tab/house/house.lua:708` | `apps/client/src/bevy_shell/game_ui/houses.rs:529` → `apps/client/src/features/0049-house/input/ownership.rs:117` |
| Offer a dated transfer to a named buyer for a price | PRESENT | `tab/house/house.lua:566` | `apps/client/src/bevy_shell/game_ui/houses.rs:531` → `apps/client/src/features/0049-house/input/ownership.rs:117` |
| Cancel a pending transfer | PRESENT | `tab/house/house.lua:455` | `apps/client/src/bevy_shell/game_ui/houses.rs:532` → `apps/client/src/features/0049-house/input/ownership.rs:117` |
| Accept an incoming transfer | PRESENT | `tab/house/house.lua:387` | `apps/client/src/bevy_shell/game_ui/houses.rs:533` → `apps/client/src/features/0049-house/input/ownership.rs:117` |
| Reject an incoming transfer | PRESENT | `tab/house/house.lua:312` | `apps/client/src/bevy_shell/game_ui/houses.rs:534` → `apps/client/src/features/0049-house/input/ownership.rs:117` |
| Inspect a house and its auction status | PRESENT | `tab/house/house.otui:325` | `apps/client/src/bevy_shell/game_ui/houses.rs:112` → `apps/client/src/features/0049-house/view.rs:3` |
| Filter houses by building type | MISSING | `tab/house/house.otui:51` | Searched house browser fields and query for guildhall versus house/flat. |
| Browse items by category | PRESENT | `tab/items/items.otui:11` | `apps/client/src/bevy_shell/game_ui/cyclopedia_items.rs:266` → `apps/client/src/features/0132-inspection/input/catalogue.rs:61` |
| Search items by name | PRESENT | `tab/items/items.otui:222` | `apps/client/src/bevy_shell/game_ui/cyclopedia_items.rs:440` → `apps/client/src/features/0132-inspection/input/catalogue.rs:48` |
| Inspect an item’s attributes | PRESENT | `tab/items/items.otui:336` | `apps/client/src/bevy_shell/game_ui/cyclopedia_items.rs:275` → `apps/client/src/features/0132-inspection/input/catalogue.rs:61` |
| Restrict items to own vocation | MISSING | `tab/items/items.otui:69` | Searched item catalogue controls and item_catalogue_filter. Only name and category are accepted. |
| Filter weapons by number of hands | MISSING | `tab/items/items.otui:91` | Searched catalogue filter for handedness, one-hand and two-hand. |
| Look up NPC buyers and sellers for an item | MISSING | `tab/items/items.otui:407` | Searched the item inspection model, item catalogue and NPC shop for a reverse lookup of merchants by item. Shop offers require a merchant session. |
| Choose the source used to value loot | PARTIAL | `tab/items/items.otui:238` | Analyser accepts a unit price. No per-item NPC/market/own source switch in Cyclopedia Items. |
| Track drops of a catalogue item | PARTIAL | `tab/items/items.otui:532` | Drop Tracker supports confirmed loot (analyser/tracker.rs:68). Catalogue entries have no tracking control. |
| Exclude an item from quick selling | MISSING | `tab/items/items.otui:576` | Searched quick sell, quick sale, skip selling and per-item sale exclusions. |
| Open loot container management from item details | PARTIAL | `tab/items/items.otui:654` | Quick Loot handles containers separately. The item catalogue has no Manage Containers action. |
| Filter items by equipment class | MISSING | `tab/items/items.lua:1066` | Searched item catalogue query and controls for class or classification. |
| Filter spells by vocation | PRESENT | `tab/magicalArchives/magicalArchives.lua:157` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:207` → `apps/client/src/features/0287-actor-spell-catalog/input.rs:60` |
| Select a spell and inspect its details | PRESENT | `tab/magicalArchives/magicalArchives.otui:55` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:73` → `apps/client/src/features/0287-actor-spell-catalog/view.rs:43` |
| Zoom the world map | PRESENT | `tab/map/map.otui:418` | `apps/client/src/bevy_shell/game_ui/satellite_map.rs:88` → `apps/client/src/bevy_shell/game_ui/satellite_map.rs:28` |
| Change map floor | PRESENT | `tab/map/map.otui:465` | `apps/client/src/bevy_shell/game_ui/cyclopedia_map.rs:238` → `apps/client/src/features/0057-cyclopedia-map/input.rs:62` |
| Switch surface and map rendering | MISSING | `tab/map/map.otui:203` | Searched atlas, satellite map and minimap render controls for surface view and map view. |
| Adjust map floor compositing opacity | MISSING | `tab/map/map.otui:217` | Searched atlas, satellite map and minimap projection for separator or layer opacity controls. |
| Navigate between Cyclopedia subjects | PARTIAL | `game_cyclopedia.lua:427` | Subject windows are reachable independently. The Bevy Character tab selector at cyclopedia.rs:42 only links character pages. |
| Return to the previous Cyclopedia subject | MISSING | `game_cyclopedia.lua:419` | Searched Cyclopedia window state and toolbar routing for a cross-subject navigation history. |
| Open a tracked creature or boss directly from its tracker | MISSING | `tab/bestiary/bestiary.lua:866` | Searched cyclopedia_tracker row controls for detail navigation. Rows offer Stop. |
| Inspect a creature’s Animus Mastery XP bonus | MISSING | `tab/bestiary/bestiary.lua:431` | Searched Bestiary detail widgets and charm progression state for Animus and mastery XP. |
| Filter houses by rental or auction status | MISSING | `tab/house/house.lua:141` | Searched house query fields and house browser controls for rented and auctioned filters. |
| Sort houses by name | MISSING | `tab/house/house.lua:1023` | Searched house catalogue controls and page state for a sort key. |
| List own houses | PRESENT | `tab/house/house.lua:73` | `apps/client/src/bevy_shell/game_ui/houses.rs:526` → `apps/client/src/features/0049-house/input/ownership.rs:97` |
| Set a custom loot value for an item | PRESENT | `tab/items/items.lua:527` | `apps/client/src/bevy_shell/game_ui/analyser.rs:188` → `apps/client/src/features/0010-analyser/input.rs:71` |
| Restrict equipment to the player’s level | MISSING | `tab/items/items.lua:725` | Searched item catalogue filters for level requirement and usable equipment. |
| Include or skip a catalogue item when quick looting | PARTIAL | `tab/items/items.lua:907` | quick_loot/widgets.rs:520 updates the filter separately. Item details have no quick-loot checkbox. |
| Look up market prices for an item | PRESENT | `tab/items/items.lua:197` | `apps/client/src/bevy_shell/game_ui/cyclopedia_items.rs:431` → `apps/client/src/features/0132-inspection/input/catalogue.rs:61` |
| Search spells by name or incantation | PRESENT | `tab/magicalArchives/magicalArchives.lua:169` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:231` → `apps/client/src/features/0287-actor-spell-catalog/input.rs:60` |
| Pan the world map | PRESENT | `tab/map/map.lua:333` | `apps/client/src/bevy_shell/game_ui/satellite_map.rs:137` → `apps/client/src/bevy_shell/game_ui/satellite_map.rs:151` |
| Filter map flags by icon or show all | MISSING | `tab/map/map.lua:206` | Searched atlas, satellite map and minimap controls for icon visibility filters. Satellite rendering checks location and floor. |
| Switch holdings between list and grid | MISSING | `tab/character/character.lua:385` | Searched holdings controls and projection for grid or list mode. |
| Filter character holdings by storage location | PARTIAL | `tab/character/character.lua:254` | cyclopedia/widgets.rs:202 offers inventory, depot, stash and inbox. Store inbox has no separate filter. |
| Sort houses by floor area | MISSING | `tab/house/house.lua:1027` | Searched the house catalogue controls and query for area or size ordering. |
| Order houses by auction activity | MISSING | `tab/house/house.lua:1032` | Searched the house catalogue for bid and auction-end sorting. Both reference comparators prioritize houses with bids. |

## Gaps ranked by player impact

Within each tier, the operation names match the evidence table.

### 1. Gold, loot and equipment decisions

- Edit the quick-loot list from a creature’s drops (PARTIAL).
- Review item holdings by storage location (PARTIAL).
- Restrict items to own vocation (MISSING).
- Filter weapons by number of hands (MISSING).
- Look up NPC buyers and sellers for an item (MISSING).
- Choose the source used to value loot (PARTIAL).
- Track drops of a catalogue item (PARTIAL).
- Exclude an item from quick selling (MISSING).
- Open loot container management from item details (PARTIAL).
- Filter items by equipment class (MISSING).
- Filter houses by rental or auction status (MISSING).
- Restrict equipment to the player’s level (MISSING).
- Include or skip a catalogue item when quick looting (PARTIAL).

### 2. Hunt planning and progression

- Page through the creature catalogue (PARTIAL).
- Search creatures by name (MISSING).
- Go back from a creature detail or search result (PARTIAL).
- Sort trackers by completion percentage (MISSING).
- Reverse the tracker sort direction (MISSING).
- Page through boss entries (PARTIAL).
- Filter bosses by rarity category (MISSING).
- Filter bosses by kill stage (MISSING).
- Search bosses by name (MISSING).
- Search eligible slot bosses by name (MISSING).
- Unlock or upgrade a charm (PARTIAL).
- Assign a charm to a creature (PARTIAL).
- Find an eligible charm target by name (MISSING).
- Open the Wheel from character stats (PARTIAL).
- Inspect blessings from combat stats (PARTIAL).
- Filter achievements by completion (PARTIAL).
- Open a tracked creature or boss directly from its tracker (MISSING).
- Inspect a creature’s Animus Mastery XP bonus (MISSING).

### 3. Catalogue navigation and presentation

- Browse character appearances (PARTIAL).
- Filter outfits by acquisition source (MISSING).
- Open the XP boost shop from general stats (PARTIAL).
- Browse the character’s familiars (PARTIAL).
- Filter houses by building type (MISSING).
- Switch surface and map rendering (MISSING).
- Adjust map floor compositing opacity (MISSING).
- Navigate between Cyclopedia subjects (PARTIAL).
- Return to the previous Cyclopedia subject (MISSING).
- Sort houses by name (MISSING).
- Filter map flags by icon or show all (MISSING).
- Switch holdings between list and grid (MISSING).
- Filter character holdings by storage location (PARTIAL).
- Sort houses by floor area (MISSING).
- Order houses by auction activity (MISSING).

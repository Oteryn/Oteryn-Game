# questpc-n3 retained engine dependencies

Merged main e9276ad7e and engine source925d8480 were inspected. Existing
QENGINE2 APIs are retained as delivery dependencies; no replacement engine is
authored here. Continue with standalone claims whose engine is already on main.

- ENGITEM/WPKG3: native11545-11552 material admission (real PG RED DefinitionNotFound87.09s); ENGNPC: source NPC final receipts/journal completion currently hardcoded false. Local reproduction retained.

  Needed by: A Father's Burden Quest

- ENGITEM/map identity: source bridge item48227 remains typed unavailable by explicit owner decision (#2781 comment6549960114); merged SEASON W05d supplies independent Cake mechanics but retains this full-path dependency.

  Needed by: A Piece of Cake

- QENGINE2: canonical authored stage binding API delivery on main (Bindings::attach, MapUse/StepIn/Kill/NpcDialogue). Qualified source925d8480 includes it, merged main e9276ad7e does not. Historical reward-only proof is insufficient for the full authored route.

  Needed by: A Pirate's Death to Me; A Pirate's Tail Quest; Adventures of Galthen Quest; Ancient Aucar Outfits Quest; Annual Autumn Vintage; Asura Palace Quest; Berserk Potion Quest; Berserker Treasure Quest; Between the Lines Quest; Bewitched; Bloody Tusks Quest; Cartography 101 Quest; Cave Explorer Outfits Quest; Child of Destiny Quest; Cults of Tibia Quest; Dangerous Depths Quest; Dead Archer Quest; Decaying Defender Outfits Quest; Demon's Lullaby; Draccoon Herald Outfits Quest; Dragon Corpse Quest; Dragon Slayer Outfits Quest; Falconer Outfits Quest; Feaster of Souls Quest; Feral Trappers Quest; Fishing Box Quest; Frost Dragon Quest; Geomancer Quest; Grave Danger Quest; Grimvale Quest; Heart of Destruction Quest; Hero of Rathleton Quest; Illuminated Warrior Outfits Quest; Illuminator Outfit Quest; Illuminator Outfits Quest; In Service of Yalahar Quest; Insectoid Outfits Quest; Inukaya Quest; Iron Helmet Quest; Kilmaresh Quest; Krailos Quest; Last Creep Standing; Lion Trophy Quest; Liquid Black Quest; Machinery of War Quest; Make Believe Quest; Measuring Tibia Quest; Medusa Shield Quest; Mystery of the Valley Quest

- QENGINE2: source-declared unordered required/alternative reward-set completion; each placement currently binds one fixed transition and failed prerequisite obligations become terminal REFUSED. Do not complete on the first chest.

  Needed by: Alawar's Vault Quest; Bear Room Quest; Behemoth Quest; Black Knight Quest; Circle Room Quest; Crystal Wand Quest; Demona Ring Quest; Double Hero Quest; Draconia Quest; Dragon Tower Quest; Dreamer's Challenge Quest; Edron Goblin Quest; Elvenbane Quest; Emperor's Cookies Quest; Fanfare Quest; Ferumbras' Ascension Quest; Fire Axe Quest; Forgotten Knowledge Quest; Ghoul Room Quest; Goblin Temple Quest; Griffin Shield Quest; Katana Quest; Koshei the Deathless Quest; Longsword Quest; Mad Mage Room Quest; Minotaur Hell Quest

- QENGINE2: atomic multiple root physical rewards (11700x1 +3035x10), still absent in925d8480.

  Needed by: An Interest In Botany Quest

- ENGITEM: revision-fenced10s ground-object reset.

  Needed by: An Uneasy Alliance Quest

- QENGINE2: occurrence-scoped personal quest progress/completion for repeatable world events, composed with existing WorldCounter owners and source shared mutations/resets. Current quest state is keyed only by character and canonical quest.

  Needed by: Awash World Change; Bank Robbery Mini World Change; Bored Mini World Change; Chakoya Iceberg Mini World Change; Chyllfroest Mini World Change; Deeplings World Change; Demon Wars World Change; Devovorga's Essence Mini World Change; Down the Drain Mini World Change; Fire from the Earth Mini World Change; Fury Gates Mini World Change; Hive Born World Change; Hive Outpost Mini World Change; Horse Station World Change; Insectoid Invasion World Change; Jungle Camp Mini World Change; Kingsday Mini World Change; Lumberjack Mini World Change

- ENGENC: credited arena death triggers source pillar removal/next-pit portal creation with exclusive pit lease and timed kick/reset.

  Needed by: Barbarian Arena Quest

- ENGITEM/QENGINE2: replay-bound random mead outcomes and live drunk-condition mammoth action.

  Needed by: Barbarian Test Quest

- QENGINE2: atomic multiple root item rewards and read-only STEP_IN gates with source-ordered refusals.

  Needed by: Bigfoot's Burden Quest

- ENGNPC: locked live necklace-slot3083 predicate; carried evidence is insufficient.

  Needed by: Blood Brothers Quest

- ENGITEM: five-lever four-cell conditional toggles, revision-fenced5s lever reset and public player speech.

  Needed by: Children of the Revolution Quest

- QENGINE2/ENGNPC: source-saga completion must preserve mainland/Rook/source-alias boundaries under the shared canonical outfit key; reward-only scoped proof cannot close the aggregate.

  Needed by: Citizen Outfits Quest

- ENGITEM/ENGENC: item-arrival event, seven-altars world readset, corpse consumption and boss summon in one fence.

  Needed by: Elemental Spheres Quest

- ENGITEM: cross-floor elevator batch item movement and displacement of every platform occupant.

  Needed by: Goblin Merchant Quest

- ENGENC/QENGINE2: source Feroxa occurrence RNG/boss lifecycle; existing full-moon gate explicitly does not create the source encounter.

  Needed by: Grimvale Mini World Change

- ENGNPC: authoritative current-year projection and once-per-calendar-year source bonus writes; existing UTC month alone is insufficient.

  Needed by: Hot Cuisine Quest

- ENGITEM: objective receipt coupled to successful native machete transform and decay.

  Needed by: Kissing a Pig Quest

- ENGITEM: pagoda/amphora15s reset and flower60s regrowth.

  Needed by: Lion's Rock Quest

- ENGNPC: final source NPC item receipt must set canonical journal completion in the same fenced operation; current npc_program writer hardcodes completes=false.

  Needed by: Marlin Trophy Quest

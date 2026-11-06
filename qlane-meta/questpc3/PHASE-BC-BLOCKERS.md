> Current continuation: the latest direct task instruction supersedes this historical file's prohibition on building missing engine pieces. Routing hooks are authorized; no owner decision is pending. Exact current terminal blockers and attempted paths are in FINAL-BLOCKERS.md and owner-final-blockers.json. No title is qualified merely by the boundary audit below.

# questpc3 remaining owner boundaries

The 17 already qualified paths remain the current implementation result. No new
quest is qualified by this investigation. Use existing source captures; no new
reference collection is needed. This file does not turn a partial storyline into
a completed quest.

## Phase B

- **Sinatuki Quest:** `wiki/Sinatuki Quest_Spoiler.wiki` requires cumulative mixed
  fish hand-ins, the final fish, five shards, Jamjam, and Barbarian/Ice Islands
  prerequisites. The captured `reference/crystal-baseline/data-global/npc/sinatuki.lua`
  instead grants five shards for a single 99/100-fish species stack. Preserve the
  wiki's cumulative requirement; do not add a repeatable 99-fish reward. Missing:
  an atomic hand-in/progress/reward boundary. `quest_dialogue.rs` commits only
  QuestState transitions; `item_craft/npc_exchange.rs` debits one unit and mints
  one item, without quest progress. Neither can substitute for the full hand-in.
- **Sam's Old Backpack Quest:** the pinned walkthrough requires the rare Orc's
  backpack, Sam's two confirmations, destruction of its contents, Backpack
  Tourist, Kroox's permission, the quest door and the Dwarven Armor chest.
  Missing: the same atomic item/progress owner and source-specific container
  destruction. The four leaf-only Rookgaard rules cannot safely consume a filled
  backpack or stand in for Kroox's access state.
- **Steal From Thieves Quest:** the pinned walkthrough permits taking Ahmet's
  400 oz bag before accepting the mission. Returning it must atomically consume
  the bag, grant the promised reward and complete the mission. Missing: the
  item/progress owner above; a repeatable leaf barter would bypass the once-only
  mission and acquisition/capacity rules.
- **Rottin Wood and the Married Men Quest:** the existing `classic_talk.rs`
  explicitly defers the physical ribbon hand-outs. Mission 1 needs seven ribbons
  and knife-on-rabbit-corpse acquisition before turn-in; later missions include
  world callbacks. The existing state-only start of Mission 2 is not a complete
  quest. Missing: the physical hand-in owner and the corpse callback path.
- **Swamp Fever World Change:** Ottokar's existing local donor script consumes
  Medicine Pouch 12517, grants Belongings 12413 and advances Doctor! Doctor!
  (100 deliveries). The pinned wiki additionally describes daily contributions
  affecting Feverish Citizen spawns. `world-change/src/content.rs` retains source
  candidates with unresolved spawn/compiler bindings; `daily.rs` does not apply
  Swamp Fever. Qualifying the barter alone would leave the named world change
  incomplete. Missing: its contribution/spawn binding, not another chest claim.

Do not build a new generic NPC engine in this lane under the owner's current
coordination note. Revisit these boundaries when their owning implementation is
available. No individual NPC-audit file for these candidates was present in the
shared research directory at this checkpoint; the existing downloaded sources
above were used. The task's directory restriction prevents writing research
outside this worktree.

## Phase C

`content/cosmetics/outfits/index.json` is still `READY_UNPOPULATED`. The item-use
owner supports source rules, but canonical outfit/addon grant mappings and their
per-field promotion evidence are independently required by
`crates/gameplay/content/src/content/native_gameplay/native_cosmetics.rs` and
`crates/gameplay/content/src/content/native_gameplay/cosmetics_loader/grant_mappings.rs`. The existing
`item_cosmetics/packet.rs` fixture explicitly authors synthetic proposed identities;
it cannot qualify a production outfit.

`tools/content-schema/cosmetics-authoring/README.md` explicitly retains unresolved
sex/look, starter/addon, fallback and source-conflict decisions and the pending CP
population slot. This lane cannot manufacture those decisions from source look IDs.

This blocks complete outfit qualification for Rascoohan, Revenant, Rift Warrior,
Rootwalker, Royal Bounacean Advisor, Royal Costume, Shaman, Soil Guardian and
Summoner. Several also depend on other lanes' base quests or encounter results.
Do not make a numeric donor look ID sufficient for a canonical grant, remove a
base-outfit prerequisite, or label a synthetic-packet test production evidence.

The three earlier Phase A blockers remain: Small Ruby and Skeleton Decoration
need shared floor-reset pickup/remint; Serpentine Tower needs the pot-conditioned
teleport. Those engine changes belong to qengine, as the coordination note states.
All 38 unfinished titles remain individually listed in QUEST-STATUS.md.

Restart recheck: reset activation itself is implemented in
`durability/world_reset/activation.rs` and composed by the game-server reset owner.
The comment in `planned_world_reset_source_map.rs` about an unimplemented step-4
writer is stale. Small Ruby's actual committed tile is (32437,32174,15):
Item3030/count1, depth0, between ground103 and fire2123. No map edit is needed.
However, `NativeMovementCollisionIndex::source_world` returns `None` for Bundle;
`CurrentNativeMapInitialization::qualify_current_binding` refuses without an
actual source world. Thus the bounded source-map reset fixture does not prove
pickup/remint of this production B3 tile. Skeleton Decoration additionally needs
the nested barrel child path (`native_spell_world.rs` skips depth != 0).
These remain qengine dependencies; this lane did not modify that shared engine.

## Remaining storyline and event boundaries — restart sweep

Scope remains all R–S titles. The following completes the boundary inventory;
it does not reduce the assignment to the 17 qualified paths. Each walkthrough
below is the existing `wiki/<title>_Spoiler.wiki` capture. No new reference was
collected. The owner directs this lane to record a missing shared feature and
move on, rather than implement that feature here (NOTE-coordination.md).

The NPC item/progress gap described in Phase B also prevents replacing a
storyline hand-out or hand-in with the four repeatable, leaf-only exchanges.
The existing `npc_command/quest_dialogue.rs` explicitly owns QuestState changes
only. A state change followed by a separate Item transaction would leave a
crash window; an unconditional chest would bypass the source prerequisite.

| Unfinished title | Source requirement and missing composed path |
| --- | --- |
| Rathleton Quest | Marvin spends voting points, promotes ranks and affects shared fortnightly shortcut/dungeon/boss selections. Missing the atomic character debit/world vote tally and reset-to-map/spawn binding; personal QuestState is insufficient. |
| Ravenous Madness Quest | Hector's mission includes Trollwut, the Maddening Mushroom corpse acquisition, vocation rewards and delivery to Benjamin. Missing the quest-bound corpse Item/progress and encounter/access composition. Its optional 20-wolf task cannot stand in for completion. |
| Realm of Dreams Quest | The source requires a previously claimed Draconia book, carried Chayenne key, clearing the live dragon lair, a field/switch/lever puzzle and a five-minute wall. Missing the live area-clear/puzzle/access composition; qualifying the final corpse reward alone would bypass it. |
| Research and Development Quest | Spectulus issues a measurement device; four grave uses spawn slimes; Sinclair issues and takes back a ghost-filled Spirit Cage. Missing atomic NPC Item/progress and captured-corpse/cage progress composition. |
| Rest in Hallowed Ground Quest | Costello issues holy water for Amanda; fifteen distinct graves consume its charges before Amanda grants the potion package. Missing atomic NPC Item/progress and the exact multi-grave flask-to-empty completion path. |
| Rise of Devovorga | The September event changes world stages across resets, coordinates five incarnation deaths, transforms the final encounter and gates participant rewards. Missing that shared event/encounter/participant owner; a personal once claim cannot implement it. |
| Roshamuul Quest | Mortar deliveries and kills rebuild a shared bridge; horns/shrines/kills open a shared wall; Gaz'haragoth advances the world. Missing contribution-to-world-stage/map/encounter composition. Individual QuestState flags cannot replace world progression. |
| Rotten Blood Quest | Existing access payment, taint/kill/replay and account entitlement owners are present. However, `gameplay_transport/endgame_entry.rs` explicitly returns `ContentUnavailable` from its W02 handoff. A seeded access/native-kill fixture does not qualify the source lever-to-instance chain. Preserve those owners; missing qualified encounter admission belongs to the shared encounter lane. |
| Sea of Light Quest | The beggar's paid plans, Spectulus' timed study and devices, crystal mining/teleport and Collector corpse capture form one chain. Missing the NPC Item/progress and captured-corpse/device completion composition; Spectulus' Liquid Black adapter is a different quest. |
| Secret Service Quest | The three mutually exclusive factions have physical mission Items and final tool rewards. Missing atomic quest-bound hand-ins/hand-outs; a repeatable barter or separate faction flag and Item write would bypass the once-only branch. |
| Shadows of Yalahar Quest | Telas' chain includes physical golem parts, charge devices and later ritual/encounter callbacks. Missing the NPC Item/progress and those world/encounter compositions. The existing Isle of Evil dialogue explicitly defers its preceding Shadows branch. |
| Shards of a Broken Moon Quest | Tide markers, refined plant samples, blessed blue lava, Rakesh and Moonsnow Magnolia precede the icy prison escape and final reward. Missing Item/progress, time-conditioned world callbacks and qualified encounter/access composition. |
| Soul War Quest | Existing Flickering Soul, taints, native death settlement, reward chest and Revenant entitlement owners are present. The same `endgame_entry.rs` W02 handoff still returns `ContentUnavailable`; source room admission/party encounters cannot be substituted with captured test deaths. Preserve the existing progression/reward owners. |
| Spike Tasks Quest | Level-banded repeatable tasks issue tools, apply per-target cooldowns, require corpse/world actions and award fame/XP. Missing atomic NPC Item/progress and task-specific world/corpse/cooldown composition; the existing generic task board is not this source path. |
| Spirithunters Quest | Research and Development precedes Spirit Cage captures and tiered 5-ghost/5-nightstalker/5-souleater turn-ins, XP, coins and achievement. Missing the same captured-corpse/cage and atomic NPC Item/progress boundary; no free repeatable leaf trade may replace it. |
| Spring into Life | Seasonal hidden nests grant Surprise Nests; dragon nests hatch until reset and hatchlings have the event XP bonus. Missing seasonal activation/world nest/reset/spawn composition. Existing carried surprise-bag tests qualify opening bags, not acquisition or this event. |
| River Runs Deep Mini World Change | The active Zao river must permit Sandfish fishing and Desert Fisher. Missing daily native river-layer/fishing/achievement binding. |
| Spider Nest Mini World Change | The active nest admits Mamma Longlegs kills and denies entry after the third kill/Nestling achievement. Missing daily spawn/portal binding and the corresponding kill-to-entry state. |
| Spirit Grounds Mini World Change | Exactly one of three location portals opens daily, independently of the hunting-ground monster variation. Missing native daily portal/layer/spawn selection binding; no personal reward claim is involved. |
| Stampede Mini World Change | The active day supplies Terrified Elephants with their loot and grants Trail of the Ape God after five kills. Missing daily spawn and source kill/achievement binding. |
| Steamship World Change | Junkar accepts shared coal contributions; 200 coal enables the next working period at server save and controls passage. Missing atomic coal contribution/world counter/reset/travel composition. |

`world-change/src/content.rs` retains source candidates with unresolved native
layer, spawn/NPC/raid, announcement and compiler bindings; it explicitly cannot
activate them. `world-change/src/daily.rs` applies its named existing daily
changes, not the R–S events above. Read-only owner searches are retained in
`restart-next-owners.log` and `restart-next-events.log`. Creature/Item catalogue
entries, reference Lua and lower-level progression owners were not treated as
proof that a whole source quest is playable.

All 38 unfinished titles now have a concrete boundary here: the earlier three
Phase A cases, five Phase B cases, nine outfit cases and these 21 paths. None
is counted as a new qualification, and no shared engine, command, capability
or migration was added in this sweep.

# EVENTS-WORLD-1 World Changes, Fury Gates and Nightmare Isles

- Decision: `EVENTSWORLD1-WORLD-CHANGES-V1`
- Status: **CANDIDATE** (architect-authored). Acceptance needs exact-head validation, independent review (determinism, persistence, map/content and protocol) and protected integration. Owner acceptance is the owner's to give; none is recorded here.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the mechanics completion plan rows 103 and 104 (`OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md`, batch EVENTS-WORLD-1: "world changes, Fury Gates, Nightmare Isles, hazard"); the task record `docs/agents/tasks/EVENTS-WORLD-1.md` ("brief child graph for world changes"); BOSS-RAID-0 §3 and §18, which leave World Changes and the Hazard System to separate decisions; the scope matrix row "Raids and events" (scope declared in the event definition, no implicit default).
- Builds on: BOSS-RAID-0 (accepted 2026-10-15: §4 raid definition, schedule, firing and channel run; §9 eligibility; §12 lock order and World jobs); ADR-0021 (accepted: §4.4 channel overlay, §4.7 planned reset, reset epoch and bundle activation); WORLD-INTERACTION-0 (accepted: §6.5 teleports, §8.5 HAZARD-1 traps, §9 volatile state and reset, §11 World clock); HOUSE-OWN-0 §9 (the World job pattern); GAME-CHANNEL-01 §13 to §15 (multiplicity classes, simulation and eligibility scope); ENCOUNTER-RT-0 (accepted 2026-10-14: encounter runtime); ADR-0001 §7; the content and ruleset tree (`OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md`); owner rule 5905825574 (Global parity).
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`
- Runtime, migration, wire and production authority: NONE until accepted and allocated. Each child needs its own #162 allocation.

## 1. Scope

This brief decides what a World Change is in Oteryn: its content, its durable state, who activates it and when, how its map, creatures and NPCs reach every channel, and how it ends. Fury Gates and Nightmare Isles are its first two instances. Mini World Changes use the same model.

It does not decide the numbers of any individual World Change (chance, rotation, weights), which are content and have no source in the repository yet (§5), and it does not decide the Hazard System (§6).

## 2. Reference grounding

- **Manual.** `docs/reference/tibia-manual/` has no text on World Changes, Fury Gates, Nightmare Isles or the Hazard System. It names an Event Schedule, a 12-month events calendar on the title screen (`starting.md:68`), and the Improved Respawn Rate area chosen per server save (`interface.md:43`, CYCLOPEDIA-0).
- **TibiaWiki achievements snapshot** (`imports/tibiawiki/achievements/2026-09-29/tibiawiki-achievements-facts.json`) names World Changes (Twisted Waters, The Mage's Tower, Thornfire, The Mummy's Curse, Horse Station) and Mini World Changes (Bank Robbery, Bored, Devovorga's Essence, Down the Drain, Fire from the Earth, Kingsday, Lumberjack, Nomads, Noodles is Gone, River Runs Deep, Stampede, Thawing, Warpath). It names the World Quest Rise of Devovorga.
- **TibiaWiki hunting places snapshot** (`imports/tibiawiki/hunting-places/fandom-snapshot-v1.json`, page "Nightmare Isles"): city Darama, **three** location coordinates, creatures Silencer, Retching Horror, Choking Fear and Terrorsleep.
- **Crystalserver source tree** (`imports/crystalserver/summer-update/source-tree.json`; paths only, no file contents are in the repository):
  - scripts `data-global/scripts/world_changes/`: `fury_gates.lua`, `nightmare_isles.lua`, `oriental_trader.lua`, `thawing.lua`, `grimvale_respawn_event.lua`, `their_masters_voice.lua`, `spawns_npc_by_time.lua`; `data-global/scripts/raids/world_changes/flower_month.lua`;
  - map files `data-global/world/world_changes/`: `fury_gates/` with **ten** city files (abdendriel, ankrahmun, carlin, darashia, edron, kazordoon, libertybay, porthope, thais, venore), `nightmare_isle/` with **three** (ankrahmun-north, darashia-north, darashia-west), and `full_moon/`, `oriental_trader/`, `grimvale/`;
  - the raid `data-global/raids/fury_gate/furyosa.xml`; world-change quest actions under `data-global/scripts/actions/worldchanges/` and `data-global/scripts/quests/deeplings_worldchange/`;
  - Hazard System: `data/libs/systems/hazard.lua`, `data-global/scripts/systems/hazard_primal.lua`, `data/scripts/eventcallbacks/monster/ondroploot_hazard.lua`.
- **Consequence.** The repository shows *which* World Changes exist and that Fury Gates and Nightmare Isles are map variants chosen from a fixed set (10 and 3). It holds no activation chance, rotation, duration or weight. Those are open points (§5), never guessed.
- **Name collision.** WORLD-INTERACTION-0 §8.5's HAZARD-1 is map traps and damaging tiles (`environment_damage`, RNG purpose `trap_damage`, `WORLDINT0-RL-15`). The Tibia Hazard System (`rulesets/encounters/hazard-system/`, the Canary files above) is a different mechanic. The task record's dependency "HAZARD-1 (LEVER-1, COND-1)" is the trap mechanic; this brief narrows it (W12).
- **Content path.** The content tree places World Changes at `content/encounters/world-changes/` and `content/encounters/mini-world-changes/` (both `READY_UNPOPULATED`). The task record's owned path `content/world_changes/` does not match; this brief follows the tree (§5).

## 3. Rules

**W1. A World Change is content.** One definition per change under `content/encounters/world-changes/<key>` (kind `WORLD_CHANGE`) or `content/encounters/mini-world-changes/<key>` (kind `MINI_WORLD_CHANGE`), compiled into the World Bundle with source provenance per field. A definition has:
- `key`, `kind`, `trigger` = `at_reset {chance_permille}` or `calendar {month, day, epochs}`; nothing else in v1 (W11);
- `variants[]`, each `{variant_key, weight, map_layer?, spawns[], npcs[], raids[]}`; a change with one outcome has one variant;
- `multiplicity`: the GAME-CHANNEL-01 §14 class of every value-producing source in it (W7); required, no default;
- `announcement` text keys (W8) and `exclusive_with[]` (keys that cannot be active in the same epoch).

The compile check refuses a definition above the §3 W14 ceilings, without a multiplicity class, with a layer that touches a house tile (HOUSE-CUSTODY-0) or with an `exclusive_with` cycle that has no resolution order. Code holds no World Change.

**W2. Activation happens once per reset epoch, at the planned reset.** In the ADR-0021 §4.7 activation transaction (step 4, which writes epoch N+1 and ACTIVATED), the World writes one row per World Change of the active bundle into `game_world_change_activations` (WorldId, reset epoch, change key): `state` ACTIVE or INACTIVE, the chosen `variant_key`, the bundle digest and the draw inputs. The draw uses the simulation-determinism RNG with the new purpose `world_change_activation`, seeded by (WorldId, change key, reset epoch): first the chance, then, for an active change, the weighted variant. Changes are evaluated in key order; an `exclusive_with` conflict makes the later key INACTIVE. A replay of a crashed reset writes the same rows (the rows are keyed; the draw is a pure function of the key). This is an ADR-0021 §4.7 amendment, pending on acceptance of this brief. A World without activation rows for the current epoch does not open admission.

**W3. One decision per World, the same on every channel.** Every channel of the World runs the same active set and the same variants for the whole epoch. A channel never draws on its own. A channel that starts mid-epoch (a restart, a new channel) reads the activation rows and builds the same state.

**W4. Map changes are bundle layers, not runtime edits.** A World Change map variant is compiled into the World Bundle as a named optional layer: tiles that replace base tiles inside a bounded area, validated by the World Project compiler with the bundle. The Crystalserver map files are source evidence only; each layer is re-authored through the World Project pipeline with provenance. A channel composes base plus the layers of its epoch's active variants at channel start, and the composition is immutable for the epoch. Layer tiles are not channel overlay state: they do not count against `MAP01-CHANNEL-OVERLAY-BYTES`, and the channel overlay (doors, levers, moved items) sits on top of the composed map exactly as on the base (WORLD-INTERACTION-0 §9.1). Because a layer is chosen only at the reset, when every overlay is empty, Ground is retired and every channel starts fresh (ADR-0021 §4.7), no live map mutation and no overlay migration ever happens. A channel restart rebuilds base plus the same layers from the durable rows (the task's restart-safety criterion). This is an ADR-0021 §4.2 and §4.7 amendment, pending on acceptance of this brief.

**W5. A World Change ends at the next planned reset.** v1 has one duration, `until_next_reset`. Nothing is reverted at runtime: the next epoch composes its own layers, the channels start with empty overlays, and runtime creatures and NPCs go with the reset. Durable results (items looted, receipts, quest state) stay. A character whose durable position falls on a tile that the new composition makes unusable takes the CHAR-POSITION-0 §3.3 fallback at admission (ADR-0021 §4.4). Mid-epoch start or end is split out (§6).

**W6. Creatures, raids and NPCs of a World Change.**
- **Spawns** of an active variant are ordinary channel-local spawn sources (GAME-AI-01), active for the epoch on every channel, rebuilt after a channel restart like any base spawn.
- **Raids**: a raid that belongs to a World Change (the Fury Gate raid `furyosa`) is a RAID-1 raid whose definition gains `requires_world_change {key, variant?}`; the BOSS-RAID-0 §4.2 check skips it in an epoch where the change is not active. This is a BOSS-RAID-0 §4.1 amendment, pending on acceptance of this brief. Everything else of the raid (firings, channel runs, LOST on restart, §9 eligibility) is unchanged.
- **NPCs**: an NPC present only during a World Change (the oriental trader) is content whose placement names the change and variant; NPC-0 places it at channel start when active.
- **Encounters**: mechanics of a World Change that need the encounter vocabulary run on ENCOUNTER-RT-0; nothing here adds a runtime.

**W7. Multiplicity and rewards.** Each value-producing source in a World Change declares its GAME-CHANNEL-01 §14 class. Ordinary creatures are `CHANNEL_LOCAL_REPEATABLE`, as base spawns. Raid bosses keep BOSS-RAID-0 §9 (`firing_id` eligibility). A unique World Change reward (a quest-like reward, a boss outside a raid) is `EXPLICIT_EVENT_POLICY_REQUIRED` and stays held (its variant is refused by the compile check) until its own policy declares the simulation and eligibility scope (GAME-CHANNEL-01 §15). ChannelId is never inserted into a Character reward key.

**W8. Announcement.** After admission opens, each channel announces the epoch's active World Changes to each admitted character once per session, through the BOSS-WIRE-1 raid message kind (BOSS-RAID-0 §13). No new wire in v1. How the client lists active changes (Event Schedule, Cyclopedia) is an open point (§5).

**W9. Operator override.** The control plane may record, before a reset, an override for the next epoch only: force a change ACTIVE with a named variant, or INACTIVE (`game_world_change_overrides`, consumed by W2 in the same transaction and recorded on the activation row with the operator and reason). There is no mid-epoch override.

**W10. Fury Gates.** One World Change `fury_gates`, kind `WORLD_CHANGE`, `at_reset`, with ten city variants (the ten Crystalserver map files). Each variant's layer holds the gate: a WORLD-INTERACTION-0 §6.5 teleport into the Fury Gate dungeon, whose region is base map. The raid `furyosa` names `requires_world_change {fury_gates}` (W6). Chance and weights are open points.

**W11. Nightmare Isles.** One World Change `nightmare_isles`, kind `WORLD_CHANGE`, with three variants (ankrahmun-north, darashia-north, darashia-west), matching the three Crystalserver map files and the three TibiaWiki coordinates. Each layer holds its isle and the isle's spawns (Silencer, Retching Horror, Choking Fear, Terrorsleep; content). How often the active isle changes is an open point; until it is sourced, the content packet authors it as `at_reset` with equal weights and marks it `PARITY_PENDING`.

**W12. HAZARD-1 is a content prerequisite only.** A World Change layer may contain WORLD-INTERACTION-0 traps and damaging tiles; they use HAZARD-1's triggers unchanged. EVENTS-WORLD-1 children that ship such a layer wait for HAZARD-1; the others do not. The Tibia Hazard System is not part of this brief (§6).

**W13. Responsibility, persistence and locks.**

| Who | Owns |
|---|---|
| Content (WCHANGE-CONTENT-1) | definitions, variants, layers, chances and weights (W1) |
| World (reset activation, WCHANGE-ACT-1) | the per-epoch decision and its rows (W2, W9); nothing else writes them |
| Channel runtime | composing base plus layers at channel start (W4), spawns and NPC placement (W6), announcements (W8); reads the rows, never writes them |
| RAID-1 | world-change raids, gated by W6's precondition; firings and eligibility unchanged |
| Control plane | the next-epoch override (W9) |

`game_world_change_activations` rows are immutable once written; override rows are consumed once. Lock order follows BOSS-RAID-0 §12: the reset record, then World rows by key. Any World job over these rows follows HOUSE-OWN-0 §9 with at most `BOSSRAID0-RL-05` (100) rows per pass. No `CharacterRevision` advances.

**W14. Rows (proposed; registered by the children before implementation).** Engineering ceilings, not reference values.

| Row | Value |
|---|---|
| `EVENTSWORLD1-RL-01` World Change definitions per bundle | 256; the compile check refuses more |
| `EVENTSWORLD1-RL-02` variants per World Change | 16 (Fury Gates needs 10) |
| `EVENTSWORLD1-RL-03` active World Changes per World per epoch | 32; W2 marks later keys INACTIVE beyond it |
| `EVENTSWORLD1-RL-04` tiles of one layer | 65,536; the composed map stays inside `MAP01-BASE-RSS-BYTES` and the MAP01 bundle rows |
| `EVENTSWORLD1-RL-05` World Change spawns per channel | set from measurement by WCHANGE-RT-1 before activation, as `HOUSERT0-RL-04` |
| Activation | 1 activation row per change per epoch, 0 items, 0 value lines |
| World job pass | `BOSSRAID0-RL-05` (cited, unchanged) |

## 4. Packets (proposed)

Allocated one by one by the control plane after acceptance; each its own PR.
1. **WCHANGE-CONTENT-1** (content): the `world-changes` and `mini-world-changes` families, schema, generator and the W1 compile check, with Fury Gates and Nightmare Isles definitions and provenance. No runtime.
2. **WCHANGE-LAYER-1** (hard, persistence and map review): optional bundle layers in the compiler and the channel composition (W4), the restart rebuild, and the map revision of a composed map (the layer set digest bound into the session's map revision, a MAP-WIRE amendment of its own). Needs MAP-LOAD-1 and MAP-OVERLAY-1.
3. **WCHANGE-ACT-1** (hard, determinism and persistence review): migration for the activation and override tables, W2 in the reset transaction, the RNG purpose, the boot refusal without rows, crash-replay tests. Needs WCHANGE-CONTENT-1 and the ADR-0021 §4.7 reset record (the MAP-OVERLAY-1 family).
4. **WCHANGE-RT-1** (hard, combat and content review): W6 spawns and NPCs, the RAID-1 precondition, the Fury Gate teleport. Needs EVENTS-RAID-1 (RAID-1), WCHANGE-ACT-1, WCHANGE-LAYER-1, FLOOR-1; HAZARD-1 only for a layer with traps (W12).
5. **WCHANGE-ANNOUNCE-1** (impl): W8. Needs BOSS-WIRE-1 and CHAT-1.

Order: WCHANGE-CONTENT-1, then WCHANGE-LAYER-1 and WCHANGE-ACT-1 in parallel, then WCHANGE-RT-1, then WCHANGE-ANNOUNCE-1.

## 5. Open points (not decided here)

- Activation chance, variant weights and any minimum gap for Fury Gates, Nightmare Isles and each Mini World Change: no repository source. The content packet sources them (Crystalserver script contents, TibiaWiki pages) or marks them `PARITY_PENDING`; never guessed.
- Whether a Tibia World Change may stay active across several server saves (W5 v1 says one epoch).
- How the client shows active World Changes (Event Schedule, Cyclopedia): CYCLOPEDIA and client presentation, with its own wire if any.
- The task record's owned path `content/world_changes/` should become `content/encounters/world-changes/` and `content/encounters/mini-world-changes/` (the content tree); the record is not edited here.

## 6. Split out, not decided here

Each needs its own future decision packet; nothing here allocates it.
- **The Hazard System** (`rulesets/encounters/hazard-system/`; Canary `hazard.lua`, `hazard_primal.lua`, `ondroploot_hazard.lua`): a per-character hazard level that strengthens monsters of designated areas and adds loot. It touches combat (GAME-ABILITY-01, ATTACK-0), loot (D3) and Character state, none of which a World Change owns. Proposed name HAZARD-SYS-0.
- **Player-triggered and mid-epoch World Changes** (changes started by quest progress, community kill counts or a time of day, such as `spawns_npc_by_time.lua` and the Twisted Waters chain): need a mid-epoch layer or spawn switch and a trigger contract.
- **World Quests** (Rise of Devovorga; `content/encounters/world-quests/`): World-scoped progress across channels.
- **Event Schedule calendar events** (the 12-month calendar; seasonal rate events): a ruleset/event-policy decision. W1's `calendar` trigger covers only a World Change on a fixed date.
- **Improved Respawn Rate** (`interface.md:43`): CYCLOPEDIA-0 owns it.

## 7. Handback

```yaml
result: CANDIDATE
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_EVENTS_WORLD1_DECISION_2026-10-15.md
resource_values_changed: false
production_authority_changed: false
implementation_may_resume: false
required_fresh_allocation: true
required_independent_review: "determinism, persistence, map/content, protocol"
amends_pending_acceptance: "ADR-0021 §4.2 and §4.7 (W2, W4); BOSS-RAID-0 §4.1 (W6)"
```

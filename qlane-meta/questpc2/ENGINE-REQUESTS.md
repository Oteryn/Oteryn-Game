# questpc2 n3 shared-engine handoff

Audited main `1076254706e4ddc05a6d0a1aba0c1777af2038ac`. Scope is the immutable packet at `2f12120aa:qlane-meta/questpc2/LANE-L3.json`. The local iteration table is `.lane/rows.md` and is deliberately untracked.

## Existing capability to reuse

- Fixed party admission, monster lists, occupancy and timed reset already exist in `world_callbacks::Encounter` and dispatch `quest_callbacks/encounter.rs`; they are not missing generic features.
- W02 catalogue entry and W10 corpse/loot are on main. A component entry/death receipt is not a whole quest fight.
- QENGINE2 `925d8480ef1cb88b16154a1dbf018a57ac22152a` exposes qualified `authored_bindings::Bindings::attach` (world proof `aa85b250027fa68c88c391f6cda490f0764dbf02`, native-admission proof `c7f9d9ba79ec4a53c4b874af54affe6898780c5a`). Do not request or rewrite that API. Its world-clock slice is compiling WIP with post-merge validation pending in `qlane-meta/qengine2/ENGINE-QUEUE.json` on that branch.
- ENGITEM `bbe0184821852828c3686d33d936709c9859618f` has sixteen-track durability receipt work but still retains `MAX_NPC_TRACK_CHANGES = 8` in the NPC planner. ENGNPC now supplies the larger atomic NPC bound; do not request a second capacity implementation.
- ENGENC `4ffd19f3ad28778f0bd283c350f27b34eae92d60` extends private boss loot; it does not supply the quest-specific phase controllers below. Reuse that loot owner when those fights land.

- ENGNPC `3423ac56e151843f62652e6bbb45984b05c18aaf` supplies sixteen-track atomic NPC/reward receipts and migration `1600_quest_atomic_track_capacity.sql`. This resolves the capacity portion of Dangerous Depths' dependency. Its `npc_rules/item_exchange.rs` retains the existing guard/effect allowlist, so combined item/points/clock reports still need implementation. This branch is available for reuse, not yet on main.

## Requests

- **25 Years of Tibia Quest** — `QENGINE2: live_blessing_and_promotion_predicates`. Check all regular blessings and vocation promotion under the same fence as time-machine admission; level/premium alone are insufficient.
- **Asura Palace Quest** — `ENGENC: participant_scoped_kill_to_door`. Credit the selected chamber participant and open the corresponding door only after that participant's two required defenders die; a global creature-name counter is insufficient.
- **Barbarian Arena Quest** — `ENGENC: arena_room_progression`. Ten ordered rooms per tier with exclusive occupancy, death-driven next-room admission and tier completion. Existing fixed-roster Encounter is reusable admission, not a ten-room controller.
- **Bewitched** — `QENGINE2: world_callback_transaction_clock_operations`. Daily personal cauldron contribution must reset from the authoritative clock; award the chosen 1000 XP only after Bane-Bringer credit. Reuse the pending world-clock implementation.
- **Child of Destiny Quest** — `ENGENC: training_target_hit_credit`. Deliver fenced spear-hit and magic-missile-hit training events to quest progress. A USE packet or NPC message must not stand in for a successful target hit.
- **Cults of Tibia Quest** — `ENGENC: cults_boss_phase_closures`. Execute the Essence of Malice multi-actor encounter and participant settlement; Gerimor's already pushed assignments do not prove boss completion.
- **Dangerous Depths Quest** — `ENGNPC: npc_item_clock_receipt`. Reuse the published sixteen-track ENGNPC receipt for Gnomus measurements/charting. The remaining report transaction combines item grant, points and SetNowPlus, which the plain-item planner still refuses. Keep Father's Burden eleven-track write atomic.
- **Elemental Spheres Quest** — `ENGNPC: live_level_vocation_npc_item_plan`. Arkulius requires live level/vocation predicates in the same source NPC item/quest receipt. Do not copy these values into ordinary quest tracks.
- **Forgotten Knowledge Quest** — `ENGENC: halls_of_hope_boss_phase_closures`. Provide encounter-owned Dragon King and Last Lore Keeper phases, vulnerable forms and participant settlement. Albinius's pushed tome and portal donations are only prerequisite slices.
- **Grimvale Quest** — `ENGENC: feroxa_form_transition_and_reset`. Run the selected full-moon Feroxa form transitions and encounter reset. Maeryn's pushed helmet-choice dialogue neither starts nor completes this fight.
- **Heart of Destruction Quest** — `ENGENC: otherworld_boss_phase_settlement`. Bind vulnerability/vortex phase deaths to the admitted Otherworld party and settle World Devourer completion through the encounter owner.
- **Hero of Rathleton Quest** — `ENGENC: jaccus_generator_phase_controller`. Execute Jaccus Maxxen's generator/construct-dependent vulnerability transitions and final death settlement; a static monster list does not execute the phase closure.
- **Make Believe Quest** — `ENGENC: moonspawn_damage_and_multi_actor_phase_controller`. Damage participation must count without last hit; cannon/shield repeats, vocation charge windows, eight simultaneous Petrified Fanatics and persistent Phosphorus hazards remain encounter-owned.
- **Ravenous Madness Quest** — `QENGINE2: room_scoped_native_kill_credit`. Accept Trollwut death credit only while the credited participant is inside his chamber, excluding doorway kills. The published authored KillCounter has no room/participant-position predicate.
- **Rotten Blood Quest** — `ENGENC: rotten_blood_additional_source_closures`. Implement Chagorz/Murcion extra_monster_closure and Bakragore nonliteral.timeToFightAgain before template admission. Ichgahal/Vemiath do not share those capture blockers.
- **Soul War Quest** — `ENGENC: goshnar_phase_and_sanity_controller`. Execute Megalomania Aspect of Power, splinter, sanity and vulnerability cycles. W02 entry/receipt support exists; soul_war.lua still has additional_zone_closure and multiple-lever capture blockers.
- **The Demon Oak Quest** — `ENGNPC: npc_bank_item_magic_atomic_receipt`. Oldrak's Hallowed Axe transaction must atomically take the Axe and exact 1000 gold including bank, grant the Hallowed Axe and commit its magic effect. Source NPC topic state must remain one owner.
- **The Dream Courts Quest** — `ENGNPC: live_player_name_dialogue_match`. Stricken Soul's source branch matches the current character's name after its confirmations; NpcRuleInput lacks that name. Do not hardcode a name or pre-set the house-access flag.
- **The Inquisition Quest** — `ENGNPC: source_npc_committed_magic_effect`. Five guard interviews require source magic-effect branches in the fenced NPC owner; storage-only evaluation rejects MagicEffect. Keep Henricus recruitment as a partial slice.
- **The Order of the Lion Quest** — `ENGENC: fugue_encounter_closure`. Run Fugue's encounter-owned phases and participant victory settlement before Kesar's final report; native boss entry alone is not proof of this closure.
- **The Pits of Inferno Quest** — `QENGINE2: live_world_callback_premium_predicate`. Enforce the chosen premium requirement at the initial hub STEP admission, together with level 80. World Callback has minimum_level but no live premium condition; a final reward premium check admits non-premium traversal too early.
- **The Roost of the Graveborn Quest** — `ENGENC: roost_sequential_boss_phase_settlement`. Bind Fatal Bug/adventurer-group and later Roost boss deaths, ritual attunements and final settlement to the admitted encounter participants.
- **Tower Defence Quest** — `ENGNPC: atomic_multi_type_npc_grant`. Commit Bunny's four chosen fifty-arrow reward types with the completion flag in one receipt. Source NPC item_exchange_keys permits at most one grant type; RewardCallback.give has one root. Do not invent a reward bag or split completion from three of the grants.
- **Twenty Miles Beneath the Sea Quest** — `ENGNPC: live_equipped_item_npc_predicate`. Check the chosen Helmet of the Deep prerequisite from the live equipped head slot when accepting the underwater mission. NpcRuleGuard::Equipped exists but neither storage-only nor plain-item NPC planners admit it.
- **Within the Tides Quest** — `ENGENC: timira_multi_stage_water_encounter`. Execute Timira's successive bucket/corrupted-water stages, creature replacement and delayed room evacuation. Candidate still carries config.encounter and additional_zone_closure; do not substitute Tarun's unrelated Lost Brother source storage.
- **Goblin Merchant Quest** — `QENGINE2: live_creature_item_use_transform_and_credit`. Guardcatcher must target the live adventurer, remove exactly that actor and grant its captured carried form with bounded per-character credit. Reuse the queued Tainted Souls live-creature item-use mechanism; world-object ItemOnTarget is insufficient.

## Concrete current API evidence

All main references below are at `1076254706e4ddc05a6d0a1aba0c1777af2038ac`:

- `crates/gameplay/quests/src/quest/npc_rules.rs:25`: eight-track bound. `npc_rules/evaluate.rs:325-368`: storage-only guard/effect allowlists and the dedicated unsplit Father's Burden exception.
- `crates/gameplay/quests/src/quest/npc_rules/item_exchange.rs:8-65`: plain item guards/effects, one grant type; equipped/level/vocation, money and magic are outside this planner.
- `crates/gameplay/quests/src/quest/world_callbacks.rs:158-225`: bounded Callback schema; level gate exists, premium/blessing/promotion predicates do not. `world_callbacks/effects.rs:176-191`: fixed Encounter roster/spawn/reset schema.
- `crates/gameplay/content/src/content/bosses/activity_template.rs:450-460`: source-hook qualification refuses unimplemented closures; `content/encounters/bosses/crystal-lever-candidates.json` records the Rotten Blood, Soul War and Timira blockers named above.
- At QENGINE2 `925d8480e`, `crates/gameplay/quests/src/quest/authored_bindings/kills.rs:11-23` contains creature/track/gate fields, without room or credited-participant position. Its existing qualified arbitrary kill binding must be extended, not copied.

## Scope and decisions

- These are first shared dependencies. Resolving one returns the quest to implementation; it does not automatically qualify the remaining content, journal, reward or completion path.
- Pits of Inferno uses the chosen seven-throne itinerary. Do not reinstate omitted source vocation/sacrifice/timing puzzles as blockers. The missing live premium admission predicate is independent of those omitted puzzles.
- Twenty Miles keeps the chosen five-skull ring-recovery arc. Helmet of the Deep means equipped head gear on mission acceptance, not an item merely in a bag. This is an explicit Oteryn decision; no new map anchor, arbitrary NPC source storage, or source-equivalence claim is introduced.
- Tower Defence must retain four distinct fifty-arrow grants. A fabricated container would change the chosen reward; four independently acknowledged commands would split its completion transaction.
- 25 Years follows the chosen four-landmark memory tour; Lord Retro is a report stage in that recipe. A boss fight is not asserted as a prerequisite.
- Within the Tides must use the Marapur/Timira source closure. Tarun's `data-global/npc/tarun.lua:65-96` at crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d` belongs to Lost Brother; SHA256 `8e032f7015dd3ee2135ba9552a588a3f26083327e23fb7a1619423486c482fde`. Timira source `data-global/scripts/quests/marapur/actions_boss_timira_fight.lua:174-241,299-364`, SHA256 `49604399bcd3bd22e62b62babc48b6ad72d61dc0f8ec889ee218ba7654b1ed68`, supplies the staged fight evidence.
- Three whole-quest routes were previously pushed on the frozen questpc2 family branches: Elvenbane, Annihilator and Desert Dungeon. Their stored PG proofs are referenced in QUEST-STATUS.json; they are not reruns on n3 and are still awaiting collector integration.
- Six world-change entries remain excluded by cross-lane ownership. No new quest outside the packet was started. No migrations or shared engine code are introduced here.

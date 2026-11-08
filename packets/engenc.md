Reasoning effort: medium.

Task ENGENC: build the quest-engine mechanics that quest lanes reported as ENGINE-REQUEST, area: encounters, bosses, waves, arenas, portals, private loot.

Base: main (merge origin/main first). Branch to push: claude/happy-davinci-ijz3wy-engenc (no force, no PR). Migration numbers reserved for you: 1620-1639 only (do not use others; never edit merged migrations). Before writing a feature, check main and branches claude/happy-davinci-ijz3wy-qengine2, -questl2b, -questpc-pc2, -wpkg2 for an existing implementation and reuse it instead of duplicating (git log -S / git grep).

Requests (deduplicate first; many are the same feature reported by several lanes):
ENGINE-REQUEST questl2: route selected carried-item use onto a live corpse with exact identity and expiry checks, atomically consuming the item and advancing quest state | needed by: Koshei the Deathless; Research and Development; Rottin Wood and the Married Men; Sea of Light | where: gameplay-dispatch/cosmetic_item_use; gameplay-durability/quest_world_callback
ENGINE-REQUEST questpc2: admit item closures from explicitly pinned alternate donor XML, retaining source/default-field checks | needed by: Rotten Blood Quest | where: tools/content-migration/encounter_loot_item_admissions.py:43
ENGINE-REQUEST questpc2: admit private reward-boss loot independently of physical corpse slot count while retaining bounded loot plans | needed by: Soul War Quest; Rotten Blood Quest | where: crates/gameplay/content/src/content/creature_reward.rs
ENGINE-REQUEST questpc2: committed item USE advancing encounter phases, world-item transforms and reset cleanup | needed by: Within the Tides Quest (Timira) | where: world_callbacks / native encounter owner; pinned `actions_boss_timira_fight.lua:174–241,299–364`
ENGINE-REQUEST questpc2: item-on-target wave progression with four counters, source-bound summons, player damage and a monster-empty completion guard | needed by: The Demon Oak Quest | where: crates/gameplay/quests/src/quest/world_callbacks/effects.rs (donor actions_demon_oak.lua:67–126, SHA 1ac4c709)
ENGINE-REQUEST questpc2: participant-bound arena fight sequencing and tier settlement | needed by: Barbarian Arena Quest | where: crates/gameplay/encounters
ENGINE-REQUEST questpc2: participant-bound multi-wave encounters with kill completion and cleanup | needed by: Hero of Rathleton Quest; Barbarian Arena Quest; The Demon Oak Quest | where: crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/timed_encounter.rs
ENGINE-REQUEST questpc2: private boss loot admission independent of physical corpse capacity | needed by: Soul War Quest | where: crates/gameplay/content/src/content/creature_reward.rs
ENGINE-REQUEST questpc2: qualify activity templates for registered source boss hooks, summons and reward closures | needed by: Rotten Blood Quest; Soul War Quest | where: crates/gameplay/content/src/content/bosses/activity_template.rs:362
ENGINE-REQUEST questpc2: timed encounter waves with shared room occupancy, cleanup and boss-death portal | needed by: Hero of Rathleton Quest | where: crates/gameplay/quests/src/quest/world_callbacks/effects.rs; donor movements_professor_maxxen.lua:3–30,56–82, SHA256 6005aec4
ENGINE-REQUEST questpc2: use already-qualified native item admission for loot without changing frozen canonical SOURCE receipts (item 49271) | needed by: Soul War Quest | where: tools/content-migration/register_spell_families.py
ENGINE-REQUEST questpc3: Source-bound boss-room exits and native portal activation | needed by: The First Dragon Quest | where: dispatch encounter lifecycle and content/world/placements
ENGINE-REQUEST questpc3: StepIn boss admission using existing occupancy, named-boss cooldown and timed room cleanup owners | needed by: The First Dragon Quest | where: gameplay/quests world_callbacks::Encounter and gameplay/dispatch quest_callbacks/encounter.rs
ENGINE-REQUEST questpc3: creature StepIn transformations and fenced boss-death phase callbacks, including nearby-player credit and temporary corpse portals | needed by: The First Dragon Quest | where: gameplay/quests world_callbacks and gameplay/dispatch encounter lifecycle
ENGINE-REQUEST questpc3: live-creature item-use callbacks with atomic soul transformation and per-character credit, including summoned Tainted Souls | needed by: The Tainted Souls Quest | where: crates/gameplay/dispatch/src/gameplay_transport/quest_carried_use.rs and crates/gameplay/durability
ENGINE-REQUEST questpc5: claimant-fenced actual corpse-loot pickup credit | needed by: Tutorial | where: quick_loot corpse settlement and quest admission
ENGINE-REQUEST questpc5: observed temporary-disguise predicates with matching portal admission and exit cleanup | needed by: Last Creep Standing | where: crates/gameplay/quests/src/quest/world_callbacks.rs; reuse questpc8’s investigation at d078ecd29
ENGINE-REQUEST questpc5: quest progress from looting an item issued by an exact creature death, fenced to the claimant | needed by: Tutorial | where: gameplay-dispatch corpse/loot settlement and gameplay-quests callback admission
ENGINE-REQUEST questpc5: temporary disguise restoration and matching portals | needed by: Last Creep Standing | where: appearance condition and portal owners
ENGINE-REQUEST questpc7: quest-tool USE on an exact native ground corpse, with atomic tool transformation and progress | needed by: Goblin Merchant Quest | where: dispatch/quest_callbacks.rs use_shared_callback_handle
ENGINE-REQUEST questpc: item-arrival trigger with atomic seven-altar read-set, corpse consumption, boss summon and world-state fence | needed by: Elemental Spheres Quest | where: quests/world_callbacks.rs and dispatch item-arrival/encounter owner

Do, in order of how many quests each unblocks:
1. One generic engine feature per commit (not per quest), in the quest engine/dispatch/durability crates, atomic and replay-safe like existing quest transitions, with its own test red then green including its own PG case (prove the test executed: duration/marker). No quest content in this task, only the engine capability plus a minimal fixture.
2. A request that is really content-only, already supported, or needs an owner decision: do not build it; list it in the reply with the reason.
3. Focused checks only: own tests, cargo check, clippy of changed crates, fmt, regenerate generated files/pins with repo tooling. No full nextest, no full PG suite, no node boot.
4. Push after every green feature.
5. Reply in at most 15 lines: SHA, features done (with which quests they unblock), requests skipped with reason.
Reasoning effort: medium.

STANDING RULES (from the 2026-11-08 night retro; apply to every push from now on, in addition to PREPUSH-2):
1. Order before every push: merge origin/main FIRST, then make your changes, then regenerate EVERY generated file/packet LAST (quest packets, NPC dialogue links, completion matrix, pins).
2. If you touched content/, quest data, NPC data or tools/content-schema: run locally the quest-authoring checks AND the quest-authoring unit-tests runner (same commands as the CI jobs 'quest-authoring (checks)' and 'quest-authoring (unit-tests)'), and the content-tree validation. Push only when they pass.
3. Sha-pinned proofs (e.g. samples/completion-matrix/reward-use-proofs pins test_sha256 of postgres_tests/qready1_reward_claims.rs): if you change a pinned test file, rerun its PG cases for real (duration > 0.01 s) and re-import with the proof tool (quest_reward_use_proofs.py import_log). Never hand-edit a sha.
4. If you touch node boot / serve / content activation: run the WHOLE node-boot script locally, every variant (including spell_input=full_manifest), not only the stage you fixed.
5. A branch with an open PR is FROZEN: push only CI fixes or a main merge I asked for.
No reply needed; keep working on your current task.


PC LANE NOTE: you run on the owner PC worktree (fresh origin/main). Do NOT commit and do NOT push (sandbox cannot write the git index); leave all changes in the worktree, the operator commits and pushes to the branch named above. Focused checks only (fmt, cargo check/clippy of changed crates, own tests incl. own PG cases if a PG route exists). End with at most 15 lines: files changed, tests run with results.

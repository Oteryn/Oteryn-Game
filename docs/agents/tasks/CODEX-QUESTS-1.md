# CODEX-QUESTS-1: Codex quest lanes (communication record)

```yaml
task_id: CODEX-QUESTS-1
title: "CODEX-QUESTS-1: Codex builds or finishes 63 global quests"
mode: COORDINATION
status: active
repository: Oteryn/Oteryn-Game
coordination: "Content session (Claude) <-> ChatGPT Codex"
branches:
  - claude/happy-davinci-ijz3wy-qcx5
  - claude/happy-davinci-ijz3wy-qcx6
merge_authority: CP_ONLY
```

This draft PR is a communication channel only. It is never marked ready and never
merged. It is closed when lanes qcx5 and qcx6 have landed.

## Protocol

- Codex reads this file and the task text below, works on the lane branches
  (not on this PR branch) and posts one PR comment per finished batch:
  `READY <branch> <full sha> <summary>`, `BLOCKER <quest> <missing mechanic> <donor file:line>`
  or `OPEN-FACTS <quest> <fact>`.
- The Content session (Claude) answers in PR comments: review findings, fact lookups,
  scope changes. A comment from the Content session takes precedence over this file.
- The coordinator merges the lane PRs; Codex opens none.

### Task text: Codex task: build or finish 63 Tibia global quests in Oteryn-Game

Repository: Oteryn/Oteryn-Game. First read AGENTS.md and the skills it names (.claude/skills/oteryn-dev, oteryn-coding, oteryn-verify, oteryn-review). The repository rules and the machine contracts in docs/agents/ win over this prompt.

### Goal
Bring each quest below to the owner's done bar: it **passes a real E2E walk on the node-boot server**, and its **rewards, requirements and mechanics match Tibia global** (TibiaWiki first, then the internet). Whatever exists on global gets built; OT-only content (not on global) does not.

### Sources, in this order
1. TibiaWiki over the API (tibia.fandom.com/api.php): the quest page plus /Spoiler.
2. If something is missing there: https://talesoftibia.com/libsearch/, https://talesoftibia.com/, https://tibiasecrets.com/transcripts (NPC dialogue), https://elkolorado.github.io/tibialibraries/ (books).
3. Then YouTube (quest walkthroughs).
4. Mechanics implementation: the Crystal/Canary donor (the reference evidence in the repo). Where the donor differs from global, global wins.
Never guess a value. Whatever stays unconfirmed goes on the report's list of open facts.

### Work rules
- Branches only claude/happy-davinci-ijz3wy-qcx5 and claude/happy-davinci-ijz3wy-qcx6 (one writer each). Never main, no force-push, no rebase, no empty commits.
- Push after every finished step: nothing may stay only in the container.
- Do not open PRs. When a batch is done, report: `READY <branch> <full sha> <summary>`.
- Never weaken or skip tests. Do not edit workflows, .github/repository-policy.json or tools/repository/**.
- World pins are regenerated (`oteryn-world-bundle-compiler pin-refresh .`), never edited by hand; `pin-check` must pass.
- Merge origin/main locally with the repository merge drivers (`tools/merge-driver/install.sh`).
- PostgreSQL tests go into the auto-mounted directories (oteryn-dev, references/mechanic.md); a case with no PG URL returns Ok(()).
- `oteryn-gameplay-dispatch --lib` needs about 21 GB of RAM. If your environment has less, do not run it; put the exact command in the report so it can be run on the PC.
- Batches of about 5 quests or about 500 changed lines; one batch is one commit series and one READY.

### Do not touch (other lanes are working on it)
- Claude lanes qa, qb, qc, qd, qg, qh (reward-claim chests, outfits, NPC storylines, travel/doors, world changes, authored approximations) and CORPSE48 (corpse limit 48).
- Order of the Stag (stagpos), Newhaven/Corruptor, ship captains (captains), the node-boot region list (edronmap).
- These PRs: #3042 #3115 #3056 #3111 #3120 #3102 #3100 #3124 #3145 #3104 #3190 #3181 #3252 #3256.

### Dependencies on server mechanics
These quests need mechanics from the server lane: kill/boss/summon credit (S1 qkill), timed storage/gates (S2 qstore), mounts (S3 qmount). First check whether they are on origin/main (or on the server-lane branches). If a mechanic is missing, **do not build it yourself in the quest batch**. Write `BLOCKER <quest> <missing mechanic> <donor file:line>` and move on to the next quest.

### Steps for each quest
1. Global: requirements (level, premium, earlier quests), every step, reward (items with exact ids and counts, XP, achievements, outfits, access), from the sources above.
2. Repo: definition in content/quests/definitions, callbacks in content/quests/world-callbacks(-held), reward claims in content/interactions/reward_claims, NPCs and placements, uniques and actions in content/world/placements. Write down every difference from global.
3. Fix the content to match global (import tools, not hand-written data where a tool exists); move the callbacks out of held when the mechanic exists.
4. Test: a PostgreSQL case that drives the real path (dispatch/route), red before the change and green after.
5. Check that the quest locations are in the node-boot regions (tools/qualification/node_boot/run.sh, compile-regions). If not, report it (do not edit run.sh: the edronmap lane owns it).
6. Report row: quest, status (DONE-CONTENT / BLOCKER / OPEN-FACTS), differences fixed, tests.

### Quest list (63)

#### Lane qcx5 — bossy, zabicia, przywołania (45)
- An Uneasy Alliance Quest (`oteryn:quest.an_uneasy_alliance_quest`), mechanizmy: door_progress, npc_dialogue, quest_state, scheduler, trigger_kill, trigger_use, world_object
- Barbarian Arena Quest (`oteryn:quest.barbarian_arena_quest`), mechanizmy: achievement, condition, door_progress, npc_dialogue, quest_state, reward_claim, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Blood Brothers Quest (`oteryn:quest.blood_brothers_quest`), mechanizmy: creature_removal, door_progress, item_consume, item_hand_out, npc_dialogue, quest_state, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Cults of Tibia Quest (`oteryn:quest.cults_of_tibia_quest`), mechanizmy: boss_cooldown, condition, creature_removal, door_progress, item_consume, item_hand_out, npc_dialogue, quest_state, reward_claim, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Dangerous Depths Quest (`oteryn:quest.dangerous_depths_quest`), mechanizmy: achievement, boss_cooldown, creature_removal, item_consume, item_hand_out, npc_dialogue, quest_state, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Dark Trails Quest (`oteryn:quest.dark_trails_quest`), mechanizmy: door_progress, npc_dialogue, quest_state, reward_claim, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Elemental Spheres Quest (`oteryn:quest.elemental_spheres_quest`), mechanizmy: item_consume, quest_state, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Feaster of Souls Quest (`oteryn:quest.feaster_of_souls_quest`), mechanizmy: boss_cooldown, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Ferumbras' Ascension Quest (`oteryn:quest.ferumbras_ascension_quest`), mechanizmy: achievement, creature_removal, door_progress, item_consume, item_hand_out, mount, npc_dialogue, outfit, quest_state, reward_claim, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Forgotten Knowledge Quest (`oteryn:quest.forgotten_knowledge_quest`), mechanizmy: achievement, creature_removal, item_consume, item_hand_out, npc_dialogue, quest_state, reward_claim, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Formorgar Mines Quest (`oteryn:quest.the_ice_islands_quest`), mechanizmy: achievement, condition, door_progress, item_consume, npc_dialogue, quest_state, reward_claim, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Grave Danger Quest (`oteryn:quest.grave_danger_quest`), mechanizmy: condition, door_progress, item_consume, item_hand_out, npc_dialogue, quest_state, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Grimvale Quest (`oteryn:quest.grimvale_quest`), mechanizmy: boss_cooldown, condition, door_progress, item_consume, item_hand_out, quest_state, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Hidden Threats Quest (`oteryn:quest.hidden_threats_quest`), mechanizmy: door_progress, npc_dialogue, quest_state, reward_claim, scheduler, summon, teleport, trigger_step, trigger_use, world_object
- Killing in the Name of... Quest (`oteryn:quest.killing_in_the_name_of_quest`), mechanizmy: npc_dialogue, quest_state, scheduler, teleport, trigger_kill, trigger_step
- Kilmaresh Quest (`oteryn:quest.kilmaresh_quest`), mechanizmy: boss_cooldown, door_progress, item_consume, item_hand_out, npc_dialogue, quest_state, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Liquid Black Quest (`oteryn:quest.liquid_black_quest`), mechanizmy: item_consume, item_hand_out, npc_dialogue, quest_state, teleport, trigger_kill, trigger_step, trigger_use
- Newhaven Quest (`oteryn:quest.newhaven_quest`), mechanizmy: npc_dialogue, quest_state, teleport, trigger_kill, trigger_step, trigger_use
- No Rest for the Wicked Quest (`oteryn:quest.no_rest_for_the_wicked_quest`), mechanizmy: creature_removal, item_consume, item_hand_out, npc_dialogue, quest_state, scheduler, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Roshamuul Quest (`oteryn:quest.roshamuul_quest`), mechanizmy: item_consume, item_hand_out, npc_dialogue, quest_state, scheduler, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Rotten Blood Quest (`oteryn:quest.rotten_blood_quest`), mechanizmy: creature_removal, kv_state, quest_state, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Rottin Wood and the Married Men Quest (`oteryn:quest.rottin_wood_and_the_married_men_quest`), mechanizmy: creature_removal, item_consume, kv_state, npc_dialogue, quest_state, scheduler, summon, teleport, trigger_step, trigger_use, world_object
- Secret Service Quest (`oteryn:quest.secret_service_quest`), mechanizmy: condition, door_progress, item_consume, item_hand_out, npc_dialogue, quest_state, reward_claim, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Shadows of Yalahar Quest (`oteryn:quest.shadows_of_yalahar_quest`), mechanizmy: condition, door_progress, item_consume, item_hand_out, kv_state, quest_state, reward_claim, summon, teleport, trigger_step, trigger_use, world_object
- Soul War Quest (`oteryn:quest.soul_war_quest`), mechanizmy: condition, creature_removal, item_consume, item_hand_out, kv_state, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Spike Tasks Quest (`oteryn:quest.spike_task`), mechanizmy: condition, item_consume, npc_dialogue, quest_state, teleport, trigger_kill, trigger_step, trigger_use, world_object
- Targuna Quest (`oteryn:quest.targuna_quest`), mechanizmy: achievement, item_consume, item_hand_out, npc_dialogue, quest_state, reward_claim, teleport, trigger_kill, trigger_step, trigger_use, world_object
- The Annihilator Quest (`oteryn:quest.the_annihilator_quest`), mechanizmy: door_progress, npc_dialogue, quest_state, reward_claim, summon, teleport, trigger_use, world_object
- The Demon Oak Quest (`oteryn:quest.demon_oak`), mechanizmy: item_consume, item_hand_out, npc_dialogue, quest_state, summon, teleport, trigger_step, trigger_use
- The Dream Courts Quest (`oteryn:quest.the_dream_courts_quest`), mechanizmy: achievement, condition, creature_removal, door_progress, item_consume, item_hand_out, npc_dialogue, quest_state, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- The First Dragon Quest (`oteryn:quest.the_first_dragon_quest`), mechanizmy: achievement, boss_cooldown, creature_removal, item_consume, item_hand_out, npc_dialogue, quest_state, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- The Gravedigger of Drefia Quest (`oteryn:quest.the_gravedigger_of_drefia_quest`), mechanizmy: item_consume, item_hand_out, npc_dialogue, quest_state, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- The Great Dragon Hunt Quest (`oteryn:quest.adventurers_guild`), mechanizmy: achievement, item_hand_out, npc_dialogue, quest_state, teleport, trigger_kill, trigger_step, trigger_use
- The Ice Islands Quest (`oteryn:quest.the_ice_islands_quest`), mechanizmy: achievement, condition, door_progress, item_consume, npc_dialogue, quest_state, reward_claim, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- The Inquisition Quest (`oteryn:quest.the_inquisition_quest`), mechanizmy: achievement, door_progress, item_consume, item_hand_out, npc_dialogue, outfit, quest_state, reward_claim, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- The Lost Brother Quest (`oteryn:quest.adventurers_guild`), mechanizmy: achievement, item_hand_out, npc_dialogue, quest_state, teleport, trigger_kill, trigger_step, trigger_use
- The New Frontier Quest (`oteryn:quest.the_new_frontier_quest`), mechanizmy: door_progress, item_consume, item_hand_out, npc_dialogue, outfit, quest_state, reward_claim, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- The Order of the Lion Quest (`oteryn:quest.the_order_of_the_lion_quest`), mechanizmy: boss_cooldown, creature_removal, door_progress, item_hand_out, npc_dialogue, quest_state, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use
- The Queen of the Banshees Quest (`oteryn:quest.the_queen_of_the_banshees_quest`), mechanizmy: door_progress, npc_dialogue, quest_state, reward_claim, scheduler, summon, teleport, trigger_step, trigger_use, world_object
- The Rookie Guard Quest (`oteryn:quest.the_rookie_guard_quest`), mechanizmy: condition, creature_removal, door_progress, experience, item_consume, npc_dialogue, quest_state, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- The Secret Library Quest (`oteryn:quest.the_secret_library_quest`), mechanizmy: achievement, condition, item_consume, item_hand_out, npc_dialogue, quest_state, reward_claim, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object
- The Thieves Guild Quest (`oteryn:quest.the_thieves_guild_quest`), mechanizmy: condition, door_progress, item_consume, npc_dialogue, quest_state, reward_claim, scheduler, teleport, trigger_kill, trigger_use, world_object
- Threatened Dreams Quest (`oteryn:quest.threatened_dreams_quest`), mechanizmy: boss_cooldown, item_consume, item_hand_out, npc_dialogue, quest_state, reward_claim, scheduler, teleport, trigger_kill, trigger_step, trigger_use, world_object
- What a Foolish Quest (`oteryn:quest.what_a_foolish_quest`), mechanizmy: condition, door_progress, item_consume, npc_dialogue, quest_state, reward_claim, scheduler, summon, teleport, trigger_step, trigger_use, world_object
- Wrath of the Emperor Quest (`oteryn:quest.wrath_of_the_emperor_quest`), mechanizmy: condition, creature_removal, door_progress, item_consume, item_hand_out, npc_dialogue, outfit, quest_state, reward_claim, scheduler, summon, teleport, trigger_kill, trigger_step, trigger_use, world_object

#### Lane qcx6 — bramy czasowe, storage, liczniki (18)
- A Piece of Cake (`oteryn:quest.a_piece_of_cake`), mechanizmy: achievement, item_consume, trigger_step, trigger_use
- Adventures of Galthen Quest (`oteryn:quest.adventures_of_galthen_quest`), mechanizmy: door_progress, item_consume, item_hand_out, kv_state, quest_state, teleport, trigger_step, trigger_use
- Barbarian Test Quest (`oteryn:quest.barbarian_test`), mechanizmy: achievement, condition, quest_state, scheduler, trigger_use, world_object
- Children of the Revolution Quest (`oteryn:quest.children_of_the_revolution_quest`), mechanizmy: door_progress, item_consume, item_hand_out, npc_dialogue, quest_state, reward_claim, scheduler, teleport, trigger_step, trigger_use, world_object
- Dreamer's Challenge Quest (`oteryn:quest.dreamer_s_challenge_quest`), mechanizmy: item_consume, quest_state, reward_claim, scheduler, teleport, trigger_step, trigger_use, world_object
- Koshei the Deathless Quest (`oteryn:quest.koshei_the_deathless_quest`), mechanizmy: door_progress, item_consume, item_hand_out, quest_state, reward_claim, scheduler, trigger_use, world_object
- Lion's Rock Quest (`oteryn:quest.lions_rock`), mechanizmy: achievement, item_consume, item_hand_out, quest_state, scheduler, teleport, trigger_step, trigger_use, world_object
- Oramond Quest (`oteryn:quest.oramond_quest`), mechanizmy: door_progress, item_consume, item_hand_out, npc_dialogue, quest_state, scheduler, trigger_use, world_object
- Realm of Dreams Quest (`oteryn:quest.chayenne_realm`), mechanizmy: item_consume, item_hand_out, quest_state, scheduler, teleport, trigger_step, trigger_use, world_object
- The Ancient Tombs Quest (`oteryn:quest.the_ancient_tombs_quest`), mechanizmy: condition, door_progress, item_consume, npc_dialogue, quest_state, scheduler, teleport, trigger_step, trigger_use, world_object
- The Ape City Quest (`oteryn:quest.the_ape_city_quest`), mechanizmy: door_progress, item_consume, npc_dialogue, quest_state, reward_claim, scheduler, teleport, trigger_step, trigger_use, world_object
- The Cursed Crystal Quest (`oteryn:quest.the_cursed_crystal`), mechanizmy: item_consume, item_hand_out, quest_state, scheduler, teleport, trigger_step, trigger_use, world_object
- The Isle of Evil Quest (`oteryn:quest.the_isle_of_evil_quest`), mechanizmy: achievement, door_progress, item_hand_out, kv_state, npc_dialogue, quest_state, teleport, trigger_step, trigger_use, world_object
- The Paradox Tower Quest (`oteryn:quest.the_paradox_tower_quest`), mechanizmy: npc_dialogue, quest_state, reward_claim, scheduler, teleport, trigger_step, trigger_use, world_object
- The Pits of Inferno Quest (`oteryn:quest.the_pits_of_inferno_quest`), mechanizmy: achievement, door_progress, item_consume, quest_state, reward_claim, scheduler, teleport, trigger_step, trigger_use, world_object
- Tinder Box Quest (`oteryn:quest.tinder_box_quest_chyllfroest`), mechanizmy: item_consume, item_hand_out, mount, quest_state, scheduler, teleport, trigger_step, trigger_use, world_object
- To Blind the Enemy Quest (`oteryn:quest.to_blind_the_enemy_quest`), mechanizmy: item_consume, npc_dialogue, quest_state, reward_claim, scheduler, trigger_use, world_object
- Unnatural Selection Quest (`oteryn:quest.unnatural_selection_quest`), mechanizmy: achievement, door_progress, npc_dialogue, quest_state, trigger_step, trigger_use
### Final report (up to 15 lines per batch)
For every batch: branch, SHA, quests with status, tests run with their results, any BLOCKERs with the mechanic and the donor file:line, open facts, and commands that need the PC.

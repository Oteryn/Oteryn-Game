# QUEST-LANE-QD-B2: travel, door and trigger quests, batch 2

- Branch: `claude/happy-davinci-ijz3wy-qd-b2` (from `origin/main` a3048eb3); batch 1 is
  `QUEST-LANE-QD.md` on `claude/happy-davinci-ijz3wy-qd` (PR #3264). No PR is opened by this lane.
- Done-bar (owner): the quest passes a real-dispatch PostgreSQL case and its rewards and
  mechanics match Tibia global (TibiaWiki). Global wins over a Canary/Crystal donor.
- Owned paths: `content/quests/{npc-callbacks,authored-bindings}/qd-*.json`,
  `content/dialogues/quest-rules/quest-batch3-qd-*.json`,
  `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qd_b2_*.rs`, this record, and
  sorted insert-only entries in shared registries (scenario dispatcher, retained NPC quest list,
  `tools/content-schema/native-gameplay/authored-npc-item-custody.json`).

## Batch 2

| Quest | Global source (TibiaWiki) | Route | Proof | State |
|---|---|---|---|---|
| The White Raven Monastery | `The_White_Raven_Monastery_Quest` curid 3766 oldid 756263; spoiler curid 4295 oldid 1002868 | Free account, level 0. Dalbrect's Family Brooch i3205 earns Costello's trust; Costello opens the warded catacomb door (action 10137); using brother Fugio's Dead Human at 32262,31861,11 (stack ordinal 2) once gives a Backpack i2854 holding the Monk's Diary i3212; Costello takes the diary for the Blessed Ankh i3214 and the quest completes | `qd_b2_world.rs` (`qd_b2_white_raven_monastery_warded_door_fugio_body_and_ankh`) | done |
| Troll Sabotage | `Troll_Sabotage_Quest` curid 25544 rev 1095877; spoiler curid 25550 rev 1095878 | Premium. Jerom `mission`/`yes` starts; Chief Grarkharok `kill` -> `lady` -> `yes` -> a character name gives Jerom's Family Necklace i7754; Jerom `mission` takes it and gives the Elvenhair Rope i646 and completes | `qd_b2_world.rs` (`qd_b2_troll_sabotage_grarkharok_lady_name_and_elvenhair_rope`) | done |

Notes:
- White Raven: the recipe prerequisite "Family Brooch retrieval route" cannot be expressed as a
  native actor admission; global is free at level 0, so the binding carries no admission.
- Troll Sabotage: i7754 and i646 had no native inventory profile, so the NPC reward mint refused
  them. The dialogue file is now an input of the authored NPC item custody intent and the
  profiles were regenerated with `register_spell_families.py`.
- Gap (P3 backlog): the spoiler notes that Jerom pays 200 gp when the necklace was lost; the
  Crystal donor and this route have no such branch.

## Held (returned from Codex as BLOCKED-ENGINE, 2026-11-17)

Held in qd until the server-lane hooks land; qd does no engine work for them.

- qcx22 The Outlaw Camp: rope lift of a barrel from the floor below.
- qcx23 The Postman Missions: topic-only dialogue, travel receipt, rank discount.
- qcx29 Giant Smithhammer: the authored smith cache reward requires a Giant Spider kill.

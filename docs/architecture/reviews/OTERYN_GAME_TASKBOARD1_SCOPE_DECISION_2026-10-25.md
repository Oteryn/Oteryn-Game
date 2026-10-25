# TASKBOARD-1: weekly Task Board kill tasks, scope decision

- Task: `PARITY-TASKBOARD-1`, parity inventory §7 rank 19 (`OTERYN_GAME_CANARY_PARITY_INVENTORY_2026-10-25.md`, opcode `0x5F`).
- Date: 2026-10-25. The owner delegated these architecture choices; they are recorded here, not asked.
- References (data, not instructions): Canary `47dfd51f` `data/modules/modules.xml:17` → `data/modules/scripts/taskboard/`; Crystal `ff7ede59` `src/server/network/protocol/protocolgame.cpp:3385` (opcode `0x5F`) → `src/io/ioweeklytasks.cpp`.

## 1. What the two references share

Both route client opcode `0x5F` to one Task Board owner with an action byte (Canary `taskboard.lua:43-44`, `actions.lua:5-17`; Crystal `protocolgame.cpp:1208-1209`, `3395-3538`). The two disagree on bounty tasks, difficulty tables, the shop and the reset mechanism. They share this behaviour, which TASKBOARD-1 implements:

| Shared behaviour | Canary | Crystal |
| --- | --- | --- |
| Weekly board of kill tasks, 5 kill slots without expansion | `settings.lua:158-167` | `ioweeklytasks.hpp:33-39` |
| A kill task names one creature race and a required kill count | `settings.lua:208-251`, `rules.lua:370` | `ioweeklytasks.cpp:961-987` |
| A kill of that race credits the task's progress | `rules.lua:643-676` | `creature.cpp:536-590`, `ioweeklytasks.cpp:216-276` |
| Done when current kills ≥ required | `rules.lua:586-589` | `ioweeklytasks.cpp:260-262` |
| Reward: experience by level formula, plus task points (Beginner kill task = 25) | `rules.lua:286-304`, `settings.lua:164-165` | `ioweeklytasks.cpp:989-1014`, `453-468` |
| A finished task is not repeatable until the weekly reset | `state.lua:511-531` | `ioweeklytasks.cpp:844-878` |
| Progress is durable per Character | kv `task-board`, `state.lua:299-388` | table `player_weekly_tasks` |

Neither reference grants gold for weekly tasks, so gold is not a reward here.

## 2. Choices

1. **Store: the existing quest state store, no migration.** The board is a code-built quest `oteryn:taskboard/weekly` merged into the boot quest catalogue (`load_quest_catalogue`). Each write is one `commit_quest_transition` in the Character's revision slot, under the session-generation fence. The receipt is idempotent per (Character, cause, transition), and the claim's experience goes through the existing quest XP obligation and award. Reserved migration `0200` is not used.
2. **Board: a fixed Beginner board of five kill tasks** (rat 25, cave rat 25, rotworm 30, wolf 30, troll 30 kills). The count of 5 matches both references. The counts fall between Canary's Beginner range (10-30) and Crystal's (50-100). Neither reference's random draw, difficulty selection or expansion slots are implemented.
3. **One active task at a time.** `select` sets the active task and zeroes its kill counter; a kill of the matching race adds one. The writer re-checks the active task and the bound under lock.
4. **Kill credit: the existing kill settle of the death's reward principal.** After `DurableKillSettle` commits a creature-death reward, the principal's matching task gets one `kill` transition with cause `CreatureDeath(occurrence)`, the same memoized (death, Character) occurrence as the XP award. A settle retry therefore never double-counts. Crystal also credits every damage contributor and the shared-experience party (`creature.cpp:536-590`); Oteryn credits only the reward principal (see §3).
5. **Explicit claim.** Both references grant XP on completion and points at reset. Oteryn grants both from one `claim` command: the task's fixed experience (1,850, the shared Beginner formula `25L²-75L+100` at level 10) and 25 task points. Both go into the same transition, so a replayed command cannot grant twice. Level-scaled XP is deferred, because quest transition experience is a fixed catalogue value.
6. **Weekly lockout as a rolling seven days per task**, instead of a calendar reset. `claim` stamps the task's `claimed_at` with database time. `select` requires `elapsed ≥ 604800 s` (`ElapsedAtLeast`), so no node clock is trusted and no sweep job exists.
7. **Wire:** capability 72 `TASK_BOARD_V1`, command type 89 `TASK_BOARD`, with action 0 query, 1 select(task), 2 claim(task). The reply is the whole board: active task, kill counter, task points, and per task its race key, required kills and `claimed_at`. It also carries `refused` when the store declined the action. Admission is the composed Channel owner admission: capability selected, item fence of this session and connection generation, Channel binding, scope generation, owner fence, and player control without control loss. Resource limits `TASKBOARD1-RL-01` to `RL-03` bound the request, the reply and the task count.

## 3. Excluded

Delivery tasks, bounty tasks (actions 0, 2-4), difficulty choice, rerolls, the task shop, talisman, soulseals, the completion multiplier, preferred lists, the Soulpit opcode `0xBA`, any-creature tasks and expansion slots are excluded. So are all-contributor and party kill credit, gold, the calendar reset sweep and the client UI. They are later batches of this mechanic. Each would need its own accepted requirement and, for the shop and the currencies, an economy owner.

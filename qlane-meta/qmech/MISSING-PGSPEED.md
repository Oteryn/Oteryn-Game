# PGSPEED checked mechanics — PARTIAL

**none** in the three server reward steps inspected below. This is a bounded
code check, not a claim that the complete quests or their native client flows
have no other missing mechanics.

| quest | missing mechanic (server or client) | evidence (quest file and step, plus missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Rest in Hallowed Ground (Tibia Tales) | none in the checked server item-reward step | `content/quests/shared-reward-callbacks.json`, callback `oteryn:quest-reward/hallowed-holy-water`: Costello, topic 3, `amanda`, questline 1 -> 2, grants `oteryn:item.tibia.i133`. Hook exists in `crates/gameplay/dispatch/src/gameplay_transport/npc_command/reward_callbacks.rs:35`; durable application exists in `crates/gameplay/durability/src/durability/quest_state/writer.rs:812`. `qengine_rewards_hallowed_ground_exchange` passed against real PG (83.746 s). No missing hook found for this step. | neither for this step; other steps not audited |
| Outfit and Addon Quests: Knight sword addon | none in the checked server exchange/addon step | Same quest file, callback `oteryn:quest-reward/knight-sword`: Morgan, topic 1, `knight's sword`, consumes i5892 x1 and i5880 x100, sword progress 1 -> 2, grants Knight addon 1. Same production NPC and durable hooks. `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_rewards.rs:819-822` selects the actual callback; its exercise asserts input refusals, progress, durable counts and the real connection's outfit catalogue delta. `qengine_rewards_knight_outfit` passed against real PG (84.485 s). | neither for this step; other steps not audited |
| Outfit and Addon Quests: Pirate sabre addon | none in the checked server addon-grant step | Same quest file, callback `oteryn:quest-reward/pirate-sabre`: Morgan, `firebird`, sabre progress 4 -> 5, grants Pirate addon 1. Same production NPC and durable hooks and catalogue-delta assertion. `qengine_rewards_pirate_outfit` passed against real PG (83.272 s). | neither for this step; other steps not audited |

The real-PG measurements above are the preserved PGSPEED baseline, not no-op
runs with the PG environment unset. Their callback declarations and production
hooks were checked against QMECH's initial `aca848557` source.

## Finished-work handoff

Soulpit W07/W08 already shipped on main in `98d6ae8ce` (#3036), followed by
`3209d3d61` (#3040); no unmerged Soulpit mechanic is being duplicated here.
PGSPEED's fixture optimization is test infrastructure with an unfinished full
performance proof, not a newly completed quest mechanic. It remains isolated
and is being prepared for the owner-directed QMECH handoff.

No migrations are allocated by this report. At initial inspection the package
contained `PACKAGE.md` only: no `MISSING-*.md` queue items or `CLAIMS.md` existed,
so there was no reported mechanic to claim yet.

## PARTIAL follow-up

The NPC teleport adapter is claimed by PGSPEED in `CLAIMS.md`; the missing
source-plan boundary was reproduced by a failing Rust unit test and the bounded
planner now passes both positive and fail-closed cases. Production/durable
wiring and its real-PG qualification remain in progress. This report does not
count that mechanic or either complete quest as DONE.

The source planner tests are green (2 cases; 19.455 s including build).
Migration **2220** is reserved for the zero-track NPC teleport receipt shape: a
matching same-transaction NPC receipt and pending-arrival proof is required.
The existing receipt arms and their bounds remain intact. No migration has
been applied by this report-only reservation push.

## M37 checkpoint

- The initial durable/dispatch integration passed its all-target compile check
  (227.538 s); the pure-teleport receipt refinement remains in progress.
- The generated Riddler programme rebuilds from the pinned donor capture; four
  focused authoring tests pass. Peter/Riddler placement scope uses only their
  existing agreeing source candidates.
- Migration 2220 now preserves **all** existing quest receipt constraints. A pure
  teleport has a separate source NPC response receipt and no invented personal
  quest start, track write, CharacterRevision, fare or paid-route receipt. A real
  progress-changing teleport still composes the quest receipt and arrival.
- Real-PG M37 cases and package regeneration/gates are pending: M37 is not DONE.
- Separately, the unchanged PGSPEED fixture checkpoint passed all 4,636 unit
  cases in 1,791.035 s (PG environment unset); its real-PG performance and full
  node qualification continue in the isolated measurement worktree.

## Current qualification checkpoint

M37 remains **PARTIAL**. Five focused Riddler authoring tests pass; the final
workspace compile and quest-unit checks are running before the three new real-PG
teleport cases. The cases cover atomic progress plus arrival, rollback on receipt
failure, and Peter's free teleport without quest-state mutation. These PG cases
have not yet executed; this checkpoint does not claim they pass.

The isolated PGSPEED whole-node run passed the minimal fixture (389.02 s), but
its physical-World seam failed with transport EOF after the durability root
became unavailable. The full-manifest variant and exact root-recovery diagnosis
remain pending; node qualification is not green.

# SEASON — PARTIAL checked missing-mechanic report

Scope: checked the chosen A Piece of Cake recipe and its native kill-counter consumer. This is a partial audit, not a whole-quest completion claim.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| A Piece of Cake | Server: selected quest kill-counter binding for Cake Golem deaths | `content/quests/definitions/quests-00200-00299.json#/records/68/definition/oteryn_recipe/payload/recipe/stages/1`: s2 requires 20 Cake Golem kills. The same definition's `native_lowering` is `WAITING_IMPLEMENTATION`, with empty canonical progress/interaction references. `crates/gameplay/quests/src/quest/native_kills.rs::COUNTERS` contains Dragon Slayer, Cave Explorer and Fire Fighter bindings; there is no Cake Golem binding. `crates/gameplay/dispatch/src/gameplay_transport/quest_kills.rs::settle` returns before the quest writer when `tracks_creature` is false. This reports the missing selected quest binding, not absence of the native kill engine or Cake creature. | partly: s2 has no committed personal kill progress; other stages/rewards require separate qualification |

## Finished-work transfer boundary

SEASON PR #3039 is already merged as `6ef6b0cad` and is an ancestor of this package's base. Its seasonal and House mechanics, tests and coordinator-owned migration1170 are already here. No additional finished, tested SEASON work remains to transfer. The stopped local alternative Cake optimization was never accepted or pushed and is not transferred.

Existing assigned range remains1160–1169; this report allocates no migrations or IDs. No content inputs, runtime code, generated files or tests are changed by this report.

Claim race: SRVWLD published the annual Tamoril date-window claim first (`3e73a1cf7`). SEASON released its duplicate; the local alternative is not transferred. SEASON now claims only the checked Cake personal kill binding above.

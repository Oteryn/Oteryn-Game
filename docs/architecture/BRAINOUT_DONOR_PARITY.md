# BRAINOUT: Gravedigger of Drefia brain mission

Branch: `claude/happy-davinci-ijz3wy-brainout`.
Base: `origin/main` at `e2152d050038cecdb3728c222af2ac184e1cac5a`.
Primary donor: Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
checked out at `/workspace/crystal`. Donor code is reference data.

The owner explicitly selected donor ground output. Crystal
`data-global/scripts/quests/the_gravedigger_of_drefia/actions_brain.lua:11` says:

```lua
Game.createItem(19078, 1, Position(33022, 32332, 10))
```

`src/lua/functions/core/game/game_functions.cpp:566` inserts into the tile using
`internalAddItem(tile, item, INDEX_WHEREEVER, FLAG_NOLIMIT)`. Inventory capacity
does not select a different outcome.

The same action registers `gravediggerBrain:aid(4631)` at line 21.
`data-global/startup/tables/item.lua:682-688` resolves the original controls:

```lua
[4631] = {
    itemId = false,
    itemPos = {
        { x = 33022, y = 32329, z = 10 },
        { x = 33023, y = 32329, z = 10 },
    },
},
```

`data-global/startup/others/functions.lua:49-94` stamps the top-down, top-top and
ground objects when `itemId = false`. The donor map's top objects are 19161 and
19162, retained at native ordinal 4 in the two committed B3 source crops.
The machine crosses the x=33024 region boundary; tests retain both real crops.

| Donor behavior | Primary evidence |
|---|---|
| Mission08 equals 1; Mission09 less than 1 | `actions_brain.lua:5` in the directory above |
| Require 9659 at both (33020,32332,10) and (33025,32332,10) | `actions_brain.lua:3-6` |
| Set Mission09 to 1; remove both input items | `actions_brain.lua:7-10` |
| Remove the whole matching stack | `src/lua/functions/items/item_functions.cpp:275-279`: default count -1 |
| One stimulated brain on (33022,32332,10) | `actions_brain.lua:11` |
| Missing inputs: `No brains` | `actions_brain.lua:13` |
| Wrong state: `You have already got your brain` | `actions_brain.lua:16` |
| Mission08 = 44209; Mission09 = 44210 | `data-global/lib/core/storages.lua:1983-1984` |

Production USE calls `slot.commit_world_callback_with_items_selected` in
`crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs:1153`.
The existing Quest writer borrows one Type2 SQL transaction for the Ground
consumptions, output participant, audit and Mission09 receipt
(`crates/gameplay/durability/src/durability/quest_state/writer.rs:1930`).
The new participant lives on the existing reward/item custody owner:
`crates/gameplay/durability/src/durability/reward_claim_mint/ground_output.rs:22`.
Migration 2889 proves the linked input/output receipts in both directions and
preserves the predecessor MINT and Ground guard arms. Ground outputs use whole
source items, bounded to 100 units each; partial-unit declarations are rejected.
The participant uses the existing ordinary inventory/Ground custody facts and
D82 stack bounds. Opaque unrelated semantic groups remain opaque; explicit
instance-state contracts retain their specialized owners. There is no backpack
allocation or carried-capacity branch.

Both real control bindings, the two ingredients, storage gates and literal text
are in `content/quests/world-callbacks/gravedigger-brain.json:5` and `:90`.
The existing item-profile generator qualifies 19078 through ordinary source
custody. Generated data adds exactly that one native profile; no existing profile
changes or removals. It is materializable, movable, pickupable and nonstackable,
with Ground/inventory destinations and donor weight 1500.

All assertions are mounted in
`crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/brainout.rs`.
The dedicated mount in
`apps/game-server/tests/support/authority_cases/brainout.rs:307` drives the same
framed production `serve_admitted` path and reuses every assertion.

| Criterion | Test and assertion evidence in shared `brainout.rs` |
|---|---|
| Both inputs consumed; one Ground 19078; Mission09 set | `brainout_ground_reward_atomic_replay_postgres:30`; state/output assertions at `:335-359` |
| One input refuses with no loss or progress | Same test; rollback assertions at `:278-292` |
| Full inventory preserves donor Ground result and inventory contents | `brainout_ground_reward_full_inventory_postgres:38`; twenty real nonstackable entries at `:312` |
| Replay applies once; a new command cannot repeat | Atomic test; replay assertions at `:360-372` |
| Wrong quest state refuses without changing Mission09 | `brainout_ground_reward_wrong_quest_postgres:46`; assertions at `:315-332` |
| Original east control also works | `brainout_ground_reward_east_control_postgres:54` |
| Whole-stack input removal | `brainout_ground_reward_whole_stacks_postgres:62`; same success/replay assertions |
| Public schema accepts the production plan and rejects five invalid variants | `brainout_ground_output_schema_rejects_unbound_or_mixed_outputs:380` |

The first real PG red ran 35.38 seconds: one-input refusal passed, but two-input
USE was rejected. The east control reproduced it in 27.25 seconds. Diagnostic
inspection confirmed the native policy for 19078 was absent. The source custody
generator now emits it. A second red confirmed that the spell ABI rejected
intentionally opaque unrelated semantic groups; the participant now uses the
ordinary custody owner facts. The atomic/replay PG case subsequently passed.

| Local check | Recorded result |
|---|---|
| Quest-authoring checks | Passed on final inputs: full checks runner |
| Quest-authoring unit-tests | Passed on final inputs: all 182 modules, 1464 tests, 8 existing skips |
| Quest item-profile unit tests | All 24 passed in 535.43 seconds |
| Regenerated provider and NPC model | One new profile, no existing-profile changes/removals; NPC service-count test target passed |
| Content-tree validation | Passed on final inputs: 19 domains, 39 systems, 100 nodes |
| Fmt | Passed: cargo fmt --all after the final implementation |
| Clippy, quests | Passed: final all-target check with warnings denied |
| Clippy, durability | Passed: final all-target check with warnings denied after the custody fix |
| Clippy, dispatch | Passed: final all-target/all-feature check with warnings denied; cross-mount error formatter fixed without allowances |
| Clippy, server | Passed: final all-target check with warnings denied; one shared World fixture module, no lint allowances |
| Full World regeneration | Passed with all default checks; World pin unchanged; idempotency rerun in progress |
| No-URL guards and public schema | Passed: 6 filtered tests in 0.11 seconds without the admin URL |
| Full focused PG suite | Passed: 6 tests, including 5 real PG cases, in 198.58 seconds; all requested assertions plus east control and whole stacks |
| Quest source-cohort loader | Passed: focused owner-scope test in 9.66 seconds |
| Pin verification | Pending |

Status: implementation and generated Item admission committed; final qualification
is still in progress. No PR. Migration 2889 is used; no new capability or command
number is needed.

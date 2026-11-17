# BRAINOUT: Gravedigger of Drefia brain mission

Base: `origin/main` at `e2152d050038cecdb3728c222af2ac184e1cac5a`.
Branch: `claude/happy-davinci-ijz3wy-brainout`.

Primary donor: Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
checked out at `/workspace/crystal`. Donor evidence is reference data.

| Behavior | Crystal evidence |
|---|---|
| Mission08 equals 1, Mission09 less than 1 | `data-global/scripts/quests/the_gravedigger_of_drefia/actions_brain.lua:5` |
| Require 9659 at both (33020,32332,10) and (33025,32332,10) | `actions_brain.lua:3-6` in that directory |
| Set Mission09 to 1 and remove both inputs | `actions_brain.lua:7-10` |
| Create one 19078 on ground at (33022,32332,10) | `actions_brain.lua:11`: `Game.createItem(19078, 1, Position(33022, 32332, 10))` |
| Missing inputs: "No brains" | `actions_brain.lua:13` |
| Wrong state: "You have already got your brain" | `actions_brain.lua:16` |
| Trigger AID 4631 | `actions_brain.lua:21` |
| AID controls at (33022,32329,10) and (33023,32329,10) | `data-global/startup/tables/item.lua:682-688` |
| Mission08 = 44209, Mission09 = 44210 | `data-global/lib/core/storages.lua:1983-1984` |
| Output goes to tile, without inventory capacity checks | `src/lua/functions/core/game/game_functions.cpp:557-566` |

The owner's clarified rule preserves donor ground output. Inventory capacity
must not change the result. The original inventory-delivery requirement is superseded.

Implementation plan: reuse the existing item exchange/custody owner and quest
callback transaction; consume both ground inputs and create the portable ground
output with Mission09 and the receipt atomically. Wire both original controls
through production USE. Test success, missing input rollback, full inventory,
replay and wrong quest state through that path, first red and then green.

The donor's `data-global/startup/others/functions.lua:49-94` resolves
`itemId = false` by stamping the top-down, top-top and ground objects with the
AID. The donor map's final objects at the two controls are 19161 and 19162.
The committed B3 region retains those as top-level ordinal 4 at the same positions;
production bindings can therefore use exact native counterparts without changing
World placement data or manufacturing an AID.

The mounted `brainout.rs` cases drive framed USE and cover one-input rollback,
two-input success, ground position/quantity, Mission09, replay, full inventory
and wrong quest state. Their initial focused run is compiling dependencies.

Implementation now includes both native control bindings, the `ground_output`
effect, an exchange-owner participant, and migration 2889. Input BURNs, the
output, audit event and Mission09 receipt share one physical transaction. The
new deferred guard arms require both directions of that composition.

Validation recorded so far:

| Check | Result |
|---|---|
| `cargo fmt --all` | Passed; include-mounted leaf files also formatted with Rust 2024 rustfmt |
| `python3 tools/content-schema/validate_full_game_content_tree.py` | Passed: 19 domains, 39 systems, 100 nodes; no unassigned entries |
| Initial focused dispatch regression | Build still running; no red/green result claimed |
| Quest-authoring checks and unit-tests runner | Running |
| Clippy, full regeneration and pin verification | Pending |

Status: implementation authored; PostgreSQL red/green qualification and remaining
validation are pending. This branch is WIP and has no PR.

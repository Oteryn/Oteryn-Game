# QWORLD M32 — conditional native deaths

The selected Oteryn recipes, rather than donor kill counts, require ten Fury deaths (Fury Gates s3) and ten Werewolf deaths (Grimvale s2). Their canonical records are48 and54 of `content/quests/definitions/quests-00100-00199.json`. The production binding input is `content/quests/native-death-objectives.json`; the existing native-death owner consumes it and carries the existing world-change predicate into the fenced quest writer.

Identity/source references retained by those selected recipes:
- Fury Gates: https://tibia.fandom.com/index.php?curid=76237&oldid=1099676 and https://tibia.fandom.com/index.php?curid=76236&oldid=887144
- Grimvale: https://tibia.fandom.com/index.php?curid=76238&oldid=847053 and https://tibia.fandom.com/index.php?curid=76239&oldid=1102848
- Native schedule implementation already sourced from Crystal: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/world_changes/fury_gates.lua and https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/world_changes/grimvale_respawn_event.lua

M31's personal journal lowering may open the `qworld/fury/hunt` and `qworld/grimvale/hunt` tracks only after the preceding selected stages; both default to0. Native death increments the corresponding `/kills` track, capped at10. This bounded adapter does not declare the exploration, seal/survey or final completion stages qualified.

M32 engine proof: `qworld_native_fury_death_requires_active_world_and_credits_once` executed in150.93681868s and `qworld_native_werewolf_death_requires_full_moon_and_credits_once` in150.9516698s. Both use real reset, native death capture/settlement and the fenced quest writer. Quest unit tests314/314 passed; quest-authoring checks, its105-module unit runner, content-tree validation, fmt and changed-crate clippy passed. This is stage/engine evidence, not whole-journey qualification.

## Full counter threshold regression

The existing real-dispatch Fury and Werewolf PG scenarios now run twelve
unique creature deaths per world occurrence: one before accepting the hunt,
then eleven after it. Each death is captured and drained twice by the shared
helper. The active-world personal count advances to the authored ten-kill
threshold and stays there after a fresh eleventh hunt death; inactive-world
counts stay zero throughout. The longer scenario retains its dead actors, so
only its fixture actor budget grows from three to fourteen slots; production
limits and the smaller consumer scenario remain unchanged.

The first run exposed the fixture's `CapacityExceeded` error in both cases.
After correcting that setup, both cases passed with PostgreSQL execution proof:
Fury **153.893309256s**, Werewolf **153.926332507s** (two passed, zero failed).

# QWORLD M32 — conditional native deaths

The selected Oteryn recipes, rather than donor kill counts, require ten Fury deaths (Fury Gates s3) and ten Werewolf deaths (Grimvale s2). Their canonical records are48 and54 of `content/quests/definitions/quests-00100-00199.json`. The production binding input is `content/quests/native-death-objectives.json`; the existing native-death owner consumes it and carries the existing world-change predicate into the fenced quest writer.

Identity/source references retained by those selected recipes:
- Fury Gates: https://tibia.fandom.com/index.php?curid=76237&oldid=1099676 and https://tibia.fandom.com/index.php?curid=76236&oldid=887144
- Grimvale: https://tibia.fandom.com/index.php?curid=76238&oldid=847053 and https://tibia.fandom.com/index.php?curid=76239&oldid=1102848
- Native schedule implementation already sourced from Crystal: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/world_changes/fury_gates.lua and https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/world_changes/grimvale_respawn_event.lua

M31's personal journal lowering may open the `qworld/fury/hunt` and `qworld/grimvale/hunt` tracks only after the preceding selected stages; both default to0. Native death increments the corresponding `/kills` track, capped at10. This bounded adapter does not declare the exploration, seal/survey or final completion stages qualified.

Status: M32 remains CLAIMED; the complete content/PG batch is still undergoing validation.

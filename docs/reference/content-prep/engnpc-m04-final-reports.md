# M04 final NPC reports

Source classification: OTS_HYPOTHESIS_ONLY. These pinned Crystal scripts establish the existing imported final-stage effects; they are not independent retail proof. Earlier missions remain with their existing owners.

| Quest | Source URL | Final branch |
| --- | --- | --- |
| A Father's Burden | https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/tereban_functions.lua#L139 | status=1, progress=8; final cape11701 and 8,000 XP |
| Djinn War Efreet | https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/malor.lua#L97-L154 | mission03=2, report prompt topic 2; mission03=3, door=1, Efreet Ally, delayed closure |
| Djinn War Marid | https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/gabel.lua#L87-L140 | mission03=2, report prompt topic 2; mission03=3, door=1, Marid Ally |
| Marlin Trophy | https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/pemaret.lua#L61-L72 | marlin901 carried, topic 1; consume one marlin, give trophy902, progress=1 |
| White Raven Monastery | https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/costello.lua#L63-L93 | diary=1, topic 2; consume diary 3212, give 3214, diary=2 |

The final affirmative branches recheck the prior stage and retain keyword precedence. Only those branches set the engine completion flag; prompts do not complete. The native programme digest binds the completion flag and replies.

## Terminal receipt qualification

`engnpc_m04_completion::m04_npc_completion_is_canonical_atomic_fenced_and_replay_bound`
(`character_authority_postgres`) drives the shipped Tereban, Malor, Gabel,
Costello and Pemaret programmes through the durable NPC owner. Prior mission
progress and carried items are fixture setup; this is final-boundary coverage,
not whole-quest or World-boot qualification. Pemaret starts with no quest header.

The checks retain exact cape/8,000-XP, faction achievement and diary/fish exchange
effects. They cover wrong topics, replay, a fresh command after completion,
missing backpack, and rollback of items, receipts, completion and revision after
late XP or response failures. Historical item setup alone uses replica mode;
all guards run during the commands under test.

The exact activation roster includes the two Djinn factions and White Raven:
20 runtime programmes; the embedded source catalogue has 211 absent-track
defaults of -1 and 52 of 0. The added
seven -1 defaults are three tracks per faction and the monastery diary. The
roster, default counts and legacy hash assertions remain exact.

## Native admission and real-caller qualification

The initial real-command extension reached Tereban, Malor and Gabel, then
failed with `DefinitionNotFound` on marlin901. The missing native profiles are
now produced by `tools/content-migration/engnpc_m04_item_profiles.py`: marlin901, trophy902, diary3212 and blessed Ankh3214. Crystal201189f items.xml witnesses
and the retained client bank qualify their physical custody; replay checks every
witness, binding and canonical definition before native registration.

The i901 canonical definition and D289 hold remain unchanged. The native profile
retains the sealed accepted `NonStackable` class and UNKNOWN stack semantics;
it does not promote the conflicting donor stack flag. Diary3212 retains its
fixed-text semantics, including UNKNOWN player writeability; only its physical
custody admission is promoted. The existing physical,
description and whole-definition hold tests remain strict. No use effect or
canonical Item enrichment is introduced by this profile cohort.

`engnpc_m04_completion_is_atomic_canonical_and_replay_safe` requires production
`NpcTalkIntent` commands for all five finales, using the active native quest
catalogue and Item policies. It checks that each command joins its completion
header, canonical receipt and NPC response in one transaction, retains the cape,
trophy and Ankh, and consumes the marlin and diary. Prior mission progress is
historical fixture setup; the terminal commands use the real dispatcher.

The talk book is installed once with all five native NPCs, just as production
boot registers `NpcRuleCatalog::runtime()` in
`apps/game-server/src/node/serve/bundle_npcs.rs`. The bounded fixture carrier
hosts one NPC at a time and removes each through the runtime owner. This does
not enlarge its capacity or replace a production admission guard.

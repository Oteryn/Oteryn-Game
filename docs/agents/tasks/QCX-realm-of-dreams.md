# qcx84 Realm of Dreams Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Wiki facts come from [the quest page, curid 61230 oldid 683951](https://tibia.fandom.com/?curid=61230&oldid=683951) and [the spoiler, curid 61232 oldid 999253](https://tibia.fandom.com/?curid=61232&oldid=999253). The repository's M07 reference note links to Crystal `201189f8`. The reward, movement and lever scripts were checked at that revision.

| Part | Difference from the wiki |
| --- | --- |
| Requirements | Premium and level 40 match the definition. M07 applies level 40 at trial start, Crystal applies it at entry, and the wiki places the level gate inside the realm. M07 uses a retained key directly on the lever and fire tile. The wiki checks possession of the retained key at the forcefield. Neither the active M07 callbacks nor Crystal's entry script checks the Draconia chest flag. The book need not remain in inventory. The selected recipe says to read the book. Rope or equivalent and at least two Destroy Field runes are listed by the wiki. The M07 atomic fire interaction bypasses the rune and switch steps. |
| Lair clearance | The selected recipe says 12 kills. The spoiler lists 14 Dragons, 8 Dragon Lords and 7 Fire Elementals across five relative floors. M07 tests live occupancy in a four-floor footprint and has no full-respawn prerequisite. Crystal's lever only tests its cooldown and key. |
| Switch and lever | The wiki sequences fire removal, stepping on the switch to remove the stone pile, and use of the exposed lever. M07 merges these into an atomic key use on the fire after a separate personal-start interaction. The checked-in lever placement is untagged in the M07 crop. Crystal gates action 55021 by key and a five-minute shared cooldown. M07 ties progress to a personal one-time phase, including after wall restoration. The wiki allows the lever to reopen the wall and describes the stone pile persisting until floor reset. |
| Wall | M07 inherited Crystal's one-minute restore. The active callback now restores the wall after the wiki's five minutes. The existing control timer is also five minutes. |
| Access and exit | Crystal binds forcefield 55022 to 32829,31451,z8 and exit 55024 to 33117,32604,z6. M07 has no entry or exit callback, and the definition has no native gate or progress refs for this route. The spoiler places the exit northwest of the island and returns the player to the lair entrance. |
| Reward | The definition has no native reward claim. The recipe lists only Beach Backpack and Music Box. The wiki describes a dead-dragon claim with a Beach Backpack containing eight items. Crystal grants one backpack 5949, containing one each of Music Box 16244, Blue Rose 3659, Dracoyle Statue 9034, Black Pearl 3027, Red Dragon Scale 5882, Stuffed Dragon 5791, Piggy Bank 2995 and Surprise Bag (Blue) 6570, once through action 55023. No quest reward claim or quest NPC callback/offer was located. The quoted wiki revisions list no XP, achievement or outfit reward. |

BLOCKER five-floor guardian clearance shared ClearArea supports only four floors `data-global/scripts/quests/chayenne_realm/actions_lever.lua:3`. Crystal does not implement the wiki census here; the five-floor requirement comes from the spoiler.

OPEN-FACTS: exact Draconia chest/progress and corpse bindings, exact forcefield and switch placement identities, fifth-floor native footprint, whether adjacent fire must also be cleared, and the full-respawn/floor-reset edge cases. The wiki itself flags inconsistent reproduction.

PARTIAL: native book prerequisite, entry and exit, internal level gate, five-floor clearance and rune/switch/lever sequence, repeat opening, and nested corpse reward claim. The current M07 binding is an explicit approximation. No callback for this quest was found in the held directory. Complete scans of the checked-in B3 placements and Crystal’s pinned `data-global/world/world.otbm` found no action or unique IDs 55021–55024. The existing Draconia1/2 reward records grant equipment and do not identify the book chest.

## Verification

- Quest authoring passed all 44 check commands, including 27 tests. The focused Realm data test failed once with the base callback's 60,000 ms and passed with 300,000 ms.
- Pin regeneration reported the prior digest and produced no tracked changes. Workspace formatting passed.
- Pin check passed for one world using the compiler's required temporary output-directory argument.
- PG via shard (pgsplit build files applied on `32b6fbb3f5667c2fe9b292690cc20ea072a3ae64`), with the base callback restored locally for the red run. The real dispatch case failed on `Some(60000)` versus `Some(300000)` in 45.25 s.
- The corrected PostgreSQL case passed in 78.51 s (1 passed). The same filter without the PostgreSQL URL returned `Ok(())` in 0.00 s (1 passed, database path skipped).
- Dispatch library Clippy passed with all features and `-D warnings`. The two local shard files were removed before the documentation commit.

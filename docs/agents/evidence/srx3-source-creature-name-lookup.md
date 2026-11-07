# SRX-3 source creature name lookup

This records source semantics, not a completed runtime consumer.

Pinned Crystal revision `201189f871d23597b82caf88fd611454a2ddefb8`:

| Source | Git blob | Relevant lines |
| --- | --- | --- |
| `src/game/game.cpp` | `5f421b1b2361280abeb8b392bca0e0344fc04648` | 1150–1178 |
| `src/lua/functions/creatures/creature_functions.cpp` | `03242ac84346da6525c6c7ad79fb98726478e804` | 117 |

`Creature(string)` calls `Game::getCreatureByName`. The implementation rejects an empty string, lowercases the requested name, checks the mapped Player names first, then iterates NPCs, then monsters. Each match returns immediately. It does not inspect current HP or require a monster role. The lookup is global in the source process; private Instance isolation in Oteryn must use the already accepted scope contract rather than borrowing actors from another Instance.

Consequently, a guard such as `Creature("Soul Cage")` can be satisfied by a currently registered Player or NPC with that name. A HealthForgotten spectator-name comparison likewise cannot substitute an `isMonster` predicate. A zero-HP monster remains relevant until the deferred source death task completes its registration removal. Actor-name existence checks need current canonical all-role names; they cannot infer names from reward attribution or role aliases.

If a callback uses the returned actor, Player/NPC/monster precedence matters. No arbitrary ActorId sorting is claimed to reproduce the source NPC/monster container iteration order. A boolean existence guard can stop once any matching current actor is proved.

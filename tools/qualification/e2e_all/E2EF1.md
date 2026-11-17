# E2EF1 movement journeys

Journey: `apps/game-server/src/qualification/client_phases/full_e2ef1.rs`.
The single `world.rs` registration mounts a wrapper around `provision.rs`.
The wrapper releases the initial ordinary character, runs these journeys with
the two Platform-created characters, logs out, then invokes the existing
profile provisioning. Thus none of these full-row checks can use its control
socket build, rope, shovel or gold grants. The existing fixture, map, combat,
spell and bounded floor journeys remain registered.

The oracle loads the same pinned bundle through the production world loader.
It reads placed objects through the bundle visitor, uses production floor and
collision facts to plan bounded town routes, and accepts movement only from
native session observations. Objects are ordered by position/ordinal. Missing
routes, tools, content, capability selection, exact results or observations
fail; they are not promoted from bounded map checks or text matches.

| Row | Audit and server entry points read | Native sub-actions and required result |
| --- | --- | --- |
| M01 | audit line 72; `connection/serve/target_move.rs`, `paced_movement/autowalk.rs` | Two immediate paced STEP commands publish movement; server target walk reaches its goal; STOP interrupts another route and holds position; all four TURN directions update the own actor without translation; FOLLOW reaches a peer, follows its subsequent movement, then stops. Fresh admission preserves the final position and inventory. |
| M02 | audit line 73; `connection/serve/autowalk.rs` | USE a plain door from three tiles away: actor walks into range and the original action commits, then close it. Independently drop an ordinarily acquired item, retreat three tiles, MOVE its live ground handle into the backpack: actor walks, original move commits, inventory returns. Fresh admission preserves position and inventory, ignoring session-scoped handles. |
| M03 | audit line 74; `paced_movement.rs`, `actor_spell.rs`, `postgres_tests/floor_exhaust.rs` | Real stair floor change and authored cross-floor teleport outside PZ; each must immediately refuse an aggressive flame-wave cast with CoolingDown and no resource delta, then execute that same cast after the two-second window with mana payment. Spell/group cooldowns settle between experiments. Fresh admission preserves landing and inventory. |
| M04 | audit line 75; `creature_push.rs` | Push a player into an occupied cell: NotEnoughRoom and unchanged target; push into a free neighbor: Moved observed by both clients. Logout the pushed peer and verify its saved position on fresh admission. Approach an actual rat spawn, push an adjacent live creature, observe that exact entity at the destination. Relog the pusher too. |
| M05 | audit line 76; `map_door.rs`, `map/door_registry.rs` | Exact plain-door family 1638/1639 and unlocked key-door family 1629/1630: closed collision, USE open, cross, occupied close refusal, leave, USE close, closed collision. Locked 1628 refuses Locked. Track transition revisions between rows. Door state is Channel-ephemeral, not character-durable (`map/door.rs`). |
| M07 | audit line 78; `item_craft/rope.rs`, `map/floor/rope.rs` | Resolve a genuinely carried rope 3003 from inventory, USE-WITH an authored rope spot, require Committed and an adjacent landing one floor upstairs, retain rope quantity, then fresh-admit and compare position and inventory. |

Audit: `docs/architecture/reviews/OTERYN_GAME_CRYSTAL_REAUDIT_2026-10-28.md`.
Dispatch paths above are under
`crates/gameplay/dispatch/src/gameplay_transport/`; map paths are under
`crates/gameplay/world/src/`.

Each row emits exactly one README-format `<row>.full` result. PASS requires
all named checks plus applicable fresh-admission comparisons. `E2EF1_CHECK`,
`E2EF1_GAP`, and `E2EF1_ROW` retain observed results and missing sub-actions;
they cannot independently qualify a row. Fatal transport/cleanup errors emit
explicit FAIL results for the remaining rows. Nonfatal row failures continue
and are consumed by the existing fail-closed Python report.

Expected outcome is conditional on real gameplay prerequisites. The ordinary
starter currently has only a backpack: absent an ordinarily acquired item or
rope, M02's far-MOVE and M07 must FAIL. An unclassed/underlevel caster, PZ
refusal, missing spell, or unavailable cross-floor route makes M03 FAIL, not
MISSING-SERVER. Missing exact door families or live push targets also FAIL.
All six production command paths exist; no missing-server claim is inferred
from absent starter equipment or a bounded route search. Adding equipment via
fixture provisioning is deliberately insufficient for these full rows.

Local unit tests check that omitting any required action prevents completeness;
they do not claim live-node qualification. Execute with the README's existing
`bash tools/qualification/e2e_all/run.sh`. Real execution needs node_boot's exact
Platform checkout and privileged disposable topology. CI must execute that
journey when those prerequisites are unavailable locally.

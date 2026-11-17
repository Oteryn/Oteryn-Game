# ENGNPC2 engine queue

Base: `e22dff5512e09cab5b9e09756b96a53731ffb6cf`.
No main merge is authorized for this lane. Migration allocation: 1650–1659.
Quest implementation and qualification remain with their quest lanes.

Rows below are in execution order, bottom to top. Source requests are the
questpc7-n3 JSON followed by the ENGNPC rows in questpc2-n2-n3 Markdown.
questpc5-n2-n3 has no ENGNPC-tagged row.

| Order | Capability | Needed by | State |
| --- | --- | --- | --- |
| 1 | Live-creature capture/release and distinct painting | Formorgar Mines | WAITING-ENGINE: W1 |
| 2 | Exact fresh Ground corpse USE | Goblin Merchant | WAITING-ENGINE: W2 |
| 3 | Exact live-creature odour sampling | Goblin Merchant | WAITING-ENGINE: W1 |
| 4 | Exact live-creature capture | Goblin Merchant | WAITING-ENGINE: W1 |
| 5 | Live equipped head predicate | Twenty Miles Beneath the Sea | DONE: engine capability; qualification below |
| 6 | Source NPC multiple grants MINT bridge | Tower Defence | WAITING-ENGINE: W3 |
| 7 | Committed source NPC magic effect | The Inquisition | OPEN |
| 8 | Live character-name dialogue match | The Dream Courts | OPEN |
| 9 | Atomic bank/item/magic receipt | The Demon Oak | OPEN |
| 10 | Live level/vocation NPC item plan | Elemental Spheres | OPEN |
| 11 | Item/points/cooldown source receipt | Dangerous Depths | OWNED ELSEWHERE: ENGNPC current row, owner instruction |

## Dependencies checked on the allocated base

W1: QENGINE2 `925d8480ef1cb88b16154a1dbf018a57ac22152a`,
`qlane-meta/qengine2/ENGINE-QUEUE.json`, explicitly queues
`live_creature_item_use_transform_and_credit` for Tainted Souls. The questpc2
request explicitly requires reusing that queued owner. Its catalogue-scoped
`authored_bindings::Bindings::attach` and `reward_callbacks::load_for` are also
absent from the allocated base. Do not duplicate these APIs or replace actor
custody with a quest counter. Release: consume the qualified live-target owner
and its exact creature-occurrence/custody participant, then compose release and
distinct painting under the selected phase. These rows are not implemented here.

W2: the same missing scoped admission plus the World corpse-handle participant.
`connection/routing/use_item.rs` resolves an Instance target, but
`cosmetic_item_use.rs::use_carried_quest_item` refuses every `use_with`,
`use_with_map_item` and `use_with_position`. `quest_callbacks.rs` routes map
objects, not the native fresh Ground corpse identity. Questpc7's native Slug
death proof is a producer proof, not a tool/quest receipt participant. Release:
World/Items exposes a custody/revision-bound corpse-use participant consumable
by the scoped quest owner. No static corpse or inferred successful use is added.

W3: L2 `be6626b49508c33b062c097480ed173472a8bdba` contains
`reward_claim_mint/backpack_outputs.rs`; the allocated base does not contain
that module or the request's `backpack_outputs` mode. Release: integrate L2's
qualified direct-backpack output primitive and migration 1213 before extending
the source NPC bridge. Do not reimplement L2's output primitive or fabricate a bag.

ENGITEM `183f49bc11a1e84ce1633b40e19b54a1b952da51` active `.lane/rows.md`
owns five different item rows; its historical ledger confirms the missing World
action-proof boundary but is not used to claim these ENGNPC2 rows done elsewhere.

## Equipment implementation boundary

The new planner admits head equipment as a read-only guard alongside storage
operations. The durable writer obtains the actual snapshot through the existing
equipment owner under the same Character root, content activation, session and
physical-lane fences as the progress receipt. Plain-item grant allowlists and
ENGNPC's Dangerous Depths planner remain unchanged. No quest content is authored.

Qualification on this tree:

- RED: `equipment_admission_requires_the_live_head_slot` returned
  `MissingNativeResolution` before the planner implementation.
- GREEN: all 328 quest-package tests and all 740 durability library tests pass.
- Real PostgreSQL 18.6:
  `engnpc2_equipped_head_admission_is_fenced_atomic_and_replay_bound`,
  marker `ENGNPC2_PG_EXECUTED equipped_head duration=4.449471887s`, one pass.
  Historical equipment is fixture input; all NPC calls use production triggers,
  digest admission, the locked equipment owner and the normal receipt transaction.
- All-target/all-feature clippy passes for quests, durability and game-server.
- Architecture check passes. Explicit formatting checks cover the lifted modules
  that `cargo fmt --all` cannot discover through generated `include!` roots.
- Binding packets, NPC dialogue links and completion matrix were regenerated and
  checked without tracked drift. Cargo.lock is unchanged under locked commands.
- World pin/lock verification passed with input digest
  `f828a0762bd45fd64c3187049a7f67ac7b15e341fd75fc22dce6aa56833f31d0`.

ROWS: done 1 / open 4 / blocked 5; OWNED ELSEWHERE 1. The five blocked rows
are the WAITING-ENGINE dependencies W1–W3; none is claimed implemented.

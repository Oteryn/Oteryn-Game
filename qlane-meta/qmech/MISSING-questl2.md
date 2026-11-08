# L2 missing mechanics — PARTIAL

Checked against qmech aca848557. Source candidates below are preserved on
`claude/happy-davinci-ijz3wy-questl2b` at be6626b49508c33b062c097480ed173472a8bdba.
This report covers three checked gaps; it is not a claim that the rest of the
51-entry packet has been audited. Existing finished work is being integrated
separately. Main has not been merged into this package by this lane.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Tomes of Knowledge | Server: NPC offers accepted independently of the current topic (`any_topic`). | Published `tools/content-schema/quest-authoring/questl2_knowledge.py:61-64,71-73`: Cael's incremental Tome offers and Zalamon's mission offer set `any_topic: true`. Current `crates/gameplay/dispatch/src/gameplay_transport/npc_command/reward_callbacks.rs:17-28` has `#[serde(deny_unknown_fields)]` on `Offer`, which has only an exact `topic` and no `any_topic` field. Thus these authored offers cannot deserialize as-is. | Partly: blocks the authored Tome NPC sequence. |
| Into The Bone Pit | Server: inventory quantity-range/absence eligibility and explicit refusal dialogue for NPC reward branches. | Published `tools/content-schema/quest-authoring/questl2_journeys.py:26-37`: Muriel's return/lost-sample branches distinguish sample 131 present versus quantity exactly zero and provide `failure_reply`. Current `crates/gameplay/quests/src/quest/reward_callbacks.rs:144-162` supports `minimum_level` and positive `present` amounts, but its deny-unknown-fields reward DTO has neither `inventory` range predicates nor `failure_reply`; the dispatcher `Offer` also has no refusal-dialogue field. The missing-sample branch needs an absence predicate, not another positive item requirement. | Partly: blocks the distinct lost-sample/refusal path. |
| Unnatural Selection | Server and client: native diagonal movement commands for the full dance route. | `content/quests/world-callbacks.json:8719` onward contains the dance callbacks and cites `data-global/scripts/quests/unnatural_selection/movements_mission3_dance_dance_evolution.lua:1-67`. `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/unnatural_selection.rs:1-6` explicitly states that its seam proof does not qualify diagonal movement and arranges fifteen setup steps through the fenced writer. `crates/protocol-oteryn/src/world_spatial.rs:41-46` defines only North/East/South/West in `StepDirection`; there is no diagonal command variant. | Partly: blocks qualification of the complete native dance journey. |

No migration is claimed by this report. L2 retains its existing 1210-1219 range.

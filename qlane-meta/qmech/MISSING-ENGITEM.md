# ENGITEM missing mechanics — PARTIAL

Checked against QMECH import3037a627c. This is a partial report; only the
source/code boundaries verified below are included. No whole-quest qualification
is asserted. Lane migration reservation remains1640–1659.

| quest | missing mechanic (server or client) | evidence (quest file and step, missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Targuna Quest | Server: carried-tool use on an exact map object, composed with quest-guarded taming/progress | Pinned Crystal9f5a72c corpus: `data-global/scripts/quests/targuna/actions_reliable_ram.lua:20–50`, blob `cacc50a95454c9ae4abbccb6dbbda1ce179951a8`: final Burning Heart turnip53073 on map Ram53439 at31926/31896/z7; earned-turnip guard, consume, mount, progress. `crates/gameplay/dispatch/src/gameplay_transport/mount_taming.rs:180–189` resolves object targets only from carried `ItemKey::Instance` backpack/nested entries. `MountTamingRequest` has no quest participant. A MapBase object is not a carried Item instance. | Partly: the Burning Heart finale; earlier contacts/rewards are separate. |
| Oramond Quest | Server: repeatable World item MINT scoped to an admitted quest epoch, retaining replay/once protection | Pinned Crystal9f5a72c corpus: `data-global/scripts/quests/oramond/to_take_roots/actions_to_take_roots.lua:11–24,31–44`, blob `71c4f55d8b4e4c150dbc43e6fb5c4033c715c76a`: harvest21291 and contribution counter on repeated successful roots. `crates/gameplay/durability/src/durability/quest_state/writer.rs` passes `false` to World `quest::mint`; `reward_claim_mint/quest.rs` therefore uses the permanent callback key. ENGITEM has an uncommitted epoch-scoped implementation with a real PG green marker40.331345675s; it is not included in this report push. | Partly: repeatable plant rewards; this does not qualify random outcomes or the120-second object reset. |

Finished work imported with this report: fixture quest clock (`bbe018482`),
readable NPC/World reward text (`68d538460`, migration1641), and ENGNPC capacity
owner reconciliation. Duplicate ENGITEM1640 and duplicate atomic tests were
removed. Real PG markers: clock3.544770921s; ENGNPC tracks4.565656988s and
reward sources38.303271647s; readable text38.077736345s. Focused check/clippy,
fmt, architecture, authoring checks/unit runner, content-tree and pin checks
passed before the finished lane import. No new main merge is performed here.

# mechwire-I: client-vs-server tests for the BOTH-WIRED I rows

Scope: every `I*` row of `apps/game-server/src/qualification/SCENARIO_MATRIX.md` that `IMS.md`
marks BOTH-WIRED (17 rows). SERVER-ONLY rows (I04, I05, I10, I18a-c) are out of scope.

How the chain is tested here. A node-boot phase (`CP` =
`apps/game-server/src/qualification/client_phases`) admits through the real client transport
(`oteryn_dev_client::connect_*` -> `oteryn_session::Session::admit`), sends the row's command
through `Session` or a `client_runtime` adapter to the booted `oteryn-game-server serve`
process, and asserts on the state `Session` reduced from the reply and deltas (`inventory()`,
`container_views()`, `take_events()`). `.client` phases also write the exact payloads to
`E2E_ALL_CAPTURE_DIR`; the replay test named in the row feeds those bytes (or, without the
directory, the committed synthetic fixture) through the client decoder/reducer. `.full` phases
re-read durable state after logout and a fresh admission. Phases run only in the node-boot
topology (`node_boot_seam_against_running_node`, `#[ignore]`, CI node-boot job).

| row | test (file:line) | new/existing | verified how | gap |
|---|---|---|---|---|
| I01 | CP/full_row_i01.rs:16 (`I01.full`, row index 306): client-ui `Movement` drag + count prompt -> `client_runtime::inventory::send` splits 1 coin into a bought nested bag (`client_runtime::container_tree`), relog, re-open, merge back. Also CP/client_row_i01.rs:50 + crates/session/src/features/0014-container-tree/tests/e2e_i01.rs:41 | new + existing | new: compiled in the game-server lib test target only; not run (needs node-boot topology). existing: replay test run locally | filling a container to capacity is not driven (needs `capacity` distinct items). Depends on the character's own gold (>= bag price + 2) |
| I03 | CP/client_row_i03.rs:39, CP/full_row_i03.rs:42; replay crates/client-runtime/src/features/0011-world-interaction/tests/e2e_i03.rs:76 | existing | replay run locally | covers LOOK at self/item/absent; signs, books and spellbook content are not driven |
| I06 | CP/client_row_i06.rs:31; replay crates/session/src/tests/e2e_i06.rs:33 | existing | replay run locally | none |
| I07 | CP/client_row_i07.rs:8 (journey :25); replay crates/client-ui/src/features/0098-quick-loot/tests/e2e_i07.rs:16 | existing | replay run locally | Gold Pouch assignment is asserted as `Rejected` (no pouch carried); no corpse is looted, so LootAll returns an empty result |
| I08 | CP/client_row_i08.rs:20; replay crates/session/src/tests/e2e_i08.rs:47 | existing | replay run locally | needs a carried Primal Bag from provisioning |
| I09 | CP/client_row_i09.rs:15; replay crates/session/src/tests/e2e_i09.rs:16 | existing | replay run locally | none |
| I11 | CP/client_row_i11.rs:30, CP/full_row_i11.rs:29 (relog); replay crates/session/src/tests/e2e_i11.rs:13 | existing | replay run locally | none |
| I12 | CP/client_row_i12.rs:13; replay crates/session/src/tests/e2e_i12.rs:7 | existing | replay run locally | none |
| I13 | CP/client_row_i13.rs:10 reports `SKIPPED`; replay crates/session/src/features/0004-item-view-move/tests/e2e_i13.rs:46 runs only its synthetic fixture | existing | replay run locally (synthetic payload, no server capture) | content gap: the Bag You Covet (43895) is only a level-250 quest reward or random loot; no path grants it to a qualification character, so no chain test can run |
| I14 | CP/client_row_i14.rs:8, CP/full_row_i14.rs:60 (relog); replay crates/session/src/tests/e2e_i14.rs:10 | existing | replay run locally | none |
| I15 | CP/client_row_i15.rs:6; replay crates/session/src/tests/e2e_i15.rs:10 | existing | replay run locally | depends on the I05 lifecycle leaving Energy Ring 3088 worn |
| I16 | CP/client_row_i16.rs:6; replay crates/client-ui/src/features/0132-inspection/tests/e2e_i16.rs:9 | existing | replay run locally | none |
| I17a | CP/full_row_i17a.rs:11 (`client_runtime::hotkey_equip`, relog) | existing | read | ends FAIL `needed:capability-180` when the session lacks carried selection |
| I17b | CP/full_row_i17b.rs:9 (relog) | existing | read | none |
| I17c | CP/full_row_i17c.rs:11 (relog) | existing | read | server gap: no two-hand/shield auto-swap in GT/hotkey_equip.rs (IMS.md, *Server gaps*); the phase fails at that leg |
| I17d | CP/full_row_i17d.rs:10 | existing | read | ends FAIL `needed:feared-condition-player-route`: no player route applies Feared, so the feared gate is not exercised |
| I19 | CP/full_row_i19.rs:10 | existing | read | ends FAIL `needed:elemental-shrine-altar-and-mana-soul-in-reach`: only the no-altar refusal is driven |

Counts: 17 rows; 16 with an existing chain test, 1 new (`I01.full`); gaps that block a full pass:
I13 (content), I17c (server), I17d and I19 (no player route/reach), I01 capacity fill.

Shared-file edits: `world.rs` gains one `include!` (before `full_row_i17a_mount.rs`) and one
`e2e_row_selected(306)` block (before `I17a.full`). Index 306 was the next free index on this
branch; a parallel branch claiming 306 must renumber on merge.

Local evidence (this branch): `cargo test -p oteryn-game-server --lib --no-run` and
`cargo clippy -p oteryn-game-server --lib --tests` clean; replay tests
`cargo test -p oteryn-session --lib` (I01, I06, I08, I09, I11, I12, I13, I14, I15: 9 passed),
`-p oteryn-client-runtime --lib e2e_i03` (1 passed), `-p oteryn-client-ui --lib` I07/I16 (2 passed),
all without `E2E_ALL_CAPTURE_DIR`, so they replay the committed fixtures, not a fresh capture.
No node-boot phase was run here (no node-boot topology, no PostgreSQL 18.6).

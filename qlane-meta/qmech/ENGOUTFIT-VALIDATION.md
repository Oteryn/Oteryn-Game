# ENGOUTFIT M36 validation

Candidate: shared owner `d3a306f67`, source executor `fe4acf8a3`, merged qmech `0bfca2195`.

The dispatch lib-test executable was built with `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib --no-run --message-format=json`. Each case ran in its own process with the real PostgreSQL test route; two processes ran concurrently. Every case returned success and emitted its PostgreSQL completion marker.

| PG case | elapsed seconds | result |
|---|---:|---|
| `engoutfit_source_outfits_are_durable_and_replay_safe` | 101.963667 | PASS |
| `engoutfit_source_outfits_rollback_when_insert_is_suppressed` | 100.774860 | PASS |
| `engoutfit_rock_steady_collection_commits_exact_handins_and_weapon` | 110.527333 | PASS |
| `engoutfit_deepling_atomic_addon` | 107.623764 | PASS |
| `engoutfit_battle_mage_atomic_outfit_and_addons` | 114.787814 | PASS |
| `engoutfit_beggar_atomic_outfit_and_addons` | 115.397314 | PASS |
| `engoutfit_assassin_atomic_outfit_and_addons` | 119.348481 | PASS |

The source cases exercise production NPC dispatch, current account outfit/sex reads, the locked source reevaluation, base/addon receipts, replay, and rollback when a trigger suppresses the receipt insert. Rock Steady proves all source-backed stages, absent-item refusal, exact consumption, final Deepling base/addon receipts and replay. The remaining cases exercise the existing complete production callback stories through the same shared grant owner.

Content gates passed on the merged data: `run_checks.py --part checks`, `run_checks.py --part unit-tests`, `validate_world_project_v2_to_tree.py`, and the World bundle compiler `pin-check`. Native regeneration preserved every incoming profile byte-for-byte and added only M36 item rows 14021/14022/14023.

Focused Rust gates are recorded in the push report. No migration was added.

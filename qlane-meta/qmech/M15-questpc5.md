# M15 — reuse of the existing readable-reward mechanic

Implementation owner: ENGITEM, commit68d538460, including migration1641. No duplicate production engine or migration is added by questpc5. The optional field is absent from old serialized reward plans and the existing MINT/Text owner owns the body, receipt binding and replay.

Fresh verification on the qmech integration snapshot:

- Own focused Rust regressions:2PASS/0FAIL,0.02s; NPC bounds, one non-container root, NUL/UTF-8/scalar/count rejection; World callback uses the same bounds.
- `cargo test -p oteryn-game-server --test character_authority_postgres engitem_literal_reward_text_is_atomic_refused_and_replayed -- --nocapture`: PASS; `ENGITEM_PG_EXECUTED literal_reward_text duration=51.497609265s`. It exercised real PostgreSQL, atomic refusal, the text/receipt binding and replay, with a non-skippable execution marker after SQL and cleanup.
- `run_checks.py --part checks`: PASS.
- `run_checks.py --part unit-tests --jobs 2`:105modules PASS,1124reported tests. Existing declared source-only skips remain declared; none were introduced.
- Content-tree CI core tests and `validate_world_project_v2_to_tree.py`: PASS. Equivalence:34111Items,252Mounts,476Item bindings.
- `cargo check --workspace --all-targets`: PASS4m24s; changed-crate strict clippy PASS6.71s; fmt PASS.
- Repository native/quest/binding/matrix/audit/world-metadata generation and current-tool world pin verification PASS. No generated differences remain.

The exploratory full quest-crate run was314PASS/1FAIL: `published_ungated_quest_hash_baseline`, outfit_and_addon_quests actual `bd44ba2f572315de58bc53958ee1bf398a7039ae5badf3e796017965d051f6c8`, expected `87f6476714ed0ad9ea3a2eb7c54e0a2d34f263f536be9d6c143153629d2f328d`. This package-wide baseline is not edited, weakened or represented as green. The only questpc5 Rust delta is the two regression modules and their cfg(test) mounts; no production hash input is changed by this delta.

No new quest activation or whole-quest qualification is claimed. An experimental NPC book callback was left out because Nomads Land's selected recipe places the readable book in the cabinet, not a new NPC reward. Its uncommitted prototype is retained locally, not published. The previous direct NPC prototype had real RED92.368s/GREEN91.029s PG results, but those do not certify the selected quest route and are not used as its qualification.

# L3 exact focused commands

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib l3_endgame_quest_entry -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib endgame_entry_postgres_real_dispatch_cannot_allocate_or_reset_without_w02 -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib boss_activity_postgres_real_wire_native_entry_and_scope_barrier -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-encounters --lib endgame_entry -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch --all-targets --all-features --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check
rustfmt --edition 2024 --config skip_children=true --check crates/gameplay/durability/src/durability/activity_handoff/boss_room/quest_entry.rs
```

RED: all three new PG cases failed after actually admitting below-level actors;
169.32s. Expanded run exposed a revision-zero fixture import CHECK; corrected
root/progression snapshot revisions together. Final new cases: 3 passed / 0
failed / 0 ignored, 212.44s. Completion markers: Bakragore 73861ms, Spite 69044ms,
purple Megalomania 69007ms. Logs: l3-endgame-{red,green,final}.log.

Remaining command results are recorded by verify-l3.py in l3-verification.json;
only a zero exit and an executed PG marker count for a PostgreSQL command.
These are admission-boundary proofs, not whole-quest source qualifications.

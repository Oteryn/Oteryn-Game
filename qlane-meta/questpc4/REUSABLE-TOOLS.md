# Reusable tool mechanism

Source: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/lib/register_actions.lua#L884-L927

Retrieved source SHA-256: `5121124402c29aead613174c80e94e6b8d5c7db1e62ad152ad68f1280fc3df6d`. Lines 884–886 accept Crowbar 3304 or multitool 9598. Lines 921–927 bind UID 3073, require SewerPipe03 < 1, set it to 1, increment Mission01 and retain the source item. This mechanism slice exercises the crowbar; it is not whole-quest qualification or proof of the multitool alternative. The fixture uses a bounded synthetic room with the literal source UID, not a claim about native collision or placement.

RED: `cargo test --locked --offline -p oteryn-gameplay-dispatch --lib l6_reusable_tool_preserves_instance_and_refuses_missing_tool -- --nocapture`, PostgreSQL STARTED and DATABASE_READY markers; 0 passed / 1 failed; 47.58 seconds. Actual USE returned NothingToUse instead of missing-tool refusal because the binding did not exist. Full log: `/tmp/questpc4-reusable-red.log`.

The generic mode locks a positive carried quantity and its current custody, commits the Quest receipt, and leaves the Item row unchanged. Consuming actions retain their positive-count requirement. Reusable actions forbid replacement and require exactly one tool. Counter increments use the existing bounded Quest Add operation in the same transaction as the per-target completion flag.

GREEN: same focused PostgreSQL case, `POSTGRES_CASE_COMPLETED l6_reusable_tool duration=68.119728391s`, 1 passed / 0 failed. The accepted runtime catalogue initializes Mission01 at 0; the test asserts that initial value and requires exactly 1 after use, replay and completed refusal. Full log: `/tmp/questpc4-reusable-final.log`.

Consuming-path regression: `quest_world_callbacks_qcb3_target_swap_with_consumption_and_teleport`, PostgreSQL completion marker, 74.836354693 seconds, 1 passed / 0 failed. Full log: `/tmp/questpc4-consuming-regression.log`.

Checks completed: cargo fmt; changed-crate cargo check; clippy for quests/durability/dispatch all targets/features with -D warnings; 15 callback unit tests; 123 content integration tests; 5 World repository tests. Full repository content regeneration completed exit0, including its registered checks and a fresh 5/0 World repository run (54.97s). Repo pin-refresh and pin-check passed with unchanged bundle digest ec1fc25ee29632c23012be0191f7a3fdcd609ccdd23214286238c953e8a6caab. Final post-regeneration content integration passed123/0 in2.47s.

Review: existing callback serialization remains unchanged when the new flags are false. Zero-count inputs remain rejected. Reusable tools require count 1 and forbid item replacement. The same transaction locks live carried custody before applying both Quest effects. No migration, protocol field, currency or charge grant was added. No test was weakened or skipped.

Repository tooling also refreshed 17 stale raid references to the unchanged starting-head region `content/world/placements/region-z07-x128-y127.b3`, whose actual SHA-256 is `3d0c40e66614cd381926712ac8febd91ee97cd6c3368ed87ac192440207dc071`; its dependent catalogue digest was regenerated. Raid behavior and the B3 itself are unchanged.

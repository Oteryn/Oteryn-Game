# QUESTPC run 7 — Falcon validation evidence

Scope: the six Falcon Bastion chest USE sources, their exact seven-entry reward pool,
durable frozen draws, per-character/per-chest 24-hour repeat eligibility, native
inventory delivery, receipt replay and session fencing. This is local chest-slice
qualification; it does not certify current Global map contents or the full Secret
Library access route.

## Final focused PostgreSQL run

Exact command (filter: `falcon_chests`; library tests only):

```sh
QPC_SLOT=6 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib falcon_chests -- --nocapture
```

Tool session: `48250`; final output chunk: `15ebd3`.
Build duration: **1m 18s**. Test duration: **58.58s**.
Result: **2 passed, 0 failed, 0 ignored**, with 2493 tests filtered out.

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 18s
running 2 tests
test gameplay_transport::monk_save::postgres_tests::falcon_chests::falcon_chests_exact_six_sources_and_seven_rewards ... ok
FALCON_CHEST POSTGRES_CASE_COMPLETED coast duration=6.902796985s
FALCON_CHEST POSTGRES_CASE_COMPLETED island duration=5.772876515s
FALCON_CHEST POSTGRES_CASE_COMPLETED throne_hall duration=5.633103452s
FALCON_CHEST POSTGRES_CASE_COMPLETED shortcut duration=5.714453674s
FALCON_CHEST POSTGRES_CASE_COMPLETED lower_bastion duration=5.61566062s
FALCON_CHEST POSTGRES_CASE_COMPLETED underground_bastion duration=5.72476515s
test gameplay_transport::monk_save::postgres_tests::falcon_chests::falcon_chests_postgres_real_use_frozen_draw_refusal_restart_and_fences ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2493 filtered out; finished in 58.58s
```

The six markers follow real PostgreSQL scenarios, each with an isolated database
and the production native Content generation. Every scenario drives literal USE
through the connection, proves stale-lease rejection before a draw is written,
checks the minted item/count against the frozen entropy and selected index,
replays through recreated composed owners, refuses a fresh claim at 24 hours minus
1 ms, and mints cycle 2 at exactly 24 hours. Replaying the original command after
cycle 2 preserves the original receipt and leaves exactly two Falcon mint receipts.

Coast additionally fills the equipped backpack through the existing mint owner,
proves full-backpack refusal preserves the draw while writing no Falcon mint
reservation, frees room through the actual custody/drop owner, then retries with
the same frozen entropy/index. The isolated database's `game_falcon_now_ms()`
function is replaced with fixed receipt-derived times for the boundary checks;
cycle/receipt guards remain active, and database cleanup removes that clock seam.

The final run followed corrections to a missing native i9081 profile, the shared
Newhaven vocation order, and the owned fixture's connection recreation/command
sequence. It supersedes the earlier failed attempts.

## Final pure rule run

Exact command (filter: `quest::falcon`; library tests only):

```sh
QPC_SLOT=6 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-quests --lib quest::falcon -- --nocapture
```

Tool session: `77004`; final output chunk: `9f0b5c`.
Build duration: **2.58s**. Test duration: **0.00s**.
Result: **3 passed, 0 failed, 0 ignored**, with 256 tests filtered out.

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 2.58s
running 3 tests
test quest::falcon::tests::repeat_opens_at_exactly_twenty_four_hours ... ok
test quest::falcon::tests::six_chests_have_the_exact_seven_entry_pool ... ok
test quest::falcon::tests::seven_entry_draw_rejects_modulo_bias ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 256 filtered out; finished in 0.00s
```

These check the exact six-source/seven-reward inventory, the day-minus-1-ms/exact-day/
day-plus-1-ms boundary and overflow refusal, and rejection sampling's unbiased
seven-way mapping with deterministic replay of frozen entropy.

## Formatting and integration boundary

Exact final formatting command, exit 0 (tool output chunk `0578bc`):

```sh
rustfmt --edition 2024 --check crates/gameplay/quests/src/quest/falcon.rs crates/gameplay/durability/src/durability/reward_claim_mint/falcon.rs crates/gameplay/dispatch/src/gameplay_transport/falcon_chests.rs crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/falcon_chests.rs
```

The worker supplied `.questpc/falcon-migration.sql`; root integrated its schema,
immutable cycle/draw proof and extra receipt guard into migration 0969. Public
Content claim keys remain the original six keys. Only the durable owner derives
internal `:cycle:N` mint-operation identities after the repeat gate.

After the successful focused run, root changed the PostgreSQL test helper's
visibility from private to `pub(super)` so the broad catalogue test can delegate
to it. The helper body was unchanged. No test was rerun for this documentation
follow-up, as explicitly instructed by root. Root reports that these six Falcon
claims now contribute to catalogue coverage of **334 total / 39 qualified**; those
aggregate counts are coordinator-provided integration state, not an additional
worker test result. Combined Clippy and broad catalogue evidence belong to root.

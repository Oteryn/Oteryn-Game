# Pick exchange — current exact commands

All calls use CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true and the absolute wrapper,
serially. Actual final results:26 tests passed (7 durability units,17 NPC units,
Pick PG and Oriental Trader PG); zero failures. Exact final PG markers54592ms
and55948ms; no route-less return.

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --lib durability::npc_trade::tests:: -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-npc --lib npc_talk -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib pick_exchange_atomic_confirmation_refusal_and_replay -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib oriental_trader_catalogue_ariki_sale_and_achievement_once -- --nocapture --test-threads=1
```

Clippy/fmt commands and logs: verify-pick-exchange.sh and
pick-exchange-verification.tsv. Migration1020; only its new guard body was
repaired, leaving the old Buy/Sell assertions and guards intact.

---

# Oriental Trader continuation — current commands/results

All cargo calls use the absolute queued wrapper below, with
`CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true`, one command at a time.
Final validation:21 tests passed,0 failed (17 dialogue units,3 server port/reset
units and1 executed PG case). The PG regression was first red with the missing
committed Yasir dialogue. First green46850ms; final green61158ms, both actual
POSTGRES_CASE_COMPLETED markers. No route-less return qualified this path.

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib oriental_trader_catalogue_ariki_sale_and_achievement_once -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-npc --lib npc_talk:: -- --test-threads=2
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --lib yasir -- --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --lib w05b_the_boot_roll_is_fixed_per_reset_epoch_and_follows_the_calendar -- --test-threads=1
```

Fresh initial dispatch check passed. Final fmt/explicit mounted-test rustfmt,
product whitespace and content/world/npc/dispatch all-targets/all-features
clippy with -Dwarnings passed. The initial clippy finding was a private test
type-complexity alias repair, with no runtime change.

```sh
/home/mole/oteryn-cache/codex-bin/cargo check --locked -p oteryn-gameplay-dispatch --quiet
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked -p oteryn-gameplay-content -p oteryn-gameplay-world -p oteryn-gameplay-npc -p oteryn-gameplay-dispatch --all-targets --all-features --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo fmt --all --check
rustfmt --edition 2024 --check crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/oriental_trader.rs
```

Logs:oriental-trader-dialogue-red.log,oriental-trader-pg-red.log,
oriental-trader-pg-green.log,oriental-trader-pg-final.log,
oriental-trader-npc-units.log,oriental-trader-ports.log,
oriental-trader-reset.log,oriental-trader-clippy-final.log.
Before NPC edits, the existing Panpipe key test also passed1/0 with its actual
33472ms marker (`routing-grant-panpipe-pg.log`); its exact filter remains below.
Earlier bulk reward cohorts were not redundantly rerun. Reward counts remain
11 sets/18 claims; Oriental Trader adds no RewardClaim ledger row.

---

# Current restart commands

Fresh continuation commands below used `CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true`;
one cargo command at a time, always the absolute queued wrapper. No new product
code was added. Batch-two passed3 tests/0 failures (122.21s), four positive-duration
claim markers. Naginata passed1/0 with its40407ms marker; neither run was route-less.

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_batch_two_ -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_naginata_pick_six_shared_hits_and_replay -- --nocapture --test-threads=1
```

Focused unit commands also passed (1+1+12+6 tests). Fresh continuation total:
24 Rust tests including4 executed PG scenarios;54 Python tests,0 failures.

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_committed_map_selectors_match_source -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_source_selectors_bind_exactly -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-world --lib map::facts -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-content --lib world_reward_claims -- --nocapture
```

Transcripts:continuation-batch-two-pg.log and continuation-naginata-pg.log.
Initial dispatch cargo check,all-targets/all-features content/world/dispatch
clippy with -Dwarnings and cargo fmt --all --check also passed. Historical checks
below remain scoped to the unchanged product tree; they are not new qualifications.

---

# Exact restart test commands

Panpipe Key4055 continuation (all use the same absolute queued wrapper and
four build jobs, offline):

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_panpipe_key_4055_once_and_replay -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_batch_one_panpipe_bag_exact_contents -- --nocapture --test-threads=1
```

Key regression was observed red with ItemReference, then passed with an actual
POSTGRES_CASE_COMPLETED/26039ms marker. Bag regression passed with25998ms.
All11 final checks in `panpipe-key-verification.tsv` pass; their exact commands
are in `verify-panpipe-key.sh`. The selector checks below now include Key4055.
This continuation ran150 Rust tests (two routed PG),0 failed,0 ignored.
Fresh content/world/dispatch all-targets/all-features clippy/-Dwarnings passed.
Python selector/overlay10, profiles15, ledger7 and governance129 passed;
content/world/governance validators, exact source/canonical coverage replay
and ledger checker passed. No route-less return qualified a claim.

Every cargo command uses the absolute queued wrapper, four build jobs and offline
mode. PostgreSQL comes only from that wrapper. These are the actual commands run
in this restart, filtered to the owned mechanic for every PG case.

```sh
export CARGO_BUILD_JOBS=4
export CARGO_NET_OFFLINE=true
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_batch_one_ -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_batch_two_ -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_n -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_committed_map_selectors_match_source -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_source_selectors_bind_exactly -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-world --lib map::facts -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-content --lib world_reward_claims -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test integration_rest content_ --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test content_world_project_repository --quiet
```

Results:5+3+7 routed PG tests,17/17 captured claims,25 completion markers with
positive durations. Two selector units,12 World units,6 reward-content units,
123 server content tests and5 World repository tests also passed:163 Rust tests
in total, with no failures or ignored cases in those runs.

The first PG batch covers Orc Fortress (including level gate), Orc Shaman,
Panpipe, Plate Armor and Purple Tome. The second covers the corrected two-instance
Poison Daggers grant, Power Bolts ammunition/book and Parchment Room's key bag.
The third covers Naginata's full mechanic journey/grant/gate, Nightmare Teddy
and Noble Armor's two grants/gate. Access/storyline limits remain in RESUME.md.

All14 original cases remain after the file/prefix rename, plus eight quest-specific
wrappers. The five original Phase A cohort aliases were not rerun redundantly
in this restart; the quest-specific tests execute their complete shared scenarios.
CI still mounts every case through dispatch --lib.

Additional checks passed:

```sh
/home/mole/oteryn-cache/codex-bin/cargo check --locked -p oteryn-gameplay-dispatch --quiet
/home/mole/oteryn-cache/codex-bin/cargo fmt --all --check
rustfmt --edition 2024 --check crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_reward_claims.rs
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked -p oteryn-gameplay-content -p oteryn-gameplay-world -p oteryn-gameplay-dispatch --all-targets --all-features --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-check . .questpc2/pin-check-phase-a
```

Python:29 selector/profile units;30 native-reward/correction/variant units;
28 registered authoring checks (including four correction-boundary cases);
7 qualification-ledger guard tests. Deterministic content, source capture,
map restoration and qualification-ledger checks passed. Pytest was unavailable;
the repository's native unittest and executable test-script runners were used.

Evidence:restart-batch-one-pg.log, restart-batch-two-pg.log,
restart-n-regression.log, restart-verification.tsv and per-check logs.


## Routing-audit continuation, 2026-11-06

No product change and no new PG qualification. Fresh check, fmt and
content/world/dispatch all-targets/all-features clippy/-Dwarnings passed.
Two filtered dispatch units passed,1 each:

```sh
CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_committed_map_selectors_match_source -- --nocapture
CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_source_selectors_bind_exactly -- --nocapture
```

Logs:current-dispatch-check.log,current-fmt.log,current-clippy.log,
current-map-selectors.log,current-source-selectors.log. Python passed21
(10 selector,4 correction,7 ledger). The exact earlier PG filters/commands
above still describe the unchanged implemented scope; no PG rerun is claimed.

## Panpipe key-door access continuation — 2026-11-06

Every cargo invocation is serialized with CARGO_BUILD_JOBS=4 and
CARGO_NET_OFFLINE=true through /home/mole/oteryn-cache/codex-bin/cargo.
The initial dispatch check passed before any new change. Exact PG commands:

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_panpipe_ -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_panpipe_locked_door_to_reward_and_replay -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_batch_one_panpipe_bag_exact_contents -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_naginata_pick_six_shared_hits_and_replay -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib oriental_trader_catalogue_ariki_sale_and_achievement_once -- --nocapture
```

The panpipe_ filter matches exactly the hollow-stone key-acquisition scenario
and the new locked-door-to-bag scenario. The standalone repair run passed1/0
with marker27802ms; the final paired run is panpipe-access-pg-final.log. No
route-less qualification. The case seeds carried key custody; it does not
claim the long approach or exit. The separate bag case retains full atomic
children/audit/cold-replay assertions.

The exact unit filters and complete commands are in verify-panpipe-access.sh:
quest_reward_claims_committed_map_selectors_match_source,
quest_reward_claims_source_selectors_bind_exactly, map::facts,
world_reward_claims, npc_talk, integration_rest content_, and
content_world_project_repository. All15 script stages PASS, including
content/world/npc/dispatch --all-targets --all-features clippy/-Dwarnings,
cargo fmt, explicit mounted-fixture rustfmt, whitespace and final pin-check.
Pin digest84deca54329e9c7fa284c895117b8c08817ea2b9c8c750c11a26b6063b178f20.

Python: overlay11, key native2, ledger7 and correction4 passed; native-provider
and raid --check passed. Broader profile discovery passed108/109; its unrelated
existing Item2816 negative assertion remains Unfinished in
panpipe-access-unrelated-python-failure.md. It is not represented as green.

Final paired run:2 passed,0 failed,65.52s; locked-door marker32777ms.
All final named Rust commands total170 passed,0 failed,0 ignored, including
five executed PG scenarios. Counted evidence:panpipe-access-final-summary.json.


## Current item-exchange continuation

All commands are serialized, CARGO_BUILD_JOBS=4, CARGO_NET_OFFLINE=true, with
the absolute queued cargo wrapper. Exact focused commands:

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --lib durability::npc_trade::tests:: -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-npc --lib npc_talk -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib postgres_tests::pick_exchange:: -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib oriental_trader_catalogue_ariki_sale_and_achievement_once -- --nocapture --test-threads=1
```

The shared exchange filter executes exactly
pick_exchange_atomic_confirmation_refusal_and_replay and
obsidian_knife_uzgod_atomic_exchange_cooldown_and_replay. Final2/0 with actual
markers63180ms and61480ms; Yasir1/0 marker67591ms. Durability8/0 and NPC17/0;
28 named test executions total, zero failures/ignored. No unfiltered PG suite,
route-less qualification, or shared fixture relaxation. All9 validation script
stages pass; server all-target clippy also passed. All10 verification rows are PASS.


## Seymour present exchange continuation — 2026-11-06

All cargo invocations use CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true and
/home/mole/oteryn-cache/codex-bin/cargo, serially. Exact filters:

- `test --locked -p oteryn-gameplay-durability --lib durability::npc_trade::tests:: -- --nocapture --test-threads=1`
- `test --locked -p oteryn-gameplay-npc --lib npc_talk -- --nocapture --test-threads=1`
- `test --locked -p oteryn-gameplay-dispatch --lib postgres_tests::pick_exchange:: -- --nocapture --test-threads=1`
- `test --locked -p oteryn-gameplay-dispatch --lib oriental_trader_catalogue_ariki_sale_and_achievement_once -- --nocapture --test-threads=1`

The exchange filter selects exactly Al Dee, Uzgod, Seymour confirmation/replay,
and Seymour filled-box preservation. No whole PG suite is selected.
First Seymour red reached PostgreSQL and failed on missing admitted dialogue
(59.21s). First successful exchange marker55946ms; the initial paired run then
exposed a wrong success-status assertion in the new filled-box branch. The
production result mapping correctly rejects that command. The new branch now
asserts Rejected at wire and NPC levels and checks unchanged custody and no
trade/audit writes. All prior Al Dee/Uzgod success assertions remain intact.
Final results must be read from present-exchange-verification.tsv; the active
script is verify-present-exchange.sh. Full quest qualification is not claimed.

Final Seymour continuation: all9 stages PASS.31 focused tests passed,0 failed/
ignored; all5 PG scenarios have positive timed completion markers. Exchange
markers: Uzgod56578ms, Al Dee71409ms, Seymour72566ms, filled60804ms; Yasir62903ms.
Check and all-target clippy -Dwarnings passed for durability/npc/dispatch/server.
Fmt, explicit mounted-file rustfmt and product whitespace passed. Source chest
admission remains unfinished; no new full-quest qualification claimed.

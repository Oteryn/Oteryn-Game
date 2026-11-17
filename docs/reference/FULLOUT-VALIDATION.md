# FULLOUT validation

Branch: `claude/happy-davinci-ijz3wy-fullout`.
Base: `e2152d050038cecdb3728c222af2ac184e1cac5a`.

The achievement table and production wiring were accepted in `098597118`.
The achievement registry and existing content achievements are unchanged.
Behaviour evidence is the owner-accepted TibiaWiki achievement pages, spoiler
fields (fetched 2026-11-17).

## PostgreSQL results

All cases executed against the retained PostgreSQL 18.6 cluster. The seven
quest cases took 61.02 seconds on the implementation and 64.04 seconds on the
base grant owners. The three additional path cases took 147.71 seconds on the
base and 111.05 seconds in the final restored-head run. No case was ignored.

| Case | Base grant owners | Implementation |
| --- | --- | --- |
| `full_outfit_quest_addons_and_replay` | FAIL: second addon leaves achievement count 0, expected 1 | PASS |
| `full_outfit_quest_reverse_addons` | FAIL: second grant leaves count 0, expected 1 | PASS |
| `full_outfit_male_mage_alumni` | FAIL: count 0, expected 1 | PASS |
| `full_outfit_female_mage_ritualist` | FAIL: count 0, expected 1 | PASS |
| `full_outfit_male_summoner_ritualist` | FAIL: count 0, expected 1 | PASS |
| `full_outfit_female_summoner_alumni` | FAIL: count 0, expected 1 | PASS |
| `full_outfit_unlisted_outfit_grants_nothing` | PASS: no award | PASS: no award |
| `fullout_voucher_last_addon_and_replay` | FAIL: award request source is `item_action`, expected `full_outfit` | PASS |
| `fullout_mixed_quest_then_voucher_and_replay` | FAIL: award request source is `item_action`, expected `full_outfit` | PASS |
| `fullout_task_board_last_addon_and_replay` | FAIL: last addon leaves count 0, expected 1 | PASS |

The unlisted outfit is a negative control and deliberately passes both versions.
The base already has a legacy content award for the Rascoohan vouchers. Their
count assertion alone therefore does not distinguish the implementations; the
provenance assertion proves that the shared appearance evaluator owns their
award. The legacy content rules were preserved. Both voucher cases also assert
one achievement fact and one grant request after exact replay.

The mixed case grants the base and addon 1 through the fenced quest reward
writer, then uses voucher 35695 through the real consume/transform USE ingress.
The voucher-only case grants the base through the quest writer and uses vouchers
35595 and 35695. The shop case buys the Feral Trapper base and both addons through
Task Board dispatch. It checks no award before the second addon and exact replay
of each purchase. Quest cases check no award after the first addon, exact replay
without error, and one grant request at completion.

## Baseline procedure and build limits

No branch was merged, reset or rebased. For the differential run the following
production files were temporarily restored byte-for-byte from the base, while
the selected test assertions, helper bodies and inputs were retained:

- `crates/gameplay/durability/src/durability/character_appearance.rs`
- `crates/gameplay/durability/src/durability/character_appearance/quest_grants.rs`
- `crates/gameplay/durability/src/durability/consume_transform.rs`
- `crates/gameplay/durability/src/durability/consume_transform_cosmetics.rs`
- `crates/gameplay/durability/src/durability/task_board/entitlements.rs`

The full baseline dispatch test compilation exceeded the 16 GiB local memory
limit, including attempts with one Cargo job and a clean package target. Those
compiler failures are not counted as red evidence. For the successful local
baseline build only, 4,195 unrelated `#[test]` registrations were omitted from
compilation. No selected case, assertion, helper body or production logic was
altered for this compile focus. The omitted cases
were already excluded by the `fullout_` runtime filter. All registrations and
production files were restored byte-for-byte afterward, with saved source hashes
checked before the restored-head run. No registration change is included in the
candidate. The head binary used its full normal registrations: 3 cases passed,
4,313 were filtered out, and 0 were ignored.

The local dispatch build used `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0` and
`CARGO_PROFILE_TEST_DEBUG=0`. The baseline build also used the already-installed
jemalloc allocator with compact allocation settings. These were command-local
resource settings; no repository build profile or dependency was changed, and
assertions and overflow checks stayed enabled.

## Focused commands

The prepared environment supplies the PostgreSQL URL without printing it.
The mounted cases return `Ok(())` when that variable is absent.

```sh
cargo test --locked -p oteryn-game-server --test character_authority_postgres full_outfit -- --nocapture
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib fullout_ -- --nocapture --test-threads=1
cargo test --locked -p oteryn-gameplay-durability --lib full_outfit -- --nocapture
cargo fmt --all --check
cargo clippy --locked -p oteryn-gameplay-dispatch --all-targets --all-features --quiet -- -D warnings
oteryn-world-bundle-compiler pin-check . /tmp/fullout-final-pins
```

The registry unit case is
`full_outfit_table_has_thirteen_rows_and_registered_achievements`.

## Local check summary

| Check | Result |
| --- | --- |
| Workspace formatting check | PASS |
| Explicit formatting check for the five mounted test files | PASS |
| Dispatch clippy, all targets and all features, warnings denied | PASS |
| Dispatch PostgreSQL `fullout_` cases, full restored registrations | 3 passed, 0 failed, 0 ignored |
| Quest PostgreSQL `full_outfit` cases | 7 passed, 0 failed, 0 ignored |
| Durability `full_outfit` registry unit case | 1 passed, 0 failed |
| World bundle `pin-check` | PASS |

The quest head result is reused from the accepted `856f9755` test change: this
follow-up changes neither those case bodies nor any production code. The base
quest cases were rerun for the missing per-case red proof. The dispatch cases,
registry unit, formatting, clippy and pins were checked for this follow-up.
The `ci-local.sh` runner is absent on this checkout, and no PR was opened.

# Changed-file qualification checkpoint

Baseline: `245ba7838e58318a2d851857b6c2b2d7c757aa09`. All 19 owned source files below exactly match their recorded SHA256. Full digests, test commands and artifact paths are in `changed_file_qualification.json`.

The final Windows protocol run passed 911 tests with 0 failures and 0 ignored. All 17 protocol edits were present together, including retained source hashes from earlier batches. Protocol all-target Clippy with `-D warnings`, package formatting and whitespace checks passed. Identity 7 and gameplay foundation 394 tests passed on the VPS; their candidate source hashes have not changed. One subsumed scheduled-events Debug smoke was removed, so the original three-package 1313-test total became 1312.

Required default package builds also passed separately on Windows for `oteryn-protocol-oteryn`, `oteryn-identity`, `oteryn-gameplay-foundation` and `oteryn-client-runtime`. The receipt `windows_default_build_qualification/results.json` records all four actual commands and 23 matching source hashes. No tests were rerun for this build gate.

There are 21 distinct mutation probes. Each recorded baseline test scope passed with its mutation, and the corresponding repaired scope failed via an assertion after compilation. The registry probe changes an aggregate input value supplied to its test helper; it is not a mutation of the production registry builder. All production mutations were restored before package qualification. Baseline test bodies in later Windows probes use exact `245ba783` content while earlier unrelated qualified tests remain present.

| Source file | Verified change | Mutation receipt |
|---|---|---|
| `crates/gameplay/foundation/src/scheduled_events.rs` | removed Debug-text smoke subsumed by neighboring holiday schedule assertions | `qualification/qualification_analyser.json` (subsumption + package regression) |
| `crates/identity/src/lib.rs` | fixed PKCE verifier/challenge and retained redaction | `qualification/qualification_initial.json` (pkce) |
| `crates/protocol-oteryn/src/analyser_catalogue.rs` | 17 individually valid unique monster IDs | `qualification/qualification_analyser.json` (analyser_catalogue) |
| `crates/protocol-oteryn/src/analyser_party.rs` | 2049 individually valid unique price IDs; existing2048 positive retained | `qualification/qualification_analyser.json` (analyser_party) |
| `crates/protocol-oteryn/src/appearance_tests/mappings_tests.rs` | 256 valid ordered mount names positive;257 exact rejection | `appearance_qualification/results.json` (mount_limit) |
| `crates/protocol-oteryn/src/atelier.rs` | valid lock fixture with only bool changed; exact paid-action byte limit | `windows_batch1_qualification/results.json` (atelier_lock_bool, atelier_request_limit) |
| `crates/protocol-oteryn/src/attack_tests.rs` | exact result byte limit distinguishes duplicate-field fallback | `windows_batch3_qualification/results.json` (attack_result_byte_limit) |
| `crates/protocol-oteryn/src/battle_metadata/tests.rs` | 16 unique valid entity refs positive;17th distinct rejection | `windows_batch1_qualification/results.json` (battle_metadata_count) |
| `crates/protocol-oteryn/src/bestiary.rs` | 1025 individually valid race rows isolate count from invalid race1025 | `windows_batch2_qualification/results.json` (bestiary_encoder_count) |
| `crates/protocol-oteryn/src/bestiary_catalogue.rs` | exact page byte-limit rejection | `windows_batch1_qualification/results.json` (bestiary_page_limit) |
| `crates/protocol-oteryn/src/boss_difficulty_tests.rs` | exact request byte-limit rejection | `windows_batch2_qualification/results.json` (boss_difficulty_request_limit) |
| `crates/protocol-oteryn/src/boss_encounter.rs` | exact reward cursor byte limit; valid7-day lifetime at deadline−1/deadline | `windows_batch2_qualification/results.json` (boss_reward_cursor_limit, boss_reward_expiry_boundary) |
| `crates/protocol-oteryn/src/boss_encounter/catalogue_tests.rs` | 16 distinct valid boss keys positive;17th rejection | `windows_batch2_qualification/results.json` (boss_catalogue_entry_limit) |
| `crates/protocol-oteryn/src/boss_encounter_tests.rs` | 65 sorted distinct cooldown keys; exact query byte limit | `windows_batch2_qualification/results.json` (boss_cooldown_entry_count, boss_cooldown_query_limit) |
| `crates/protocol-oteryn/src/boss_rewards_tests.rs` | exact browse/collect request and collect-result byte limits | `windows_batch3_qualification/results.json` (boss_reward_reader_byte_limits) |
| `crates/protocol-oteryn/src/contract_registries.rs` | full ordered fragment aggregate equality | `qualification/qualification_initial.json` (registry) |
| `crates/protocol-oteryn/src/cyclopedia.rs` | eight valid unique viewport points positive; distinct ninth point isolates encoder count guard | `windows_batch4_qualification/results.json` (cyclopedia_encoder_point_count) |
| `crates/protocol-oteryn/src/daily_boost.rs` | literal22-byte wire oracle replaces test-only mirror decoder | `qualification/qualification_initial.json` (daily) |
| `crates/protocol-oteryn/src/guild_war.rs` | oversize request requires exact LimitExceeded | `qualification/qualification_initial.json` (guild) |

## Root-owned validation completed by this lane

Client-runtime 59 tests passed after the house-limit and three component-replay changes. House guard removal passed the old test and failed the repaired test; all-target Clippy, package formatting and whitespace checks passed. No formatting changes were needed. The earlier synthetic CLI probe passed its normal path and rejected suppressed input actions and missing target entities; both faulty outputs had passed the old CLI. This is synthetic qualification; live end-to-end gameplay was outside the run.

UI Unicode export qualification passed 729 tests (671 unit and 58 integration), with 0 failures or ignored tests. All-target Clippy with `-D warnings`, package build, formatting and whitespace checks passed. The ASCII-drop mutation passed the old test and failed the final `is_ok_and` assertion. The initial denied-`expect` Clippy failure is preserved; root fixed it before final qualification. Root applied the formatting-only patch, and the current shared source SHA matches the compiled receipt. Production mutations were restored and all pre-existing remote changed-source hashes were preserved.

## Hireling missing-command follow-up

The root-owned hireling test now returns an error when its Summon request produces no command. On Windows Rust 1.100.0, the exact same Summon-only production mutation passed the original named test (1 executed) and failed the corrected test (1 executed, missing-command error). Production was restored before the full UI run: **729 passed, 0 failed, 0 ignored**, followed by default build and strict all-target Clippy. The original compiled library contained 671 IDs; all 671 identical IDs executed successfully in the restored run. No test was removed or renamed.

Package formatting initially found only inherited CRLF in three unchanged architecture-check sources imported by UI tests. The receipt preserves that failure and proves each normalized file matched the HEAD bytes exactly after CRLF removal. Formatting and whitespace gates then passed without a source behavior change. The final hireling SHA is `061a3c719bc7a297c7ffb10550bfc114663330ceb8719b913388d9713146b7bb`; the Unicode chat file remains `ef84a79b75de54b714ddd644f285813f968a3e1e9b6338d7078566a53f440a66`. Every pre-existing changed-source hash in the Windows checkout was preserved. Detailed evidence: `windows_hireling_ui_qualification/results.json` and its nine hash-verified logs.

## Boss-panel duplicate and ordering follow-up

The two endgame-chain tests had exact matching bodies after replacing only the function name: 1,706 normalized bytes, SHA `263320b9e6135bc4159c95c55aa89cd7ca1182bdf28f5b34e9f34b1a31fa954b`. The first compiled ID remains; only `boss_encounter::tests::endgame_chain_selection_preserves_confirmed_progress_boundary` was removed. The cooldown-order test now requires both text positions to exist before comparing them.

The same isolated rendering mutation that omitted only `b:b` passed the old test and failed the strengthened assertion. After restoring production, Windows Rust 1.100.0 executed **728 passing tests: 670 library + 58 integration**, with zero failures, ignored or filtered cases. Comparing the original compiled library IDs to final actual outcomes showed exactly the single intended duplicate removal and no additions. Default build, strict all-target Clippy, package formatting and whitespace checks passed. All prior changed-source hashes, including hireling and Unicode chat, were retained. This supersedes the earlier 729-test package total for the final three-file UI batch; earlier receipts remain historical evidence.

Current state-test SHA: `a08947ccd0b93908526c5976683d13ab538b85e8e5ec9d595044e9ac2187c9de`. Evidence: `windows_boss_ui_qualification/results.json`, the exact equality receipt, validator and eight verified logs. This lane implemented the root-authorized patch and therefore does not label its own patch an independent review.

## Audit boundary

This checkpoint covers changed-file qualification. Manual semantic review remains ongoing in `semantic_review.json`; the 404-file mechanical inventory is not evidence of completed semantic review. No commit, push, review trigger, label or merge was performed by this lane.

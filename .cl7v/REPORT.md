# CL7-VERIFY

**Result: the requested local verification is complete. 1,584 tests passed; zero failures, ignored tests or PostgreSQL early returns. No actionable correctness finding or source repair. Nothing was committed.**

## Scope and checkout

- Reviewed delivery: `28cfe8937510b659caea7ad4763e3bc60ddf0c9c`, against `b2cfa798`.
- The checkout is detached at that delivery SHA; the packet names branch `claude/happy-davinci-ijz3wy-cl7raw`. No branch/history mutation was performed.
- Actual delivery: **37 files, +846/-85**, versus the packet's 36 files/+846/-84. The deleted one-line proficiency unsupported-baseline fragment accounts for the difference.
- The implementation extends existing capability **195**, adds command **641** and mechanic **638**. Reserved capability **371** and offer order **637** are unused. This is a packet/implementation discrepancy, not a local number collision. The checked-in browse contract explicitly specifies capability 195.
- I18, P05 and W06 client behavior is inherited from the base. I17 adds carried-item capability negotiation; its existing hotkey editor is inherited. P08 and M04 contain the substantive new client wiring.
- META organization policy 3.1.0 was resolved by a read-only GitHub file fetch at bound commit `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`. Required repository development, coding, review, debugging and verification skills were read. Optional design/performance skills were not used.

## Per-row verdicts

**DONE below means this task's bounded client implementation review and local verification are complete. It does not reclassify the whole gameplay mechanic or replace the mechanics ledger.** Counts are named evidence subsets of the package runs, not extra executions.

| Row | Verdict | Named passing evidence count | Scope |
|---|---|---:|---|
| P08 | DONE — client verification | 16 | 5 native feature tests, 1 client capability route test, 8 dispatch units, browse PG and existing perk-selection PG. Browsing exposes the complete pinned catalogue and current session XP. |
| M04 | DONE — client verification | 6 | 4 native drag/input tests, 1 client capability route test, 1 real-dispatch PG case with pushable/unpushable scenarios. |
| I17 | DONE — client verification | 4 | 3 native hotkey tests plus carried-capability negotiation test. |
| I18 | DONE — inherited client verification | 6 | Existing browse-field open, server window selection, item movement and up-navigation controls. No new I18 implementation in this diff. |
| P05 | DONE — inherited client verification | 1 | Existing talisman purchase/full-owner routing and visible refusal. No new P05 implementation in this diff; full task-board tests also passed in the client run. |
| W06 | DONE — inherited client verification | 1 | Existing server-projected portal click, removal and ordinary reward-container path. No new W06 implementation in this diff. |

### P08: 16 passing named tests

- `play::tests::feature_0195_weapon_proficiency::controls_encode_exact_request_and_reply_and_refusal_are_visible`
- `play::tests::feature_0195_weapon_proficiency::native_proficiency_browse_decodes_experience_and_pages_the_complete_list`
- `play::tests::feature_0195_weapon_proficiency::native_proficiency_rejects_invalid_fields_and_clears_perks`
- `play::tests::feature_0195_weapon_proficiency::proficiency_page_rejects_truncation_trailing_and_oversized_rows`
- `play::tests::feature_0195_weapon_proficiency::native_proficiency_actions_and_input_gates`
- `capability_support::weapon_proficiency_tests::proficiency_browse_is_advertised_and_routed_with_item_identity`
- `gameplay_transport::monk_save::postgres_tests::proficiency_browse::proficiency_browse_postgres_pinned_catalogue_live_xp_and_session_fence`
- `gameplay_transport::monk_save::postgres_tests::proficiency_select_perk::proficiency_select_perk_wire_refusals_replay_and_durable_selection`
- `gameplay_transport::weapon_proficiency::tests::real_kill_dispatch_credits_every_recent_contributor_full_points_once`
- `gameplay_transport::weapon_proficiency::tests::real_attack_dispatch_applies_selected_threshold_effects_once`
- `gameplay_transport::weapon_proficiency::tests::an_active_modification_replaces_its_level_perk_in_attack_damage`
- `gameplay_transport::weapon_proficiency::tests::production_catalogue_resolves_promoted_weapon_for_durable_writer`
- `gameplay_transport::weapon_proficiency::tests::promoted_selected_attack_perk_reaches_live_dispatch`
- `gameplay_transport::weapon_proficiency::tests::selected_source_defense_perks_reach_mitigation_projection_and_clear`
- `gameplay_transport::kill_reward::proficiency_kill_tests::proficiency_kill_credit_full_points_window_replay_and_checkpoint_decision`
- `gameplay_transport::kill_reward::proficiency_kill_tests::daily_proficiency_bonus_credits_exact_chance_and_never_replays`

### M04: 6 passing named tests

- `play::tests::feature_0082_creature_push::creature_pointer_preserves_cl1_ground_clicks_and_cl2_cursor_targeting`
- `play::tests::feature_0082_creature_push::death_discards_queued_cl3_controls_without_discarding_logout`
- `play::tests::feature_0082_creature_push::native_drag_cancels_on_focus_loss_or_stale_target_and_decodes_refusal`
- `play::tests::feature_0082_creature_push::native_drag_sends_push_once_without_attacking_or_walking`
- `capability_support::creature_push_tests::creature_push_is_advertised_and_routed_with_its_prerequisites`
- `gameplay_transport::monk_save::postgres_tests::creature_push::creature_push_real_dispatch_moves_and_refuses_on_postgres`

### I17: 4 passing named tests

- `play::tests::feature_0179_hotkey_equip::assigned_item_key_sends_the_definition_and_waits_for_the_server`
- `play::tests::feature_0179_hotkey_equip::native_hotkey_configuration_and_visible_refusal`
- `play::tests::feature_0179_hotkey_equip::hotkey_equip_panel_opens_and_sends`
- `capability_support::hotkey_equip_tests::hotkey_equip_carried_is_advertised_with_its_command_owner`

### I18: 6 passing named tests

- `play::tests::feature_0230_browse_field::browse_field_native_controls_and_results`
- `play::tests::feature_0230_browse_field::browse_field_native_item_controls_preserve_move_and_up_routes`
- `play::tests::feature_0230_browse_field::browse_field_panel_opens_and_sends`
- `play::tests::feature_0230_browse_field::browse_field_selects_the_first_visible_server_window`
- `play::tests::feature_0230_browse_field::browse_field_native_moves_use_session_item_commands`
- `play::tests::feature_0230_browse_field::browse_field_up_keeps_the_ground_container_view_id`

### P05: 1 passing named tests

- `play::tests::feature_0072_task_board::talisman_purchase_uses_full_owner_and_displays_refusal`

### W06: 1 passing named tests

- `play::tests::feature_0006_world_spatial_entities::echo_portal_click_walks_and_reward_remains_in_native_container`

## Commands and results

All Cargo invocations below used the absolute queued wrapper. Every build/test command used `--offline --locked`; `fmt` used the pinned toolchain. No dependencies were downloaded. Commands ran sequentially in this checkout. The remaining-command runner supplied `CARGO_NET_OFFLINE=true` to child processes as well.

| Command | Result | Evidence |
|---|---|---|
| `/home/mole/oteryn-cache/codex-bin/cargo fmt --all --check` | PASS, exit 0 | [fmt-initial.log](fmt-initial.log) |
| `/home/mole/oteryn-cache/codex-bin/cargo clippy --offline --locked -p oteryn-client -p oteryn-client-runtime -p oteryn-client-ui -p oteryn-gameplay-dispatch -p oteryn-protocol-oteryn -p oteryn-session --all-targets --all-features --quiet -- -D warnings` | PASS, exit 0 | [clippy-initial.log](clippy-initial.log) |
| `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-client -p oteryn-client-runtime -p oteryn-client-ui -p oteryn-protocol-oteryn -p oteryn-session --no-fail-fast` | PASS, exit 0; 1,527 tests | [unit-tests-initial.log](unit-tests-initial.log) |
| `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-gameplay-dispatch --all-features --lib proficiency_browse_postgres_pinned_catalogue_live_xp_and_session_fence -- --nocapture --test-threads=1` | PASS, exit 0; 1 tests; 365.86s wall time | [pg-browse.log](pg-browse.log) |
| `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-gameplay-dispatch --all-features --lib proficiency_select_perk_wire_refusals_replay_and_durable_selection -- --nocapture --test-threads=1` | PASS, exit 0; 1 tests; 42.72s wall time | [pg-selection.log](pg-selection.log) |
| `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-gameplay-dispatch --all-features --lib weapon_proficiency::tests` | PASS, exit 0; 6 tests; 0.95s wall time | [dispatch-proficiency-unit.log](dispatch-proficiency-unit.log) |
| `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-gameplay-dispatch --all-features --lib proficiency_kill_tests` | PASS, exit 0; 2 tests; 0.67s wall time | [dispatch-proficiency-kill-unit.log](dispatch-proficiency-kill-unit.log) |
| `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-gameplay-dispatch --all-features --lib creature_push_real_dispatch_moves_and_refuses_on_postgres -- --nocapture --test-threads=1` | PASS, exit 0; 1 tests; 82.06s wall time | [pg-creature-push.log](pg-creature-push.log) |
| `/home/mole/oteryn-cache/codex-bin/cargo test --offline --locked -p oteryn-architecture-check --no-fail-fast` | PASS, exit 0; 46 tests; 9.04s wall time | [architecture-unit.log](architecture-unit.log) |
| `/home/mole/oteryn-cache/codex-bin/cargo run --offline --locked -p oteryn-architecture-check -- workspace .` | PASS, exit 0; 3.36s wall time | [architecture-workspace.log](architecture-workspace.log) |
| `/home/mole/oteryn-cache/codex-bin/cargo run --offline --locked -p oteryn-architecture-check -- client-structure .` | PASS, exit 0; 1.17s wall time | [architecture-client-structure.log](architecture-client-structure.log) |

No failed or zero-match test command was treated as qualification. No retries were needed. The five-package test command ran every selected package's own unit/integration/doc-test targets. Dispatch was restricted to the listed mechanic filters; neither full dispatch/PG suites nor nextest/workspace tests were run.

| Package/check | Passed | Failed | Ignored |
|---|---:|---:|---:|
| `oteryn-client` (library, launcher and integration targets) | 338 | 0 | 0 |
| `oteryn-client-runtime` | 33 | 0 | 0 |
| `oteryn-client-ui` | 278 | 0 | 0 |
| `oteryn-protocol-oteryn` | 706 | 0 | 0 |
| `oteryn-session` | 172 | 0 | 0 |
| Filtered dispatch unit tests | 8 | 0 | 0 |
| Filtered dispatch PostgreSQL cases | 3 | 0 | 0 |
| `oteryn-architecture-check` (32 units + 14 integrations) | 46 | 0 | 0 |
| **Total** | **1,584** | **0** | **0** |

Clippy emitted three existing vendored-dependency warnings (Tokio deprecated `fetch_update` and missing documentation; SQLx deprecated `fetch_update`). No selected-package warning/error was reported, and the command with `-D warnings` exited successfully. Vendor files were left unchanged.

The architecture workspace and client-structure commands both ended with `workspace-boundaries: PASS`. The latter reports `0082-creature-push` and `0195-weapon-proficiency` as supported. Architecture integration tests `per_mechanic_unsupported_baseline_files_count_once`, `duplicate_unknown_misnamed_missing_and_unsupported_growth_fail`, and `structure_command_checks_real_tree_and_reports_every_mechanic` passed, validating both unsupported-baseline removals without weakening the guard.

### Proof PostgreSQL actually executed

The absolute wrapper supplied the lane's database route; each harness created its own test database. No URL or credentials were read or printed. None of the three logs contains the unconfigured-URL early-return marker.

- New browse case: **1 passed**, 2,512 filtered out, test duration **44.40s**. It printed `POSTGRES_CASE_COMPLETED proficiency_browse: 665 weapons, 44.308277248s`. The 365.86s command wall time includes the initial 5m19s dispatch build. The scenario checks all pages, empty-track zero XP, live XP 12,345, invalid offset, wrong session, wire decoding and malformed wire requests through `serve_admitted`.
- Existing selection case: **1 passed**, 2,512 filtered out, **41.29s** test duration. It printed `proficiency_select_perk: wire cases executed (in_zone=false)` and reached command ID 9/server sequence 9 through the real connection. This also qualifies the helper whose visibility changed in the delivery.
- Existing push case: **1 passed**, 2,512 filtered out, **81.41s** test duration. Its two connection runs emitted serve-end records at command ID 2/sequence 1 and command ID 14/sequence 16, proving pushable and unpushable scenario execution.

## Review audit

The complete 1,497-line diff was read, then reread in four consecutive ranges against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. `reviewed.diff` retains that exact input. All 37 changed paths were covered.

- Persistence/SQL/migration classes: no production SQL, schema, role, receipt or value-moving writer added. SQL additions only seed the PG scenario. Existing session/durable owners remain authoritative.
- Transport/session classes: existing connection capability gating runs before mechanic dispatch; browse re-proves owner fence, exact actor/session and content pin. No network/DB await occurs while the runtime and proficiency-state locks are held. Responses use the specified status and bounded codec. Malformed/trailing page bytes are rejected; failed replies preserve the prior page.
- Bounds: at most 20 rows, 128 UTF-8 name bytes, 7 selected-perk bytes and level 9. The declared maximum response is exactly `9 + 20 * (4 + 1 + 128 + 4 + 1 + 1 + 7) = 2929` bytes. The local registry/mechanic reservation scan found one command-641 owner and one ordinal-638 fragment.
- Wiring: the mechanic fragment generates the transport/authority/composed routing; the new PG file is auto-mounted in the existing dispatch library target. No new unregistered PG target was introduced. The browse PG test exercised the generated real route.
- New public APIs have production callers: `creature_push::request` from world pointer input; `action_clicked` from Win32 command dispatch; `weapon_keys` from browse execution; `ProficiencyPage::encode` from the server route and `decode` from client result application; the new authority method from the generated mechanic route. The tests listed above cover the shared logic and server dispatch. The test-only connection helper is reused by both PG cases.
- Workspace symbol sweep: `rg` over **apps/, crates/, tools/** inspected all `ProficiencyRow`, `ProficiencyPage`, `WeaponProficiencyPanel` construction/reference sites, the new authority method and public function callers. The UI-only `Intent::{Browse,NextPage,PreviousPage}` variants are handled by the shared request adapter and native submit mappings; existing struct construction uses defaults for added panel fields. No unhandled constructor/match/trait implementation site was found. Evidence: `symbol-audit.log` and `proficiency-references.txt`.
- Hygiene/compatibility: all six changed Rust packages passed all-target/all-feature Clippy; rustfmt passed; protocol golden/capability tests passed. No wire code was removed or renumbered, no test deleted/weakened, no pin hand-edited and no protected workflow/repository-policy file changed.

**Findings: no actionable correctness defect in the reviewed delta. No repairs or deferred code findings.**

### Additional read-only audit commands

| Command/inspection | Result |
|---|---|
| `pwd`; `git status --short`; `git branch --show-current`; `git rev-parse HEAD` | Correct worktree and delivery SHA; initially clean, detached checkout. |
| `git diff --stat b2cfa798 HEAD`; `git diff --numstat b2cfa798 HEAD`; `git diff --name-only b2cfa798 HEAD` | 37 files, +846/-85; complete reviewed file inventory below. |
| `git log --oneline b2cfa798..HEAD` | One raw CL7 delivery commit. |
| `git diff b2cfa798 HEAD` and path-scoped views of it | Full diff read; saved as `reviewed.diff`. |
| `cat /home/mole/oteryn-cache/CODEX-CHECKLIST.md`; repository checklist/required skills/policy reads | Checklist and routed instructions loaded; final complete diff checked against all applicable classes. |
| `sed -n '1,360p' .cl7v/reviewed.diff`; `sed -n '361,750p' ...`; `sed -n '751,1135p' ...`; `sed -n '1136,1510p' ...` | Full final reread, no gaps. |
| `rg --files` / `rg -n` / `sed -n` scoped to touched sources, test mounts, capability registry, task/row definitions and callers | Inspected real production paths and inherited row implementations. Exploratory missing-path probes returned 1/2 and were corrected to existing paths; these were not validation failures. |
| `rg -n 'WeaponProficiencyPanel|ProficiencyRow|ProficiencyPage|weapon_proficiency_browse|creature_push::request|action_clicked|weapon_keys' apps crates tools --glob '*.rs' --max-columns 200` | Whole-workspace symbol audit retained in `symbol-audit.log`. |
| `rg -n 'weapon_proficiency' apps crates tools --glob '*.rs' --files-with-matches` | 58 reference files, retained in `proficiency-references.txt`. |
| Inline `python3` analyses of registry JSON and recorded test output | Exact 2,929-byte response cap, unique local allocation, per-package/row pass counts and PG marker verified; results retained in JSON/report. |
| `df -h .`; `du -sh .cl7v`; `wc -l` and targeted `tail` of logs | Build/evidence inspection only; no cleanup or source mutation. |
| `git diff --check b2cfa798 HEAD`; `git diff --quiet`; `git status --short --branch` | PASS; no tracked local changes; only `.cl7v/` untracked. |

## Files changed by CL7-VERIFY

**No tracked delivery/source file changed.** Only this report and local evidence under `.cl7v/` were created. Exact artifact inventory:

- `.cl7v/REPORT.md`
- `.cl7v/architecture-client-structure.log`
- `.cl7v/architecture-unit.log`
- `.cl7v/architecture-workspace.log`
- `.cl7v/clippy-initial.log`
- `.cl7v/dispatch-proficiency-kill-unit.log`
- `.cl7v/dispatch-proficiency-unit.log`
- `.cl7v/fmt-initial.log`
- `.cl7v/initial-results.json`
- `.cl7v/pg-browse.log`
- `.cl7v/pg-creature-push.log`
- `.cl7v/pg-selection.log`
- `.cl7v/proficiency-references.txt`
- `.cl7v/results.json`
- `.cl7v/reviewed.diff`
- `.cl7v/row-tests.json`
- `.cl7v/symbol-audit.log`
- `.cl7v/unit-tests-initial.log`

### Delivery files reviewed (unchanged by this task)

- `apps/client/src/features/0082-creature-push/input.rs`
- `apps/client/src/features/0195-weapon-proficiency/native.rs`
- `apps/client/src/features/0195-weapon-proficiency/tests/play.rs`
- `crates/client-runtime/src/lib.rs`
- `crates/client-ui/src/features/0082-creature-push/fixtures/registrations.json`
- `crates/client-ui/src/features/0082-creature-push/mod.rs`
- `crates/client-ui/src/features/0082-creature-push/registrations/handlers.json`
- `crates/client-ui/src/features/0195-weapon-proficiency/fixtures/registrations.json`
- `crates/client-ui/src/features/0195-weapon-proficiency/protocol.rs`
- `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-378.json`
- `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-379.json`
- `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-380.json`
- `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-392.json`
- `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-393.json`
- `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-394.json`
- `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-395.json`
- `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-641.json`
- `crates/client-ui/src/features/0195-weapon-proficiency/state.rs`
- `crates/client-ui/src/features/0195-weapon-proficiency/view.rs`
- `crates/client-ui/src/lib.rs`
- `crates/gameplay/dispatch/mechanics/0638-weapon_proficiency_browse.mechanic`
- `crates/gameplay/dispatch/src/gameplay_transport/connection/routing/weapon_proficiency_browse.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/proficiency_browse.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/proficiency_select_perk.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/weapon_proficiency.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/weapon_proficiency_browse.rs`
- `crates/protocol-oteryn/capabilities/0195-weapon-proficiency-v1.json`
- `crates/protocol-oteryn/src/tests/capabilities_golden/0195-weapon-proficiency-v1.json`
- `crates/protocol-oteryn/src/weapon_proficiency.rs`
- `crates/session/src/capability_support.rs`
- `crates/session/src/session_tests_capabilities.rs`
- `crates/session/src/session_tests_chat.rs`
- `docs/contracts/protocol-oteryn/v1/registry/capabilities/0195-weapon-proficiency-v1.json`
- `docs/contracts/protocol-oteryn/v1/registry/command_types/0641-proficiency-browse.json`
- `docs/contracts/protocol-oteryn/v1/weapon-proficiency-v1.md`
- `tools/architecture-check/client-unsupported-baseline.d/0195-weapon-proficiency.json`
- `tools/architecture-check/client-unsupported-baseline.json`

## Unfinished / validation limits

- No requested local command is unfinished, no local failure remains, and no contention-only failure required a CI-only deferral.
- Windows target compilation and live Win32 window interaction were not run here. The Linux tests exercise shared native parsing, button selection, paging, display and command logic; source inspection covered the Win32 wiring. Windows/graphical qualification remains with its normal CI/manual lane.
- No claim of full gameplay parity, deployment, CI status or merge readiness. I18/P05/W06 tests cover existing client behavior; their full server mechanics were not requalified.
- Donor/wiki references were not independently re-researched. The browse contract retains its `wiki-pending` note; this audit verified the implementation against the checked-in protocol contract and existing authoritative owners.
- The packet's unused capability/offer reservations and detached checkout are recorded above; neither was changed.

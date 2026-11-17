# CLHUD client presentation reconciliation

Base main and assigned branch predecessor: `9668e0a04e1a631ebfc8752772102a1fd98ce8fa`.
Branch: `claude/happy-davinci-ijz3wy-clhud`.

Gap list: `MECHANICS-STATUS-2026-11-07.md` from audit branch
`claude/happy-davinci-ijz3wy-aud1107`, read at
`d531f725dc25c70c268c85c40ad14e63a7a9c7cd`.
The 2026-11-05 status review predates the implementations retained below.
This task changes C11's eligibility wording and closes native-flow test gaps;
the other rows retain their existing production decoders, native controls and
tests. It does not duplicate their contracts or allocate replacement routes.

| Row | Existing native regression / evidence |
| --- | --- |
| C10 | `rest_status_native_queue_refusal_replay_logout`; donor and server projection in `NX-7-server-client-status.md`. |
| C11 | `nx7_battle_instinct_reply_reaches_hud_beside_harmony`; panel correction and donor evidence in `CLHUD-C11-CONTRACT-NOTES.md`. |
| C18a | Native directional-spell coverage is documented in `CLHUD-C18a-CONTRACT-NOTES.md`. |
| C19b | `newhaven_starter_wire_data_reaches_native_equipment_backpack_and_quiver_panels`; Crystal `9f5a72c`, `data-global/lib/others/newhaven.lua:3-95,97-144`. |
| C20c | `death_protection_is_requested_at_login_and_after_respawn_and_shown_in_hud`; exact committed consumption and donor evidence in `NX-8-C20c.md`. |
| S13c | `party_member_effects_server_result_reaches_native_panel_and_removes`; negotiated durations and donor evidence in `NX-8-S13c.md`. |
| I04 | `srx1_native_writing_selected_item_utf8_save_and_write_once_refusal`; Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, `src/game/game.cpp:4957-5018`. |
| I10 | `srx1_native_furniture_catalogue_wrap_unwrap_and_stale_refusal`; retain the 11-07 audit's accepted full-vessel refusal (`SRX-1-items-2026-11-05.md`). |
| P04 | `full_task_board_reply_reaches_the_play_panel`; full weekly/delivery/shop controls retained, `NX-9-client-package-5-verification.md`. |
| P12c | `gap_character_tabs_decode_every_body_and_retain_confirmed_state`, `gap_character_tabs_render_and_reject_malformed_replies`; all character/death/PvP bodies retained. |
| P18 | `forge_native_flow_decodes_result_and_history`; bounded receipt and history decode retained, `NX-9-client-package-5-verification.md`. |
| W09 | `nx8_potion_durations_native_queue_refusal_replay_logout`; timer sources and donor evidence in `NX-8-W09.md`. |
| E12 | `hireling_management_crosses_the_real_session_loop`; native shortcut-to-identity/outfit coverage in `CLHUD-E12-CONTRACT-NOTES.md`. |
| P19 | `cl6_forge_marks_reach_native_name_glyphs_and_clear_with_visibility`, `nx8_current_dust_is_displayed_without_history`; donor and current-balance projection in `NX-8-P19.md`. |
| S12 | `staff_nx1` integration target, `pc02_player_report_scenario`, `analyser_push_reaches_closed_panel_and_end_clears_totals`; full signed GM controls and donor evidence in `NX-1.md`. |
| ADD-0066 | Native step/use refusal coverage is documented in `CLHUD-ADD-0066-CONTRACT-NOTES.md`. |

No numbers allocated: migrations 1120–1129 remain unused. Existing capabilities,
commands, mechanic ordinals, offer orders and registry entries are reused.
Number discovery against the exact base found next free capability 385,
command 670, mechanic ordinal 643 and offer order 640; the lane-offset candidates
405/690/663/660 are **not claimed**. No generated file is edited manually.

Validation is headless native execution on Linux; physical Windows/GPU interaction
requires a Windows runner. Focused PostgreSQL cases run with the supplied local
PostgreSQL 18 harness; the full PostgreSQL suite is outside this task's checks.

Fresh native verification: `cargo test --locked -p oteryn-client-ui -p
oteryn-client --quiet` passed every target (345 app library cases in 17.16 s;
269 UI library cases in 0.02 s; 652 total cases; command 36.03 s wall). This includes the
red-then-green C11 three-state regression. All existing tests remain enabled;
E12's entry-point assertions are strengthened without removing its outfit checks.

Focused retained PostgreSQL cases executed (not route-less returns): rest status
5.72 s; Battle Instinct 56.79 s; initial Newhaven kit 45.83 s; live kit inventory
publication 41.75 s; death protection 5.88 s; party member effects 48.72 s;
write-once production dispatch 47.89 s. Each filter ran one passing case.
The current dust projection case passed in 69.01 s and printed its executed
generation-fence/current-balance evidence marker.

Final changed-crate checks: `cargo check --locked -p oteryn-client-ui -p
oteryn-client` passed (1.02 s wall); all-target Clippy with `-D warnings` passed
(5.68 s wall); `cargo fmt --all --check` and `git diff --check` passed.
`oteryn-architecture-check workspace .` passed (3.80 s wall).
Repository pin tooling completed refresh (465.04 s wall including build/lock
wait) and check (275.54 s wall including build/lock wait), with unchanged digests.
The final diff changes only client code/tests and these evidence notes.
The optional broad dispatch test run was stopped after the revised gap list
removed the proposed I10 server change; no test source or assertion was weakened.
No full PostgreSQL suite was run.

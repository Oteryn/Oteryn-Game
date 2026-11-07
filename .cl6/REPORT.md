# CL6 + CL7 package validation

Base/current main verified: `b2cfa798e3def61002d39a6da8b9de5df5aa2fd8`.
CL6 HEAD: `465f6c9dbb64ab7b9ad244ce59ac5367c928f7a6`; incoming CL7: `28cfe8937510b659caea7ad4763e3bc60ddf0c9c`.

## Final numbers

| Slice | Capability | Command | Mechanic | Offer |
|---|---:|---:|---:|---:|
| CL6 mechanics status | 384 | 669 | 642 | 639 |
| CL7 proficiency browse | 195 (existing) | 641 | 638 | none added |

CL7 also enables existing creature-push capability 82 / command 98 and carried-hotkey capability 180 under existing owner 179 / command 348. No number clashes or renumbering. Capability 384 and existing command 384 are separate namespaces. The client advertises 100 sorted capabilities, within the server's selected capacity of 112.

## Repairs and generated files

- Resolved the diff3 conflict in `crates/session/src/session_tests_chat.rs` by retaining both sides' capability assertions: 82, 180, 195 and 384. No tests removed or weakened.
- Corrected existing command-fragment filenames 383/384 through `registry_fragments.split`; their names were not the canonical names emitted by the round-trip generator. IDs, JSON bytes and assembled registry bytes are identical. The registry round-trip test was red before this repair and green afterward.
- Restricted the new test-only mechanics-status snapshot accessor to test builds within the play module; all remaining new public production functions have callers.
- Regenerated client registration fixtures and capability goldens with the repository scripts; regenerated and checked World pins with the compiler. Build scripts regenerate dispatch mounts, capability tables and client registrations from fragments.

## Self-review

Reviewed the complete package diff against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and the repository checklist. Searched all constructor, match and implementation sites under `apps/`, `crates/` and `tools/` for added structs, fields, variants and trait methods (local `symbol-sites.log`, `proficiency-sites.log`, `caller-sites.log`). Checked bounded reads, semantic deadlines, capability negotiation, current-session/scope fencing, reply preservation, production callers and real-dispatch coverage. No remaining finding. Existing durable owners remain authoritative; there are no migrations, new durable writes, or changed wire IDs.

## Command results

Wall durations include queued-wrapper waiting and compilation. Every Cargo command uses the absolute queued wrapper with the repository toolchain and locked dependencies; dependency downloads were disabled with `CARGO_NET_OFFLINE=true`. Historical failures are retained below; their repaired reruns pass.

| Result | Wall duration | Command | Log |
|---|---:|---|---|
| PASS | 25.561s | `/home/mole/oteryn-cache/codex-bin/cargo fmt --all` | `fmt.log` |
| PASS | 0.071s | `python3 tools/contracts/registry_fragments.py` | `registry.log` |
| PASS | 0.133s | `python3 .cl6/audit_numbers.py` | `numbers.log` |
| PASS | 0.032s | `python3 tools/contracts/update_client_registration_golden.py 0082-creature-push 0195-weapon-proficiency 0384-mechanics-status` | `registration-goldens.log` |
| PASS | 0.027s | `python3 tools/contracts/update_capability_golden.py 0195-weapon-proficiency-v1.json` | `proficiency-golden.log` |
| PASS | 0.027s | `python3 tools/contracts/update_capability_golden.py 0384-mechanics-status-v1.json` | `status-golden.log` |
| PASS | 255.825s | `/home/mole/oteryn-cache/codex-bin/cargo check --locked --all-targets -p oteryn-client -p oteryn-client-ui -p oteryn-client-runtime -p oteryn-session -p oteryn-gameplay-dispatch -p oteryn-synthetic-client-harness -p oteryn-protocol-oteryn -p oteryn-gameplay-durability` | `check.log` |
| PASS | 0.880s | `python3 .cl6/audit_numbers.py` | `numbers-and-registry.log` |
| FAIL (repaired) | 0.565s | `python3 -m unittest discover -s tools/contracts/tests` | `registry-roundtrip-before.log` |
| PASS | 0.066s | `python3 .cl6/regenerate_registry_names.py` | `registry-filenames.log` |
| PASS | 0.557s | `python3 -m unittest discover -s tools/contracts/tests` | `registry-roundtrip.log` |
| PASS | 217.108s | `/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --all-targets --all-features -p oteryn-client -p oteryn-client-ui -p oteryn-client-runtime -p oteryn-session -p oteryn-gameplay-dispatch -p oteryn-synthetic-client-harness -p oteryn-protocol-oteryn -p oteryn-gameplay-durability -- -D warnings` | `clippy.log` |
| PASS | 35.572s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-client --lib -- --list` | `client-list.log` |
| PASS | 0.881s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-client --lib -- --exact --nocapture play::tests::feature_0006_world_spatial_entities::snapshot_builds_a_view_centred_on_the_own_position_with_overlay_markers play::tests::feature_0006_world_spatial_entities::generated_dispatch_keeps_spatial_event_order_and_ignores_undrawn_domains play::tests::feature_0006_world_spatial_entities::live_names_reach_scene_and_disappear_with_visibility play::tests::feature_0006_world_spatial_entities::own_identity_draws_once_between_step_and_appearance_publish play::tests::feature_0006_world_spatial_entities::live_presentations_keep_target_clock_and_light_through_rebuilds play::tests::feature_0006_world_spatial_entities::echo_portal_click_walks_and_reward_remains_in_native_container scene::live_entities::tests::cl6_forge_marks_reach_native_name_glyphs_and_clear_with_visibility scene::live_entities::tests::live_outfit_uses_server_look_and_empty_outfit_suppresses_pixels scene::live_entities::tests::other_actor_keeps_object_marker_and_local_player scene::live_entities::tests::recenter_reuses_loaded_assets_outfits_glyphs_and_packed_atlas scene::live_entities::tests::play_view_rebuild_retains_the_same_cached_image play::tests::feature_0019_world_light::clxa_character_light_reaches_native_scene_and_expires_on_update play::tests::feature_0019_world_light::clock_and_light_survive_scene_rebuilds play::tests::feature_0082_creature_push::native_drag_sends_push_once_without_attacking_or_walking play::tests::feature_0082_creature_push::native_drag_cancels_on_focus_loss_or_stale_target_and_decodes_refusal play::tests::feature_0082_creature_push::creature_pointer_preserves_cl1_ground_clicks_and_cl2_cursor_targeting play::tests::feature_0082_creature_push::death_discards_queued_cl3_controls_without_discarding_logout play::tests::feature_0179_hotkey_equip::hotkey_equip_panel_opens_and_sends play::tests::feature_0179_hotkey_equip::assigned_item_key_sends_the_definition_and_waits_for_the_server play::tests::feature_0179_hotkey_equip::native_hotkey_configuration_and_visible_refusal play::tests::feature_0195_weapon_proficiency::controls_encode_exact_request_and_reply_and_refusal_are_visible play::tests::feature_0195_weapon_proficiency::native_proficiency_actions_and_input_gates play::tests::feature_0195_weapon_proficiency::native_proficiency_rejects_invalid_fields_and_clears_perks play::tests::feature_0195_weapon_proficiency::native_proficiency_browse_decodes_experience_and_pages_the_complete_list play::tests::feature_0195_weapon_proficiency::proficiency_page_rejects_truncation_trailing_and_oversized_rows play::tests::feature_local_status_flows::cl6_mechanics_status_native_queue_refusal_replay_logout` | `client-tests.log` |
| PASS | 13.381s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-client-ui --lib -- --list` | `ui-list.log` |
| PASS | 0.323s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-client-ui --lib -- --exact --nocapture mechanics_status::tests::cl6_all_owner_projections_reach_native_text_and_refusals_retain_confirmation` | `ui-tests.log` |
| PASS | 13.371s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-protocol-oteryn --lib -- --list` | `protocol-list.log` |
| PASS | 0.310s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-protocol-oteryn --lib -- --exact --nocapture mechanics_status::tests::cl6_status_codec_bounds_and_all_fields mechanics_status::tests::cl6_status_largest_owner_bounds_fit_one_reply weapon_proficiency::tests::select_perk_request_is_six_bytes_with_a_clear_marker weapon_proficiency::tests::select_perk_reply_round_trips_and_bounds_its_levels weapon_proficiency::tests::modification_requests_carry_their_operation_in_the_command_type weapon_proficiency::tests::modification_requests_reject_malformed_payloads weapon_proficiency::tests::modification_replies_round_trip_within_sixteen_bytes` | `protocol-tests.log` |
| PASS | 16.234s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-session --lib -- --list` | `session-list.log` |
| PASS | 3.765s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-session --lib -- --exact --nocapture tests::the_advertised_set_carries_chat_and_stays_closed tests::derived_gated_routes_match_previous_table tests::appearance_session_tests::appearance_result_first_and_observation_first_converge_through_session_dispatch capability_support::creature_push_tests::creature_push_is_advertised_and_routed_with_its_prerequisites capability_support::weapon_proficiency_tests::proficiency_browse_is_advertised_and_routed_with_item_identity capability_support::hotkey_equip_tests::hotkey_equip_carried_is_advertised_with_its_command_owner` | `session-tests.log` |
| PASS | 7.428s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-synthetic-client-harness --bin oteryn-synthetic-client-harness -- --list` | `synthetic-list.log` |
| PASS | 0.526s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-synthetic-client-harness --bin oteryn-synthetic-client-harness -- --exact --nocapture native_map::native_live_map_snapshot_then_delta_changes_the_scene_and_item_selection native_map::native_live_appearance_snapshot_reaches_visible_scene` | `synthetic-tests.log` |
| PASS | 0.205s | `python3 .cl6/audit_numbers.py` | `final-numbers.log` |
| PASS | 397.388s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features -- --list` | `pg-list.log` |
| PASS | 130.530s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::cl6_status::cl6_status_postgres_real_dispatch_existing_owners_and_fences -- --exact --nocapture --test-threads=1` | `pg-cl6_status_postgres_real_dispatch_existing_owners_and_fences.log` |
| PASS | 238.949s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::party_effects::party_effects_real_dispatch_postgres_membership_and_shared_eligibility -- --exact --nocapture --test-threads=1` | `pg-party_effects_real_dispatch_postgres_membership_and_shared_eligibility.log` |
| PASS | 39.751s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::condition_potions::condition_potions_postgres_berserk_buff_expiry_receipt_and_refusals -- --exact --nocapture --test-threads=1` | `pg-condition_potions_postgres_berserk_buff_expiry_receipt_and_refusals.log` |
| PASS | 37.922s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::condition_potions::condition_potions_postgres_mastermind_buff_expiry_receipt_and_refusals -- --exact --nocapture --test-threads=1` | `pg-condition_potions_postgres_mastermind_buff_expiry_receipt_and_refusals.log` |
| PASS | 42.411s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::condition_potions::condition_potions_postgres_bullseye_buff_expiry_receipt_and_refusals -- --exact --nocapture --test-threads=1` | `pg-condition_potions_postgres_bullseye_buff_expiry_receipt_and_refusals.log` |
| PASS | 42.243s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::condition_potions::condition_potions_postgres_transcendence_buff_expiry_receipt_and_refusals -- --exact --nocapture --test-threads=1` | `pg-condition_potions_postgres_transcendence_buff_expiry_receipt_and_refusals.log` |
| PASS | 94.967s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::special_potions::magic_shield_potion_postgres_use_capacity_expiry_requirements_replay -- --exact --nocapture --test-threads=1` | `pg-magic_shield_potion_postgres_use_capacity_expiry_requirements_replay.log` |
| PASS | 31.380s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::special_potions::antidote_potion_postgres_use_cures_poison_flask_cooldown_replay -- --exact --nocapture --test-threads=1` | `pg-antidote_potion_postgres_use_cures_poison_flask_cooldown_replay.log` |
| PASS | 56.998s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::w22a_loot::w22b_wealth_duplex_ignores_wall_clock_time_spent_offline -- --exact --nocapture --test-threads=1` | `pg-w22b_wealth_duplex_ignores_wall_clock_time_spent_offline.log` |
| PASS | 56.370s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::w22a_loot::w22b_wealth_duplex_ends_after_one_online_hour -- --exact --nocapture --test-threads=1` | `pg-w22b_wealth_duplex_ends_after_one_online_hour.log` |
| PASS | 32.037s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::proficiency_browse::proficiency_browse_postgres_pinned_catalogue_live_xp_and_session_fence -- --exact --nocapture --test-threads=1` | `pg-proficiency_browse_postgres_pinned_catalogue_live_xp_and_session_fence.log` |
| PASS | 68.499s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::creature_push::creature_push_real_dispatch_moves_and_refuses_on_postgres -- --exact --nocapture --test-threads=1` | `pg-creature_push_real_dispatch_moves_and_refuses_on_postgres.log` |
| PASS | 13.013s | `/home/mole/oteryn-cache/codex-bin/cargo fmt --all --check` | `fmt-final.log` |
| PASS | 164.026s | `/home/mole/oteryn-cache/codex-bin/cargo check --locked --all-targets -p oteryn-client -p oteryn-client-ui -p oteryn-client-runtime -p oteryn-session -p oteryn-gameplay-dispatch -p oteryn-synthetic-client-harness -p oteryn-protocol-oteryn -p oteryn-gameplay-durability` | `check-final.log` |
| PASS | 161.924s | `/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --all-targets --all-features -p oteryn-client -p oteryn-client-ui -p oteryn-client-runtime -p oteryn-session -p oteryn-gameplay-dispatch -p oteryn-synthetic-client-harness -p oteryn-protocol-oteryn -p oteryn-gameplay-durability -- -D warnings` | `clippy-final.log` |
| PASS | 12.519s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-client --lib -- --exact --nocapture play::tests::feature_0006_world_spatial_entities::snapshot_builds_a_view_centred_on_the_own_position_with_overlay_markers play::tests::feature_0006_world_spatial_entities::generated_dispatch_keeps_spatial_event_order_and_ignores_undrawn_domains play::tests::feature_0006_world_spatial_entities::live_names_reach_scene_and_disappear_with_visibility play::tests::feature_0006_world_spatial_entities::own_identity_draws_once_between_step_and_appearance_publish play::tests::feature_0006_world_spatial_entities::live_presentations_keep_target_clock_and_light_through_rebuilds play::tests::feature_0006_world_spatial_entities::echo_portal_click_walks_and_reward_remains_in_native_container scene::live_entities::tests::cl6_forge_marks_reach_native_name_glyphs_and_clear_with_visibility scene::live_entities::tests::live_outfit_uses_server_look_and_empty_outfit_suppresses_pixels scene::live_entities::tests::other_actor_keeps_object_marker_and_local_player scene::live_entities::tests::recenter_reuses_loaded_assets_outfits_glyphs_and_packed_atlas scene::live_entities::tests::play_view_rebuild_retains_the_same_cached_image play::tests::feature_0019_world_light::clxa_character_light_reaches_native_scene_and_expires_on_update play::tests::feature_0019_world_light::clock_and_light_survive_scene_rebuilds play::tests::feature_0082_creature_push::native_drag_sends_push_once_without_attacking_or_walking play::tests::feature_0082_creature_push::native_drag_cancels_on_focus_loss_or_stale_target_and_decodes_refusal play::tests::feature_0082_creature_push::creature_pointer_preserves_cl1_ground_clicks_and_cl2_cursor_targeting play::tests::feature_0082_creature_push::death_discards_queued_cl3_controls_without_discarding_logout play::tests::feature_0179_hotkey_equip::hotkey_equip_panel_opens_and_sends play::tests::feature_0179_hotkey_equip::assigned_item_key_sends_the_definition_and_waits_for_the_server play::tests::feature_0179_hotkey_equip::native_hotkey_configuration_and_visible_refusal play::tests::feature_0195_weapon_proficiency::controls_encode_exact_request_and_reply_and_refusal_are_visible play::tests::feature_0195_weapon_proficiency::native_proficiency_actions_and_input_gates play::tests::feature_0195_weapon_proficiency::native_proficiency_rejects_invalid_fields_and_clears_perks play::tests::feature_0195_weapon_proficiency::native_proficiency_browse_decodes_experience_and_pages_the_complete_list play::tests::feature_0195_weapon_proficiency::proficiency_page_rejects_truncation_trailing_and_oversized_rows play::tests::feature_local_status_flows::cl6_mechanics_status_native_queue_refusal_replay_logout` | `client-tests.log` |
| PASS | 0.346s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-protocol-oteryn --lib -- --exact --nocapture registered_capabilities::tests::caps_split_1_golden_preserves_every_current_capability_fact registered_capabilities::tests::capability_tables_are_all_fragment_facts capability_tables::tests::the_gated_commands_and_domains_are_the_registry_ones tests::module_registration_block_is_sorted_and_complete contract_registries::tests::protocol_registry_is_its_base_and_fragments` | `protocol-generated.log` |
| PASS | 0.289s | `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-session --lib tests::the_advertised_set_is_closed_under_the_registry_requires -- --exact --nocapture` | `session-closure.log` |
| PASS | 3.239s | `/home/mole/oteryn-cache/codex-bin/cargo run --locked -p oteryn-architecture-check -- workspace .` | `architecture.log` |
| PASS | 147.692s | `/home/mole/oteryn-cache/codex-bin/cargo run --locked -p oteryn-world-bundle-compiler -- pin-refresh .` | `pin-refresh.log` |
| PASS | 129.325s | `/home/mole/oteryn-cache/codex-bin/cargo run --locked -p oteryn-world-bundle-compiler -- pin-check . .cl6/pin-check` | `pin-check.log` |
| PASS | 0.131s | `git diff --check origin/main` | `diff-check.log` |

The first test-plan discovery attempt found duplicate short test names in different client modules. The runner was corrected to use fully qualified exact names; the actual Cargo test runs below execute the complete selected set.

## Exact test filters and execution

### client-tests: PASS

`test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 297 filtered out; finished in 0.30s`

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-client --lib -- --exact --nocapture play::tests::feature_0006_world_spatial_entities::snapshot_builds_a_view_centred_on_the_own_position_with_overlay_markers play::tests::feature_0006_world_spatial_entities::generated_dispatch_keeps_spatial_event_order_and_ignores_undrawn_domains play::tests::feature_0006_world_spatial_entities::live_names_reach_scene_and_disappear_with_visibility play::tests::feature_0006_world_spatial_entities::own_identity_draws_once_between_step_and_appearance_publish play::tests::feature_0006_world_spatial_entities::live_presentations_keep_target_clock_and_light_through_rebuilds play::tests::feature_0006_world_spatial_entities::echo_portal_click_walks_and_reward_remains_in_native_container scene::live_entities::tests::cl6_forge_marks_reach_native_name_glyphs_and_clear_with_visibility scene::live_entities::tests::live_outfit_uses_server_look_and_empty_outfit_suppresses_pixels scene::live_entities::tests::other_actor_keeps_object_marker_and_local_player scene::live_entities::tests::recenter_reuses_loaded_assets_outfits_glyphs_and_packed_atlas scene::live_entities::tests::play_view_rebuild_retains_the_same_cached_image play::tests::feature_0019_world_light::clxa_character_light_reaches_native_scene_and_expires_on_update play::tests::feature_0019_world_light::clock_and_light_survive_scene_rebuilds play::tests::feature_0082_creature_push::native_drag_sends_push_once_without_attacking_or_walking play::tests::feature_0082_creature_push::native_drag_cancels_on_focus_loss_or_stale_target_and_decodes_refusal play::tests::feature_0082_creature_push::creature_pointer_preserves_cl1_ground_clicks_and_cl2_cursor_targeting play::tests::feature_0082_creature_push::death_discards_queued_cl3_controls_without_discarding_logout play::tests::feature_0179_hotkey_equip::hotkey_equip_panel_opens_and_sends play::tests::feature_0179_hotkey_equip::assigned_item_key_sends_the_definition_and_waits_for_the_server play::tests::feature_0179_hotkey_equip::native_hotkey_configuration_and_visible_refusal play::tests::feature_0195_weapon_proficiency::controls_encode_exact_request_and_reply_and_refusal_are_visible play::tests::feature_0195_weapon_proficiency::native_proficiency_actions_and_input_gates play::tests::feature_0195_weapon_proficiency::native_proficiency_rejects_invalid_fields_and_clears_perks play::tests::feature_0195_weapon_proficiency::native_proficiency_browse_decodes_experience_and_pages_the_complete_list play::tests::feature_0195_weapon_proficiency::proficiency_page_rejects_truncation_trailing_and_oversized_rows play::tests::feature_local_status_flows::cl6_mechanics_status_native_queue_refusal_replay_logout
```

### ui-tests: PASS

`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 259 filtered out; finished in 0.00s`

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-client-ui --lib -- --exact --nocapture mechanics_status::tests::cl6_all_owner_projections_reach_native_text_and_refusals_retain_confirmation
```

### protocol-tests: PASS

`test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 701 filtered out; finished in 0.00s`

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-protocol-oteryn --lib -- --exact --nocapture mechanics_status::tests::cl6_status_codec_bounds_and_all_fields mechanics_status::tests::cl6_status_largest_owner_bounds_fit_one_reply weapon_proficiency::tests::select_perk_request_is_six_bytes_with_a_clear_marker weapon_proficiency::tests::select_perk_reply_round_trips_and_bounds_its_levels weapon_proficiency::tests::modification_requests_carry_their_operation_in_the_command_type weapon_proficiency::tests::modification_requests_reject_malformed_payloads weapon_proficiency::tests::modification_replies_round_trip_within_sixteen_bytes
```

### session-tests: PASS

`test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 166 filtered out; finished in 0.11s`

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-session --lib -- --exact --nocapture tests::the_advertised_set_carries_chat_and_stays_closed tests::derived_gated_routes_match_previous_table tests::appearance_session_tests::appearance_result_first_and_observation_first_converge_through_session_dispatch capability_support::creature_push_tests::creature_push_is_advertised_and_routed_with_its_prerequisites capability_support::weapon_proficiency_tests::proficiency_browse_is_advertised_and_routed_with_item_identity capability_support::hotkey_equip_tests::hotkey_equip_carried_is_advertised_with_its_command_owner
```

### synthetic-tests: PASS

`test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 48 filtered out; finished in 0.01s`

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-synthetic-client-harness --bin oteryn-synthetic-client-harness -- --exact --nocapture native_map::native_live_map_snapshot_then_delta_changes_the_scene_and_item_selection native_map::native_live_appearance_snapshot_reaches_visible_scene
```

### protocol-generated: PASS

`test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 703 filtered out; finished in 0.03s`

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-protocol-oteryn --lib -- --exact --nocapture registered_capabilities::tests::caps_split_1_golden_preserves_every_current_capability_fact registered_capabilities::tests::capability_tables_are_all_fragment_facts capability_tables::tests::the_gated_commands_and_domains_are_the_registry_ones tests::module_registration_block_is_sorted_and_complete contract_registries::tests::protocol_registry_is_its_base_and_fragments
```

### session-closure: PASS

`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 171 filtered out; finished in 0.00s`

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-session --lib tests::the_advertised_set_is_closed_under_the_registry_requires -- --exact --nocapture
```

### PostgreSQL: twelve exact cases, PASS

No unfiltered PostgreSQL suite was run. Each case uses `--exact --nocapture --test-threads=1` through the absolute wrapper. No absent-configuration early return counts as qualification.

| Case | Test duration | Execution evidence |
|---|---:|---|
| `cl6_status_postgres_real_dispatch_existing_owners_and_fences` | 66.98s | CL6_STATUS POSTGRES_CASE_COMPLETED pvp=false elapsed=33.965846777s; CL6_STATUS POSTGRES_CASE_COMPLETED pvp=true elapsed=32.869432502s |
| `party_effects_real_dispatch_postgres_membership_and_shared_eligibility` | 238.24s | CL6_PARTY POSTGRES_ASSERTION_EXECUTED spell=protect_party affected=true; CL6_PARTY POSTGRES_ASSERTION_EXECUTED spell=protect_party affected=false; CL6_PARTY POSTGRES_ASSERTION_EXECUTED spell=enlighten_party affected=true; CL6_PARTY POSTGRES_ASSERTION_EXECUTED spell=enlighten_party affected=false; POSTGRES_EXECUTED case=party_buffs elapsed=61.994s; CL6_PARTY POSTGRES_ASSERTION_EXECUTED spell=train_party affected=true; CL6_PARTY POSTGRES_ASSERTION_EXECUTED spell=train_party affected=false; POSTGRES_EXECUTED case=train_party elapsed=83.903s; CL6_PARTY POSTGRES_ASSERTION_EXECUTED spell=enchant_party affected=true; CL6_PARTY POSTGRES_ASSERTION_EXECUTED spell=enchant_party affected=false; POSTGRES_EXECUTED case=enchant_party elapsed=42.384s; CL6_PARTY POSTGRES_ASSERTION_EXECUTED spell=heal_party affected=true; CL6_PARTY POSTGRES_ASSERTION_EXECUTED spell=heal_party affected=false; POSTGRES_EXECUTED case=heal_party elapsed=41.692s; POSTGRES_EXECUTED case=party_effects elapsed=238.244s |
| `condition_potions_postgres_berserk_buff_expiry_receipt_and_refusals` | 39.15s | CL6_TIMER POSTGRES_ASSERTION_EXECUTED key=oteryn:item.tibia.i7439 active=true; Q6 POSTGRES_CASE_COMPLETED q6_berserk |
| `condition_potions_postgres_mastermind_buff_expiry_receipt_and_refusals` | 37.38s | CL6_TIMER POSTGRES_ASSERTION_EXECUTED key=oteryn:item.tibia.i7440 active=true; Q6 POSTGRES_CASE_COMPLETED q6_mastermind |
| `condition_potions_postgres_bullseye_buff_expiry_receipt_and_refusals` | 41.83s | CL6_TIMER POSTGRES_ASSERTION_EXECUTED key=oteryn:item.tibia.i7443 active=true; Q6 POSTGRES_CASE_COMPLETED q6_bullseye |
| `condition_potions_postgres_transcendence_buff_expiry_receipt_and_refusals` | 40.82s | CL6_TIMER POSTGRES_ASSERTION_EXECUTED key=oteryn:item.tibia.i49271 active=true; Q6 POSTGRES_CASE_COMPLETED q6_transcendence |
| `magic_shield_potion_postgres_use_capacity_expiry_requirements_replay` | 94.37s | test gameplay_transport::monk_save::postgres_tests::special_potions::magic_shield_potion_postgres_use_capacity_expiry_requirements_replay ... PG_MARKER i14-special-potion-start 35563; CL6_TIMER POSTGRES_ASSERTION_EXECUTED key=oteryn:item.tibia.i35563 active=true; CL6_TIMER POSTGRES_ASSERTION_EXECUTED key=oteryn:item.tibia.i35563 active=false; PG_MARKER i14-special-potion-effect-burn-flask-replay-cooldown 35563; PG_MARKER i14-special-potion-start 35563; PG_MARKER i14-magic-shield-requirement-refusal druid 13; PG_MARKER i14-magic-shield-requirement-refusal knight 50 |
| `antidote_potion_postgres_use_cures_poison_flask_cooldown_replay` | 30.93s | test gameplay_transport::monk_save::postgres_tests::special_potions::antidote_potion_postgres_use_cures_poison_flask_cooldown_replay ... PG_MARKER i14-special-potion-start 7644; CL6_TIMER POSTGRES_ASSERTION_EXECUTED key=oteryn:item.tibia.i7644 active=false; PG_MARKER i14-special-potion-effect-burn-flask-replay-cooldown 7644 |
| `w22b_wealth_duplex_ignores_wall_clock_time_spent_offline` | 56.41s | test gameplay_transport::monk_save::postgres_tests::w22a_loot::w22b_wealth_duplex_ignores_wall_clock_time_spent_offline ... CL6_TIMER POSTGRES_ASSERTION_EXECUTED key=wealth_duplex active=true; CL6_TIMER POSTGRES_ASSERTION_EXECUTED key=wealth_duplex active=true |
| `w22b_wealth_duplex_ends_after_one_online_hour` | 55.89s | test gameplay_transport::monk_save::postgres_tests::w22a_loot::w22b_wealth_duplex_ends_after_one_online_hour ... CL6_TIMER POSTGRES_ASSERTION_EXECUTED key=wealth_duplex active=true; CL6_TIMER POSTGRES_ASSERTION_EXECUTED key=wealth_duplex active=false |
| `proficiency_browse_postgres_pinned_catalogue_live_xp_and_session_fence` | 31.55s | POSTGRES_CASE_COMPLETED proficiency_browse: 665 weapons, 31.452711088s |
| `creature_push_real_dispatch_moves_and_refuses_on_postgres` | 67.97s | Real-dispatch test completed with non-zero runtime; this existing test emits no marker. |

Exact commands:

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::cl6_status::cl6_status_postgres_real_dispatch_existing_owners_and_fences -- --exact --nocapture --test-threads=1
```

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::party_effects::party_effects_real_dispatch_postgres_membership_and_shared_eligibility -- --exact --nocapture --test-threads=1
```

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::condition_potions::condition_potions_postgres_berserk_buff_expiry_receipt_and_refusals -- --exact --nocapture --test-threads=1
```

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::condition_potions::condition_potions_postgres_mastermind_buff_expiry_receipt_and_refusals -- --exact --nocapture --test-threads=1
```

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::condition_potions::condition_potions_postgres_bullseye_buff_expiry_receipt_and_refusals -- --exact --nocapture --test-threads=1
```

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::condition_potions::condition_potions_postgres_transcendence_buff_expiry_receipt_and_refusals -- --exact --nocapture --test-threads=1
```

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::special_potions::magic_shield_potion_postgres_use_capacity_expiry_requirements_replay -- --exact --nocapture --test-threads=1
```

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::special_potions::antidote_potion_postgres_use_cures_poison_flask_cooldown_replay -- --exact --nocapture --test-threads=1
```

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::w22a_loot::w22b_wealth_duplex_ignores_wall_clock_time_spent_offline -- --exact --nocapture --test-threads=1
```

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::w22a_loot::w22b_wealth_duplex_ends_after_one_online_hour -- --exact --nocapture --test-threads=1
```

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::proficiency_browse::proficiency_browse_postgres_pinned_catalogue_live_xp_and_session_fence -- --exact --nocapture --test-threads=1
```

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --all-features gameplay_transport::monk_save::postgres_tests::creature_push::creature_push_real_dispatch_moves_and_refuses_on_postgres -- --exact --nocapture --test-threads=1
```

## Changed files

Complete package relative to current main (including incoming merge files):

- `M` `apps/client/src/features/0006-world-spatial-entities/scene.rs`
- `M` `apps/client/src/features/0006-world-spatial-entities/tests/play.rs`
- `M` `apps/client/src/features/0006-world-spatial-entities/tests/scene.rs`
- `M` `apps/client/src/features/0019-world-light/tests/play.rs`
- `M` `apps/client/src/features/0082-creature-push/input.rs`
- `M` `apps/client/src/features/0195-weapon-proficiency/native.rs`
- `M` `apps/client/src/features/0195-weapon-proficiency/tests/play.rs`
- `A` `apps/client/src/features/0384-mechanics-status/play.json`
- `A` `apps/client/src/features/0384-mechanics-status/protocol.rs`
- `M` `apps/client/src/features/local-status-flows/tests/play.rs`
- `M` `apps/client/src/play/mailbox.rs`
- `M` `apps/client/src/play/session_loop.rs`
- `M` `apps/client/src/play/status_window.rs`
- `M` `apps/client/src/windows_shell/input.rs`
- `M` `crates/client-runtime/src/lib.rs`
- `M` `crates/client-ui/src/features/0082-creature-push/fixtures/registrations.json`
- `A` `crates/client-ui/src/features/0082-creature-push/mod.rs`
- `M` `crates/client-ui/src/features/0082-creature-push/registrations/handlers.json`
- `M` `crates/client-ui/src/features/0195-weapon-proficiency/fixtures/registrations.json`
- `M` `crates/client-ui/src/features/0195-weapon-proficiency/protocol.rs`
- `M` `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-378.json`
- `M` `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-379.json`
- `M` `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-380.json`
- `M` `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-392.json`
- `M` `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-393.json`
- `M` `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-394.json`
- `M` `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-395.json`
- `A` `crates/client-ui/src/features/0195-weapon-proficiency/registrations/handler-641.json`
- `M` `crates/client-ui/src/features/0195-weapon-proficiency/state.rs`
- `M` `crates/client-ui/src/features/0195-weapon-proficiency/view.rs`
- `M` `crates/client-ui/src/features/0276-condition-status/mod.rs`
- `M` `crates/client-ui/src/features/0276-condition-status/view.rs`
- `A` `crates/client-ui/src/features/0384-mechanics-status/fixtures/registrations.json`
- `A` `crates/client-ui/src/features/0384-mechanics-status/input.rs`
- `A` `crates/client-ui/src/features/0384-mechanics-status/mod.rs`
- `A` `crates/client-ui/src/features/0384-mechanics-status/protocol.rs`
- `A` `crates/client-ui/src/features/0384-mechanics-status/registrations/handlers.json`
- `A` `crates/client-ui/src/features/0384-mechanics-status/registrations/open.json`
- `A` `crates/client-ui/src/features/0384-mechanics-status/registrations/panel.json`
- `A` `crates/client-ui/src/features/0384-mechanics-status/state.rs`
- `A` `crates/client-ui/src/features/0384-mechanics-status/view.rs`
- `M` `crates/client-ui/src/lib.rs`
- `A` `crates/gameplay/dispatch/mechanics/0638-weapon_proficiency_browse.mechanic`
- `A` `crates/gameplay/dispatch/mechanics/0642-mechanics_status.mechanic`
- `A` `crates/gameplay/dispatch/src/gameplay_transport/connection/routing/mechanics_status.rs`
- `A` `crates/gameplay/dispatch/src/gameplay_transport/connection/routing/weapon_proficiency_browse.rs`
- `A` `crates/gameplay/dispatch/src/gameplay_transport/mechanics_status.rs`
- `A` `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/cl6_status.rs`
- `M` `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/condition_potions.rs`
- `M` `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/party_effects.rs`
- `A` `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/proficiency_browse.rs`
- `M` `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/proficiency_select_perk.rs`
- `M` `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/special_potions.rs`
- `M` `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/w22a_loot.rs`
- `M` `crates/gameplay/dispatch/src/gameplay_transport/weapon_proficiency.rs`
- `A` `crates/gameplay/dispatch/src/gameplay_transport/weapon_proficiency_browse.rs`
- `M` `crates/gameplay/durability/src/durability/core_modules.rs`
- `A` `crates/gameplay/durability/src/durability/mechanics_status.rs`
- `M` `crates/gameplay/durability/src/durability/world_party.rs`
- `M` `crates/protocol-oteryn/capabilities/0195-weapon-proficiency-v1.json`
- `A` `crates/protocol-oteryn/capabilities/0384-mechanics-status-v1.json`
- `M` `crates/protocol-oteryn/src/lib.rs`
- `A` `crates/protocol-oteryn/src/mechanics_status.rs`
- `M` `crates/protocol-oteryn/src/tests/capabilities_golden/0195-weapon-proficiency-v1.json`
- `A` `crates/protocol-oteryn/src/tests/capabilities_golden/0384-mechanics-status-v1.json`
- `M` `crates/protocol-oteryn/src/weapon_proficiency.rs`
- `M` `crates/session/src/appearance_state.rs`
- `M` `crates/session/src/capability_support.rs`
- `M` `crates/session/src/lib.rs`
- `M` `crates/session/src/session_tests_capabilities.rs`
- `M` `crates/session/src/session_tests_chat.rs`
- `M` `crates/session/src/tests/appearance_session_tests.rs`
- `A` `docs/contracts/protocol-oteryn/v1/mechanics-status-v1.md`
- `M` `docs/contracts/protocol-oteryn/v1/registry/capabilities/0195-weapon-proficiency-v1.json`
- `A` `docs/contracts/protocol-oteryn/v1/registry/capabilities/0384-mechanics-status-v1.json`
- `D` `docs/contracts/protocol-oteryn/v1/registry/command_types/0383-cyclopedia-bestiary-tracker.json`
- `D` `docs/contracts/protocol-oteryn/v1/registry/command_types/0384-cyclopedia-bosstiary-tracker.json`
- `A` `docs/contracts/protocol-oteryn/v1/registry/command_types/0641-proficiency-browse.json`
- `A` `docs/contracts/protocol-oteryn/v1/registry/command_types/0669-mechanics-status-query.json`
- `M` `docs/contracts/protocol-oteryn/v1/weapon-proficiency-v1.md`
- `D` `tools/architecture-check/client-unsupported-baseline.d/0195-weapon-proficiency.json`
- `M` `tools/architecture-check/client-unsupported-baseline.json`
- `M` `tools/synthetic-client-harness/src/native_map.rs`
- `A` `docs/contracts/protocol-oteryn/v1/registry/command_types/0383-bestiary-tracker.json`
- `A` `docs/contracts/protocol-oteryn/v1/registry/command_types/0384-bosstiary-tracker.json`
- `A` `.cl6/REPORT.md`
- `A` `.cl6/.gitignore` (keeps local logs, helpers and compiler output out of accidental staging)

The deleted `0383-cyclopedia-bestiary-tracker.json` and `0384-cyclopedia-bosstiary-tracker.json` files are byte-identical renames to `0383-bestiary-tracker.json` and `0384-bosstiary-tracker.json`.

## Unfinished

None for implementation or requested local qualification. The index intentionally remains unmerged for `crates/session/src/session_tests_chat.rs` until the owner stages the resolution. No index writes, commits, pushes, resets, checkouts, stashes, credential changes, Docker operations or dependency downloads were performed. Publication and full CI are outside this local task.

ALL PASS

# OTV2-20261103-event-items — event items and toys (W19)

```yaml
task_id: OTV2-20261103-event-items
title: "Event items and toys: bounded W19 ordinary USE delivery"
type: mechanic
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/event-items-6
pr: 2855
base_sha: 128ebc4f6aebf264d01c54003d269fa798e9524c
head_sha: 091f7940131129e930d4ce31bc341ac59b64db53
final_head_sha: null
final_head_frozen_at: null
owner: event-items-task-lead
created_at: 2026-11-03
updated_at: 2026-11-03
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0880_event_item_actions.sql
  - apps/game-server/migrations/0881_event_container_craft.sql
  - apps/game-server/migrations/0882_item_cosmetics.sql
  - apps/game-server/migrations/0883_event_item_deadlines.sql
  - apps/game-server/migrations/0884_anniversary_house_balloons.sql
  - crates/gameplay/dispatch/src/gameplay_transport/actor_spell.rs
  - crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs
  - crates/gameplay/dispatch/src/gameplay_transport/chat_intent/item_actions.rs
  - crates/gameplay/dispatch/src/gameplay_transport/composed_admission/grace_expiry.rs
  - crates/gameplay/dispatch/src/gameplay_transport/composed_admission/release.rs
  - crates/gameplay/dispatch/src/gameplay_transport/composed_admission/training.rs
  - crates/gameplay/dispatch/src/gameplay_transport/composed_admission/vitals.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/authority.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/authority/composed.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/routing/use_item.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/house_hireling/map.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/house_hireling/mod.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/house_hireling/snapshot.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/house_transfer.rs
  - crates/gameplay/dispatch/src/gameplay_transport/container_view/browse_projection.rs
  - crates/gameplay/dispatch/src/gameplay_transport/control_loss_reconciliation.rs
  - crates/gameplay/dispatch/src/gameplay_transport/cosmetic_item_use.rs
  - crates/gameplay/dispatch/src/gameplay_transport/equipment_move.rs
  - crates/gameplay/dispatch/src/gameplay_transport/house_item_use.rs
  - crates/gameplay/dispatch/src/gameplay_transport/item_craft.rs
  - crates/gameplay/dispatch/src/gameplay_transport/item_craft/harvest.rs
  - crates/gameplay/dispatch/src/gameplay_transport/item_craft/planting.rs
  - crates/gameplay/dispatch/src/gameplay_transport/item_craft/terrain.rs
  - crates/gameplay/dispatch/src/gameplay_transport/loop_move.rs
  - crates/gameplay/dispatch/src/gameplay_transport/loop_move/house_container.rs
  - crates/gameplay/dispatch/src/gameplay_transport/loop_move/seed_drop.rs
  - crates/gameplay/dispatch/src/gameplay_transport/mining/mod.rs
  - crates/gameplay/dispatch/src/gameplay_transport/mod.rs
  - crates/gameplay/dispatch/src/gameplay_transport/novelty_ground_use.rs
  - crates/gameplay/dispatch/src/gameplay_transport/novelty_item_owner.rs
  - crates/gameplay/dispatch/src/gameplay_transport/novelty_item_pending.rs
  - crates/gameplay/dispatch/src/gameplay_transport/novelty_item_use.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/event_containers.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/event_containers/ground.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/event_containers/wire.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/event_object_uncertainty.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/house_balloon.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/house_balloon/scenario.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/house_balloon/wire.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/house_wrap.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics/packet.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics/positive.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics/race.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_seed_planting.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/map_harvest.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/map_seed_planting.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/novelty_items.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/novelty_items/pending.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/novelty_items/scenario.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/rust_lottery.rs
  - crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs
  - crates/gameplay/dispatch/src/gameplay_transport/reward_bag_use.rs
  - crates/gameplay/dispatch/src/gameplay_transport/seam_rooms.rs
  - crates/gameplay/dispatch/src/gameplay_transport/world_map.rs
  - crates/gameplay/dispatch/src/gameplay_transport/world_map/house.rs
  - crates/gameplay/dispatch/src/gameplay_transport/world_map_tests.rs
  - crates/gameplay/durability/src/durability/character_appearance.rs
  - crates/gameplay/durability/src/durability/character_appearance/earned_unlocks.rs
  - crates/gameplay/durability/src/durability/character_appearance/grant_lock.rs
  - crates/gameplay/durability/src/durability/character_appearance/quest_grants.rs
  - crates/gameplay/durability/src/durability/character_title/cosmetics.rs
  - crates/gameplay/durability/src/durability/consume_transform.rs
  - crates/gameplay/durability/src/durability/consume_transform_audit.rs
  - crates/gameplay/durability/src/durability/consume_transform_cosmetics.rs
  - crates/gameplay/durability/src/durability/consume_transform_ground.rs
  - crates/gameplay/durability/src/durability/consume_transform_house.rs
  - crates/gameplay/durability/src/durability/consume_transform_planting.rs
  - crates/gameplay/durability/src/durability/core_modules.rs
  - crates/gameplay/durability/src/durability/item_craft.rs
  - crates/gameplay/durability/src/durability/item_craft/audit.rs
  - crates/gameplay/durability/src/durability/item_craft/event_containers.rs
  - crates/gameplay/durability/src/durability/item_craft/harvest.rs
  - crates/gameplay/durability/src/durability/item_loop_move.rs
  - crates/gameplay/durability/src/durability/item_loop_move/seed_drop.rs
  - crates/gameplay/durability/src/durability/item_use/mount_taming.rs
  - crates/gameplay/durability/src/durability/map_item_mint.rs
  - crates/gameplay/durability/src/durability/map_item_mint/planting.rs
  - crates/gameplay/durability/src/durability/native_world_items.rs
  - crates/gameplay/durability/src/durability/spell_item_temporal.rs
  - crates/gameplay/durability/src/durability/spell_item_temporal/event_items.rs
  - crates/gameplay/durability/src/durability/task_board/entitlements.rs
  - crates/gameplay/durability/src/durability/terrain_tool.rs
  - crates/gameplay/items/src/event_items.rs
  - crates/gameplay/items/src/event_planting.rs
  - crates/gameplay/items/src/item_cosmetics.rs
  - crates/gameplay/items/src/item_cosmetics/tests.rs
  - crates/gameplay/items/src/item_cosmetics/tests/additional.rs
  - crates/gameplay/items/src/lib.rs
  - crates/gameplay/items/src/novelty_items.rs
  - crates/gameplay/items/src/novelty_items/tests.rs
  - crates/gameplay/items/src/reward_bag.rs
  - crates/gameplay/items/src/reward_bag/event_containers.rs
  - crates/gameplay/quests/src/quest/world_callbacks.rs
  - crates/gameplay/spell/src/spell/actor_conditions.rs
  - crates/gameplay/spell/src/spell/actor_conditions/item_actions.rs
  - crates/gameplay/world/src/map/boot.rs
  - crates/gameplay/world/src/map/facts.rs
  - crates/gameplay/world/src/map/facts/house_base_tests.rs
  - crates/gameplay/world/src/map/view.rs
  - docs/agents/tasks/OTV2-20261103-event-items.md
  - docs/agents/tasks/archive/OTV2-20261103-event-items.md
  - tools/content-migration/item_cosmetic_actions.py
  - tools/content-schema/native-gameplay/item-cosmetic-actions.json
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

This final authoring record owns the move from `docs/agents/tasks/OTV2-20261103-event-items.md` to `docs/agents/tasks/archive/OTV2-20261103-event-items.md`. Its 109 owned paths comprise the 107 frozen source/test files plus that deletion and addition. Transient generation files are excluded.

The original base is `128ebc4f6aebf264d01c54003d269fa798e9524c`. Main `c9974cd880b69e0d0b2019d745fa5c91e0f83b00` and then `00d0ff102a0eff1c2df00de331fed9668c7a5242` were integrated before latest main `91c29a8129a0dbd7acd4dc3f633550e80111b948`, merged locally as `091f7940131129e930d4ce31bc341ac59b64db53`. The metadata head identifies that merged authoring snapshot; the task delta is bound by `frozen-pg6-source.json`, SHA256 `0d8276701f8311319c0a759281ef73907e7635b28c54d575149e6340561cda71`. The exact published candidate belongs in the external FREEZE_SHA entry after the final authoring commit exists. This document contains no self-referential commit or document hash. Local and API publication histories remain distinct.

## Outcome

The bounded W19 delivery extends the existing ordinary item USE, reward-bag/item-craft, consume-transform, appearance, achievement, terrain, House and timed-item owners. It retains server-selected outcomes and raw intent in existing durable receipts, makes source costs and granted state atomic, and reuses current session/Character/lease/scope/custody checks. It adds no transaction engine, parallel receipt owner, capability or command.

The implementation covers W19c novelty and music; W19a requested reward-container, costume and die families; W19d source-qualified cosmetic acquisition; W19b authored-map harvest/planting and actual Item-pot actions; and W19e anniversary bundles and named-House balloon transformations. Missing canonical cosmetic mappings refuse before spending items or contribution progress. Positive cosmetic tests use explicit synthetic mappings; production canonical catalogues and World population remain unchanged.

The executed-case matrix has 57 distinct PostgreSQL cases with latest PASS, including four existing rust/lottery controls, plus four dispatch unit cases from the selected batch. This is a union of explicitly recorded runs, not a claim that all cases were rerun in one invocation on latest main. The final two Ground reward cases passed on latest main after the actual root-capacity defect and bounded wire-replay fixture order were repaired. The completed task status describes this bounded draft delivery; it does not declare overall mechanic-71 DONE, production presentation, merge or post-merge qualification.

## Architecture and source of truth

- **PROVEN source pins:** Crystal `201189f871d23597b82caf88fd611454a2ddefb8` is primary; Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446` is the secondary audit. Crystal determines the Revenant addon order. Source catalogues remain `event-item-actions.json`, `novelty-item-actions.json`, `item-cosmetic-actions.json` and `event-item-profiles.json` under `tools/content-schema/native-gameplay/`.
- **PROVEN retained ownership:** DUR-03 item transactions and the existing item_craft, consume_transform, character_appearance, account_achievement, spell_item_temporal and terrain_tool owners remain authoritative. Character writes retain session/generation fences; WorldId, ChannelId and Instance scope remain distinct.
- **PROVEN accepted decision:** [Answer A on PR #2855](https://github.com/Oteryn/Oteryn-Game/pull/2855#issuecomment-6606411752) keeps capability 23 unoffered. Exact source effect, missile and sound cues remain data; playback is outside this delivery. Costume, health/death, chat and original timed deadlines use their existing owners.
- **PROVEN canonical boundary:** `content/cosmetics/outfits/index.json:7` and `content/assets/catalog/index.json:7` remain `READY_UNPOPULATED`. Captured source rules do not create canonical identities. The cosmetic catalogue has 48 rules and 205 grant records, with ten pinned generator input hashes checked.
- **PROVEN authoring scope:** migrations 0880–0884 extend existing owners. Reserved capabilities 290–292, commands 520–524 and mechanic orders 660–664 are unused. Existing registrations, wire formats and selected capabilities are reused.
- **DERIVED acceptance boundary:** authored-map interactions require admitted native map entries. Real PostgreSQL placements qualify those owner paths; they do not establish global production World population.

The latest-main impact review found no change to durability, appearance, Premium or current-Character authority owners. The new client recovery codecs retain existing wire fields; resume preserves selected capabilities and requires a newer generation plus nondecreasing sequence/next-command values. Server session_ingress retains its single terminal-outcome replay window. Latest-main regeneration and routed quality gates are recorded below.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - current recovery and runtime scope holder/generation
  - current Character, game session, connection generation and Character lease
  - exact actor and live runtime write permission before successor installation
  - current native source definition, admitted canonical mapping and account Premium
  - exact owned source/target custody, quantity and placement identity
  - immutable command, raw intent, resolved items, source draw and durable outcome
  - one atomic source/progress/grant/achievement/deadline/audit transition
consumer_boundaries:
  - ordinary item USE and existing reward-bag/item-craft dispatch
  - current inventory, nested ContainerView and Ground observations
  - named-House inventory, map and carried-container observation
  - existing consume-transform and craft receipt transactions
  - runtime-role SQL admission and deferred completion guards
  - cancelled pending item work, control loss, release and reconnect grace
mutation_operators:
  applicable:
    - unavailable_content_or_canonical_mapping
    - missing_current_authority
    - stale_generation_or_lease
    - mismatched_identity_scope_or_raw_intent
    - wrong_custody_or_changed_source
    - unavailable_or_expired_trusted_Premium
    - missing_distinct_component_or_prerequisite
    - duplicate_owned_result
    - omitted_atomic_grant_or_output
    - concurrent_competing_grant_or_duplicate
    - cancelled_unknown_ack_and_exact_receipt_replay
    - locally_fenced_live_successor
    - moved_item_or_restarted_original_deadline
    - Channel_projection_leak_into_House
    - saturated_Ground_cell_with_retained_or_retired_source_root
  considered_not_applicable:
    - new_protocol_version_or_capability
    - cross_repository_or_live_production_authority
one_invariant_per_negative_case: scoped_named_cases_reviewed_and_executed
independent_current_fact_sources:
  - registered_current_node_and_runtime_scope_assignment
  - fresh_admission_session_and_Character_lease
  - live_runtime_actor_binding_and_bound_attacker_lease
  - current_native_content_and_canonical_grant_mapping
  - authenticated_session_account_and_trusted_Premium_clock
  - durable_item_custody_and_named_House_context
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: carried_nested_Ground_and_named_House_owners_reviewed
  protocol_versions: unchanged_existing_item_and_House_capability_surfaces
  direct_and_reconciled_paths: normal_finish_and_retained_pending_settlement_guarded
  fenced_durable_writes: existing_Type2_and_craft_owners_plus_migrations0880_through0884
  restart_retry_replay_concurrency_pg_reload: latest_57_case_PostgreSQL_matrix_PASS
  evidence:
    - PR2855_source_implementation_named_test_tables
    - CODEX_CHECKLIST_scope_reviews
    - pending_lifecycle_committed_rollback_unknown_and_local_write_fence_case
    - Ground_capacity_RED_and_final_PG6_both_cases
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - table_specific_NEW_field_resolution_in_shared_cosmetic_deferred_trigger
    - cosmetic_Premium_selection_through_existing_registered_account_owner
    - House_action_reachability_past_Channel_only_early_probes
    - current_House_nested_observation_and_changed_custody_refusal
    - exact_original_turn_duplicate_cosmetic_feedback
    - cancelled_item_successor_retention_and_current_authority_recheck
    - live_successor_local_write_fence_recheck_inside_mutation_turn
    - current_Ground_qualifier_and_seed_target_owner_reachability
    - Ground_reward_root_cap_uses_actual_post_debit_census
  p0_p1_rejected_with_exact_evidence:
    - House_pending_successor_branch_unreachable_in_current_separate_House_route
  p2_fixed_accepted_or_deferred:
    - Outfit_fixture_captures_actual_published_source_and_exact_no_spend_receipt
    - wire_replay_occurs_within_existing_terminal_window_historical_owner_replay_retained
```

Historical receipts prove original outcomes but cannot alone authorize live effects. Committed pending effects recheck current node/scope assignment and nonterminal session/lease. Same-session recovery may advance connection generation without adopting another Character, lease or scope. Settlement rechecks the bound lease and local write fence under the uninterrupted runtime-to-state lock before consuming a memo. Known absence and noncommitted outcomes preserve cleanup; runtime/state locks do not span durable reads. Release and grace-expiry reconciliation precede local transition fencing.

House observations recheck durable authority after awaited reads and verify current House actor/native context before exposing handles. House projections exclude Channel movement, volatile overlays and base overrides. Ground reward capacity reuses the existing 64-root owner after the actual source debit and before insertion: quantity 2 retains the source root, quantity 1 releases it. NoRoom rolls back the entire fenced transaction. The existing per-Channel item/spell lane serializes the relevant source-lock/census path.

## Acceptance criteria

- [x] The 57 distinct PostgreSQL cases have real passing executions with completion markers, preserved refusals, replay and atomicity assertions; four dispatch unit cases also passed.
- [x] Scoped source-rule, condition and House isolation tests passed on their recorded snapshots; source generator checks preserved the pinned donor inputs.
- [x] Latest-main normal regeneration, seven-package lint, workspace formatting, explicit 19-file PostgreSQL formatting, compiler pin-check and source whitespace passed.
- [x] Canonical governance and its tests passed on the complete latest-main source before archive authoring.
- [x] Concrete scoped self/peer findings have code or evidence dispositions. External review was not requested.
- [x] Source-to-owner-to-named-test and retained cue tables form the canonical delivery evidence for Draft PR #2855.
- [x] The final authoring record preserves the user's draft-only, no-merge and no-external-review boundary.

## Reference parity

The complete source/value tables, exact Crystal file:line values, Oteryn owner anchors, named rule tests and real PostgreSQL transitions are retained once in [PR #2855](https://github.com/Oteryn/Oteryn-Game/pull/2855), under “Source → implementation → test” and “Source cue record under Answer A,” including the 48-row cosmetic cue inventory. The task record references that evidence without duplicating those tables.

The approved Christmas interpretation uses a fresh ten-entry pool per opening and seven distinct fixed-count gifts. Source numeric mount identity remains authoritative over misleading display names. Source quantities, probability boundaries, component requirements, one-time grants and absolute deadlines remain separate from canonical admission and runtime qualification.

## Excluded scope / Left out

- **live presentation consumer (capability 23, client effect/missile/sound playback):** accepted Answer A keeps cap23 unoffered. The client handler is unsupported (`crates/client-ui/src/features/0023-presentation/registrations/handlers.json:5`), and there is no completed source effect/asset/state/render chain. Persisted cue data is not delivery evidence.
- **House action scope:** House scope is opened only for anniversary balloons in this slice; other W19 novelty/container/seed/cosmetic actions inside House scope remain unavailable/outside this delivery.
- Staff Ghost dice cue/speech suppression is outside this change; ordinary-player die behavior is the qualified scope.
- Production canonical cosmetic population remains unavailable. Spirit of Purity item 44048 has no physical definition in the pinned donor item snapshots and remains unavailable.
- Carried reward overflow to a player's Ground tile is not implemented by the existing craft owner; insufficient carried slots/weight refuse atomically.
- Seed AddItem DROP onto an already-materialized durable Ground pot remains outside this slice; authored base-map pot DROP is covered.
- Nested seed USE on an authored base-map pot retains the existing main-backpack craft eligibility. Actual carried/Ground Item pot routes are a distinct covered owner path.
- This task does not populate a production World, add W05 dynamic objects or unrelated daily-world machinery, or admit Glooth 21203 and unrelated Cake/Autumn actions.
- No client renderer/assets, new command/capability, protected workflow, repository policy, existing migration, dependency/vendor, production account, credential or external repository mutation is part of this task.
- The Crystal re-audit/progress ledger remains read-only; this task does not claim overall mechanic-71 DONE.
- The user's requested validation excludes full-workspace nextest/full PostgreSQL expansion. Focused W19 plus four existing rust/lottery controls are recorded below; unrun suites are not counted as passes.

## Implementation / findings

The five authoring groups follow C → A → D → B → E. Shared registration and durable owners remain coordinated by the task lead, with one writer per allocated source file and one serialized Cargo lane.

The shared consume-transform receipt covers novelty, cosmetic and actual Item-pot outcomes. Account-grant locking is shared with quest, taming and task-board grant producers. Table-specific deferred-trigger branches preserve appearance/progress consistency. Competing-owner and omitted-write fixtures exercise actual locking and transaction rollback.

Unknown acknowledgements retain prepared work, random choice and exact command identity. Receipt-only reconciliation distinguishes committed, known rollback and still-unknown outcomes. The lifecycle case exercises a local write fence without losing memo or changing vitals, then verifies one-time settlement after that exact fence is lifted. House tests exercise actual carried, nested, placed and changed-custody paths. Source deadlines remain absolute across movement, replay and catch-up.

The additional Ground tests prove the existing reward owner reaches inventory and Ground outputs. The Special case first reproduced the missing root limit, then qualified no-write refusal at saturation, quantity 2 retaining its root, quantity 1 releasing it, immediate wire replay, historical restarted-owner replay and changed-intent refusal. The Anniversary case proves insufficient-slot rollback and the complete fourteen-item output from source quantity 2, including empty item 39693.

## Validation

Cargo ran serially with the task target directory and CARGO_BUILD_JOBS=4. PostgreSQL execution used only the task's PostgreSQL 18.6 container. Skipped or unconfigured cases are not counted as database qualification.

### Focused behavior and retained history

- `cargo test --locked -p oteryn-gameplay-items -p oteryn-gameplay-spell -p oteryn-gameplay-world -p oteryn-gameplay-quests --all-features --lib -- event_containers event_planting item_cosmetics novelty_items item_actions dryad_garden_uses_source_unique_ids_in_both_directions w19_house_base_facts_exclude_channel_door_rotation_and_callback_overrides w19_reward_effects_preserve_source_origin_and_per_reward_multiplicity --nocapture --test-threads=4` — pass on its recorded snapshot after main c9974cd: 44 tests, zero failed/ignored; `unit-w19-core-merged.log`. This is not a claim that those unit cases were rerun on the final published SHA.
- `python3 tools/content-migration/item_cosmetic_actions.py --crystal /home/mole/chatgpt6-evidence/crystal --root . --check` — pass on the recorded source snapshot: ten pinned input hashes, 48 rules/205 grants; `w19d-generator-final-check.log`. The generated source file is included in the frozen current inventory.
- PG3 `pg-w19-final-3.log` — raw FAIL, 57 passed/2 failed/0 ignored, exit 101, 509.61s; nested House observation and the Outfit fixture's hardcoded source UUID failed.
- PG4 `pg-w19-final-4.log` — pass, all four House cases and corrected Outfit, 5 passed/0 failed/0 ignored, exit 0, compilation 50.45s and runtime 61.70s. The PG3/PG4 union is 55 actual PostgreSQL cases plus four dispatch unit cases; repeated executions are not added to the distinct count.
- Ground behavior RED `pg-w19-ground-red.log` — raw FAIL, 1 passed/1 failed, exit 101, 37.34s. Anniversary passed; Special returned Committed where an actual saturated 64-root cell required NoRoom. The three-file cap-reservation repair followed this behavioral failure.
- PG5 `pg-w19-final-5.log` — raw FAIL, 4 passed/1 failed, exit 101, 65.97s. Both saturation and positive capacity boundaries executed, then Special attempted wire command 2 after accepted command 3 and correctly reached CommandOutcomeExpired. The test-only correction moved each wire replay immediately after its original command, preserving both historical owner replays, raw-conflict assertions and full-state equality.
- PG6 `pg-w19-final-6.log` — pass on latest main: exactly the two Ground reward cases, 2 passed/0 failed/0 ignored, exit 0, test runtime 32.31s and whole command 187.29s. Both actual completion markers are present.
- `pg-case-matrix-current.json`, SHA256 `72588fd1af5b213006ae69f3fa9657a3fe1150e6fd7ba31b217d6af7207bd006` — pass: 57 unique PostgreSQL cases, all latest outcomes passed, no latest failure. Each status is checked against its actual attempt log; earlier aggregate failures are retained above. The two Ground entries are qualified by PG6.
- Fresh migrations 0880–0884 executed in the real isolated PostgreSQL harness; completed database cases qualify their selected production transitions.

### Latest-main routed checks

- `cargo fmt --all` — pass, exit 0, 6.96s; evidence `fmt-before-regen-main91.log`.
- `sh tools/merge-driver/regen.sh` — pass, exit 0, 816.1s; evidence `regen-final-91c29a8.log`.
- `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib -- w19_ground_event_container_ --nocapture --test-threads=4` — pass, exit 0, 187.29s; evidence `pg-w19-final-6.log`.
- `cargo clippy --locked -p oteryn-gameplay-dispatch -p oteryn-gameplay-durability -p oteryn-gameplay-items -p oteryn-gameplay-quests -p oteryn-gameplay-spell -p oteryn-gameplay-world -p oteryn-game-server --all-targets --all-features --quiet -- -D warnings` — pass, exit 0, 131.69s; evidence `clippy-final-main91-7.log`.
- `cargo fmt --all -- --check` — pass, exit 0, 8.75s; evidence `fmt-final-main91.log`.
- `rustfmt --edition 2024 --check /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/event_containers.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/event_containers/ground.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/event_containers/wire.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/event_object_uncertainty.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/house_balloon.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/house_balloon/scenario.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/house_balloon/wire.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/house_wrap.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics/packet.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics/positive.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics/race.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_seed_planting.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/map_harvest.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/map_seed_planting.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/novelty_items.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/novelty_items/pending.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/novelty_items/scenario.rs /home/mole/chatgpt6/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/rust_lottery.rs` — pass, exit 0, 0.19s; evidence `pg-fmt-final-main91.log`.
- `cargo +1.99.1 run --locked --release -p oteryn-world-bundle-compiler -- pin-check . /home/mole/chatgpt6-evidence/world-pin-check-final-main91` — pass, exit 0, 21.51s; evidence `pin-check-final-main91.log`.
- `git diff --check HEAD` — pass, exit 0, 0.03s; evidence `diff-final-main91.log`.

The single seven-package Clippy invocation includes game-server because its migration path selects the package under CONTEXT_ROUTING, despite no changed app Rust source. Normal regen.sh runs the registered content checks, app content tests and pin refresh; a duplicate regenerate_content.py --check is not a separate gate. The final explicit rustfmt command checks all 19 mounted PostgreSQL files.

### Governance and numbering

- `python tools/agents/validate_governance.py` — pass on the complete latest-main source before archiving: 22 required policy documents, 9 project lanes, exit 0; evidence `governance-main91-pre-archive.log`.
- `python -m unittest discover -s tools/agents/tests` — pass on that same pre-archive candidate: 129 tests in 0.239s, OK, exit 0; evidence `governance-tests-main91-pre-archive.log`.

`governance-main91-pre-archive-results.json` binds both canonical executions to the exact frozen source SHA256 `0d8276701f8311319c0a759281ef73907e7635b28c54d575149e6340561cda71`. These are actual complete-source **pre-archive** results. They do not claim a post-move validation. The task lead reruns both commands after placing this record and records the actual result and archive hash in `governance-main91-post-archive-results.json`; the final closeout evidence reports that separate execution.

- `python3 .claude/skills/oteryn-dev/scripts/next-free-numbers.py` — unavailable full-script attempt on main91c29a8, exit 1 because gh is absent; `numbers-main91-full.log`. This is not a pass.
- `python3 .claude/skills/oteryn-dev/scripts/next-free-numbers.py --no-prs` — pass against latest main91c29a8, exit 0; `numbers-main91-main-only.log`.
- Latest-main read-only open-PR audit — pass at 2026-11-03T16:50:35.540Z: `numbers-open-prs-api-final.json` records 5 other open PR heads with no collision in the reserved blocks. The task's own PR is excluded. Later publication rechecks are recorded externally at the actual candidate.

### E2E and consumer boundary

Real framed ordinary USE, nested OPEN, actual Ground/House handles, durable read-back, immutable receipts, appearance, chat and timed-state consumers provide the focused integration evidence. This does not claim native UI/rendering E2E or capability-23 playback.

### Exact-head CI

- Final published head: external FREEZE_SHA after the last authoring commit; the record cannot contain its own commit SHA.
- Trigger source / workflow / run / job / runner assignment: NOT_EVALUATED for that future published head.
- Classification/result: NOT_EVALUATED for the final head.
- Merge/post-merge gate: NOT_REQUESTED under the user's draft-only instruction.

## Self-review

- Exact source: latest merged-main snapshot `091f7940131129e930d4ce31bc341ac59b64db53` plus the 107-file implementation inventory bound by `frozen-pg6-source.json`. Earlier test attempts retain their recorded snapshot scope.
- Method/reviewer: implementing owners performed scoped CODEX-CHECKLIST and source-to-owner-to-test reviews; the coordinating lead owns the final combined-path and publication review.
- Material findings: source/owner reachability, deferred-trigger typing, Premium ownership, House observation/isolation, pending reconciliation, same-turn write fencing and Ground capacity were repaired and qualified by named tests.
- Verdict: scoped implementation review complete, with the explicit Left out limits retained. Exact published-head CI and publication identity are recorded externally; no mergeable claim is made.

## Independent review

- External review required/requested: NO, under the user's explicit no-review-request instruction.
- Internal peer review: separate allocated workers reviewed source/owner boundaries, House paths, cosmetic admission/atomicity, pending settlement and the Ground capacity/test patches.
- Exact source: the recorded authoring files and reviewed narrow repairs, without claiming an external frozen-head review.
- Material findings/verdict: accepted findings have dispositions above. The final same-turn lease guard, actual Outfit identity assertions, Ground lock-order proof and bounded replay-order correction received read-only peer review with no remaining scoped blocker. Peer review is not external review or merge authorization.

## PR and closeout

- Canonical PR: [Draft #2855](https://github.com/Oteryn/Oteryn-Game/pull/2855), branch `chatgpt/event-items-6`.
- Changed-file review: 107 frozen source/test paths plus original-task deletion and archive addition; the exact published tree and final review are recorded in the lead's publication evidence.
- Review threads: accepted Answer A governs the presentation boundary; the lead's bounded live pre-publication read supplies the current thread state.
- Related reference: prior #2781 is read-only implementation reference; #2855 carries this independently qualified bounded candidate.
- Protected auto-merge: NOT_REQUESTED.
- Merge commit/result: NOT_MERGED. A separately authorized future merge would use the repository notation `squash merge of #2855`.
- Ownership: this archive ends the allocated W19 source authoring lane; the coordinating lead retains publication responsibility through draft handoff. No external review request or merge is authorized.

## Context checkpoint

```yaml
last_progress: latest_main_source_gates_and_57_case_matrix_qualified_for_final_authoring
status: completed
branch: chatgpt/event-items-6
head_sha: 091f7940131129e930d4ce31bc341ac59b64db53
pr: 2855
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: record_actual_post_archive_governance_and_publish_final_draft_candidate
```

Evidence-derived record generated at 2026-11-03T17:10:49.361565+00:00; source-freeze and case-matrix hashes identify the inputs. The archive payload hash is kept only in the separate sealing manifest, avoiding self-reference.

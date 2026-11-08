# pc2b into pc7 merge resolution

All 85 requested paths have no conflict markers. All 43 conflicted JSON documents parse. The Git index remains unmerged because staging was explicitly prohibited. No Git add, commit, push, checkout, reset or stash was run. Full content regeneration was not run.

## How the sources were combined

- JSON: recursively merged records by stable identity, retained disjoint entries and fields from both branches, and combined unkeyed entries without dropping either side. Derived hashes and generated diagnostics remain subject to the deferred regeneration.
- Native corpse/death handling: retained pc7's canonical typed Channel/Instance storage, native death byte encoding and compatibility receipts; incorporated pc2b's scope decoding, exact parent-scope and assignment-history checks, admission paths and negative tests. The incoming migration recognizes the already installed typed schema instead of adding duplicate columns.
- Quest callbacks and kills: combined pc7 death objectives, read-only predicates, achievements and reward paths with pc2b native goals, position aliases, party transfers, reward doors, encounter entry and terminal completion effects.
- Test constructors and callers: supplied fields required by both branches, retained all test cases, adapted movement-owner borrowing and assignments, covered the four diagonal enum variants, and kept the existing terrain projectile default.
- Python authoring: retained both item-admission cohorts, charged rewards, poison daggers, selected chest checks, outfit validation and quest completion validation. Rejected mismatched existing native charge proofs even when the merged canonical item is materializable.
- Single derived digest slots in Rust, TOML and balance notes retain pc7's value pending the requested later regeneration.

## Resolved files

| File | Combination |
| --- | --- |
| [content/creatures/prey-slot-values.json](content/creatures/prey-slot-values.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/creatures/prey-tiers.json](content/creatures/prey-tiers.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/interactions/reward_claims/reward-claim-variants-00000-00099.json](content/interactions/reward_claims/reward-claim-variants-00000-00099.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/items/definitions/items-18000-18499.json](content/items/definitions/items-18000-18499.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/items/definitions/items-20000-20499.json](content/items/definitions/items-20000-20499.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/items/definitions/spell-native-profiles.json](content/items/definitions/spell-native-profiles.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/items/definitions/srx1-item-updates.json](content/items/definitions/srx1-item-updates.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/items/taxonomy/items.json](content/items/taxonomy/items.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/quests/definitions/quests-00000-00099.json](content/quests/definitions/quests-00000-00099.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/quests/definitions/quests-00100-00199.json](content/quests/definitions/quests-00100-00199.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/quests/definitions/quests-00200-00299.json](content/quests/definitions/quests-00200-00299.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/quests/grizzly-boss-rooms.json](content/quests/grizzly-boss-rooms.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/quests/missions/completion-candidate.json](content/quests/missions/completion-candidate.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/quests/missions/quest-state-completion-candidate.json](content/quests/missions/quest-state-completion-candidate.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/quests/missions/quest-state.json](content/quests/missions/quest-state.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/spells.manifest.json](content/spells.manifest.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/world/pins/oteryn.json](content/world/pins/oteryn.json) | Merged both sets of records/fields; derived refresh deferred. |
| [content/world/placements/index.json](content/world/placements/index.json) | Merged both sets of records/fields; derived refresh deferred. |
| [crates/client-assets/src/world.rs](crates/client-assets/src/world.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/combat/src/combat/death_reward.rs](crates/gameplay/combat/src/combat/death_reward.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/content/src/content/native_gameplay/test_support/activation_fixture.rs](crates/gameplay/content/src/content/native_gameplay/test_support/activation_fixture.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/dispatch/src/gameplay_transport/boss_activity.rs](crates/gameplay/dispatch/src/gameplay_transport/boss_activity.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/dispatch/src/gameplay_transport/endgame_entry.rs](crates/gameplay/dispatch/src/gameplay_transport/endgame_entry.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/dispatch/src/gameplay_transport/grizzly_kills.rs](crates/gameplay/dispatch/src/gameplay_transport/grizzly_kills.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/dispatch/src/gameplay_transport/party_kill_settlement.rs](crates/gameplay/dispatch/src/gameplay_transport/party_kill_settlement.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/boss_activity.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/boss_activity.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_container_chests.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_container_chests.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/unnatural_selection.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/unnatural_selection.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/soul_war.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/soul_war.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs](crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs](crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/durability/src/durability/character_hazard/primal.rs](crates/gameplay/durability/src/durability/character_hazard/primal.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/durability/src/durability/item_mint.rs](crates/gameplay/durability/src/durability/item_mint.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/durability/src/durability/item_mint_audit.rs](crates/gameplay/durability/src/durability/item_mint_audit.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/durability/src/durability/item_mint_audit/tests/oneof_wire_tests.rs](crates/gameplay/durability/src/durability/item_mint_audit/tests/oneof_wire_tests.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/durability/src/durability/quest_state/rotten_blood.rs](crates/gameplay/durability/src/durability/quest_state/rotten_blood.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/durability/src/durability/quest_state/soul_war/native.rs](crates/gameplay/durability/src/durability/quest_state/soul_war/native.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/durability/src/durability/quest_state/soul_war/storage.rs](crates/gameplay/durability/src/durability/quest_state/soul_war/storage.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/durability/src/durability/quest_state/writer.rs](crates/gameplay/durability/src/durability/quest_state/writer.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/entities/src/carrier/boss_activity_scope_tests.rs](crates/gameplay/entities/src/carrier/boss_activity_scope_tests.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/entities/src/carrier/corpse_views.rs](crates/gameplay/entities/src/carrier/corpse_views.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [crates/gameplay/quests/src/quest/world_callbacks.rs](crates/gameplay/quests/src/quest/world_callbacks.rs) | Combined runtime/test changes; reconciled overlapping definitions and callers. |
| [deploy/synology-game/preprod-topology.toml](deploy/synology-game/preprod-topology.toml) | Kept the pc7 digest baseline; merged source content retained elsewhere. |
| [docs/agents/evidence/quest-claim-qualification.json](docs/agents/evidence/quest-claim-qualification.json) | Merged both sets of records/fields; derived refresh deferred. |
| [docs/balance/PREY-SLOT-VALUE-2026-11-01.md](docs/balance/PREY-SLOT-VALUE-2026-11-01.md) | Kept the pc7 digest baseline; merged source content retained elsewhere. |
| [docs/balance/PREY-TIME-SIM-2026-11-01.md](docs/balance/PREY-TIME-SIM-2026-11-01.md) | Kept the pc7 digest baseline; merged source content retained elsewhere. |
| [docs/contracts/game-events/v1/native_one_item_transaction.proto](docs/contracts/game-events/v1/native_one_item_transaction.proto) | Kept one additive Instance field with its original wire tag. |
| [docs/repository/QUEST_CALLBACK_COVERAGE.json](docs/repository/QUEST_CALLBACK_COVERAGE.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-migration/quest_callback_profiles.py](tools/content-migration/quest_callback_profiles.py) | Combined authoring rules and tests from both branches. |
| [tools/content-migration/register_spell_families.py](tools/content-migration/register_spell_families.py) | Combined authoring rules and tests from both branches. |
| [tools/content-migration/test_monster_seven_spell_overlay.py](tools/content-migration/test_monster_seven_spell_overlay.py) | Combined authoring rules and tests from both branches. |
| [tools/content-migration/test_quest_callback_profiles.py](tools/content-migration/test_quest_callback_profiles.py) | Combined authoring rules and tests from both branches. |
| [tools/content-schema/native-gameplay/loot-tables.json](tools/content-schema/native-gameplay/loot-tables.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/native-gameplay/prey-model-results.json](tools/content-schema/native-gameplay/prey-model-results.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/native-gameplay/quest-batch3-item-profiles.json](tools/content-schema/native-gameplay/quest-batch3-item-profiles.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/check_quest_claim_qualification.py](tools/content-schema/quest-authoring/check_quest_claim_qualification.py) | Combined authoring rules and tests from both branches. |
| [tools/content-schema/quest-authoring/quest_batch4.py](tools/content-schema/quest-authoring/quest_batch4.py) | Combined authoring rules and tests from both branches. |
| [tools/content-schema/quest-authoring/samples/binding_packets/index.json](tools/content-schema/quest-authoring/samples/binding_packets/index.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/binding_packets/npc/quest-npc-dialogue-links.json](tools/content-schema/quest-authoring/samples/binding_packets/npc/quest-npc-dialogue-links.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/binding_packets/recipes/supplementary-index.json](tools/content-schema/quest-authoring/samples/binding_packets/recipes/supplementary-index.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/binding_packets/rewards/chosen-reward-item-candidates.json](tools/content-schema/quest-authoring/samples/binding_packets/rewards/chosen-reward-item-candidates.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/binding_packets/rewards/perquest-352.json](tools/content-schema/quest-authoring/samples/binding_packets/rewards/perquest-352.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/binding_packets/rewards/requirement-choices.json](tools/content-schema/quest-authoring/samples/binding_packets/rewards/requirement-choices.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/binding_packets/rewards/reward-holds.json](tools/content-schema/quest-authoring/samples/binding_packets/rewards/reward-holds.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/binding_packets/rewards/reward-only-105.json](tools/content-schema/quest-authoring/samples/binding_packets/rewards/reward-only-105.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/binding_packets/source/crosswalk.json](tools/content-schema/quest-authoring/samples/binding_packets/source/crosswalk.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/completion-matrix/all373.json](tools/content-schema/quest-authoring/samples/completion-matrix/all373.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/completion242/source-fix-approval.json](tools/content-schema/quest-authoring/samples/completion242/source-fix-approval.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/completion242/source-fix-receipt.json](tools/content-schema/quest-authoring/samples/completion242/source-fix-receipt.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/rollout/quest-rollout.json](tools/content-schema/quest-authoring/samples/rollout/quest-rollout.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/server-completion/collect-claim-candidates.json](tools/content-schema/quest-authoring/samples/server-completion/collect-claim-candidates.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/server-completion/reward-only-completion/candidates.json](tools/content-schema/quest-authoring/samples/server-completion/reward-only-completion/candidates.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/server-completion/use-trigger-candidates.json](tools/content-schema/quest-authoring/samples/server-completion/use-trigger-candidates.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/v2-export/declarations.json](tools/content-schema/quest-authoring/samples/v2-export/declarations.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/samples/v2-export/report.json](tools/content-schema/quest-authoring/samples/v2-export/report.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/quest-authoring/test_native_claim_source_readiness.py](tools/content-schema/quest-authoring/test_native_claim_source_readiness.py) | Combined authoring rules and tests from both branches. |
| [tools/content-schema/quest-authoring/test_quest_claim_qualification.py](tools/content-schema/quest-authoring/test_quest_claim_qualification.py) | Combined authoring rules and tests from both branches. |
| [tools/content-schema/quest-authoring/test_quest_completion_matrix.py](tools/content-schema/quest-authoring/test_quest_completion_matrix.py) | Combined authoring rules and tests from both branches. |
| [tools/content-schema/reward-claim-authoring/reward_claim_variant_authoring.py](tools/content-schema/reward-claim-authoring/reward_claim_variant_authoring.py) | Combined authoring rules and tests from both branches. |
| [tools/content-schema/reward-claim-authoring/samples/migration/reward-claim-variants.json](tools/content-schema/reward-claim-authoring/samples/migration/reward-claim-variants.json) | Merged both sets of records/fields; derived refresh deferred. |
| [tools/content-schema/reward-claim-authoring/test_container_native.py](tools/content-schema/reward-claim-authoring/test_container_native.py) | Combined authoring rules and tests from both branches. |
| [tools/content-schema/reward-claim-authoring/test_reward_claim_variant_authoring.py](tools/content-schema/reward-claim-authoring/test_reward_claim_variant_authoring.py) | Combined authoring rules and tests from both branches. |
| [tools/content-schema/reward-claim-authoring/test_variant_stack_serialization.py](tools/content-schema/reward-claim-authoring/test_variant_stack_serialization.py) | Combined authoring rules and tests from both branches. |
| [tools/content-schema/world-authoring/samples/world-base-capture-v1.json](tools/content-schema/world-authoring/samples/world-base-capture-v1.json) | Merged both sets of records/fields; derived refresh deferred. |

## Additional integration fixes

These files required changes because constructors, scope representations, movement owners or enum variants differed between the merged branches.

- [apps/game-server/migrations/1244_instance_corpse_mint.sql](apps/game-server/migrations/1244_instance_corpse_mint.sql)
- [crates/gameplay/dispatch/src/gameplay_transport/bundle_ladder.rs](crates/gameplay/dispatch/src/gameplay_transport/bundle_ladder.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/connection/tests/steps_pacing.rs](crates/gameplay/dispatch/src/gameplay_transport/connection/tests/steps_pacing.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/annihilator_door.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/annihilator_door.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/annihilator_route.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/annihilator_route.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/boss_activity/instance_corpse.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/boss_activity/instance_corpse.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/desert_earned_route.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/desert_earned_route.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/desert_party_rite.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/desert_party_rite.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_reward_claims.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_reward_claims.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc7_ferumbras.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc7_ferumbras.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc7_hoist_story.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc7_hoist_story.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc7_world_outfit.rs](crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc7_world_outfit.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/party_transfer.rs](crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/party_transfer.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/reward_door.rs](crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/reward_door.rs)
- [crates/gameplay/quests/src/quest/elvenbane.rs](crates/gameplay/quests/src/quest/elvenbane.rs)
- [crates/gameplay/quests/src/quest/world_callbacks/effects.rs](crates/gameplay/quests/src/quest/world_callbacks/effects.rs)

## Validation

- `cargo check --locked -p` passed for `oteryn-client-assets`, `oteryn-gameplay-combat`, `oteryn-gameplay-content`, `oteryn-gameplay-dispatch`, `oteryn-gameplay-durability`, `oteryn-gameplay-entities`, and `oteryn-gameplay-quests`.
- `cargo clippy --locked` for those seven packages, with `--all-targets -- -D warnings`, passed, including the final incremental run after the last test assertion edit.
- Reviewed the complete source diff against the workspace copy of `docs/agents/review/CODEX-CHECKLIST.md`; the outside-directory checklist was not read under the hard directory restriction. Searched constructor/caller sites throughout `apps/`, `crates/`, and `tools/`.
- Rustfmt on all touched Rust files, `git diff --check`, Python syntax and TOML parsing passed.
- The exact marker scan `rg -n '^(<<<<<<<|=======|>>>>>>>)'` on the 85 requested paths found nothing (exit 1). All 43 conflicted JSON documents parse.
- Focused Rust unit checks: 5 carrier, 2 corpse-placement, 34 mint-audit, 45 world-callback, and 9 Grizzly capture tests passed (95 tests total). The standalone carrier invocation requires `--features test-support`; the initial invocation without it could not compile existing test helpers, and the corrected invocation passed.
- Python: 17 callback profile tests, 18 container tests, 6 stack serialization tests, 7 qualification tests, 1 source readiness test, and 8 completion-matrix tests passed (57 tests).
- The variant-authoring module had 13 passing tests and one generated-snapshot failure: `test_committed_family_reproduces_with_separate_holds` reports stale charged-source diagnostics in `source_checks`.

### Exact cargo test filters

Every command uses `/home/mole/oteryn-cache/codex-bin/cargo`; no unfiltered PostgreSQL suite was invoked.

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet -- instance_corpse_postgres_native_scope_mint_audit_and_replay
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-entities --features test-support --lib --quiet -- boss_activity_carrier
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-combat --lib --quiet -- ground_placement
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --lib --quiet -- item_mint_audit::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-quests --lib --quiet -- world_callbacks
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet -- grizzly_kills::tests
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-content --lib --quiet -- reward_boss_test_fixture_preserves_source_loot_and_production_unavailability
```

### Unfinished

- Full content regeneration is intentionally deferred. Generated hashes, pins, diagnostics and other derived metadata need their generator refresh.
- The filtered content fixture unit test `reward_boss_test_fixture_preserves_source_loot_and_production_unavailability` failed with the same stale native-gameplay input digest before reaching its merged admission assertions. Rerun it after regeneration.
- The filtered Instance-corpse PostgreSQL test failed during content activation with `RevisionMismatch("native gameplay input digest")`, before its mechanic ran. It is not qualified; rerun it after regeneration. No missing-PostgreSQL early return was counted as qualification.
- The generated variant snapshot described above also needs refresh and revalidation.
- The merge remains unstaged and uncommitted as requested. Git will continue to list the original paths as unmerged until the owner stages them.

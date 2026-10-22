Module layout (planned before implementation): L1 content catalogue and typed NPC Services; L3 promotion rules; L4 durability purchase/reader/death modules and NPC dialogue; L5 dispatch command, login and Premium adapters. Each new mechanic has a child test module, mounted once.

PREM-2b+PREM-5 local authoring evidence, 2026-10-22. Baseline `9f814e72a3a32ad2a6ed67787c2d67a2d39d4af3`; branch `w4/prem-2b`; no frozen SHA or PR. This is local validation, not remote candidate qualification.

Implemented: durable once-only 20,000 coin promotion, current-only Premium blessing purchases with locked-level prices and Bank fallback, receipt replay/resume, fenced login/death/Wheel/Charm reads, held-blessing death consumption, and seven original NPC Service declarations. Soul maxima retain the existing Premium owner. `shown_vocation` has no accepted wire caller and is intentionally omitted.

Validation (all cargo commands used the queued wrapper with `login=false`; configured PG harnesses assert server version `170006`):

```bash
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 clippy --locked -p oteryn-gameplay-dispatch -p oteryn-game-server -p oteryn-gameplay-content -p oteryn-gameplay-npc -p oteryn-gameplay-progression -p oteryn-gameplay-durability --all-targets -- -D warnings
cargo +1.95.0 test --locked -p oteryn-gameplay-progression -p oteryn-gameplay-npc promotion  # PASS 3
cargo +1.95.0 test --locked -p oteryn-gameplay-content blessing  # PASS 2
cargo +1.95.0 test --locked -p oteryn-gameplay-domain premium  # PASS 7
cargo +1.95.0 test --locked -p oteryn-gameplay-durability item_fee_burn  # PASS 19
cargo +1.95.0 test --locked -p oteryn-gameplay-durability character_revision_sequencer  # PASS 11
cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch wheel_projection_tests  # PASS 8
cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch character_wheel_cast_postgres_cases -- --test-threads=1  # PASS 4
cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib promotion_npc_postgres -- --nocapture  # PASS 1
cargo +1.95.0 test --locked -p oteryn-game-server --test character_authority_postgres promotion -- --test-threads=1 --nocapture  # PASS 3
cargo +1.95.0 test --locked -p oteryn-game-server --test character_authority_postgres wheel -- --test-threads=1  # PASS 5
cargo +1.95.0 test --locked -p oteryn-game-server --test character_authority_postgres death -- --test-threads=1  # PASS 46
cargo +1.95.0 test --locked -p oteryn-game-server --test character_authority_postgres gold_fee -- --test-threads=1  # PASS 8
cargo +1.95.0 test --locked -p oteryn-game-server --test character_authority_postgres item_fee_burn_postgres_cases -- --test-threads=1  # PASS 8
cargo +1.95.0 test --locked -p oteryn-game-server --test content_npc_catalogue --test content_npc_service_model  # PASS 5
cargo +1.95.0 test --locked -p oteryn-game-server --test content_world_project_repository  # PASS 5
python3 tools/agents/validate_governance.py
python3 tools/repository/validate_repository_policy.py
python3 -m unittest discover -s tools/repository -p test_validate_pr_gate_pg_sim.py  # PASS 7
python3 tools/content-migration/validate_world_project_v2_to_tree.py
python3 tools/content-migration/test_world_project_v2_to_tree.py
```

The materialized-tree validator passed 97/97. The real dispatch marker is `POSTGRES_STARTED PREM-2b+PREM-5 promotion_npc version=17.6`; both new canonical purchase tests emit their own markers. Unit/PG assertions cover callback rollback, failure after fee staging, account/backpack/lease refusals, native pre-delivery gating, current-only blessings, purchase replay/resume, permanent promotion recovery, Charm Echoes, login projection, Bank-only blessing payment and death replay after Premium lapse.

The dispatch suite runs in CI through CI-PG-LIB-1 (#2300), which runs the whole dispatch library suite against PostgreSQL in the PR early signal and Merge Queue rest legs; this change edits no workflow and no pin, and all prior qualification commands remain. The whole-diff checklist and all enum/field/trait construction/implementation sites were searched across apps/, crates/ and tools/. No tests were removed or skipped.

Reuse and extraction:

- Existing `NpcTalkBook`, ChatRuntime conversation/rate owner and Bank NPC context checks; the shared check moved into `npc_command/service_context.rs` and both consumers call it.
- Existing `bank_coin_definitions`, `generation_item_facts`, `npc_trade_command::derived`, Bank receipt reconciliation and existing command/session fences; no parallel identity or fee planner.
- Existing Type2 transaction, native fee BURN/change planner, Bank fallback and item audit codec; existing Charm request bytes and tag 1 remain unchanged.
- Existing Character revision slot, recovery proof, gameplay/item fences and semantic DB deadlines; new child participants retain those owners through commit.
- Existing Premium coordinator, consumer, clock and borrowed latch witness; `tests/purchase_fixture.rs` is the small test-only wrapper over private clock/evidence helpers.
- Existing canonical ProjectV2 materializer, NPC catalogue/model, content-tree generator and staged BLESSINGS-CONTENT-1 evidence. The evidence payload is byte-identical; the separate catalogue carries the owner pricing ruling.
- Existing domain promotion benefit, soul cap/clamp, death/build-loss calculator, Wheel eligibility and Charm balance rules; no replacement formulas.
- Existing death writer and settlement bodies moved to `character_death/commit.rs` and `settlement.rs`, deleting their parent copies.
- Existing cross-kind SQL consistency guard reused through a common receipt view, rather than copied; existing immutable-row trigger also supplies the negative post-fee failure test.
- Existing GOLD-FEE item/request/id/definition builders moved once into test-only `item_fee_burn/test_support.rs`; their old bodies were deleted, and old plus new suites reuse them.
- Existing monk-save PostgreSQL Harness/Dependencies/session preparation; the one constructor body accepts a real pin and `monk_save_content_fixture.rs` retains the default test constructor. No duplicate test registration.
- Existing private Charm catalogue is exposed only by `charm_state_promotion_fixture.rs`, mounted once by its original test owner; the canonical PG target now reuses native durability modules instead of path-loading them again.
- Existing sequencer structural gate moved to `writer_registration_tests.rs`, preserving its checks and registering only the new participant.

Every file growing by more than 200 lines (all are new):

| File | Lines | Layer justification |
|---|---:|---|
| `crates/gameplay/dispatch/src/gameplay_transport/npc_command/premium_purchase.rs` | 299 | L5 composition: live NPC/session facts, lower-layer transaction and receipt reply/resume. |
| `crates/gameplay/dispatch/src/gameplay_transport/promotion_npc_postgres_tests.rs` | 437 | Own mechanic e2e suite over the existing dispatch/PG harness. |
| `crates/gameplay/durability/src/durability/character_death/commit.rs` | 316 | Existing L4 fenced SQL writer extracted from the oversized parent. |
| `crates/gameplay/durability/src/durability/character_death/settlement.rs` | 312 | Existing L4 sequenced settlement extracted from the oversized parent. |
| `rulesets/character/blessings/reference-evidence.json` | 752 | Archived source evidence data; no runtime logic. |

No existing file grows by more than 150 lines. Generated World changes are exactly seven Service declarations and six NPC service lists; other generated indices and the creature profile source pin follow those bytes.

CP retains independent review, freeze, CI/Merge Queue and publication; migration 0138 remains provisional. No implementation is left pending within the accepted wire-field exclusion.

Changed files (106, including this report and the task card):

```text
apps/game-server/examples/materialize_content_world_project_v2.rs
apps/game-server/migrations/0138_character_promotion_purchases.sql
apps/game-server/tests/character_authority_postgres.rs
apps/game-server/tests/content_npc_catalogue.rs
apps/game-server/tests/content_npc_service_model.rs
apps/game-server/tests/content_world_project_repository.rs
apps/game-server/tests/support/character_death_receipts_postgres_cases.rs
apps/game-server/tests/support/character_death_writer_postgres_cases.rs
apps/game-server/tests/support/character_promotion_fixture.rs
apps/game-server/tests/support/character_promotion_postgres_cases.rs
apps/game-server/tests/support/character_wheel_cast_postgres_cases.rs
apps/game-server/tests/support/character_wheel_postgres_cases.rs
apps/game-server/tests/support/character_wheel_promotion_tests.rs
apps/game-server/tests/support/charm_state_postgres_cases.rs
apps/game-server/tests/support/charm_state_promotion_fixture.rs
apps/game-server/tests/support/gold_fee_bank_postgres_cases.rs
apps/game-server/tests/support/item_fee_burn_postgres_cases.rs
content/content.lock.json
content/cosmetics/mounts/index.json
content/creatures/definitions/index.json
content/creatures/definitions/spell-native-profiles.json
content/dialogues/definitions/index.json
content/documents/index.json
content/encounters/definitions/index.json
content/manifest.json
content/npcs/definitions/index.json
content/npcs/definitions/npcs-00000-00499.json
content/npcs/definitions/npcs-00500-00999.json
content/services/bank/index.json
content/services/blessings/blessing-00000-00002.json
content/services/blessings/index.json
content/services/promotion/index.json
content/services/promotion/promotion-00000-00003.json
content/services/trade/index.json
content/services/travel/index.json
content/spells.manifest.json
content/world/content.lock.json
content/world/definitions/declarations.json
content/world/manifest.json
content/world/project.json
crates/gameplay/content/src/content/blessing_catalogue.rs
crates/gameplay/content/src/content/blessing_catalogue_tests.rs
crates/gameplay/content/src/content/mod.rs
crates/gameplay/content/src/content/npc_catalogue.rs
crates/gameplay/content/src/content/npc_catalogue/premium_services.rs
crates/gameplay/dispatch/src/gameplay_transport/mod.rs
crates/gameplay/dispatch/src/gameplay_transport/monk_save_content_fixture.rs
crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs
crates/gameplay/dispatch/src/gameplay_transport/npc_command/premium_purchase.rs
crates/gameplay/dispatch/src/gameplay_transport/npc_command/service_context.rs
crates/gameplay/dispatch/src/gameplay_transport/promotion_death.rs
crates/gameplay/dispatch/src/gameplay_transport/promotion_login.rs
crates/gameplay/dispatch/src/gameplay_transport/promotion_npc_postgres_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/spell_access_facts.rs
crates/gameplay/dispatch/src/gameplay_transport/spell_premium_coordinator.rs
crates/gameplay/dispatch/src/gameplay_transport/spell_premium_coordinator/promotion_gate.rs
crates/gameplay/dispatch/src/gameplay_transport/spell_premium_coordinator/tests/purchase_fixture.rs
crates/gameplay/dispatch/src/lib.rs
crates/gameplay/durability/src/durability/account_achievement.rs
crates/gameplay/durability/src/durability/blessing_purchase.rs
crates/gameplay/durability/src/durability/character_authority.rs
crates/gameplay/durability/src/durability/character_death.rs
crates/gameplay/durability/src/durability/character_death/commit.rs
crates/gameplay/durability/src/durability/character_death/promotion.rs
crates/gameplay/durability/src/durability/character_death/settlement.rs
crates/gameplay/durability/src/durability/character_progression.rs
crates/gameplay/durability/src/durability/character_promotion.rs
crates/gameplay/durability/src/durability/character_revision_sequencer.rs
crates/gameplay/durability/src/durability/character_revision_sequencer/death_promotion.rs
crates/gameplay/durability/src/durability/character_revision_sequencer/npc_purchase.rs
crates/gameplay/durability/src/durability/character_revision_sequencer/writer_registration_tests.rs
crates/gameplay/durability/src/durability/character_wheel.rs
crates/gameplay/durability/src/durability/charm_state.rs
crates/gameplay/durability/src/durability/core_modules.rs
crates/gameplay/durability/src/durability/item_fee_burn.rs
crates/gameplay/durability/src/durability/item_fee_burn/purchase_cause.rs
crates/gameplay/durability/src/durability/item_fee_burn/test_support.rs
crates/gameplay/durability/src/durability/item_fee_burn_audit.rs
crates/gameplay/durability/src/durability/item_fee_burn_audit/purchase_cause.rs
crates/gameplay/durability/src/durability/item_fee_burn_audit/tests/purchase_cause_tests.rs
crates/gameplay/durability/src/durability/item_mint_audit.rs
crates/gameplay/durability/src/durability/item_text.rs
crates/gameplay/durability/src/durability/mail.rs
crates/gameplay/durability/src/durability/npc_purchase.rs
crates/gameplay/durability/src/durability/npc_purchase/receipt.rs
crates/gameplay/durability/src/durability/npc_purchase/recovery.rs
crates/gameplay/durability/src/durability/premium_activation_latch.rs
crates/gameplay/durability/src/durability/promotion_purchase.rs
crates/gameplay/npc/src/lib.rs
crates/gameplay/npc/src/npc_promotion.rs
crates/gameplay/npc/src/npc_promotion_tests.rs
crates/gameplay/npc/src/npc_talk.rs
crates/gameplay/progression/src/lib.rs
crates/gameplay/progression/src/promotion.rs
crates/gameplay/progression/src/promotion_tests.rs
crates/gameplay/progression/src/spell_wheel.rs
docs/agents/evidence/PREM-2b-PREM-5-local-validation.md
docs/agents/tasks/PREM-2b-PREM-5.md
docs/contracts/game-events/v1/native_one_item_transaction.proto
rulesets/character/blessings/catalogue.json
rulesets/character/blessings/index.json
rulesets/character/blessings/reference-evidence.json
tools/content-migration/test_world_project_v2_to_tree.py
tools/content-migration/validate_world_project_v2_to_tree.py
tools/content-migration/world_project_v2_to_tree.py
```

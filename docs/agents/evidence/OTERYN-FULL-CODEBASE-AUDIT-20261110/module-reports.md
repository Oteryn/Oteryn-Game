# Module reports and remaining scope

Base: `e2e5ced306040113dc9c395251a802f50eb5776a`. [Coverage matrix](coverage.md) · [Exact evidence ledger](coverage-ledger.json) · [Findings](findings.json).

Each real Cargo package and supplemental tracked-file domain is listed once. Dependency lists are factual metadata, not completed dependency audits. No package has a blanket defect-free or runtime-qualified verdict.

## oteryn-client

- **Scope:** `apps/client` (Cargo package).
- **Status:** NOT STARTED; 0 of 1164 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-client-assets`, `oteryn-client-domain`, `oteryn-client-runtime`, `oteryn-client-simulation`, `oteryn-client-ui`, `oteryn-diagnostics`, `oteryn-foundation`, `oteryn-identity`, `oteryn-input-actions`, `oteryn-input-platform`, `oteryn-platform-client`, `oteryn-platform-contracts`, `oteryn-protocol-oteryn`, `oteryn-renderer`, `oteryn-session`, `oteryn-session-tcp`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-game-server

- **Scope:** `apps/game-server` (Cargo package).
- **Status:** IN PROGRESS; 22 of 867 tracked files have substantive analysis, including 12 complete files.
- **Entry points checked:** main, NodeConfig, serve supervision, public publisher and selected migration owners.
- **Flows checked:** Boot/configuration -> runtime tasks; public projection delivery; death and trade persistence proofs.
- **Declared local dependencies:** `oteryn-dev-client`, `oteryn-error-codes`, `oteryn-foundation`, `oteryn-gameplay-admission-evidence`, `oteryn-gameplay-character-recovery-fence`, `oteryn-gameplay-characters`, `oteryn-gameplay-combat`, `oteryn-gameplay-combat-rules`, `oteryn-gameplay-content`, `oteryn-gameplay-content-promotion`, `oteryn-gameplay-dispatch`, `oteryn-gameplay-domain`, `oteryn-gameplay-durability`, `oteryn-gameplay-economy`, `oteryn-gameplay-encounter-rules`, `oteryn-gameplay-encounters`, `oteryn-gameplay-entities`, `oteryn-gameplay-foundation`, `oteryn-gameplay-housing`, `oteryn-gameplay-items`, `oteryn-gameplay-monster-ai`, `oteryn-gameplay-native-admission-source`, `oteryn-gameplay-node-files`, `oteryn-gameplay-npc`, `oteryn-gameplay-progression`, `oteryn-gameplay-quests`, `oteryn-gameplay-social`, `oteryn-gameplay-social-rules`, `oteryn-gameplay-source-critical-baseline`, `oteryn-gameplay-spell`, `oteryn-gameplay-spell-rules`, `oteryn-gameplay-weather`, `oteryn-gameplay-world`, `oteryn-gameplay-world-change`, `oteryn-ntp-clock`, `oteryn-protocol-oteryn`, `oteryn-simulation-determinism`, `oteryn-world-bundle`, `oteryn-world-bundle-compiler`.
- **Findings:** 3 confirmed associated findings (DUR-003, SEC-001, SEC-003); all associated IDs: DUR-003, SEC-001, SEC-003.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Remaining boot branches, full migrated schema, TLS/service integration, deployment configurations and job/load behavior.
- **Exact evidence files:** `apps/game-server/migrations/0076_character_inbox.sql`, `apps/game-server/migrations/0128_item_loop_move.sql`, `apps/game-server/migrations/0147_player_trade.sql`, `apps/game-server/migrations/0191_market_settlement.sql`, `apps/game-server/migrations/0296_market_fee_2026_10.sql`, `apps/game-server/migrations/0424_mail_parcels.sql`, `apps/game-server/migrations/0531_player_death_item_primitives.sql`, `apps/game-server/migrations/0536_player_death_item_selection.sql`, `apps/game-server/migrations/0585_depot_search_retrieve.sql`, `apps/game-server/migrations/0630_boss_reward_collection.sql`, `apps/game-server/migrations/0914_house_rent.sql`, `apps/game-server/migrations/0941_house_rent_parity.sql`, `apps/game-server/migrations/2604_house_sale_authority.sql`, `apps/game-server/migrations/2703_house_sale_named_scope.sql`, `apps/game-server/src/main.rs`, `apps/game-server/src/node/cancellation.rs`, `apps/game-server/src/node/config.rs`, `apps/game-server/src/node/market_scheduler.rs`, `apps/game-server/src/node/serve.rs`, `apps/game-server/src/node/serve/channel_boot.rs`, `apps/game-server/src/node/serve/supervision.rs`, `apps/game-server/src/node/serve/world_reset.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-admin-agent

- **Scope:** `crates/admin/agent` (Cargo package).
- **Status:** NOT STARTED; 0 of 12 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-admin-contracts`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-admin-contracts

- **Scope:** `crates/admin/contracts` (Cargo package).
- **Status:** NOT STARTED; 0 of 7 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-client-assets

- **Scope:** `crates/client-assets` (Cargo package).
- **Status:** IN PROGRESS; 1 of 19 tracked files have substantive analysis, including 1 complete files.
- **Entry points checked:** Asset store entry and cache/file validation.
- **Flows checked:** Asset key/path -> validated cached content.
- **Declared local dependencies:** `oteryn-client-domain`, `oteryn-world-bundle`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Other asset sources, package/update lifetime, concurrency and renderer consumers.
- **Exact evidence files:** `crates/client-assets/src/store.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-client-domain

- **Scope:** `crates/client-domain` (Cargo package).
- **Status:** NOT STARTED; 0 of 4 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-foundation`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-client-runtime

- **Scope:** `crates/client-runtime` (Cargo package).
- **Status:** IN PROGRESS; 2 of 91 tracked files have substantive analysis, including 2 complete files.
- **Entry points checked:** Connection and combat state adapters.
- **Flows checked:** Connection/refusal/reconnect state; combat projection application.
- **Declared local dependencies:** `oteryn-diagnostics`, `oteryn-foundation`, `oteryn-session`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Full command/replay/UI/network integration and unreviewed runtime modules.
- **Exact evidence files:** `crates/client-runtime/src/combat.rs`, `crates/client-runtime/src/connection.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-client-simulation

- **Scope:** `crates/client-simulation` (Cargo package).
- **Status:** NOT STARTED; 0 of 7 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-client-domain`, `oteryn-foundation`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-client-ui

- **Scope:** `crates/client-ui` (Cargo package).
- **Status:** NOT STARTED; 0 of 1297 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-client-domain`, `oteryn-client-runtime`, `oteryn-input-actions`, `oteryn-renderer`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-diagnostics

- **Scope:** `crates/diagnostics` (Cargo package).
- **Status:** NOT STARTED; 0 of 2 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-foundation`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-error-codes

- **Scope:** `crates/error-codes` (Cargo package).
- **Status:** NOT STARTED; 0 of 12 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-foundation

- **Scope:** `crates/foundation` (Cargo package).
- **Status:** REVIEWED; 3 of 3 tracked files have substantive analysis, including 3 complete files.
- **Entry points checked:** BoundedText, cancellation and deadline primitives.
- **Flows checked:** Cancellation signal -> waiter; time arithmetic -> deadline.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Completed source review only; independent small-package report retained separately; native integrations not run.
- **Remaining:** Source review complete for this package. Full dependent-runtime scheduling and OS behavior were not tested.
- **Exact evidence files:** `crates/foundation/Cargo.toml`, `crates/foundation/src/lib.rs`, `crates/foundation/src/time.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-admission-evidence

- **Scope:** `crates/gameplay/admission-evidence` (Cargo package).
- **Status:** NOT STARTED; 0 of 2 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-gameplay-character-recovery-fence

- **Scope:** `crates/gameplay/character-recovery-fence` (Cargo package).
- **Status:** NOT STARTED; 0 of 2 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-gameplay-characters

- **Scope:** `crates/gameplay/characters` (Cargo package).
- **Status:** IN PROGRESS; 5 of 10 tracked files have substantive analysis, including 4 complete files.
- **Entry points checked:** Selected vitals, soul, stamina and daily reward/regeneration calculations.
- **Flows checked:** Character facts -> bounded arithmetic and regeneration.
- **Declared local dependencies:** `oteryn-gameplay-content`, `oteryn-gameplay-domain`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Remaining character state and all downstream durable/protocol combinations.
- **Exact evidence files:** `crates/gameplay/characters/src/hunting_experience.rs`, `crates/gameplay/characters/src/soul.rs`, `crates/gameplay/characters/src/speed.rs`, `crates/gameplay/characters/src/stamina.rs`, `crates/gameplay/characters/src/vitals.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-combat

- **Scope:** `crates/gameplay/combat` (Cargo package).
- **Status:** IN PROGRESS; 3 of 29 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** Selected creature death reward and settlement functions.
- **Flows checked:** Death -> captured principal -> loot/XP/Bestiary replay.
- **Declared local dependencies:** `oteryn-error-codes`, `oteryn-gameplay-combat-rules`, `oteryn-gameplay-content`, `oteryn-gameplay-domain`, `oteryn-gameplay-durability`, `oteryn-gameplay-encounter-rules`, `oteryn-gameplay-entities`, `oteryn-gameplay-foundation`, `oteryn-gameplay-items`, `oteryn-gameplay-source-critical-baseline`, `oteryn-gameplay-world`, `oteryn-simulation-determinism`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Complete combat, PvP legality, condition ticks, party settlement and source integration.
- **Exact evidence files:** `crates/gameplay/combat/src/ability/commit.rs`, `crates/gameplay/combat/src/ability/player_lethal.rs`, `crates/gameplay/combat/src/combat/death_reward.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-combat-rules

- **Scope:** `crates/gameplay/combat-rules` (Cargo package).
- **Status:** IN PROGRESS; 4 of 29 tracked files have substantive analysis, including 2 complete files.
- **Entry points checked:** Selected arithmetic, reduction and reward rules.
- **Flows checked:** Validated combat inputs -> rule outcome.
- **Declared local dependencies:** `oteryn-gameplay-content`, `oteryn-gameplay-progression`, `oteryn-protocol-oteryn`, `oteryn-simulation-determinism`, `oteryn-test-support`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Remaining rules, all extrema and production caller combinations.
- **Exact evidence files:** `crates/gameplay/combat-rules/src/attack/formulas.rs`, `crates/gameplay/combat-rules/src/attack/ranged.rs`, `crates/gameplay/combat-rules/src/pvp/death.rs`, `crates/gameplay/combat-rules/src/pvp/legality.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-content

- **Scope:** `crates/gameplay/content` (Cargo package).
- **Status:** IN PROGRESS; 2 of 138 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** Native digest and qualified outer-content staging.
- **Flows checked:** Server artifact -> qualified state; server/client digest separation.
- **Declared local dependencies:** `oteryn-gameplay-domain`, `oteryn-protocol-oteryn`, `oteryn-simulation-determinism`, `oteryn-world-bundle`, `oteryn-world-bundle-compiler`.
- **Findings:** 0 confirmed associated findings; all associated IDs: RNG-001.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** The vast majority of content codecs, importers, authored records, resources and source combinations.
- **Exact evidence files:** `crates/gameplay/content/src/content/native_gameplay.rs`, `crates/gameplay/content/src/content/production.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-content-promotion

- **Scope:** `crates/gameplay/content-promotion` (Cargo package).
- **Status:** NOT STARTED; 0 of 36 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-gameplay-content`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-gameplay-dispatch

- **Scope:** `crates/gameplay/dispatch` (Cargo package).
- **Status:** IN PROGRESS; 45 of 1647 tracked files have substantive analysis, including 9 complete files.
- **Entry points checked:** Selected real spell, NPC command, item/trade, death, listener and composed-admission routes.
- **Flows checked:** Cross-module qualification, transaction failure, retained command, retry/reconnect and observer lifetimes.
- **Declared local dependencies:** `oteryn-dev-client`, `oteryn-error-codes`, `oteryn-foundation`, `oteryn-gameplay-admission-evidence`, `oteryn-gameplay-character-recovery-fence`, `oteryn-gameplay-characters`, `oteryn-gameplay-combat`, `oteryn-gameplay-combat-rules`, `oteryn-gameplay-content`, `oteryn-gameplay-domain`, `oteryn-gameplay-durability`, `oteryn-gameplay-economy`, `oteryn-gameplay-encounter-rules`, `oteryn-gameplay-encounters`, `oteryn-gameplay-entities`, `oteryn-gameplay-foundation`, `oteryn-gameplay-housing`, `oteryn-gameplay-items`, `oteryn-gameplay-monster-ai`, `oteryn-gameplay-native-admission-source`, `oteryn-gameplay-node-files`, `oteryn-gameplay-npc`, `oteryn-gameplay-progression`, `oteryn-gameplay-quests`, `oteryn-gameplay-social`, `oteryn-gameplay-social-rules`, `oteryn-gameplay-spell`, `oteryn-gameplay-spell-rules`, `oteryn-gameplay-weather`, `oteryn-gameplay-world`, `oteryn-gameplay-world-change`, `oteryn-protocol-oteryn`, `oteryn-simulation-determinism`, `oteryn-world-bundle`, `oteryn-world-bundle-compiler`.
- **Findings:** 5 confirmed associated findings (DUR-003, SEC-001, GP-001, DUR-001, SEC-002); all associated IDs: DUR-003, SEC-001, GP-001, DUR-001, SEC-002, RNG-001.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Most dispatch adapters, unreviewed command families, full connection lifecycle, every resource/cancellation boundary and native end-to-end behavior.
- **Exact evidence files:** `crates/gameplay/dispatch/src/gameplay_transport/actor_conditions.rs`, `crates/gameplay/dispatch/src/gameplay_transport/cast_observer.rs`, `crates/gameplay/dispatch/src/gameplay_transport/channel_states.rs`, `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/admission.rs`, `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/death.rs`, `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/first_entry.rs`, `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/npc.rs`, `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/quest.rs`, `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/resume.rs`, `crates/gameplay/dispatch/src/gameplay_transport/connection/authority/composed.rs`, `crates/gameplay/dispatch/src/gameplay_transport/connection/routing/npc.rs`, `crates/gameplay/dispatch/src/gameplay_transport/connection/serve/command.rs`, `crates/gameplay/dispatch/src/gameplay_transport/connection/serve/setup.rs`, `crates/gameplay/dispatch/src/gameplay_transport/depot.rs`, `crates/gameplay/dispatch/src/gameplay_transport/depot_search.rs`, `crates/gameplay/dispatch/src/gameplay_transport/gameplay_drain.rs`, `crates/gameplay/dispatch/src/gameplay_transport/item_move.rs`, `crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs`, `crates/gameplay/dispatch/src/gameplay_transport/listener.rs`, `crates/gameplay/dispatch/src/gameplay_transport/loop_move.rs`, `crates/gameplay/dispatch/src/gameplay_transport/mail.rs`, `crates/gameplay/dispatch/src/gameplay_transport/mining/mod.rs`, `crates/gameplay/dispatch/src/gameplay_transport/mount_taming.rs`, `crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast.rs`, `crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs`, `crates/gameplay/dispatch/src/gameplay_transport/npc_command/premium_purchase.rs`, `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs`, `crates/gameplay/dispatch/src/gameplay_transport/npc_command/reward_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/player_death_items.rs`, `crates/gameplay/dispatch/src/gameplay_transport/player_trade.rs`, `crates/gameplay/dispatch/src/gameplay_transport/player_trade/command.rs`, `crates/gameplay/dispatch/src/gameplay_transport/player_trade/observation.rs`, `crates/gameplay/dispatch/src/gameplay_transport/player_trade/preflight.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/dark_trails.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/monk_virtues.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/nx6_wayfarer.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/player_death_items.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_rewards.rs`, `crates/gameplay/dispatch/src/gameplay_transport/pvp_death.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_admission.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/rewards.rs`, `crates/gameplay/dispatch/src/gameplay_transport/serving.rs`, `crates/gameplay/dispatch/src/gameplay_transport/spell_access_facts.rs`, `crates/gameplay/dispatch/src/gameplay_transport/weapon_proficiency_cast.rs`, `crates/gameplay/dispatch/src/gameplay_transport/world_item_cast.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-domain

- **Scope:** `crates/gameplay/domain` (Cargo package).
- **Status:** IN PROGRESS; 1 of 66 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** Selected death loss rule.
- **Flows checked:** Exemptions and container loss selection -> immutable obligation.
- **Declared local dependencies:** `oteryn-simulation-determinism`, `oteryn-test-support`.
- **Findings:** 1 confirmed associated findings (DUR-003); all associated IDs: DUR-003.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Remaining domain rules and source-profile combinations.
- **Exact evidence files:** `crates/gameplay/domain/src/domain/death.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-durability

- **Scope:** `crates/gameplay/durability` (Cargo package).
- **Status:** IN PROGRESS; 42 of 718 tracked files have substantive analysis, including 26 complete files.
- **Entry points checked:** Selected item movement, trade, mail, reward, quest, death, outbox and semantic transaction owners.
- **Flows checked:** Authority/fence -> locked state -> transaction/receipt -> replay/recovery.
- **Declared local dependencies:** `oteryn-error-codes`, `oteryn-gameplay-admission-evidence`, `oteryn-gameplay-character-recovery-fence`, `oteryn-gameplay-characters`, `oteryn-gameplay-combat-rules`, `oteryn-gameplay-content`, `oteryn-gameplay-domain`, `oteryn-gameplay-economy`, `oteryn-gameplay-encounter-rules`, `oteryn-gameplay-entities`, `oteryn-gameplay-foundation`, `oteryn-gameplay-housing`, `oteryn-gameplay-items`, `oteryn-gameplay-native-admission-source`, `oteryn-gameplay-progression`, `oteryn-gameplay-quests`, `oteryn-gameplay-social-rules`, `oteryn-gameplay-world-change`, `oteryn-protocol-oteryn`, `oteryn-simulation-determinism`, `oteryn-world-bundle`.
- **Findings:** 3 confirmed associated findings (DUR-003, SEC-003, DUR-001); all associated IDs: DUR-003, SEC-003, DUR-001, DUR-002.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Most durability files and migrations; full transformed schema, controlled concurrency, crash/ack-loss and broad custody/service paths.
- **Exact evidence files:** `crates/gameplay/durability/src/durability/bank.rs`, `crates/gameplay/durability/src/durability/character_authority.rs`, `crates/gameplay/durability/src/durability/character_death/commit.rs`, `crates/gameplay/durability/src/durability/character_death/items.rs`, `crates/gameplay/durability/src/durability/character_inbox.rs`, `crates/gameplay/durability/src/durability/character_progression.rs`, `crates/gameplay/durability/src/durability/character_revision_sequencer.rs`, `crates/gameplay/durability/src/durability/daily_reward_claim.rs`, `crates/gameplay/durability/src/durability/daily_reward_delivery.rs`, `crates/gameplay/durability/src/durability/daily_reward_delivery/picker.rs`, `crates/gameplay/durability/src/durability/db.rs`, `crates/gameplay/durability/src/durability/depot_move.rs`, `crates/gameplay/durability/src/durability/depot_search_retrieve.rs`, `crates/gameplay/durability/src/durability/house_disposition.rs`, `crates/gameplay/durability/src/durability/house_disposition/guildhall.rs`, `crates/gameplay/durability/src/durability/house_disposition/jobs.rs`, `crates/gameplay/durability/src/durability/house_rent.rs`, `crates/gameplay/durability/src/durability/house_sale.rs`, `crates/gameplay/durability/src/durability/house_transfer.rs`, `crates/gameplay/durability/src/durability/house_transfer/admission.rs`, `crates/gameplay/durability/src/durability/house_transfer/recovery.rs`, `crates/gameplay/durability/src/durability/item_craft.rs`, `crates/gameplay/durability/src/durability/item_loop_move.rs`, `crates/gameplay/durability/src/durability/item_transfer.rs`, `crates/gameplay/durability/src/durability/mail.rs`, `crates/gameplay/durability/src/durability/market.rs`, `crates/gameplay/durability/src/durability/market/book.rs`, `crates/gameplay/durability/src/durability/market/operations.rs`, `crates/gameplay/durability/src/durability/market/settlement.rs`, `crates/gameplay/durability/src/durability/npc_purchase.rs`, `crates/gameplay/durability/src/durability/npc_purchase/recovery.rs`, `crates/gameplay/durability/src/durability/player_death_items.rs`, `crates/gameplay/durability/src/durability/player_death_items/recovery.rs`, `crates/gameplay/durability/src/durability/player_trade.rs`, `crates/gameplay/durability/src/durability/player_trade/snapshot.rs`, `crates/gameplay/durability/src/durability/player_trade/storage.rs`, `crates/gameplay/durability/src/durability/public_projections/v2.rs`, `crates/gameplay/durability/src/durability/quest_state.rs`, `crates/gameplay/durability/src/durability/quest_state/storage.rs`, `crates/gameplay/durability/src/durability/quest_state/writer.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint/quest.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-economy

- **Scope:** `crates/gameplay/economy` (Cargo package).
- **Status:** IN PROGRESS; 4 of 28 tracked files have substantive analysis, including 4 complete files.
- **Entry points checked:** Player trade and selected market pure planners.
- **Flows checked:** Offer/accept lifecycle -> leaf/weight plan; market fee/history controls.
- **Declared local dependencies:** `oteryn-gameplay-domain`, `oteryn-test-support`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Remaining economy policies, bank/fee behavior and all durable effects.
- **Exact evidence files:** `crates/gameplay/economy/src/bank.rs`, `crates/gameplay/economy/src/market.rs`, `crates/gameplay/economy/src/player_trade.rs`, `crates/gameplay/economy/src/player_trade/lifecycle.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-encounter-rules

- **Scope:** `crates/gameplay/encounter-rules` (Cargo package).
- **Status:** IN PROGRESS; 4 of 21 tracked files have substantive analysis, including 2 complete files.
- **Entry points checked:** Selected admission, contribution and reward calculations.
- **Flows checked:** Room eligibility -> bounded plan; reward contribution -> collection.
- **Declared local dependencies:** `oteryn-gameplay-content`, `oteryn-protocol-oteryn`, `oteryn-simulation-determinism`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Encounter-specific rules and all source variations.
- **Exact evidence files:** `crates/gameplay/encounter-rules/src/boss/reward_chest.rs`, `crates/gameplay/encounter-rules/src/boss_room.rs`, `crates/gameplay/encounter-rules/src/thresholds.rs`, `crates/gameplay/encounter-rules/src/timers.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-encounters

- **Scope:** `crates/gameplay/encounters` (Cargo package).
- **Status:** IN PROGRESS; 2 of 93 tracked files have substantive analysis, including 2 complete files.
- **Entry points checked:** Selected boss/encounter owner functions.
- **Flows checked:** Admission and reward ownership checkpoints.
- **Declared local dependencies:** `oteryn-gameplay-combat`, `oteryn-gameplay-content`, `oteryn-gameplay-domain`, `oteryn-gameplay-durability`, `oteryn-gameplay-encounter-rules`, `oteryn-gameplay-entities`, `oteryn-gameplay-foundation`, `oteryn-gameplay-monster-ai`, `oteryn-gameplay-quests`, `oteryn-gameplay-source-critical-baseline`, `oteryn-gameplay-spell`, `oteryn-gameplay-world`, `oteryn-protocol-oteryn`, `oteryn-simulation-determinism`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Complete encounter scheduler, restart, group interactions and timer combinations.
- **Exact evidence files:** `crates/gameplay/encounters/src/bosses/room.rs`, `crates/gameplay/encounters/src/bosses/threshold_heals/source_threshold_heal.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-entities

- **Scope:** `crates/gameplay/entities` (Cargo package).
- **Status:** NOT STARTED; 0 of 86 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-gameplay-content`, `oteryn-gameplay-domain`, `oteryn-gameplay-foundation`, `oteryn-simulation-determinism`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-gameplay-foundation

- **Scope:** `crates/gameplay/foundation` (Cargo package).
- **Status:** NOT STARTED; 0 of 55 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-gameplay-domain`, `oteryn-protocol-oteryn`, `oteryn-simulation-determinism`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-gameplay-housing

- **Scope:** `crates/gameplay/housing` (Cargo package).
- **Status:** IN PROGRESS; 6 of 24 tracked files have substantive analysis, including 6 complete files.
- **Entry points checked:** Auction, move-out/notice and selected planning modules.
- **Flows checked:** Escrow -> bids/release/winner charge; notice deadline arithmetic.
- **Declared local dependencies:** `oteryn-gameplay-domain`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Full sale/disposition/reclaim and durable/source integration; executed pure checks do not prove DB behavior.
- **Exact evidence files:** `crates/gameplay/housing/src/house_auction.rs`, `crates/gameplay/housing/src/house_auction/eligibility.rs`, `crates/gameplay/housing/src/house_auction/value.rs`, `crates/gameplay/housing/src/house_move_out.rs`, `crates/gameplay/housing/src/house_scope_handoff.rs`, `crates/gameplay/housing/src/house_transfer.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-items

- **Scope:** `crates/gameplay/items` (Cargo package).
- **Status:** IN PROGRESS; 4 of 59 tracked files have substantive analysis, including 1 complete files.
- **Entry points checked:** Selected quick-loot, ammunition and container/item planners.
- **Flows checked:** Bounded item plan and guarded count transitions.
- **Declared local dependencies:** `oteryn-gameplay-content`, `oteryn-gameplay-domain`, `oteryn-test-support`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Remaining item services, all nested custody paths, expiry/transform/equipment combinations.
- **Exact evidence files:** `crates/gameplay/items/src/item_transfer.rs`, `crates/gameplay/items/src/mail.rs`, `crates/gameplay/items/src/quick_loot.rs`, `crates/gameplay/items/src/reward_bag.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-monster-ai

- **Scope:** `crates/gameplay/monster-ai` (Cargo package).
- **Status:** IN PROGRESS; 3 of 64 tracked files have substantive analysis, including 2 complete files.
- **Entry points checked:** Selected think, summon and defensive-heal entry points.
- **Flows checked:** Timer/current actor -> decision; summon registration -> compensation.
- **Declared local dependencies:** `oteryn-error-codes`, `oteryn-gameplay-combat`, `oteryn-gameplay-content`, `oteryn-gameplay-domain`, `oteryn-gameplay-durability`, `oteryn-gameplay-entities`, `oteryn-gameplay-foundation`, `oteryn-gameplay-spell`, `oteryn-gameplay-world`, `oteryn-simulation-determinism`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Complete perception, threat, path planning, spells, spawn/restart and every creature profile.
- **Exact evidence files:** `crates/gameplay/monster-ai/src/monster_owner_cycle.rs`, `crates/gameplay/monster-ai/src/monster_summon.rs`, `crates/gameplay/monster-ai/src/monster_summon/owner.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-native-admission-source

- **Scope:** `crates/gameplay/native-admission-source` (Cargo package).
- **Status:** IN PROGRESS; 4 of 81 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** HTTP operation envelope and actual public projection V2 publisher.
- **Flows checked:** Durable snapshot -> native encoder -> mTLS sink -> success-only acknowledgement/retry.
- **Declared local dependencies:** `oteryn-error-codes`, `oteryn-gameplay-admission-evidence`.
- **Findings:** 1 confirmed associated findings (SEC-003); all associated IDs: SEC-003.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Most admission/security/Platform contracts, all other transport methods, TLS and queue runtime behavior.
- **Exact evidence files:** `crates/gameplay/native-admission-source/src/descriptor.rs`, `crates/gameplay/native-admission-source/src/http1_mtls.rs`, `crates/gameplay/native-admission-source/src/public_projections.rs`, `crates/gameplay/native-admission-source/src/public_projections/v2.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-node-files

- **Scope:** `crates/gameplay/node-files` (Cargo package).
- **Status:** NOT STARTED; 0 of 4 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-gameplay-npc

- **Scope:** `crates/gameplay/npc` (Cargo package).
- **Status:** NOT STARTED; 0 of 17 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-gameplay-content`, `oteryn-gameplay-domain`, `oteryn-gameplay-economy`, `oteryn-gameplay-entities`, `oteryn-gameplay-world`, `oteryn-protocol-oteryn`, `oteryn-simulation-determinism`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-gameplay-progression

- **Scope:** `crates/gameplay/progression` (Cargo package).
- **Status:** IN PROGRESS; 5 of 43 tracked files have substantive analysis, including 5 complete files.
- **Entry points checked:** Selected hunting XP, loyalty, training and regeneration arithmetic.
- **Flows checked:** Current progression facts -> bounded projected values.
- **Declared local dependencies:** `oteryn-gameplay-content`, `oteryn-gameplay-domain`, `oteryn-simulation-determinism`, `oteryn-test-support`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Remaining progress rules, full persistence and all reward/source integrations.
- **Exact evidence files:** `crates/gameplay/progression/src/daily_reward.rs`, `crates/gameplay/progression/src/hit_training.rs`, `crates/gameplay/progression/src/loyalty.rs`, `crates/gameplay/progression/src/mana_training.rs`, `crates/gameplay/progression/src/regeneration.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-quests

- **Scope:** `crates/gameplay/quests` (Cargo package).
- **Status:** IN PROGRESS; 4 of 102 tracked files have substantive analysis, including 1 complete files.
- **Entry points checked:** Selected state transition and receipt/callback functions.
- **Flows checked:** Eligible state -> authored command -> shared durable transition/replay.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** All quest content, reset cycles, individual NPC adapters and source-specific rewards.
- **Exact evidence files:** `crates/gameplay/quests/src/quest/mod.rs`, `crates/gameplay/quests/src/quest/reward_callbacks.rs`, `crates/gameplay/quests/src/quest/reward_callbacks/state_ops.rs`, `crates/gameplay/quests/src/quest/world_callbacks/native.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-social

- **Scope:** `crates/gameplay/social` (Cargo package).
- **Status:** NOT STARTED; 0 of 56 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-gameplay-combat`, `oteryn-gameplay-content`, `oteryn-gameplay-domain`, `oteryn-gameplay-durability`, `oteryn-gameplay-entities`, `oteryn-gameplay-foundation`, `oteryn-gameplay-progression`, `oteryn-gameplay-social-rules`, `oteryn-gameplay-world`, `oteryn-ntp-clock`, `oteryn-protocol-oteryn`, `oteryn-simulation-determinism`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-gameplay-social-rules

- **Scope:** `crates/gameplay/social-rules` (Cargo package).
- **Status:** NOT STARTED; 0 of 22 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-gameplay-domain`, `oteryn-protocol-oteryn`, `oteryn-simulation-determinism`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-gameplay-source-critical-baseline

- **Scope:** `crates/gameplay/source-critical-baseline` (Cargo package).
- **Status:** NOT STARTED; 0 of 2 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-simulation-determinism`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-gameplay-spell

- **Scope:** `crates/gameplay/spell` (Cargo package).
- **Status:** IN PROGRESS; 7 of 83 tracked files have substantive analysis, including 2 complete files.
- **Entry points checked:** Qualified caster, actor conditions, selected preparation/gates.
- **Flows checked:** Loyalty/proficiency -> effective magic -> self/native/conjure checks.
- **Declared local dependencies:** `oteryn-gameplay-combat`, `oteryn-gameplay-combat-rules`, `oteryn-gameplay-content`, `oteryn-gameplay-domain`, `oteryn-gameplay-durability`, `oteryn-gameplay-entities`, `oteryn-gameplay-foundation`, `oteryn-gameplay-progression`, `oteryn-gameplay-source-critical-baseline`, `oteryn-gameplay-spell-rules`, `oteryn-gameplay-world`, `oteryn-protocol-oteryn`, `oteryn-simulation-determinism`, `oteryn-test-support`.
- **Findings:** 1 confirmed associated findings (GP-001); all associated IDs: GP-001.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** All ordinary/native spell families, target legality, cooldowns, Wheel and downstream mutations.
- **Exact evidence files:** `crates/gameplay/spell/src/spell/actor_conditions.rs`, `crates/gameplay/spell/src/spell/cast/gate.rs`, `crates/gameplay/spell/src/spell/cast/live_skill.rs`, `crates/gameplay/spell/src/spell/cast/prepare.rs`, `crates/gameplay/spell/src/spell/food_regeneration.rs`, `crates/gameplay/spell/src/spell/native_cast_commit.rs`, `crates/gameplay/spell/src/spell/owned_cast_facts.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-spell-rules

- **Scope:** `crates/gameplay/spell-rules` (Cargo package).
- **Status:** IN PROGRESS; 2 of 24 tracked files have substantive analysis, including 1 complete files.
- **Entry points checked:** Selected regeneration/rule wrappers.
- **Flows checked:** Current vocation/feeding state -> timing/effect policy.
- **Declared local dependencies:** `oteryn-gameplay-combat-rules`, `oteryn-gameplay-content`, `oteryn-simulation-determinism`, `oteryn-test-support`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Remaining rule tables and all caller/source combinations.
- **Exact evidence files:** `crates/gameplay/spell-rules/src/cast.rs`, `crates/gameplay/spell-rules/src/dispatch_plans/ordinary_combat.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-weather

- **Scope:** `crates/gameplay/weather` (Cargo package).
- **Status:** IN PROGRESS; 1 of 3 tracked files have substantive analysis, including 1 complete files.
- **Entry points checked:** Weather module production entry and transition logic.
- **Flows checked:** Calendar/source -> bounded weather state changes.
- **Declared local dependencies:** `oteryn-protocol-oteryn`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Separate tests, world integration, authored weather profiles and runtime scheduling.
- **Exact evidence files:** `crates/gameplay/weather/src/lib.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-world

- **Scope:** `crates/gameplay/world` (Cargo package).
- **Status:** IN PROGRESS; 10 of 73 tracked files have substantive analysis, including 6 complete files.
- **Entry points checked:** Movement pacing, relocation, floor changes and path kernel/labels/tiles.
- **Flows checked:** Movement -> legality -> bounded search -> relocation; world clock.
- **Declared local dependencies:** `oteryn-error-codes`, `oteryn-foundation`, `oteryn-gameplay-characters`, `oteryn-gameplay-content`, `oteryn-gameplay-content-promotion`, `oteryn-gameplay-domain`, `oteryn-gameplay-durability`, `oteryn-gameplay-entities`, `oteryn-gameplay-foundation`, `oteryn-gameplay-housing`, `oteryn-gameplay-items`, `oteryn-gameplay-node-files`, `oteryn-protocol-oteryn`, `oteryn-simulation-determinism`, `oteryn-world-bundle`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Remaining spatial/kernel code, creatures, world events, physics and cross-session races.
- **Exact evidence files:** `crates/gameplay/world/src/movement.rs`, `crates/gameplay/world/src/movement/pacing.rs`, `crates/gameplay/world/src/movement/path_budget.rs`, `crates/gameplay/world/src/movement/path_search.rs`, `crates/gameplay/world/src/movement/path_search/path_kernel.rs`, `crates/gameplay/world/src/movement/path_search/path_labels.rs`, `crates/gameplay/world/src/movement/path_search/path_tiles.rs`, `crates/gameplay/world/src/movement/relocate.rs`, `crates/gameplay/world/src/movement/source_floor_change.rs`, `crates/gameplay/world/src/world_clock.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-gameplay-world-change

- **Scope:** `crates/gameplay/world-change` (Cargo package).
- **Status:** NOT STARTED; 0 of 14 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-simulation-determinism`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-identity

- **Scope:** `crates/identity` (Cargo package).
- **Status:** NOT STARTED; 0 of 2 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-diagnostics`, `oteryn-foundation`, `oteryn-platform-contracts`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-input-actions

- **Scope:** `crates/input-actions` (Cargo package).
- **Status:** NOT STARTED; 0 of 7 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-input-platform

- **Scope:** `crates/input-platform` (Cargo package).
- **Status:** NOT STARTED; 0 of 7 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-input-actions`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-ntp-clock

- **Scope:** `crates/ntp-clock` (Cargo package).
- **Status:** REVIEWED; 2 of 2 tracked files have substantive analysis, including 2 complete files.
- **Entry points checked:** read_synchronized and synchronized conversion.
- **Flows checked:** Read-only adjtimex -> checked synchronization/uncertainty record.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Completed source review only; independent small-package report retained separately; native integrations not run.
- **Remaining:** Source review complete for this package. Actual target-kernel and dependent premium behavior were not exercised.
- **Exact evidence files:** `crates/ntp-clock/Cargo.toml`, `crates/ntp-clock/src/lib.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-platform-client

- **Scope:** `crates/platform-client` (Cargo package).
- **Status:** NOT STARTED; 0 of 21 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-diagnostics`, `oteryn-error-codes`, `oteryn-foundation`, `oteryn-platform-contracts`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-platform-contracts

- **Scope:** `crates/platform-contracts` (Cargo package).
- **Status:** NOT STARTED; 0 of 2 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-foundation`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-protocol-oteryn

- **Scope:** `crates/protocol-oteryn` (Cargo package).
- **Status:** IN PROGRESS; 2 of 580 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** Frame length, varint, envelope bounds and selected market command limits.
- **Flows checked:** Wire frame -> bounded borrowed payload -> direction-specific validation.
- **Declared local dependencies:** `oteryn-test-support`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Most of 314 Rust files, every message codec, actual network composition and fuzz/runtime campaigns.
- **Exact evidence files:** `crates/protocol-oteryn/src/lib.rs`, `crates/protocol-oteryn/src/market.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-renderer

- **Scope:** `crates/renderer` (Cargo package).
- **Status:** NOT STARTED; 0 of 18 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-client-domain`, `oteryn-foundation`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-session

- **Scope:** `crates/session` (Cargo package).
- **Status:** NOT STARTED; 0 of 115 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-protocol-oteryn`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-session-tcp

- **Scope:** `crates/session-tcp` (Cargo package).
- **Status:** NOT STARTED; 0 of 4 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-session`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-simulation-determinism

- **Scope:** `crates/simulation-determinism` (Cargo package).
- **Status:** NOT STARTED; 0 of 3 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; all associated IDs: RNG-001.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-synthetic-assets

- **Scope:** `crates/synthetic-assets` (Cargo package).
- **Status:** NOT STARTED; 0 of 2 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-foundation`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-task-supervisor

- **Scope:** `crates/task-supervisor` (Cargo package).
- **Status:** REVIEWED; 2 of 2 tracked files have substantive analysis, including 2 complete files.
- **Entry points checked:** Task registration, run guard, completion, heartbeat and watchdog primitives.
- **Flows checked:** Bounded registry -> async task -> outcome/cleanup; included tests read.
- **Declared local dependencies:** `oteryn-error-codes`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Completed source review only; independent small-package report retained separately; native integrations not run.
- **Remaining:** Source review complete for this package. Production integrations and scheduler stress were not executed.
- **Exact evidence files:** `crates/task-supervisor/Cargo.toml`, `crates/task-supervisor/src/lib.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-test-support

- **Scope:** `crates/test-support` (Cargo package).
- **Status:** NOT STARTED; 0 of 6 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-diagnostics`, `oteryn-foundation`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-world-bundle

- **Scope:** `crates/world-bundle` (Cargo package).
- **Status:** IN PROGRESS; 1 of 11 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** Selected bundle header/section parser and visitor bounds.
- **Flows checked:** Declared offset/length -> checked contiguous section -> visit.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Full compiler/loader, map content, malformed corpus and production-scale artifacts.
- **Exact evidence files:** `crates/world-bundle/src/bundle.rs`. Ranges and reviewers are in coverage-ledger.json.

## oteryn-pre-native-client-tests

- **Scope:** `tests/pre-native-client` (Cargo package).
- **Status:** NOT STARTED; 0 of 2 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-client`, `oteryn-platform-contracts`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-security-auth-tests

- **Scope:** `tests/security/auth` (Cargo package).
- **Status:** NOT STARTED; 0 of 2 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-client-runtime`, `oteryn-diagnostics`, `oteryn-foundation`, `oteryn-identity`, `oteryn-platform-client`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-architecture-check

- **Scope:** `tools/architecture-check` (Cargo package).
- **Status:** NOT STARTED; 0 of 29 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-dev-client

- **Scope:** `tools/dev-client` (Cargo package).
- **Status:** NOT STARTED; 0 of 11 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-client-runtime`, `oteryn-client-ui`, `oteryn-protocol-oteryn`, `oteryn-session`, `oteryn-session-tcp`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-game-ops-top

- **Scope:** `tools/oteryn-game-ops-top` (Cargo package).
- **Status:** NOT STARTED; 0 of 2 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-player-bots

- **Scope:** `tools/player-bots` (Cargo package).
- **Status:** NOT STARTED; 0 of 3 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-dev-client`, `oteryn-protocol-oteryn`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-synthetic-asset-compiler

- **Scope:** `tools/synthetic-asset-compiler` (Cargo package).
- **Status:** NOT STARTED; 0 of 2 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-synthetic-assets`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-synthetic-client-harness

- **Scope:** `tools/synthetic-client-harness` (Cargo package).
- **Status:** NOT STARTED; 0 of 40 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-client`, `oteryn-client-domain`, `oteryn-client-runtime`, `oteryn-client-simulation`, `oteryn-client-ui`, `oteryn-dev-client`, `oteryn-foundation`, `oteryn-input-actions`, `oteryn-protocol-oteryn`, `oteryn-renderer`, `oteryn-session`, `oteryn-synthetic-assets`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## oteryn-world-bundle-compiler

- **Scope:** `tools/world-bundle-compiler` (Cargo package).
- **Status:** NOT STARTED; 0 of 31 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** `oteryn-world-bundle`.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## .github

- **Scope:** `.github` (Supplemental domain).
- **Status:** IN PROGRESS; 1 of 75 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** Candidate/governance job in merge-group-gate.yml.
- **Flows checked:** Exact identity -> trusted base -> fail-closed lane selection -> candidate checkout.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Remaining workflow jobs, other workflows, repository protection state, runners and current check results.
- **Exact evidence files:** `.github/workflows/merge-group-gate.yml`. Ranges and reviewers are in coverage-ledger.json.

## agent-configuration-and-retained-evidence

- **Scope:** `.agents, .claude, .codex-report, qlane-meta` (Supplemental domain).
- **Status:** NOT STARTED; 0 of 113 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## content

- **Scope:** `content` (Supplemental domain).
- **Status:** NOT STARTED; 0 of 8030 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## deploy

- **Scope:** `deploy` (Supplemental domain).
- **Status:** NOT STARTED; 0 of 16 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## docs

- **Scope:** `docs` (Supplemental domain).
- **Status:** IN PROGRESS; 2 of 7379 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** Selected owning determinism and economy/death contract clauses.
- **Flows checked:** Contract -> implementation comparison for selected hypotheses.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** Selected static paths; critical findings independently verified where applicable. Full module verification incomplete.
- **Remaining:** Most contracts, design documents and retained historical evidence; none treated as current runtime proof.
- **Exact evidence files:** `docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md`, `docs/architecture/reviews/OTERYN_GAME_BANK0_ACCOUNT_BANK_BALANCE_DECISION_2026-09-30.md`. Ranges and reviewers are in coverage-ledger.json.

## imports

- **Scope:** `imports` (Supplemental domain).
- **Status:** NOT STARTED; 0 of 565 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## repository-root

- **Scope:** `.editorconfig, .gitattributes, .gitignore, AGENTS.md, CONTRIBUTING.md, Cargo.lock, Cargo.toml, LICENSE, LICENSE-ASSETS.md, README.md, SECURITY.md, TRADEMARKS.md, deny.toml, rust-toolchain.toml, rustfmt.toml, workspace-boundaries.toml` (Supplemental domain).
- **Status:** NOT STARTED; 0 of 16 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## rulesets

- **Scope:** `rulesets` (Supplemental domain).
- **Status:** NOT STARTED; 0 of 76 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## tests

- **Scope:** `tests` (Supplemental domain).
- **Status:** NOT STARTED; 0 of 12 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## tools

- **Scope:** `tools` (Supplemental domain).
- **Status:** NOT STARTED; 0 of 2681 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## vendor

- **Scope:** `vendor` (Supplemental domain).
- **Status:** NOT STARTED; 0 of 793 tracked files have substantive analysis, including 0 complete files.
- **Entry points checked:** None substantively traced in this pass.
- **Flows checked:** Not started.
- **Declared local dependencies:** No Cargo-local dependency edges for this entry.
- **Findings:** 0 confirmed associated findings; none established; this is not a clean bill of health.
- **Verification degree:** No behavioral review completed.
- **Remaining:** Entire module/domain remains for behavioral review, caller tracing and appropriate validation.

## Cross-module flows

| Flow | Result of this pass | Remaining validation |
|---|---|---|
| Quest → reward → inventory → database | Selected shared dialogue route traced through atomic participant/state/receipt/XP obligation and historical replay; no new defect established | Every authored reward/source and native interruption fixture |
| Trade → containers → economy → persistence | Reviewed leaf-only admission, frozen-dialogue mutation exclusion, ordered locks and reciprocal atomic custody; no dupe/loss established | Concurrent SQL scheduling, process/ack loss and all physical services |
| Combat → conditions → player state | GP-001 confirmed on selected effective-magic gates | Full native regression and remaining combat/condition paths |
| Player state → networking → client | Selected client connection/combat and envelope bounds inspected | Most presentation/codec/client integration remains unreviewed |
| Authentication → session → character lifecycle | Selected listener/admission/death paths checked; SEC-001 and DUR-003 independently traced | Broad auth/session lane remains blocked/incomplete |
| World → movement → creatures | Bounded path search, floor changes, pacing and relocation inspected | Full creature lifecycle, collisions, source maps and integration |
| NPC → dialogue → quest progress | Real command-7, retained cause, receipt locks, reconnect reconciliation and obligation refresh traced | All adapters/content and native retry executions |
| Server boot → config → runtime | Selected closed config limits, initialization, supervision, reset and publisher composition checked | Full boot/service integration and operational configuration |
| Death → loot/rewards → persistence | Selected creature reward settlement and full DUR-003 failure chain | Nested-inventory SQL reproduction and broad reward source coverage |

No row means the entire flow is proven correct. The current findings and identified counterguards are narrower than complete end-to-end coverage.

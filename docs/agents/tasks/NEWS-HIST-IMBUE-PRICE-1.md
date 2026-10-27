# NEWS-HIST-IMBUE-PRICE-1

```yaml
task_id: NEWS-HIST-IMBUE-PRICE-1
type: mechanic
title: Reconcile current imbuement application fees
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: work
owner: Codex root
created_at: 2026-10-27
updated_at: 2026-10-27
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/items/src/imbuement.rs
  - crates/gameplay/items/src/imbuement_tests.rs
  - crates/gameplay/items/src/imbuement_catalogue.rs
  - crates/gameplay/durability/src/durability/item_fee_burn_audit/imbuement_cause.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/imbuement.rs
  - apps/game-server/migrations/0298_imbuement_current_fees.sql
  - tools/content-migration/imbuement_catalogue.py
  - tools/content-schema/imbuement-authoring/**
  - rulesets/items/imbuements/**
  - content/world/*.json
  - docs/agents/tasks/NEWS-HIST-IMBUE-PRICE-1.md
```

## Outcome

The existing imbuement fee/receipt owners charge the current official application fees, preserving the separate single-slot shrine removal price. Account access, creator Premium checks, and shrine access are unchanged.

## Reference parity

| Behaviour | Canary 47dfd51f | tibia.com news | Oteryn before | Oteryn after |
|---|---:|---:|---:|---:|
| Basic application | 5,000 | 7,500 (news 8396) | 5,000 | 7,500 |
| Intricate application | 30,000 | 60,000 (news 8396) | 30,000 | 60,000 |
| Powerful application | 200,000 | 250,000 (news 8396) | 200,000 | 250,000 |
| Clear one shrine slot | 15,000 | No new shrine price; news 8396 prices the separate cleansing item at 30,000 | 15,000 | 15,000 |

## Acceptance criteria

- [x] Unit fee rules cover all three application tiers and cleansing.
- [x] The native dispatch apply and clear path asserts the updated debits.
- [x] Migration 0298 advances existing PostgreSQL receipt constraints.
- [x] No account, Premium, shrine, fee-owner, or receipt-owner behavior changes.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this changes fixed fee values inside the existing fenced atomic owner; it introduces no authority, session, recovery, or controller boundary.

## Cloud validation (historical; superseded by local qualification in IMBUE-FINAL.md)

- PASS: `cargo test --locked -p oteryn-gameplay-items --lib imbuement --quiet` (4 passed).
- PASS: `cargo test --locked -p oteryn-gameplay-durability --lib imbuement_cause --quiet` (4 passed).
- PASS: `python3 -m unittest discover -s tools/content-schema/imbuement-authoring -p 'test_*.py'` (255 passed).
- PASS: `cargo fmt --all -- --check`.
- PASS: `python3 tools/agents/validate_governance.py` and agent policy tests (129 passed).
- PASS: `oteryn-world-bundle-compiler pin-refresh .` completed; no World pin changed.
- WARNING: native dispatch test compiled and selected the real route but returned route-less because `OTERYN_TEST_POSTGRES_ADMIN_URL` is not configured.
- WARNING: full `python3 tools/content-migration/regenerate_content.py` was invoked as required but its predecessor fetch cannot run because this checkout has no `origin`; the affected imbuement generators and pin refresh were then run directly and passed.

## Local repair scope

- Correct the unsupported 30,000-gold single-slot shrine fee: news 8396 prices the separate cleansing item, not shrine removal. News 7907 and 8442 could not be verified (HTTP 403); they are not evidence for a price change.
- Keep migration 0298 as the only new migration. Preserve immutable historical rows with `NOT VALID` checks that enforce current fees on every new receipt and fee burn.
- Decode historical V1 imbuement fee events at their original tier prices; the production writer and PostgreSQL still require current prices for new writes.
- Restore the exact two-change-mint resource-bound assertion, update the dispatch offer test, and add a focused PostgreSQL migration regression.
- Additional owned repair paths: `crates/gameplay/durability/src/durability/item_fee_burn_audit/purchase_cause.rs`, `crates/gameplay/durability/src/durability/item_fee_burn_audit/tests/imbuement_cause_tests.rs`, `crates/gameplay/dispatch/src/gameplay_transport/imbuement_tests.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/imbuement_fees_migration.rs`, and `IMBUE-FINAL.md`.

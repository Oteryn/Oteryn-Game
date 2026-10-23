# ITEM-FORGE-1 — exaltation forge rules and durable storage

```yaml
task_id: ITEM-FORGE-1
title: "Exaltation forge rules and durable storage"
mode: IMPLEMENT
status: IN_PROGRESS
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
base_sha: 43914f1836ac9ec96397b36a39c5e141edfe0ea0
branch: claude/happy-davinci-ijz3wy-item-forge-1
needs_migration: true
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY
MERGE_AUTHORITY: CP_ONLY
```

## Current scope — 2026-10-23

This scope supersedes the historical preparation/slot instructions in the base record.
The current owner authorizes local edits only: no staging, commits, publication,
workflow/repository-policy edits, external writes or dependency downloads.
ADR-0022 places new pure rules in `oteryn-gameplay-items` (L3), extending the
existing domain tier/dust types. L4 tier storage stays in the existing durability
forge owner. No new crate or third-party dependency is needed.

## Dependencies (re-verified on origin/main `43914f183`)

Every named dependency was checked with `git log --oneline origin/main --grep <ID>`
and a bounded `rg` over `crates/`, `apps/` and archived task records. Status is
source qualification, not a claim that a dormant backend has a playable caller.
A final origin/main refresh at `4a5d6c0f7` adds only protocol fuzz tests and a House
fixture-path repair; the forge dependency/code/migration cut is unchanged.

| Dependency | Actual main state and source evidence |
|---|---|
| ITEM-LOOP-1 | MERGED #2190; live `dispatch/gameplay_transport/loop_move.rs`; later #2290/#2301 extend pickup/nested use. |
| ITEM-MOVE-2a | Required carried-item path MERGED within #2190; `equipment_move.rs` and nested inventory owners exist. Historical PARTIAL rows predate that completion. |
| FORGE-1a | MERGED #1691; migration 0059 and `durability/character_forge_dust.rs` own the sole Character dust balance/ledger. |
| GOLD-FEE-2 | MERGED #1781/#1850; native fee/bank debit in `durability/item_fee_burn.rs`. Forge fee causes are still absent. |
| GAME-AI-01 runtime | MERGED foundation; `monster-ai/ai.rs`, `ai_think.rs`, `ai_monster_melee.rs`. Creature forge income remains a later child. |
| ATTACK-1b | MERGED attack foundation; superseded/extended by ATTACK-2 #2117, live `dispatch/ordinary_combat.rs`. |
| SPELL-D3 | MERGED foundation (#1355/#1403); live cast composition in `dispatch/native_combat_cast.rs`. |
| PREMIUM-1 | MERGED #2110; native Premium gate, with later runtime consumers (#2119). |
| MARKET-1 | Skeleton/checkpoint #2134 only; tier wares remain BLOCKING for MARKET-TIER-1, outside this pass. |
| IMBUE_FORGE0 acceptance | ACCEPTED by #2209 (`e2da9805d`), ACCEPT_BATCH owner item 5. |
| DEPOT-1 | MERGED #2334; migration 0151 and `durability/character_depot.rs`; dormant custody backend, Inbox still migration 0076. No duplicate storage. |
| ITEM-IMBUE-1 | MERGED #2333; migration 0150, dormant `durability/imbuement.rs`. Retained rows, including zero-time rows, remain occupied. |
| FORGE-CONTENT-1 | Genuinely UNMERGED/BLOCKING: #2293 stages reference packets only; `rulesets/items/exaltation-forge/index.json` is READY_UNPOPULATED, and promotion modules expose authoring metadata rather than executable recipes. |

## Existing mechanic (extend, never duplicate)

- Domain `forge.rs`: ItemTier, supplied-fact fusion/transfer/convergence checks,
  D1230 dust-limit quotes and checked 60-dust/3-sliver, 50-sliver/1-core conversions.
- Domain `forge_dust.rs`: capped Character balance and limit progression.
- Durability `forge.rs`: raw item-tier reader and formerly private tier DDL;
  `character_forge_dust.rs`/`character_forge_dust_audit.rs`: dust ledger, native
  DustLimit reserve/commit/reconcile/history. Its D1230 schema is still private.
- Content forge289/3332 promotion modules: authoring profiles only.
- No forge dispatch handler, FORGE_V1 capability or admitted executable recipe.

## Delivered in this pass

The blocking-content exception permits pure rules and own dormant storage only.
New L3 checked conversions produce standard fusion/transfer quotes and plans:
class caps, gold/dust/core costs, 50/65% success, optional 50% loss protection,
source burn/downgrade, deterministic supplied-roll outcomes and failure spending.
Reference prices remain reference data; no current Content binding is fabricated.
Dust arithmetic reuses the existing Character asset. Slivers/cores are native
items under IMBUE_FORGE0 §9; projected counts are not a second durable wallet.

Migration 0156 (main's highest was 0152; owner requires >=0156) installs the
existing item-tier model and an immutable, automatically written storage audit.
Tier and audit commit/rollback together; identity cannot be reassigned, tier zero
has no row, and reload preserves the value. Runtime/control have SELECT only.
This raw storage audit is not a ForgeCause occurrence receipt, resource ledger,
session witness or DUR-03 economic operation. No protocol/domain/resource IDs
are consumed; the assigned 0680–0686 range remains available for runtime work.

## Owned paths

- `crates/gameplay/items/src/forge.rs`, `forge/`, `forge_tests.rs`, `forge_property_tests.rs`
- `crates/gameplay/items/src/lib.rs` (one wiring line)
- `crates/gameplay/durability/src/durability/forge.rs`
- `apps/game-server/migrations/0156_forge_item_tiers.sql`
- `apps/game-server/tests/support/forge_cases.rs` (use installed schema)
- `apps/game-server/src/lifted_tests/auto/forge_postgres_cases.rs` (one automatic mount)
- This task card. `MECHANICS_STATUS.md` does not exist on main; no new index is created.

## Acceptance

- [x] Complete pure standard rules, class caps, failure costs and exact draw boundaries.
- [x] Table-driven reference parity and arithmetic properties use the existing test-support property module.
- [x] Installed tier storage, immutable atomic storage audit, rollback/reload and competing compare-and-set tests.
- [x] Existing tier-reader corruption, identity, uniqueness and FK assertions still pass against the installed migration.
- [x] Changed packages compile; strict all-target Clippy, library tests, scoped PostgreSQL, formatting and repository policy pass.
- [ ] Complete fenced economic writer, native ForgeCause receipt and dispatch E2E (blocked, outside the permitted subset).

## Out of scope / Unfinished

- Admitted Content recipes/revision binding; convergence and success bonus rules.
- Atomic fusion/transfer economic owner: Character-generation fence, reservation,
  native ForgeCause item/dust/core/fee audit, terminal revision refusal, stable
  RNG outcome, replay/reconciliation and bounded combined forge history (1,000).
- Durable sliver/core counts must derive from native item custody, not duplicate
  depot/inbox storage or numeric wallets. Main-backpack placement and no-imbuement
  checks must be re-read under the real writer's locks before resources are spent.
- The existing DustLimit D1230 SQL proposal is not installed by this pass. Its
  100+20*n / maximum4600 pure policy is retained; the numbered dust migration and
  native operation remain an inseparable later bundle. No old 225-row limit is
  silently reinterpreted as 225 paid upgrades.
- FORGE_V1/wire/dispatch, forge USE admission, creatures/income, tier effects and
  Market tier wares. No runtime entry point is enabled; dispatch E2E remains blocked with the economic writer.
- Independent exact-head review/CI and publication remain with the integrator.

## Validation

All final Cargo checks used the queued wrapper via the non-login shell. Early
route-less direct-Cargo runs are not PostgreSQL evidence; the wrapper reruns below
executed the real PostgreSQL cases. No whole PostgreSQL suite was run.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --locked -p oteryn-gameplay-items -p oteryn-gameplay-durability -p oteryn-game-server --all-targets --quiet -- -D warnings` | PASS; unchanged third-party dependency warnings remain outside task crates |
| `cargo test --locked -p oteryn-gameplay-items --lib --quiet` | 51 passed |
| `cargo test --locked -p oteryn-gameplay-items --lib forge --quiet` | 8 passed, including three property groups |
| `cargo test --locked -p oteryn-gameplay-durability --lib forge --quiet` | 1 passed |
| `cargo test --locked -p oteryn-game-server --lib forge --quiet` | 19 passed |
| `cargo test --locked -p oteryn-game-server --lib forge_storage_ --quiet` | 3 PostgreSQL cases passed, 9.28s |
| `cargo test --locked -p oteryn-game-server --test character_authority_postgres forge_cases:: --quiet` | 4 passed (3 PostgreSQL cases), 16.47s |
| `python3 tools/repository/validate_repository_policy.py` | PASS |
| `python3 tools/agents/validate_governance.py` and `python3 -m unittest discover -s tools/agents/tests` | PASS; 124 governance tests |
| Whole-diff checklist, `git diff --check`, workspace constructor/match/impl search | PASS for the delivered subset; dispatch/economic-writer acceptance remains open |

The new enum/struct/standard-trait implementation sites were searched across
`apps/`, `crates/`, `tools/`; all new consumers are confined to the new L3 module
and its sibling tests. No wire enum or schema cause code changed. The caller
of the removed private tier DDL now uses the installed migration; no references
remain. No tests were deleted, ignored or weakened. No load-contention failures
were observed. Full CI and independent review are not claimed.

## Reference parity

Price/core rows below cite Canary's immutable
[data/scripts/systems/item_tiers.lua](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data/scripts/systems/item_tiers.lua).
Here t indexes fusion result tier, transfer donor tier for gold, and transfer
result tier for core count. `forge_reference_prices_and_transfer_indices`
asserts every row and exercises each legal standard transfer.

| Class / t | Gold | Transfer cores indexed by t | Test |
|---|---:|---:|---|
| 1 / 1 | 25,000 | 1 | `forge_reference_prices_and_transfer_indices` |
| 2 / 1 | 750,000 | 1 | `forge_reference_prices_and_transfer_indices` |
| 2 / 2 | 5,000,000 | 1 | `forge_reference_prices_and_transfer_indices` |
| 3 / 1 | 4,000,000 | 1 | `forge_reference_prices_and_transfer_indices` |
| 3 / 2 | 10,000,000 | 2 | `forge_reference_prices_and_transfer_indices` |
| 3 / 3 | 20,000,000 | 3 | `forge_reference_prices_and_transfer_indices` |
| 4 / 1 | 8,000,000 | 1 | `forge_reference_prices_and_transfer_indices` |
| 4 / 2 | 20,000,000 | 2 | `forge_reference_prices_and_transfer_indices` |
| 4 / 3 | 40,000,000 | 5 | `forge_reference_prices_and_transfer_indices` |
| 4 / 4 | 65,000,000 | 10 | `forge_reference_prices_and_transfer_indices` |
| 4 / 5 | 100,000,000 | 15 | `forge_reference_prices_and_transfer_indices` |
| 4 / 6 | 250,000,000 | 25 | `forge_reference_prices_and_transfer_indices` |
| 4 / 7 | 750,000,000 | 35 | `forge_reference_prices_and_transfer_indices` |
| 4 / 8 | 2,500,000,000 | 50 | `forge_reference_prices_and_transfer_indices` |
| 4 / 9 | 8,000,000,000 | 60 | `forge_reference_prices_and_transfer_indices` |
| 4 / 10 | 15,000,000,000 | 85 | `forge_reference_prices_and_transfer_indices` |

Other reference rules use [Canary config.lua.dist](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/config.lua.dist),
[Player forge operations](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/creatures/players/player.cpp),
and [Tibia's forge FAQ](https://www.tibia.com/support/?entryid=224&subtopic=gethelp).
The accepted [D1230 override](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6226138153)
supersedes historical Canary dust-cap values; it is a declared policy difference.

| Rule | Value | Table-driven test |
|---|---|---|
| Tier caps by class | 1→1, 2→2, 3→3, 4→10 | `forge_reference_class_caps_and_shape_refusals` |
| Base fusion / success core | 50% / 65%, costs 0 / 1 core | `forge_reference_success_protection_and_failure_costs` |
| Tier protection core | 1 core, 50% chance to retain B on failure; independent of boost | `forge_reference_success_protection_and_failure_costs` |
| Fusion/transfer dust | 100 each, resources consumed even when fusion fails | `forge_reference_prices_and_transfer_indices`, `forge_reference_success_protection_and_failure_costs` |
| Standard fusion | A gains one tier, B consumed; failure preserves A and downgrades B (burn at0) | `forge_reference_success_protection_and_failure_costs` |
| Standard transfer | 100% success; donor>=2, receiver0, same class; burns donor and sets receiver=donor-1 | `forge_reference_prices_and_transfer_indices`, `forge_missing_resources_invalid_draws_and_transfer_prerequisites` |
| Dust conversion | 60 dust→3 slivers | `forge_reference_conversions_and_d1230_dust_cap` |
| Core conversion | 50 slivers→1 core | `forge_reference_conversions_and_d1230_dust_cap` |
| D1230 dust cap (override) | stages0/1/224/225: 100/120/4580/4600, next costs25/26/249/none | `forge_reference_conversions_and_d1230_dust_cap` |

## Historical evidence

Earlier private qualification and obsolete slot instructions remain in the
[base task record](https://github.com/Oteryn/Oteryn-Game/blob/43914f183/docs/agents/tasks/ITEM-FORGE-1.md).
They do not qualify this pass or override its current scope and validation rules.
The owning accepted brief remains
`docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md`
§8–10 and its accepted D1230 dust override.

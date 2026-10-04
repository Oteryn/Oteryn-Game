# OTV2-20261003-prof-shape-1b

```yaml
task_id: OTV2-20261003-prof-shape-1b
title: "PROF-SHAPE-1b: perk modification dust SPEND shape"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/prof-shape-1b-20261003
pr: "see the FREEZE_SHA entry"
head_sha: "exact frozen head in the FREEZE_SHA entry"
final_head_sha: "exact frozen head in the FREEZE_SHA entry"
owner: claude-code-session-01FZCPgXLttJ2W3fpZVncuKJ (hard worker)
created_at: 2026-10-03
updated_at: 2026-10-04
execution_policy: continuous_progress
migration_lease: "0060"
owned_paths:
  - apps/game-server/migrations/0060_proficiency_modification_value_lines.sql
  - apps/game-server/src/durability/character_proficiency_modification.rs
  - apps/game-server/src/durability/character_proficiency.rs
  - apps/game-server/tests/support/character_proficiency_modification_postgres_cases.rs
  # Added by the MQ 37165988723 repair: the 0060 pairing guard breaks this file's bare spends.
  - apps/game-server/tests/support/character_forge_dust_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20261003-prof-shape-1b.md
public_contracts: []
depends_on: ["#1685 merged", "#1691 merged"]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

PROFICIENCY-1B §7.1, §7.2 and §9, option (a) (CP decision): the dust SPEND is admitted; the orb
BURN is not, and orb ranks stay `NOT_ADMITTED`.

- Migration 0060: `..._line_no_ledger` replaced by `..._line_no_orb_burn` (`orb_cost = 0`) and a
  deferred guard on lines and `proficiency` dust entries: a receipt with dust has exactly one line
  and exactly one equal `SPEND` entry; no dust, no entry; every `proficiency` entry is a `SPEND`
  with its line.
- `character_proficiency_modification`: a dust cost is no longer refused `NotAdmitted`; the dust
  row is locked after the modification rows (§7.2), an insufficient balance is refused
  `InsufficientDust` before any write, and the spend (deterministic entry and transaction ids per
  occurrence) commits with the receipt; replay pays once. `ProficiencyModificationUsage` checks the
  registered maxima before any write. History verification also proves the line and its entries.
- Merge Queue run 37163256159 (#1704): the 0060 guard named both tables' NEW fields in one CASE,
  which PL/pgSQL resolves for every row, so every dust entry insert failed; each branch now reads
  only its own table's fields.
- Merge Queue run 37165988723 (#1704): the 0060 pairing guard refused the 0059 ledger cases'
  bare `proficiency` spends, so those three cases now switch off that one trigger in their own
  database (the pairing stays covered by the modification cases). The orphan-spend case also
  advances the balance row, so the 0060 guard, not the 0059 chain guard, is what refuses it.
- Carry-overs from #1685: P2 4174729829 (reconciliation takes the occurrence lock, shared) and P2
  4174729831 (PROF-1 commit and reconciliation return `ConflictingOccurrence` for an occurrence
  with a modification terminal) fixed.
- DUR-03 §15 and §39.3 amendments; IMBUE-FORGE-0 §9 amendment; registry rows `DUR03-RL-03-PROF`,
  `DUR03-RL-06-PROF-PARTICIPANTS` and `DUR03-RL-06-PROF-EFFECT-WORK-UNITS`.

Left to follow-up PROF-ORB-BURN-1: the orb BURN shape, ORB_RANK and the `DUR03-RL-01/02-PROF` rows.

## Validation

- `cargo fmt --all --check`, `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`,
  `cargo test --locked -p oteryn-game-server --quiet`: pass
- PostgreSQL cases: run by CI on PostgreSQL 17.6 (the local server is 17.11 and the version
  assertion stays unchanged).
- `python3 tools/agents/validate_governance.py`, `git diff --check`: pass

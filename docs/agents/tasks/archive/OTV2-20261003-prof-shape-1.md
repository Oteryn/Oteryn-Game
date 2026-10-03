# OTV2-20261003-prof-shape-1

```yaml
task_id: OTV2-20261003-prof-shape-1
title: "PROF-SHAPE-1a: perk modification rows, lines, terminal records and operations (migration 0055)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/prof-shape-1
pr: "the PR of branch claude/prof-shape-1"
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: claude-code-session-01FZCPgXLttJ2W3fpZVncuKJ (hard worker, control plane session_013KJX6mv8LQveCKKXYgAX94)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
leases: [migration 0055 (D353)]
owned_paths:
  - apps/game-server/migrations/0055_character_proficiency_modification.sql
  - apps/game-server/src/durability/character_proficiency_modification.rs
  - apps/game-server/src/durability/character_proficiency.rs
  - apps/game-server/src/durability/character_proficiency_codec.rs  # D398
  - apps/game-server/src/durability/character_revision_sequencer.rs  # D398
  - apps/game-server/src/durability/character_authority.rs
  - apps/game-server/src/durability/content_activation.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/domain/weapon_proficiency.rs
  - apps/game-server/tests/character_proficiency_modification_postgres.rs
  - apps/game-server/tests/support/character_proficiency_modification_postgres_cases.rs
  - apps/game-server/tests/durability_postgres.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY1_PERK_MODIFICATION_AND_CATALYSTS_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261003-prof-shape-1.md
depends_on: ["PROF-1 (0032, merged)", "PROFICIENCY-1B (#1662, merged)", "TIMED-PROF-0C (#1667)", "PROF-SHAPE-CONTENT-1 (#1670)"]
blocks: [PROF-SHAPE-1b, PROF-SHAPE-WIRE-1]
```

## Outcome

PROFICIENCY-1B §4-§10 and §12, split at the value ledger (D393, D394):
FORGE-1's dust balance and ledger do not exist yet, and the orb BURN is a full DUR-03
item shape. This slice ships everything else; PROF-SHAPE-1b adds the dust SPEND and orb BURN.

- **Migration 0055:** the `perk_modification` cause in the receipt and track line CHECKs (with a
  track-line arm that changes no track value); `game_character_proficiency_modifications`
  (never deleted, cleared state, one level per slot, retention-checked); immutable
  `..._modification_lines` with per-operation direction CHECKs, the bound shaping revision and
  costs spent exactly; `..._modification_terminals` outside the revision chain; the retained
  shaping set with `game_proficiency_shaping_activate` under the §8 exclusive lock. The line CHECK
  `..._line_no_ledger` pins dust and orb costs to 0 until PROF-SHAPE-1b replaces it. The shared
  progression guard is not replaced: `perk_modification` receipts are rows it already counts.
- **Domain:** §3.3 admission, the §5 checks 5-8 in order, `proficiency_shaping` draws, the active
  rule, the rate cap and the reconciliation hold bound.
- **Writer:** replay by occurrence before any write, revision binding with a terminal
  `REVISION_CHANGED`, the receipt with one track line and one modification line, sequenced through
  `RevisionSlot` with one retry under the same occurrence (§6.4). Operations with a dust or orb
  cost and ORB_RANK answer `NOT_ADMITTED`.
- **PROF-1 writer:** `MODIFIED_LEVEL` (§10) and `MIGRATION_CLEAR` lines (§4.3).
- **Integrity:** `verify_character_integrity` checks every chain, row, cost, binding and terminal.
- Registry rows PROF1-RL-01/02 and PROF1B-RL-02..05. DUR-03 §15/§39.3, IMBUE-FORGE-0 §9 and the
  `DUR03-RL-*-PROF` rows belong to the value shapes, so PROF-SHAPE-1b writes them.

## Validation

- `cargo fmt --all --check`; `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`
- `cargo test --locked -p oteryn-game-server --quiet` (lib)
- `cargo test --locked -p oteryn-game-server --test character_proficiency_modification_postgres` on
  local PostgreSQL 17.11 (the harness's 17.6 assert relaxed locally only, not committed)
- `python tools/agents/validate_governance.py`; `git diff --check`

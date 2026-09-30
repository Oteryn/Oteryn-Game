# OTV2-20260930-imbue-forge0

```yaml
task_id: OTV2-20260930-imbue-forge0
title: "IMBUE-FORGE-0 imbuements and the Exaltation Forge"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-imbue-forge-0
pr: "the PR named in the #162 FREEZE_SHA entry"
base_sha: a6a054e6
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20260930-imbue-forge0.md
  - docs/architecture/reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

IMBUE-FORGE-0 decides imbuements and the Exaltation Forge at full Global parity (owner direction,
2026-09-30).

- **Imbuing:** shrine USE under `IMBUE_V1`; 24 types × 3 tiers; up to 3 slots, one per category;
  always succeeds, no protection charm (ruling R2); materials burned, fee burned (with I1).
- **Imbuement state:** an item-state row per (ItemInstance, slot), not Character state; time
  ticks while equipped and in fight outside a protection zone (non-aggressive types: while
  equipped); exact runtime counter checkpointed every 60 s of ticking and at every boundary, so a
  node crash returns at most 60 s (ruling R1); audited expiry; effects via GAME-ABILITY-01 stages.
- **Forge:** item tier as an item-state row (`PRESERVE_INSTANCE`); fusion, convergence fusion,
  transfer, convergence transfer and conversions with Global outcomes, costs spent on failure,
  server-seeded rolls stored in the receipt; closed `ForgeCause`.
- **Resources:** forge dust as a per-Character non-item asset with a ledger and a limit (100-225);
  slivers and exalted cores as items; dust from influenced and fiendish kills and sliver loot
  (with I2).
- **Market:** default state excludes tier and imbuements; imbued items never listed; tiered wares
  wait for MARKET-TIER-1.
- **Owner questions I1** (fees, D178) and **I2** (value sources, D208) are open.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: DUR-03 §14-§18, §39; the composition decision; the gold fee decision; BANK-0,
  BANK-FEE-0, MARKET-0, NPC-0, ITEM-USE-0, ATTACK-0, CHARM-0 (repository texts).
- `DERIVED`: the Tibia manual (`characters.md` §5.1.8, `interface.md`, `world.md`); Canary
  `04b83b51` as `OTS_HYPOTHESIS_ONLY`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. IMBUE-1, FORGE-1 and MARKET-TIER-1 need persistence and economy
review; IMBUE-RT-1, FORGE-CREATURE-1 and TIER-EFFECT-1 combat review; the wire children protocol
review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence, economy, combat, protocol).
- [ ] Protected Merge Queue integration.
- [ ] Owner answers to I1 and I2 recorded on #162 before IMBUE-1 and FORGE-1 are allocated.

## Excluded scope

- Code, migrations and content; the Stash, the house shrine, Store cores, the Soul Pit, Find
  Fiend.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the draft authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the draft authoring tree.
- `git diff --cached --check`: clean.
- Codex round-1 repair (PR #1415, 4 findings): forge revision binding (§10), failed-checkpoint
  suspension and committed Featherweight capacity (§4.3, §5.1), the quest predicate on every direct
  shrine imbuement (§3), `ScrollCreate` producer eligibility (§7).

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Amendments follow the control-plane rule (#162 5912405163): pending on acceptance. Amended:
  MARKET-0 §3.1, the gold fee decision §4.4. DUR-03 §15, §17, §18, §39.3 and composition rule 1
  are left to IMBUE-1 and FORGE-1.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: draft authored; awaiting architect review and publication
status: completed
branch: claude/arch-imbue-forge-0
owner_action_required: "I1: imbuing and forge gold fees; I2: forge value sources"
blocker: null
next_action: "architect reviews, commits, opens the PR and posts I1 and I2 to the owner"
```

# OTV2-20260930-wheel-gem0

```yaml
task_id: OTV2-20260930-wheel-gem0
title: "WHEEL-GEM-0 Wheel of Destiny gems: the Gem Atelier"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-wheel-gem-0
pr: "the PR named in the #162 FREEZE_SHA entry"
base_sha: "origin/main at branch creation"
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_WHEEL_GEM0_GEM_ATELIER_DECISION_2026-09-30.md
  - docs/architecture/OTERYN_WHEEL_OF_DESTINY_STATE_CONTRACT_CANDIDATE_V1.md
  - docs/agents/tasks/archive/OTV2-20260930-wheel-gem0.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

WHEEL-GEM-0 decides Wheel of Destiny gems at full Global parity (owner direction, 2026-09-30),
the part WHEEL-0 deferred. It integrates after WHEEL-0.

- **Gems as items:** unrevealed gems and fragments are ordinary stackable items; drops are loot
  content (bosses now, fiendish creatures after the Forge registry); NPC offers already exist.
- **Revealed gems** are Character build state (`game_character_gems`, cap 250 `PARITY_PENDING`),
  with per-character mod grades, an Atelier state row and receipts, written by
  `commit_character_atelier` on CHAR-REV-SEQ-1 with expected `atelier_revision`.
- **Reveal:** one gem unit BURN under the closed `GemAtelierCause`, the fee under D177 and
  BANK-FEE-0, mods drawn under the RNG purpose `gem_reveal` and stored in the receipt.
- **Dismantle** (fragment MINT), **switch domain**, **lock** and **grade up** (fragment BURN plus
  fee); crushing with a crusher as an item-only transaction.
- **Vessels** in the Wheel writer with expected `wheel_revision`, anywhere while eligible
  (ruling R1); a gem in a vessel cannot switch domain (ruling R2).
- **Effects** through W-FX-1's contribution and the ability hooks; resonance activates mods;
  revelation mastery adds domain points; each Grade IV mod adds a Wheel point.
- **Wire:** capability `WHEEL_GEM_V1` (requires `WHEEL_V1`), `ATELIER_QUERY`, `ATELIER_INTENT`,
  and a vessel field on `WHEEL_INTENT`; numbers reserved on #162 at allocation.
- **Owner questions** (answered 2026-09-30): G1 a) the reveal, domain switch and grade up fees
  (D178); G2 b) dismantling, crushing and the 8 free initial gems as value sources (D208).

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: the Wheel state candidate, QUEST-STATE-0 (CHAR-REV-SEQ-1), DUR-03, the composition
  decision, the gold fee decision (D174-D178, D208), content loot and trade files.
- `DERIVED`: WHEEL-0, BANK-FEE-0, ITEM-USE-0 (candidates); the Tibia manual (`characters.md`
  §5.1.7); Canary `src/creatures/players/components/wheel/` as OTS hypothesis only.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. GEM-1 and GEM-VESSEL-1 need persistence and economy review; GEM-FX-1
combat review; GEM-WIRE-1 protocol review; GEM-R content review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (Character state, persistence, economy, protocol, combat).
- [ ] Protected Merge Queue integration after WHEEL-0.
- [x] Owner answers to G1 and G2 recorded on #162 (2026-09-30).

## Excluded scope

- Code, migrations and content; fiendish drops, the amber crusher, presets, vocation change; the
  DUR-03, gold fee and WHEEL-0 amendments, which the children write.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the draft authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the draft authoring tree.
- `git diff --cached --check`: clean.
- Codex round-1 repair (PR #1414, 3 findings): ruleset revision compatibility §5.3
  (`WHEELGEM0-RV`, R3), protected reveal RNG root (`WHEELGEM0-RNG`), runtime projection refresh
  (`WHEELGEM0-RT`); validators re-run PASS.
- Codex round-2 repair (PR #1414, 1 finding): replay resolves the receipt by occurrence first
  and returns the outcome under its bound ruleset revision (`WHEELGEM0-RP`, §5.2, §5.3, §6);
  validators re-run PASS.
- Owner answers (2026-09-30, #162): G1 a) (all three Atelier gold fees, as in Tibia) and G2 b)
  (dismantling, crushing and the 8 free initial gems, as in Tibia) made binding; the initial gems
  modelled as one idempotent `GEM_INIT` Atelier write (`WHEELGEM0-INIT`, `WHEELGEM0-RL-12`).

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Amendments follow the control-plane rule (#162 5912405163): pending on acceptance. Amended: the
  Wheel state candidate §5 (pointer).
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: draft authored; awaiting architect review and publication
status: completed
branch: claude/arch-wheel-gem-0
owner_action_required: null
blocker: null
next_action: "architect freezes the head with the owner answers and requests review"
```

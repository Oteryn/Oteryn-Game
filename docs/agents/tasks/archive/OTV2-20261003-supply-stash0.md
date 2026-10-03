# OTV2-20261003-supply-stash0

```yaml
task_id: OTV2-20261003-supply-stash0
title: "SUPPLY-STASH-0: the Supply Stash"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/supply-stash0-decision-20261003
pr: null
base_sha: 3d8c14d0f6326e278cf09e97acd2453b6af31926
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_SUPPLY_STASH0_SUPPLY_STASH_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-supply-stash0.md
public_contracts: []
depends_on:
  - "DEPOT-0 acceptance (locker, depot capacity guard; gates STASH-1)"
  - "BANK-RET-0 (ECONOMY_LEDGER retention profile; gates STASH-1)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Allocation D293 (#1622, control plane) answers the Stash deferrals of DEPOT-0, MARKET-0 and
IMBUE-FORGE-0 with SUPPLY-STASH-0.

- **Storage.** The Stash is DUR-03 §18 value: units per Item definition, per Character + World,
  with an immutable ledger and replayable operations in the BANK-0 idiom.
- **Eligibility.** Market wares in default state, stackable or not; no timed row, tier,
  imbuement, contents or Store source.
- **Operations.** Stow one whole item from the backpack or a depot box (Premium, fail closed until
  PREM-3); withdraw up to 20 fresh outputs (no Premium). Both are `CONVERSION` lines under
  `StashConversionCause`, at a locker only.
- **Capacity.** `ceil(quantity / 100)` per definition counts toward the depot limit.
- **Deferred.** Inbox stow, stow all, container stow, loot routing, filters, Market and imbuing
  sources, Steward, Cyclopedia summary, depot search.

## Architecture and source of truth

- `PROVEN`: DEPOT-0 §3-§5; DUR-03 §18 and §39.1; BANK-0 §3-§5; MARKET-0 §3.1; TIMED-ITEM-0 §4;
  OFFLINE-0 §6.
- `CIPSOFT_OFFICIAL`: Tibia manual `world.md` line 47, `characters.md` line 120, `controls.md`
  lines 83-85.
- `TIBIAWIKI_STRUCTURED`: "Your Supply Stash" rev 1057911, Depot rev 1057910.
- `OTS_HYPOTHESIS_ONLY`: Canary `04b83b5` stash storage, eligibility and withdraw.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. STASH-1 implements and tests the conversions with persistence and
value review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence, value conservation, protocol).

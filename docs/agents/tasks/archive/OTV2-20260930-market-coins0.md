# OTV2-20260930-market-coins0

```yaml
task_id: OTV2-20260930-market-coins0
title: "MARKET-COINS-0 Tibia Coins on the Market"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-market-coins-0
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
  - docs/architecture/reviews/OTERYN_GAME_MARKET_COINS0_TIBIA_COINS_ON_THE_MARKET_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20260930-market-coins0.md
  - docs/architecture/reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md
  - docs/architecture/OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: OTV2-MARKET-COINS
external_repositories: []
```

## Outcome

MARKET-COINS-0 decides Tibia Coin offers on the World Market at full Global parity (MARKET-0
owner answer Q2b; owner direction, 2026-09-30).

- **Authority:** Platform's Wallet holds every coin, its transferable part and the coin ledger.
  Game holds the book, the fee and the gold (MARKET-0, DUR-03) and never mints or burns a coin.
- **Coin ware:** a virtual ware `oteryn:market.tibia_coin`, one book per World, never an Item.
- **Custody:** a coin sell offer, and an accept of a coin buy offer, first hold transferable coins
  on Platform (T1 intent row, HOLD, T2 commit or ABORT with a tombstone); every other fill,
  match, cancel and expiry is Game-only and queues SETTLE or RELEASE in a transactional outbox,
  delivered at least once and applied once by Platform.
- **Failure:** Platform down refuses only new holds (`COINS_UNAVAILABLE`); settlements wait. A
  daily reconciler compares holds with Platform; differences are corrected by compensation only.
- **Gates:** same 2% Market fee (no new fee source); no Premium gate for coins; junior and
  same-Account refused; coin offers count toward the 100-offer limit.
- **Wire:** capability `MARKET_COINS_V1` (number reserved on #162 at allocation), two results,
  a display-only balance line.
- **Owner questions C1-C3** (open): which coins are Tibia Coins; the transferable and chargeback
  rule; when coin trading goes live.

No code, migration, content or Platform change is made.

## Architecture and source of truth

- `PROVEN`: MARKET-0, BANK-0, the gold fee decision (D178, D238), DUR-03 §18, §26, §34, §35 and
  §38, the Store catalog owner decision, PROD-ENTITLEMENTS-01, PREMIUM-DELIVERY-0 §3; Platform
  Wallet (`Oteryn-Platform` `c914564`, read only: MODULE_CATALOG, DATA_OWNERSHIP, ADR 0016).
- `DERIVED`: the Tibia manual (`store.md`, `controls_trading.md`, `accounts.md`,
  `products.md`); Canary `04b83b5` `src/game/game.cpp` market functions (OTS hypothesis only).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. MKTCOIN-1 needs persistence, economy and security review;
MKTCOIN-CONTRACT-1 and MKTCOIN-E2E-1 security and cross-repository review; MKTCOIN-WIRE-1
protocol review; the instruction payload privacy review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (security, economy, persistence, protocol, privacy,
      cross-repository integration).
- [ ] Protected Merge Queue integration.
- [ ] Owner answers to C1-C3 recorded on #162 before MKTCOIN-CONTRACT-1 and before activation.
- [ ] Platform acceptance of §5 (MKTCOIN-P) before MKTCOIN-E2E-1.

## Excluded scope

- Code, migrations, content and any Platform edit; Store purchases, coin gifts, payments,
  prices, the world transfer rule, statistics, Tournament Coins.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the draft authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the draft authoring tree.
- `git diff --cached --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Amendments follow the control-plane rule (#162 5912405163): pending on acceptance. Amended:
  MARKET-0 §3.1 and §10, the Store catalog owner decision (follow-up 1). The Platform Wallet
  contract change is an external dependency (MKTCOIN-P), not edited here.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: draft authored; awaiting architect review and publication
status: completed
branch: claude/arch-market-coins-0
owner_action_required: "C1-C3: coin identity, transferable and chargeback rule, activation"
blocker: null
next_action: "architect reviews, commits, opens the PR and posts C1-C3 to the owner"
```

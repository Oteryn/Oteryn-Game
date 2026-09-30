# OTV2-20260930-market-coins0

```yaml
task_id: OTV2-20260930-market-coins0
title: "MARKET-COINS-0 Tibia Coins on the Market"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-market-coins-0
pr: 1412  # oteryn/oteryn-game#1412
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
- **Failure:** Platform down refuses only steps needing a new hold or claim
  (`COINS_UNAVAILABLE`); settlements wait. A daily reconciler compares holds with Platform's
  acknowledged snapshot net of pending outbox instructions; differences are corrected by
  compensation only, and only after Platform reports the instruction terminal (`APPLIED` or
  `REJECTED`, `MKTCOIN0-DISPOSITION`). Platform bounds SETTLE by hold, claim and caps, and
  retains keys, tombstones and receipts 90 days. Any restore crossing coin activity keeps coin
  trading closed until a reviewed manual reconciliation (`MKTCOIN0-RESTORE`, owner-confirmed,
  #162 Q9a).
- **Gates:** same 2% Market fee (no new fee source); no Premium gate for coins; junior and
  same-Account refused; coin offers count toward the 100-offer limit.
- **Wire:** capability `MARKET_COINS_V1` (number reserved on #162 at allocation), two results,
  a display-only balance line.
- **Owner questions C1-C3** (answered): C1 a, the Wallet's Oteryn Coins with a transferable part;
  C2 b, every coin transferable (chargeback stays with Platform and never reverses a Market
  trade: confirmed by the owner, A2); C3 a, test Worlds first, production after Platform accepts its
  payment policy.

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
- [x] Owner answers to C1-C3 recorded on #162 (2026-09-30).
- [ ] Platform acceptance of §5 (MKTCOIN-P) before MKTCOIN-E2E-1.

## Excluded scope

- Code, migrations, content and any Platform edit; Store purchases, coin gifts, payments,
  prices, the world transfer rule, statistics, Tournament Coins.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the draft authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the draft authoring tree.
- `git diff --cached --check`: clean.
- Codex round-1 repair (PR #1412, 4 findings): reconciliation nets pending outbox instructions;
  discriminated ware key amends MARKET-0 §4 and §7; Platform settlement fence with CLAIM-bound
  destinations and SETTLE caps; key, tombstone and receipt retention and restore rule.
- Codex round-2 repair (PR #1412, 3 findings): ABORT carries the exact unused amount and a
  settlement watermark, and Platform defers it behind that key's SETTLEs (`MKTCOIN0-ABORT-ORDER`);
  typed SETTLE sides `maker_offer_id` and `taker_operation_id` with PLACE/ACCEPT bindings
  (`MKTCOIN0-SETTLE-SIDES`); task record PR and closeout state.
- Owner answers (2026-09-30, #162 Q10-Q12): C1 a, C2 b (chargeback stated assumption), C3 a;
  Platform requirements kept as MKTCOIN-P contract dependencies; validators re-run PASS.
- Codex round 3 (#1412, 2 P1, 0 P2): STATUS and reconciliation now cover claim keys and their
  settled/aborted state (§5, §7); Game-side restore defined as a coordinated cut with ENUMERATE
  (MKTCOIN0-GAME-RESTORE, §5; MKTCOIN-P dependency); validators re-run PASS.
- Codex round 4 (#1412, 4 P1, 0 P2): the round-3 automated ENUMERATE restore protocol is
  withdrawn; any Game or Platform restore crossing coin activity keeps coin trading on the World
  closed until a reviewed manual reconciliation (MKTCOIN0-RESTORE, §5; MKTCOIN-P dependency);
  validators re-run PASS.
- Owner confirmation A2 (2026-09-30, #162 5919339646): the C2 chargeback rule is confirmed (§5,
  §12, §13); validators re-run PASS.
- Owner confirmation Q9a (2026-09-30, #162): `MKTCOIN0-RESTORE` is owner-confirmed, including
  "a restore older than Platform's retention stays closed until the owner decides" (header, §5).
- Codex round 5 (final batched round; #1412, 1 P1, 0 P2): 4149455120 P1 expired or cap-deferred
  SETTLE needs a terminal fence before compensation: fixed. New `MKTCOIN0-DISPOSITION` (§5):
  every instruction ends `APPLIED` or `REJECTED` (final, recorded by `instruction_id`, never
  applied after rejection); `DEFERRED` never expires and a cap-deferred SETTLE ends only by a
  Platform operator applying or rejecting it; `STATUS` reports each instruction's disposition;
  the deliverer resends until a terminal disposition (`MKTCOIN0-RL-14` is now an escalation, not
  a stop); compensation touching an instruction's effect starts only after it is terminal (§7).
  Validators re-run PASS.

## Closeout

- PR: oteryn/oteryn-game#1412 (opened; in review). Exact frozen head: the #162 FREEZE_SHA entry.
  Merge commit/result: squash merge of #1412.
- Amendments follow the control-plane rule (#162 5912405163): pending on acceptance. Amended:
  MARKET-0 §3.1, §4, §7 and §10, the Store catalog owner decision (follow-up 1). The Platform Wallet
  contract change is an external dependency (MKTCOIN-P), not edited here.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: PR oteryn/oteryn-game#1412 in review (Codex round-5 final batched repair authored)
status: completed
branch: claude/arch-market-coins-0
owner_action_required: null
blocker: null
next_action: "#162 freezes the updated head of #1412, validates it and routes independent review"
```

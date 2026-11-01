# OTV2-20261101-gem-atelier

```yaml
task_id: OTV2-20261101-gem-atelier
title: Gem Atelier carried inventory and Supply Stash parity
type: mechanic
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/gem-atelier
pr: 2795
base_sha: 38641fecd4558a4a14e6b15b3874110ffa2dff19
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: gem-atelier-lead
created_at: 2026-11-01
updated_at: 2026-11-01
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/progression/src/atelier.rs
  - crates/gameplay/progression/src/atelier/
  - crates/gameplay/durability/src/durability/character_atelier/
  - crates/gameplay/durability/src/durability/item_fee_burn.rs
  - crates/gameplay/durability/src/durability/item_fee_burn/
  - crates/gameplay/durability/src/durability/item_fee_burn_audit.rs
  - crates/gameplay/durability/src/durability/item_fee_burn_audit/
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier.rs
  - crates/gameplay/dispatch/src/gameplay_transport/atelier.rs
  - apps/game-server/migrations/0744_character_atelier_carried_stash.sql
  - apps/game-server/migrations/0745_character_atelier_carried_coins.sql
  - apps/game-server/migrations/0746_character_atelier_carried_outputs.sql
  - apps/game-server/migrations/0747_character_atelier_audit_bounds.sql
  - docs/contracts/protocol-oteryn/v1/atelier-v1.md
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
  - docs/contracts/resource-limits/entries/1684-ATELIER-RL-ASSET-LINES.json
  - docs/contracts/resource-limits/entries/1685-ATELIER-RL-EFFECT-WORK.json
  - docs/contracts/resource-limits/entries/1686-ATELIER-RL-VALUE-LINES.json
  - docs/contracts/resource-limits/entries/1687-ATELIER-RL-AUDIT-BYTES.json
  - docs/agents/tasks/OTV2-20261101-gem-atelier.md
  - docs/agents/evidence/OTV2-20261101-gem-atelier-parity.md
public_contracts:
  - docs/contracts/protocol-oteryn/v1/atelier-v1.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Complete P07's missing carried-container reveal and fragment payments through the existing Atelier command, custody owner, revision sequencer and transaction receipt. Audit every donor rule and retain named unresolved decisions rather than silently changing accepted contracts.

## Architecture and source of truth

- PROVEN: admission main `38641fecd4558a4a14e6b15b3874110ffa2dff19`.
- PROVEN: Crystal reference `201189f871d23597b82caf88fd611454a2ddefb8`; `player_wheel.cpp:1218-1250,1514-1597`, `player.cpp:5304-5356,5821-5844`.
- PROVEN: META policy 3.1.0 bound to `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`; existing Atelier decision and protocol contract remain applicable.
- AUTHORING: the task lead is the sole publisher of `chatgpt/gem-atelier`. Isolated file ownership covers donor/rules, durable custody, and protocol tests. Local Git networking is unavailable; the lead selects the bounded connector-compatible new-candidate route, with exact predecessor and successor readback. Each candidate requires its own validation.

## Acceptance criteria

- [x] A complete sourced parity table records DONE, PARTIAL, MISSING and named native adaptations; unresolved decisions remain explicit.
- [x] Reveal consumes a carried nested gem, never a stashed gem.
- [x] Grade upgrades consume carried fragments first, then Supply Stash, in one fenced transaction with gold and the mutation.
- [x] Refusal, replay, stale-session and durable readback cases exercise the real dispatch path.
- [x] Required focused validation and number collision audit are recorded for the implementation; the immutable-head closeout is recorded in the PR.

## Reference parity

| Behaviour | Reference value/source | Oteryn | Test |
|---|---|---|---|
| Nested reveal | Crystal `player_wheel.cpp:1218-1250`; recursive inventory `player.cpp:5821-5844` | Existing Atelier custody owner with bounded recursive carried selection | Mounted Atelier PostgreSQL wire cases |
| Stash-paid grades | Crystal `player_wheel.cpp:1514-1597`; inventory-before-stash `player.cpp:5304-5356` | Existing stash ledger plus same-transaction Atelier proof | Mounted Atelier PostgreSQL grade/replay cases |
| Gold sources | Crystal `game.cpp:2862-2969`, recursive carried coins then bank | Existing item fee owner extended only for the three Atelier causes | Nested coins, more than20 sources and bank readback |
| Dismantle destinations | Crystal `game.cpp:2782-2805`, `player.cpp:4932-5047` | Ordinary carried breadth-first destinations and compatible partial stacks | Nested output, split yield and complete-capacity refusal |

The complete41-row table is retained in `docs/agents/evidence/OTV2-20261101-gem-atelier-parity.md` and the draft PR. Pending source-priority/scope decisions stay visible rather than being counted as complete Crystal parity.

## High-risk authority/recovery qualification

Applicable: session-generation-fenced durable item, bank, stash and Atelier writes. Existing transaction guard and receipt own authority and replay. Negative cases independently change generation, source quantity, source ownership and command identity; refusal must preserve all original values. The mounted qualification covers direct dispatch, committed readback, same-byte replay after content removal, changed eligibility and locks, later actions and process restart. SQL probes reject historical or mismatched stash, coin, output and enlarged-audit evidence. Ordered migration reapplication compares 26 retained function definitions and committed split-output history. PUBLIC revocation and runtime grants are separately checked from the migration source.

## Excluded scope

No production activation, new gem store, protected-branch writes, merge, review requests, heavy-tests label, donor source redistribution, merged migration edits, audit-ledger edits or changes to unrelated mechanics.

## Implementation / findings

The existing native command424 and capability148 carry the resource-selection extension. No new client choice is needed for recursive selection or the automatic stash remainder. Baseline PostgreSQL cases first reproduced the missing nested reveal and stash debit; those same wire cases now pass in focused checkpoints. Four additive migrations bind the extended resources and output shapes to the existing immutable records. Fresh paid actions reload current equipment facts under the existing action lane before refreshing Wheel-derived vitals.

Atelier audit limits are separately registered:540 assets,1620 work units,2 value lines,196608 payload bytes and262144 envelope bytes. Other fee causes retain their old limits. The decoder preserves legacy oversize error categories and requires a positively identified Atelier fee before using a larger limit. Migration0747 binds larger stored envelopes to current matching Atelier evidence.

Questions Q1-Q7 are on [draft PR2795](https://github.com/Oteryn/Oteryn-Game/pull/2795): second basic modifier pool; dismantle guards; occupied-vessel rotation;225-gem bound; managed obtain destinations; coin-change delivery; and a separately negotiated seven-balance panel projection. None has an ANSWER at the latest channel read. Existing equipment profiles also prohibit equipping these resources; this observed native boundary is documented without forging a positive fixture or inventing an owner ruling.

## Validation

Tests run on an isolated checkout on the owner-authorized computer because the scratch authoring environment has no Rust toolchain. Exact commands, measured results and remaining candidate qualification are recorded in the parity evidence. The final immutable head and its readback/qualification are recorded in the PR closeout comment; this source file does not embed its own eventual commit hash.

## Self-review

Reviewed the existing owner chain, bounded carried graph, source ordering, immutable receipt replay, complete resource rollback, per-cause audit limits and SQL runtime privileges. A full package run caught a legacy decoder error-category regression, which was fixed without weakening the existing assertion and gained a focused regression case. The full rerun passed604 durability tests plus4 doctests; dispatch passed1907 tests with16 pre-existing ignored tests, and all27 mounted Atelier PostgreSQL cases passed.

## Independent review

User requests a draft PR without review requests. Three peer lanes reviewed donor parity, durable source/SQL constraints, and protocol/PostgreSQL coverage. The decoder correction received an independent read-only peer check. A coordinator-dispatched frozen-head review has not been requested or claimed; integration review remains with the coordinator.

## PR and closeout

Draft PR [2795](https://github.com/Oteryn/Oteryn-Game/pull/2795) was opened immediately after the first task-record commit. The lead is the only publisher. Used numbers: migrations0744-0747 and resource-limit1684-1687. Capability272-273, command484-487 and mechanic/offer624-627 remain unused; capability148 and commands308/309/424 with Wheel161 are reused. Pending questions remain under Left out. No merge, readiness transition, review request or heavy-tests label is authorized by this task.

## Context checkpoint

Production and tests are frozen. The isolated qualification checkout is synchronized with main38641fe; merge-driver regeneration and the complete component/PostgreSQL gates passed. The parity result is32 DONE /8 PARTIAL /1 MISSING, withQ1-Q7 open. The lead publishes through the bounded API route, reads back the exact successor and records its unchanged-head qualification in the draft PR. Coordinator answers and integration review are the next scope-changing inputs; no ready/merge transition is requested.

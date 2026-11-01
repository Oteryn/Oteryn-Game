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
pr: null
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
  - crates/gameplay/durability/src/durability/item_fee_burn_audit.rs
  - crates/gameplay/durability/src/durability/item_fee_burn_audit/
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/
  - apps/game-server/migrations/0744_character_atelier_carried_stash.sql
  - docs/contracts/protocol-oteryn/v1/atelier-v1.md
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

- [ ] A complete sourced parity table records DONE, PARTIAL, MISSING and accepted deviations.
- [ ] Reveal consumes a carried nested gem, never a stashed gem.
- [ ] Grade upgrades consume carried fragments first, then Supply Stash, in one fenced transaction with gold and the mutation.
- [ ] Refusal, replay, stale-session and durable readback cases exercise the real dispatch path.
- [ ] Required focused validation and number collision audit are recorded on the tested candidate.

## Reference parity

| Behaviour | Reference value/source | Oteryn | Test |
|---|---|---|---|
| Nested reveal | Crystal `player_wheel.cpp:1218-1250`; recursive inventory `player.cpp:5821-5844` | Existing Atelier custody owner, implementation in progress | Mounted Atelier PostgreSQL wire case |
| Stash-paid grades | Crystal `player_wheel.cpp:1514-1597`; inventory-before-stash `player.cpp:5304-5356` | Existing stash ledger plus Atelier transaction proof, implementation in progress | Mounted Atelier PostgreSQL grade/replay cases |
| Gold sources | Crystal `game.cpp:2862-2969`, recursive carried coins then bank | Existing item fee owner, audit in progress | Nested coins and bank readback |

The complete table will be retained in `docs/agents/evidence/OTV2-20261101-gem-atelier-parity.md` and the draft PR.

## High-risk authority/recovery qualification

Applicable: session-generation-fenced durable item, bank, stash and Atelier writes. Existing transaction guard and receipt own authority and replay. Negative cases will independently change generation, source quantity, source ownership and command identity; refusal must preserve all original values. The final qualification covers direct dispatch, committed readback and repeated requests.

## Excluded scope

No production activation, new gem store, protected-branch writes, merge, review requests, heavy-tests label, donor source redistribution, merged migration edits, audit-ledger edits or changes to unrelated mechanics.

## Implementation / findings

The existing native command424 and capability148 carry the required actions. No new client choice is needed. Existing negative tests explicitly cover the two missing custody paths and will become regression cases. Explicit historical rule deviations are being raised through the PR question channel before changing those rules.

## Validation

Focused, component/PostgreSQL and exact-head validation: pending implementation. No test pass is claimed. Tests run on an isolated checkout on the owner-authorized computer because this workspace has no Rust toolchain.

## Self-review

Pending complete bounded diff and transaction guard review.

## Independent review

User requests a draft PR without review requests. Peer source and test review is in progress; integration review remains with the coordinator.

## PR and closeout

One draft PR on `chatgpt/gem-atelier`; retain pending questions under Left out. Reserved numbers: migrations0744-0747, capability272-273, command484-487, mechanic/offer624-627, resource-limit1684-1687. Reuse existing ids wherever possible. No merge is authorized by this task.

## Context checkpoint

Next action: open the draft PR after this first commit, publish rule questions there, and implement the path-disjoint custody and regression-test changes.

# OTV2-20260929-d3-6-corpse-decay

```yaml
task_id: OTV2-20260929-d3-6-corpse-decay
title: D3-6 corpse decay - owner-timer decay family and DUR-03 DECAY_RETIRE as N+1 one-item steps
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/d3-6-corpse-decay
issue: 162
allocation: "#162 control plane child D3-6 of the merged D3 decision (worker 'Oteryn: sol combat lead')"
base_sha: d4d5e2c4
head_sha: b3398ecd0aac87579ecf2645c8766be238c95b39
final_head_sha: b3398ecd0aac87579ecf2645c8766be238c95b39
final_head_frozen_at: null
owner: "Oteryn: sol combat lead (claude-code-session-01U1WRHgL9X8RbuiG1pwczrF)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0015_corpse_decay_retire.sql
  - apps/game-server/src/durability/item_decay_retire.rs
  - apps/game-server/src/durability/item_decay_retire_audit.rs
  - apps/game-server/src/durability/item_mint_audit.rs                      # shared: oneof tag 5 only
  - apps/game-server/src/durability/mod.rs                                  # shared: module declarations + linkage test
  - apps/game-server/src/foundation/owner_timer.rs
  - apps/game-server/tests/corpse_decay_postgres.rs
  - apps/game-server/tests/support/corpse_decay_postgres_cases.rs
  - apps/game-server/tests/support/corpse_transfer_postgres_cases.rs        # shared: pub(crate) visibility only
  - apps/game-server/tests/character_authority_postgres.rs                  # shared: #[path] include only
  - docs/contracts/game-events/v1/native_one_item_transaction.proto         # shared: additive tag 5 + two messages
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json                      # shared: type 2 note only
  - docs/agents/tasks/active/OTV2-20260929-d3-6-corpse-decay.md
public_contracts:
  - DUR-03
  - ANL-01
  - GAME-ITEM-01
  - FND-03
depends_on: [D3-1, D3-2, D3-4]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A corpse decays at the durable absolute deadline `materialized_at + 60 s` of its own
`CORPSE_MATERIALIZATION` receipt (decision
`reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md` §4.6/§4.7, D135/D136;
DUR-03 §39.1/§39.4):

- `DECAY_RETIRE` runs as N+1 separate one-item logical transactions: one per live entry of the corpse
  container, then the corpse itself, admitted only once zero live entries remain. Each step retires one
  item to `RETIRED`, quantity 0, no location, fits the default `DUR03-RL-01`/`-RL-06` shape (1 item,
  1 location line, 1 participant, 3 work units; no new resource row) and commits its own receipt and its
  own admitted `CorpseDecay` audit event.
- Never early: refused before the deadline by the database clock in Rust admission and, authoritatively,
  by the commit-time guard (`clock_timestamp()`). The owner timer fires at the deadline, never later.
- Resumable from durable state only: the Ground-only, `lifecycle = 1` recovery query lists every live
  corpse of a scope with its deadline; a partly drained corpse looks like a fresh one and the drain issues
  only the remaining steps. There is no in-memory progress state.
- An entry picked up before decay reaches it is no longer in the corpse and is never retired.

## Design

- `foundation/owner_timer.rs`: `CorpseDecayFamily` (registered maximum `COMBAT01-CORPSES-PER-SCOPE` = 64,
  target-less key, `DeadlineState`), `CorpseDecayOccurrence { corpse_item_instance_id }`,
  `CORPSE_DECAY_POLICY`, `corpse_decay_due` (maps the durable deadline onto the owner's monotonic clock;
  a passed deadline is due now) and `schedule_corpse_decay` (inherits every lane check).
- `durability/item_decay_retire.rs`: `freeze_decay_retire` / `commit_decay_retire` /
  `reconcile_decay_retire` (the MINT idiom: semantic pass, recovery fence, cause advisory lock, RL-08
  budget, frozen event bytes, identity-reuse check, scope-assignment + node-incarnation fence), the
  recovery query `read_corpse_decay_schedule`, and `retire_decayed_corpse` (the N+1 drain). The fence is
  the current owner generation of the corpse's scope, which may be later than the corpse's own.
- Keys: one receipt per retired `ItemInstanceId`, forever (the corpse's step is thereby keyed by its unique
  `CORPSE_MATERIALIZATION` receipt). Reservations are keyed by (item, owner generation): a later generation
  reserves afresh after a restart/handoff because the ended generation can never commit (fence), so at most
  one receipt per item can exist.
- Migration 0015 (0013/0014 untouched): `game_item_decay_retire_reservations` and
  `game_item_decay_retire_receipts`; a deferred `SECURITY DEFINER` consistency guard (corpse receipt row
  lock, deadline = `materialized_at + 60000` and reached by `clock_timestamp()`, item retired to 0 with no
  location, quantity evidence, reservation, pending audit event, the removal proven from guarded evidence of
  this physical transaction, and for the corpse step no live entry). `game_item_ground_removal_proven`
  gains, as the D3-4 record assigned to D3-6, one first clause admitting exactly the corpse's own
  `DECAY_RETIRE` receipt of the same physical transaction; the rest of its 0014 body (the D134 corpse
  refusal for TRANSFER) is verbatim. `game_item_corpse_entry_removal_proven` and
  `game_item_instance_change_proven` likewise only gain one admitting clause.
- Audit: `OneItemTransactionV1` oneof tag 5 `decay_retire` = `OneItemDecayRetireV1 { before = 1,
  after = 2, ground = 3, corpse_entry = 4 (OneItemCorpseSourceV1), cause = 5 (OneItemCorpseDecayV1
  { corpse_item_instance_id = 1, deadline_unix_ms = 2 }), runtime_scope_ownership_generation = 6 }`.
  Server-originated: no session, CommandRef or causation in the envelope. The MINT destination widening
  (`corpse_container_entry` = 5, D3-2) and the TRANSFER source widening (`corpse_source` = 7, D3-4) were
  already present; D3-6 adds only lossless round-trip tests for them against the Ground-only shapes. No
  existing field number or golden byte changes; no `_fixture` or candidate names.

## High-risk authority/recovery qualification

```yaml
applicable: YES   # fenced durable write consuming current runtime-scope/incarnation authority; interprets persisted recovery evidence (the decay schedule)
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - identity/binding: one retirement per ItemInstance; an entry step only for a live entry of that corpse; the corpse step only for a corpse with zero live entries; the corpse in the fenced scope
  - current liveness/authority: the fenced generation is the current assignment held by the node's current incarnation, at freeze and at commit
  - temporal/provenance: deadline = durable materialized_at + 60 s, judged by the database clock (never a caller or runtime value); the audited occurred_at is never before it
consumer_boundaries: [freeze_decay_retire, commit_decay_retire, commit-time database guard, recovery query]
mutation_operators:
  applicable: [early by one second, exactly at the deadline, wrong deadline, corpse step with an entry remaining,
               non-corpse item, entry of another corpse, generation not owned, ended generation after handoff,
               stranded frozen step of an ended generation, replay, reconcile, pickup before decay, pickup racing a
               frozen decay step, bare item retirement without receipt, bare corpse Ground DELETE]
  considered_not_applicable:
    - "session/CommandRef/CharacterLease: server-originated, no player command"
    - "expected_character_revision: not an item-transaction fence"
one_invariant_per_negative_case: yes
record_derived_matching_helper: not used
evidence: apps/game-server/tests/support/corpse_decay_postgres_cases.rs
red_green: "disabling the commit-time clock clause makes database_admits_decay_only_as_its_own_proven_step fail at the not-yet-due forgery; restored"
finding_dispositions: {p0_p1_accepted_and_repaired: [], p0_p1_rejected_with_exact_evidence: [], p2_fixed_accepted_or_deferred: []}
```

## Acceptance criteria

- [x] Decay at `materialized_at + 60 s`, never later (timer due = remaining time; a passed deadline is due
  now) and never early (refused 1 s before; admitted exactly at the deadline; DB guard on the database clock).
- [x] `DECAY_RETIRE` commits as separate one-item steps (4 distinct physical transactions for 3 entries and
  the corpse), each with its own admitted `CorpseDecay` event decoded through the registered gate.
- [x] The corpse step is admitted only once zero live entries remain (Rust and DB).
- [x] A partial retire resumes after a handoff from the durable recovery query alone; the ended generation's
  frozen step never commits; no entry retires twice.
- [x] An entry picked up before decay (including a pickup racing a frozen decay step) is never retired.
- [x] `OneItemMintV1.corpse_container_entry` (5) and `OneItemTransferV1.corpse_source` (7) round-trip
  losslessly against the Ground-only shapes.
- [x] `cargo fmt`, `cargo clippy --lib --tests -D warnings`, `cargo test --lib` (965 passed),
  `corpse_decay_postgres` (618 passed), `corpse_transfer_postgres` (613 passed),
  `character_authority_postgres` (649 passed; the 10 failures are the canonical PostgreSQL 17.6 version-pin
  assertions on the local 17.11 server), `validate_governance.py`, `git diff --check`.
- [x] Protected PostgreSQL 17.6 lane green on the exact frozen head.
- [ ] PR opened and independent review routed by the control plane (not done by this worker).

## Deviations and gaps

- No production runtime composition calls `schedule_corpse_decay`/`retire_decayed_corpse` yet:
  `settle_creature_death_rewards` has no production caller and no owner timer lane exists at that point.
  The owner reads the deadline after a corpse MINT commits (and at scope admission) with
  `read_corpse_decay_schedule`, since `materialized_at` is written only at commit by the deferred trigger
  and cannot be read back inside the MINT's own transaction. Wiring into the future Channel owner loop is
  left to that owner's integration.
- The corpse step is keyed by the corpse `ItemInstanceId`, which uniquely identifies its
  `CORPSE_MATERIALIZATION` receipt, rather than by repeating the full cause tuple.
- No PR and no GitHub comment by this worker, per the allocation.

## Closeout

- merge commit/result: `3e3ed1a` on protected `main` (#1229); every file the PR changed is byte-identical to `b3398ec`
- ownership release: all leases released at merge

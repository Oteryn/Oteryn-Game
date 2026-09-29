# OTV2-20260929-d3-4-corpse-transfer

```yaml
task_id: OTV2-20260929-d3-4-corpse-transfer
title: D3-4 DUR-03 TRANSFER out of a corpse container with the D133 exclusivity window and the corpse never a source
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/d3-4-corpse-transfer
issue: 162
allocation: "#162 control plane child D3-4 of the merged D3 decision (worker 'Oteryn: impl durability')"
base_sha: 6133bde
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl durability (claude-code-session-01U1WRHgL9X8RbuiG1pwczrF)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0014_corpse_container_transfer.sql
  - apps/game-server/src/durability/item_transfer.rs
  - apps/game-server/src/durability/item_transfer_audit.rs
  - apps/game-server/tests/corpse_transfer_postgres.rs
  - apps/game-server/tests/support/corpse_transfer_postgres_cases.rs
  - apps/game-server/tests/support/item_transfer_postgres_cases.rs          # shared: pub(crate) visibility only
  - apps/game-server/tests/character_authority_postgres.rs                  # shared: #[path] include only
  - docs/contracts/game-events/v1/native_one_item_transaction.proto         # shared with D3-2: additive field 7 only
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json                      # shared: type 2 note only
  - docs/agents/tasks/active/OTV2-20260929-d3-4-corpse-transfer.md
public_contracts:
  - DUR-03
  - ANL-01
  - GAME-ITEM-01
depends_on: [D3-1]
blocks: [D3-5]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

TRANSFER additionally admits `Container(parent = a live corpse ItemInstance)` as its source, into the
unchanged D80-D83 destinations (decision
`reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md` §4.4/§4.5, D133/D134;
DUR-03 §39.4 "Pickup source"):

- D133 window: during `[materialized_at, materialized_at + 10 s)` only the corpse's
  `corpse_top_damage_character_id` may take an entry out; afterwards anyone may. Refusal:
  `ItemTransferRefusal::CorpseExclusiveWindow`. Judged by the database clock in Rust admission and,
  authoritatively, in the commit-time database guard (`clock_timestamp()`), so a bypass of the Rust
  path cannot skip it.
- D134: the corpse item itself is never a source, whatever it holds
  (`ItemTransferRefusal::CorpseNotPickupable`, checked independently of `ContainerNotEmpty`); a DB
  constraint trigger (`game_item_ground_removal_proven`) and the TRANSFER consistency guard refuse it
  independently of Rust.
- The source family is derived from the durable state of the source item (Ground, or a corpse entry),
  not from a new request field, so `combat/**` callers compile unchanged.

## Design

Migration 0014 (0013 is merged and untouched):

- `game_item_corpse_entry_removal_evidence`: guarded pre-DELETE evidence of a corpse entry and its
  corpse's live Ground World/Channel, written only by a `SECURITY DEFINER` BEFORE DELETE trigger (same
  idiom as the Ground removal evidence of 0011). A decayed/retired corpse (no live Ground) cannot be
  picked from.
- 0013's corpse entries were fully immutable. UPDATE stays rejected; DELETE is admitted only through the
  capture trigger and a deferred proof trigger that needs a TRANSFER receipt of the same physical
  transaction (`runtime` gets `DELETE` on that one table; `SELECT` on the evidence; nothing else).
- `game_item_ground_removal_proven` additionally refuses any corpse Ground row (D134). D3-6's
  `DECAY_RETIRE` extends this for its own transaction.
- `game_item_transfer_consistency_guard` (0011) is replaced with the same body plus: the corpse item is
  refused as a source; the removed-custody evidence may be a Ground row or a corpse entry (World/Channel
  bound to the reservation); a corpse entry counts as one location for the single-location check; the
  D133 window is judged at commit by `clock_timestamp()` against the corpse's own committed
  `materialized_at` and `corpse_top_damage_character_id`.

Audit (control-plane note to D3-4): `OneItemTransferV1.source` is widened additively. The Ground source
stays field 3; a new message `OneItemCorpseSourceV1 { corpse_item_instance_id, placement_ordinal,
corpse_ground }` is field 7 (`corpse_source`), exactly one of the two present. The corpse's own live
Ground is carried because it is the scope authority (World/Channel/generation) of the source. No
existing field or golden byte changes. The MINT `destination` widening and `DECAY_RETIRE` remain D3-6
(shared file with D3-2; whichever PR merges second takes a base merge).

Concurrency: the corpse-entry source path locks the item row in its own statement before reading the
entry, so a concurrent winner's committed removal is seen and the loser gets a clean
`SourceNotOnGround` refusal instead of a stale join.

## High-risk authority/recovery qualification

```yaml
applicable: YES   # fenced durable write consuming current session/lease/scope/incarnation authority (unchanged fence) plus a new time-gated custody source
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - identity/binding: only the top-damage Character inside the window; the fenced scope owns the corpse's Ground; one TRANSFER per CommandRef
  - temporal/provenance: window judged by the database clock against durable materialized_at (never a caller or runtime value)
  - single location: a corpse entry counts as one location; exactly one place after a TRANSFER
consumer_boundaries: [freeze_item_transfer, commit_item_transfer, commit-time database guard]
mutation_operators:
  applicable: [non-owner inside window, owner inside window, non-owner after window, owner after window,
               corpse item as source (empty and full and emptied), raw-SQL forged TRANSFER of the corpse,
               bare DELETE of corpse Ground row, bare DELETE of a corpse entry, forged TRANSFER of an entry
               inside the window (DB gate independent of Rust), replay, changed intent, stale duplicate under
               a new CommandRef, two players at the window boundary]
  considered_not_applicable:
    - "expected_character_revision: not an item-transaction fence"
one_invariant_per_negative_case: yes
record_derived_matching_helper: not used
evidence: apps/game-server/tests/support/corpse_transfer_postgres_cases.rs
finding_dispositions: {p0_p1_accepted_and_repaired: [], p0_p1_rejected_with_exact_evidence: [], p2_fixed_accepted_or_deferred: []}
```

## Acceptance criteria

- [x] `cargo fmt`, `cargo clippy --all-targets -D warnings`, `cargo test --lib` (900 passed) pass locally.
- [x] PG targets `corpse_transfer_postgres`, `item_transfer_postgres`, `character_authority_postgres` build
  (`--no-run`); execution is left to the protected PostgreSQL lane (no local PostgreSQL here).
- [ ] Protected PostgreSQL lane green on the exact frozen head.
- [ ] PR opened and independent review routed by the control plane (not done by this worker).

## Deviations and gaps

- No PR and no GitHub comment by this worker, per the allocation.
- The PG cases were not executed locally (no PostgreSQL 17 available to this worker); CI runs them.

# OTV2-20260929-death0-character-death-receipts

```yaml
task_id: OTV2-20260929-death0-character-death-receipts
title: DEATH-0 - Character death receipts, blessings and pending respawns (migration 0016, no writer)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/death0-character-death-receipts
issue: 162
allocation: "#162 comment 5895588092"
pr: null
base_sha: 48de3868
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "DEATH-0 worker subagent (claude-code-session-01LphUANMfC2q2WKdfEb39eC)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0016_character_death_receipts.sql
  - apps/game-server/src/durability/schema.rs                               # conditional; not needed, untouched
  - apps/game-server/tests/character_death_receipts_postgres.rs
  - apps/game-server/tests/support/character_death_receipts_postgres_cases.rs
  - apps/game-server/tests/character_progression_postgres.rs                # shared, minimal; not needed, untouched
  - docs/agents/tasks/archive/OTV2-20260929-death0-character-death-receipts.md
public_contracts: []
depends_on: [DEATH0-CHARACTER-DEATH-RECEIPT-V1]
blocks: [DEATH-1]
cross_repository_coordination_id: null
external_repositories: []
jira: null   # sync pending
```

## Outcome

Migration `0016_character_death_receipts.sql` gives the Character store what a death transaction needs,
exactly as decided in `reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md`
§3.1-§3.4 and §4 (DEATH-0 row). There is no writer: `commit_character_death` and
`reconcile_character_death` are DEATH-1.

- `game_character_death_receipts` (§3.1): immutable, keyed by the `PlayerDeathOccurrence` UUIDv7.
  - Stores the revisions (original + 1), level and experience before and after, and
    `experience_lost` (= before − after).
  - Stores the canonical blessing sets before and after (after ⊆ before), the Amulet of Loss selection
    and the lost-item set.
  - Stores the death cell: World, Channel, spatial position ≤ 128 B and map revision ≤ 512 B, the same
    columns as 0010 Ground.
  - Stores the respawn position (≤ 128 B), the death policy revision, `command_binding` (1..1024 B),
    `policy_digest` (32 B), the eight XP-receipt revision fields and `committed_at`.
- `game_character_blessings` (§3.3): (`character_id`, `blessing_key` ≤ 128 B, `provenance`).
- `game_character_pending_respawns` (§3.4): `character_id` primary key, `death_occurrence_id` and
  respawn position. It sits outside the revision chain, with a deferred FK to its receipt.
- The 0009 state guard and deferred consistency guard are replaced (§3.2), as listed under Design.
- The 0009 XP receipt table, its CHECKs, the root guard and the revision-one initializer path are
  unchanged. No existing migration file is edited.

## Design

- **State guard.** `revision + 1` with unchanged revision fields is admitted in one of two directions:
  - XP: experience strictly larger and level not lower;
  - death: experience not larger and level not higher.
- **Consistency guard.** It runs deferred on root, state, XP receipt and death receipt writes, over the
  union of both receipt kinds:
  - revision one has no receipt of either kind;
  - otherwise XP + death receipts = revision − 1, with exactly one receipt per revision (distinct
    committed revisions);
  - the current receipt matches the state (level, experience, the eight revision fields);
  - `before` = predecessor `after` holds across kinds, and no receipt is ahead of the root.
- **Transition binding (addition, see owner question).** A state UPDATE must be explained by the
  receipt of its successor revision, whose `before` equals the replaced row (`OLD`).
  - The predecessor chain cannot reach the first receipt, because revision one has none.
  - Without this binding, a death could commit with a forged-`before` XP receipt, and the reverse.
  - It changes nothing for a correct writer: the XP writer already writes `before` = the locked state.
- **Death outcome.**
  - Before insert: the receipt's `blessings_before` must be the held set, and no other death's respawn
    may be pending. This is §3.4's "a second death cannot commit while a pending respawn exists".
  - At commit: the death cell's World must be the Character's World, the held set must equal
    `blessings_after`, and the death's own pending row must exist at the receipt's respawn position.
- **Blessings.** A blessing DELETE is admitted only with a death receipt of the same physical
  transaction (`created_xact_id`, stamped) that lists it before and not after. UPDATE and TRUNCATE are
  rejected. Runtime gets SELECT and DELETE; INSERT is left to DEATH-4's receipt kind.
- **Pending respawns.** Only the death transaction that commits the receipt may insert one, so a
  consumed respawn is never recreated. UPDATE and TRUNCATE are rejected; DELETE is the DEATH-1
  consumption.
- **Death receipts.** UPDATE, DELETE and TRUNCATE are rejected.
- **DEATH-3 gap.** Two named CHECKs (`..._no_amulet_until_death3`, `..._no_lost_items_until_death3`)
  enforce "empty until DEATH-3 is admitted" (death decision §4.4). DEATH-3 replaces them with its
  bound.
- **Blessing sets.** They are canonical: strictly ascending in collation "C", distinct keys, each
  ≤ 128 B, at most 32 per set. The 32 is a storage bound, not a game rule.

## High-risk authority/recovery qualification

```yaml
applicable: YES   # persistence and CharacterRevision chain semantics; no fence consumer is added (no writer)
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - identity/binding: one receipt (either kind) per CharacterRevision; the receipt matches the state; before = predecessor after across kinds; before = the replaced state row
  - identity/binding: a death receipt binds its blessings (held before, held after), its pending respawn (occurrence, position) and its World
  - temporal/provenance: blessing consumption and pending respawn creation only in the death's own physical transaction
consumer_boundaries: [state guard (row), consistency guard (deferred), death receipt admission (row), death outcome guard (deferred), blessing consumption guard (deferred), pending respawn guard (deferred)]
mutation_operators:
  applicable: [xp receipt recording a loss, loss explained by a forged award, death receipt recording a gain or level up,
               gain explained by a forged death, mixed-direction successors, no receipt, receipt without successor,
               root-only successor, xp and death for one revision, receipt ahead of root, receipt level/experience/revision-field mismatch,
               cross-kind before != predecessor after, no pending respawn, pending respawn elsewhere, death cell in another World,
               amulet or lost item before DEATH-3, blessings_before understated or invented, blessing not consumed,
               blessing deleted without a death, kept blessing deleted, blessing or pending rewritten, after not within before,
               death while respawn pending, respawn recreated, bounds (binding 0/1025 B, digest 31 B, lost mismatch, negative, respawn 129 B, non-v7 occurrence),
               receipt update/delete, truncate of all three tables, grant matrix]
  considered_not_applicable:
    - "session/lease/scope fences, replay and reconcile: DEATH-1 writer (no writer in DEATH-0)"
    - "restart/PostgreSQL reload of verify_character_integrity with death receipts: DEATH-1 extends the startup check"
one_invariant_per_negative_case: yes (each rejected script changes one fact of an otherwise valid death; rejection asserts the SQLSTATE and an unchanged durable snapshot)
record_derived_matching_helper: not used
evidence: apps/game-server/tests/support/character_death_receipts_postgres_cases.rs
red_green: "disabling the transition binding fails 'loss explained by a forged award'; dropping the same-transaction clause of the pending guard fails 'respawn recreated'; both restored"
finding_dispositions: {p0_p1_accepted_and_repaired: [], p0_p1_rejected_with_exact_evidence: [], p2_fixed_accepted_or_deferred: []}
```

## Acceptance criteria

- [x] A death receipt lowers experience and level and advances the revision. An XP receipt still requires
  a strict increase. The mixed chain XP → death → XP → zero-loss death (consuming 2 of 3 blessings) →
  death to 0 XP commits.
- [x] A gap, a duplicate revision and a before/after mismatch across kinds each fail the deferred guard.
  A death cannot masquerade as an award, nor the reverse, including at the first revision.
- [x] Death receipts are immutable and untruncatable. Blessings and pending respawns are untruncatable
  and never rewritten.
- [x] The existing P03 progression cases pass unchanged (`character_progression_postgres` 599,
  `character_authority_postgres` 659).
- [x] The PostgreSQL 17.6 runs used the CI-pinned digest `sha256:f3bd19c6…` (pulled from `mirror.gcr.io`
  because Docker Hub returned 429):
  - `character_death_receipts_postgres` 6 tests;
  - `durability_postgres`, `native_admission_source_postgres`, `runtime_scope_assignment_postgres`,
    `combat_death_reward_postgres`, `combat_pickup_postgres`, `corpse_decay_postgres`,
    `corpse_transfer_postgres`, `item_mint_postgres`, `item_transfer_postgres` and
    `reward_claim_mint_postgres` all green.
- [x] `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -D warnings`, and
  `cargo test -p oteryn-game-server` pass: lib 1005, all 39 integration targets and the bins green.
  `validate_governance.py` and `validate_repository_policy.py` pass.
- [ ] Protected PostgreSQL 17.6 lane: see Deviations (routing).
- [ ] Independent exact-head review, routed by the control plane.

## Deviations and gaps

- **CI routing.** The protected PostgreSQL lane runs only its registered targets, such as
  `character_authority_postgres`. The new standalone target runs locally only until a one-line
  `#[path]` include of the cases file is added to `apps/game-server/tests/character_authority_postgres.rs`.
  That file is outside this allocation's owned paths.
- **Startup integrity (DEATH-1).** `durability/character_authority.rs::verify_character_integrity`
  still counts XP receipts only. A store holding a death receipt fails closed at
  `open_character_authority` until DEATH-1 extends that check to both kinds. No path can write a
  death receipt before DEATH-1.
- The "exactly one receipt per revision" distinct-count clause is also implied by the count, chain and
  current-receipt clauses together. It is kept as the explicit form of §3.2.
- `schema.rs` and `character_progression_postgres.rs` did not need changes.

## PR and closeout

- PR: opened by this worker as one non-draft PR; no auto-merge, no paid review.
- merge commit/result: squash merge of the PR, if it merges.
- ownership release: at merge.

# OTV2-20260927-character-revision-item-composition

```yaml
task_id: OTV2-20260927-character-revision-item-composition
title: Character revision and item transaction composition decision
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1033
base_sha: bab42d5c9900b05d9a7b4ff941df1fb60d2ea760
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-27
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-character-revision-item-composition.md
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task resolves the DUR-03 §39.3 `CONFLICT` and the reward-chest decisions §5: how a non-XP
item transaction concerning a Character (pickup TRANSFER, reward MINT plus `RewardClaim`) relates
to the global `CharacterRevision` chain in migration `0009`. The decision is
`CHARACTER-REVISION-ITEM-TRANSACTION-COMPOSITION-V1`:

- such a transaction is a DUR-03 value transaction;
- it does not advance `CharacterRevision`;
- it is fenced like the XP writer;
- it serializes on the `character_root` row lock.

`0009` is unchanged.

## Architecture and source of truth

- `PROVEN`: DUR-02 owner baseline rule 2 and schema packet §4.1; DUR-03 §§5.1, 7.2, 39.3;
  GAME-ITEM-01 location ownership; `0009_character_progression.sql`;
  `durability/character_progression.rs` fence and lock order; D40-D42.
- `DERIVED`: item locations and `RewardClaim` are DUR-03 state, not Character root semantic state.
- `UNKNOWN`: inventory position, capacity and weight policy; `RewardClaim` schema; cooldown
  identity.

## High-risk authority/recovery qualification

Applicable at design level. §§3.2–3.3 of the decision define which current session-generation
evidence authorizes a DUR-03 COMMIT, and how a pending CommandRef continues across a
same-GameSession reconnect. No code exists yet. Each negative case below is therefore a required
test that the implementing allocation must pass; the decision handback binds that allocation to it.

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - I1a recovery fence current (assert_recovery_fence) on every path, including cause replay and reconcile
  - I1b admission relations locked (lock_admission_relations, EXCLUSIVE) on the commit path before the cause lock
  - I2 reconnect-session row matches GameSession, Character, World, runtime scope, current_generation, lease generation, scope ownership generation, session_state IN (1,2)
  - I3 runtime-scope assignment active at the fenced ownership generation, held by the committing node registration, with current node incarnation proven
  - I4 admission character, account and runtime guards (eligible, lease generation, holder GameSession, presence, ready, ownership generation)
  - I5 cause CharacterId equals the fenced Character; the fenced runtime scope owns the source placement or Ground (DUR-03 §32)
  - I6 cause identity (for a reward, (claim, character)) unique; same cause and binding replays the first outcome, changed binding conflicts
  - I7 a continued CommandRef is still pending in the current owner's FND-02 CommandIngress (#663)
  - I8 character_root held FOR UPDATE before any occupancy, capacity or claim read; lock order is the XP writer's
  - I9 no write to game_character_roots, game_character_progression_state or game_character_xp_receipts
  - I10 capacity check, every MINT, the RewardClaim and mandatory audit commit together or not at all
consumer_boundaries:
  - DUR-03 COMMIT for MINT into CharacterInventory with a RewardClaim (D40-D42)
  - DUR-03 COMMIT for pickup TRANSFER into CharacterInventory (after its own admission gate)
  - cause replay after a lost response, a retry or a process restart
  - concurrent XP award (commit_character_experience) for the same Character
mutation_operators:
  applicable:
    - missing fact (no session row, no assignment, no guard row, no pending CommandRef)
    - stale generation (connection_generation, lease generation, scope ownership generation, node registration or incarnation)
    - mismatched identity or binding (other GameSession, other Character in the cause, other World or scope, changed cause binding)
    - provenance substitution (a fence taken from another session or scope; a cause keyed to another Character)
    - replay and concurrency (duplicate cause, retry after an ambiguous commit, a concurrent XP award, two concurrent claims)
  considered_not_applicable:
    - "expired, future or non-monotonic time: cooldown timing is not decided here (RewardClaim cooldown identity stays UNKNOWN) and is qualified by the allocation that defines it"
one_invariant_per_negative_case: true
negative_cases_required_of_implementation:
  - I1a stale recovery fence on a new commit -> AuthorityRejected, nothing written
  - I1a stale recovery fence on cause replay or reconcile -> rejected, and no outcome returned
  - I1b an authority-changing transaction (character-guard revocation, session replacement or reconnect-session update, scope-assignment move) runs concurrently with an item commit -> the two serialize on the admission relations: either the change waits until the item commit ends, or the item commit starts after it and rejects; the item never commits under revoked authority
  - I2 older connection_generation -> rejected; replaced GameSession -> rejected; moved lease generation -> rejected; session_state outside 1-2 -> rejected
  - I3 moved scope ownership generation -> rejected; other holder node or registration -> rejected; stale incarnation -> rejected
  - I4 ineligible character guard -> rejected; account presence on another Character -> rejected; runtime guard not ready -> rejected
  - I5 cause keyed to another Character -> rejected; fence scope does not own the source placement -> rejected
  - I6 same cause and binding -> first outcome and no second item; same cause with changed binding -> conflict
  - I7 CommandRef not pending in the current owner -> not continued, nothing written
  - I8 concurrent XP award and item transaction for one Character -> both serialize with no deadlock, and the XP expected revision is unaffected by the item commit
  - I9 CharacterRevision unchanged after a committed item transaction
  - I10 refused capacity -> no item, no claim, no audit row
positive_cases_required_of_implementation:
  - fresh MINT with RewardClaim commits under the full current fence
  - a still-pending reserved CommandRef commits after an eligible same-GameSession reconnect with the current connection_generation
  - retry after an ambiguous commit replays the first outcome under the current recovery fence, without the live session, lease or runtime checks
  - restart and PostgreSQL reload keep the cause record, and a later replay returns the first outcome
independent_current_fact_sources:
  - game_durability_reconnect_sessions
  - game_runtime_scope_assignments plus node incarnation proof
  - game_durability_admission_character_guards, _account_guards, _runtime_guards
  - FND-02 CommandIngress pending state (in the current owner)
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: "commit_character_experience is the reference fence; the item transaction adopts it unchanged apart from the cause key and I5"
  protocol_versions: NOT_APPLICABLE (no wire change)
  direct_and_reconciled_paths: "new commit path fenced by I1a-I7; commit-path replay by I1a, I1b and I6; reconcile by I1a and I6. This matches commit_character_experience and reconcile_character_experience, which both assert the recovery fence before the receipt lookup"
  fenced_durable_writes: "only DUR-03 item, cause and audit rows; I9 excludes Character root and progression writes"
  restart_retry_replay_concurrency_pg_reload: "covered by I6, I8 and the positive cases"
  evidence:
    - apps/game-server/src/durability/character_progression.rs (commit_character_experience)
    - apps/game-server/migrations/0009_character_progression.sql
    - DUR-03 §§7.2, 31, 32, 39.3
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - "Codex 4117445998 on 432dc37 (incomplete session-generation fence)"
    - "Codex 4117578651 on ec82b6c (this qualification was marked NOT_APPLICABLE)"
    - "Codex 4119146493 on f7e4970 (replay dropped the recovery fence)"
    - "Codex 4119146499 on f7e4970 (no negative case for the admission-relation locks)"
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred:
    - "Codex 4117446002 on 432dc37 (0009 claim): fixed"
```

## Acceptance criteria

- [ ] The decision document is on an exact frozen head with passing governance and repository
  policy.
- [ ] Independent exact-head review, routed by #162.
- [ ] Protected Merge Queue integration by the Work coordinator. This role has no merge authority.

## Excluded scope

- Runtime code, migrations, the resource registry, protocol, client and production.
- Editing the DUR-03, DUR-02, GAME-ITEM-01 or reward-chest texts. The D40-D42 contract text is
  step 2 of the reward-chest order of work, under its own allocation.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Independent review

- required: YES. The decision is about persistence/value and session-fence semantics.
- Codex findings, all accepted and repaired. Each head's review and CI are historical once it is
  superseded; details are in the PR #1033 threads.
  - `432dc37`, P1 4117445998: incomplete session fence. Fixed in §3.2–3.3.
  - `432dc37`, P2 4117446002: `0009` claim. §3.6 narrowed to XP-backed writes.
  - `ec82b6c`, P1 4117578651: qualification marked `NOT_APPLICABLE`. Completed; I5 added.
  - `f7e4970` (base merge of `1cb6aab`), P1 4119146493: replay dropped the recovery fence. I1
    split into I1a and I1b; §3.3 keeps the fence on replay.
  - `f7e4970`, P1 4119146499: no admission-lock negative case. I1b concurrency case added.
- Self-review also corrected §39.3 from "forbids" to "declines".
- The current successor head needs fresh exact-head review and CI.

## Context checkpoint

```yaml
last_progress: returned to AUTHORING after two Codex P1s on f7e4970; I1 split and replay fence kept
status: validating
branch: claude/gifted-rubin-a0axzx
head_sha: null
pr: 1033
owner_action_required: null
blocker: null
next_action: "#162 routes fresh exact-head review of the successor head and integrates it through the governed Merge Queue."
```

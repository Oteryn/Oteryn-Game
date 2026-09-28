# OTV2-20260928-char-progression-init

```yaml
task_id: OTV2-20260928-char-progression-init
title: Character progression readiness, fenced D88 initializer (level 1, total experience 0)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/oteryn-work-coordinator-jv59l0
issue: 162
pr: 1143
allocation: "#162 comment 5875188437 (Character progression readiness, owner decision D88)"
base_sha: e5cbcfa
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: #162 coordinator lane (claude-code-session-01U1WRHgL9X8RbuiG1pwczrF)"
authored_by: coordinator lane
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/character_progression.rs
  - apps/game-server/src/durability/mod.rs                          # test linkage only
  - apps/game-server/tests/support/character_progression_postgres_cases.rs
  - docs/agents/tasks/active/OTV2-20260928-char-progression-init.md
public_contracts:
  - DUR-02
  - DUR-03
  - VSL-COMBAT-01
depends_on: []
blocks:
  - "Combat D admission and generic Combat XP settlement (VSL-COMBAT-01 §24.1)"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

`DurabilityRoot::initialize_character_progression` gives a bootstrap-only Character
(CharacterRevision one, no `game_character_progression_state` row) its D88 typed progression,
level 1 and total experience 0, exactly once. The initializer runs under the same fence as the
R7 P03 XP writer, inside the one transaction that writes the row. It is idempotent, never
overwrites or regresses an existing row, and the XP writer still fails closed on absence.
Skip-tutorial level-2 starts are out of scope.

## Architecture and source of truth

- PROVEN: owner decision D88 (allocation comment 5875188437).
- PROVEN: VSL-COMBAT-01 §24.1 and DUR-03 require a Character-owned initialization/readiness
  proof before Combat D and XP settlement.
- PROVEN: migration 0009 permits a progression row at root revision one with zero receipts
  (`game_character_progression_consistency_guard`, and the `verify_character_integrity` progression
  query). No new CharacterRevision and no migration are needed.
- DERIVED: the fence proof was extracted from `commit_character_experience` into
  `assert_gameplay_fence` and is shared by both writers, so the two fences are identical.

## High-risk authority/recovery qualification

```yaml
applicable: true   # fenced durable Character write
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - "recovery fence and admission relation locks precede the write"
  - "live FND-04 session/connection/lease/scope, current scope assignment and node incarnation"
  - "locked live root at the expected CharacterRevision under the current interpretation"
  - "profile/ruleset/content of the row equal the root; level/experience are not caller input"
  - "an existing row is never overwritten; absence after revision one is corrupt, never zero"
consumer_boundaries: [DurabilityRoot::initialize_character_progression]
mutation_operators:
  applicable:
    - stale connection generation
    - stale lease generation
    - stale scope generation
    - stale CharacterRevision
    - root context substitution
    - exact repeat
    - changed policy binding on an existing row
    - existing progressed row
  considered_not_applicable:
    - "time: no source-time input"
    - "occurrence replay: initialization has no occurrence identity"
one_invariant_per_negative_case: true
independent_current_fact_sources:
  - game_character_recovery_admissions
  - game_durability_reconnect_sessions and admission guards
  - game_runtime_scope_assignments and game_node_registrations
  - game_character_roots and game_character_interpretations
record_derived_matching_helper:
  allowed_for_positive_happy_path: false
  forbidden_for_negative_authority_or_provenance_cases: true
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [x] A bootstrap Character becomes initialized at (1, 0) at revision one (PG).
- [x] An exact repeat is an idempotent no-op (PG).
- [x] An existing progressed row is never overwritten (PG, seeded 50/1000 and post-award 2/150).
- [x] A policy that does not map level 1 to 0 experience is rejected up front (unit + PG).
- [x] Stale connection, lease or scope generation, stale revision and root context substitution
      are rejected with no write (PG).
- [x] After initialization, the existing XP writer awards XP (PG, 0 -> 150, level 1 -> 2).
- [ ] Exact-head CI and required independent review (control plane, after freeze).

## Excluded scope

XP formula, level curve, skip-tutorial level-2 starts, client/protocol, registries, Combat code,
migrations. No production caller is added: the XP writer has none either, and the play-entry
composition that supplies the progression policy binding belongs to the Combat D/E admission.
Coordinator decision: Combat D composition calls the idempotent initializer immediately before
the first XP award, with the same policy binding it passes to the XP writer.

## Validation

- `cargo fmt --all --check`; `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`.
- `character_progression_postgres` and `character_authority_postgres` on local PostgreSQL 17.11.
  The only failures are the known `170006` version-pin asserts. The progression cases were also
  run with the pin locally relaxed (not committed), and all of them passed.

## Context checkpoint

```yaml
last_progress: local commit on wip/char-progression-init; not pushed
status: implementing
pr: 1143
blocker: null
next_action: control plane freezes, publishes and routes exact-head CI and independent review
```

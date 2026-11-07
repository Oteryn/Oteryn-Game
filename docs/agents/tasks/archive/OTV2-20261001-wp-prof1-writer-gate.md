# PROF-1 complete durability gate candidate
```yaml
task_id: OTV2-20261001-wp-prof1-writer-gate
title: Fenced proficiency writes, explicit migrations and retained recovery
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
branch: codex/weapon-proficiency-writer-gate-20261001
base_branch: codex/weapon-proficiency-request-binding-20261001
base_sha: f5b6f92e98bc8b9a936cb4f022bf136b56d621fb
head_sha: null
pr: 1532
issue: 162
owner: WP worker, D281 A2 / D283b
created_at: 2026-10-01T21:15:00Z
updated_at: 2026-10-02T04:24:25Z
execution_policy: continuous_progress
owned_paths: [apps/game-server/migrations/0032_character_proficiency.sql, apps/game-server/src/durability/character_proficiency.rs, apps/game-server/src/durability/character_proficiency_codec.rs, apps/game-server/src/durability/character_authority.rs, apps/game-server/tests/support/character_proficiency_postgres_cases.rs, apps/game-server/tests/character_authority_postgres.rs, apps/game-server/tests/character_progression_postgres.rs, docs/agents/tasks/active/OTV2-20261001-wp-prof1-writer-gate.md, docs/agents/tasks/archive/OTV2-20261001-wp-prof1-writer-gate.md, docs/agents/evidence/wp-gem-completion-20261002/]
depends_on: [PR1492, PR1495, PROFICIENCY0, D283b]
```

## Outcome and authority

Draft PR1532 preserves the complete gate on a new authoring branch. Its base assembly3807ef1b merges frozen Request1492 and Reader1495 without modifying either.
The owner explicitly requested draft publication on2026-10-02 to preserve the completed proposal, overriding the earlier draft-publication/batch hold.0032 is published exactly once with the complete writer/integrity/support gate under D283b. Review/integration disposition remains coordinator-owned. No new source-adapter authority is invented. Existing BuildFormula-style semantic policy supplies independently resolved content, active N, bindings and actual/retained shapes.

Fresh writes require independent recovery, issued semantic pass, current node/session/connection/lease/scope/root/progression/content fences, exact before values, shape, Mastery cap and selection track revision/unlock. Header, lines, tracks and one root/progression successor commit atomically. Replay binds immutable original intent before querying current gameplay/content policy. Migration Keep/Clear/Remap recomputes all retained choices and validates selected and unselected indices; missing retained declarations refuse history. Eight-kind global integrity preserves all seven existing kinds and scans all three WP relations in bounded batches.

## High-risk qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: [identity_binding, current_authority, temporal_provenance]
consumer_boundaries: [fresh_commit, committed_replay, timeout_reconcile, retained_read, recovery_open, recovery_reconcile]
mutation_operators:
  applicable: [missing_progression, wrong_character_session, stale_connection_lease_scope_generation, wrong_node_secret, unassigned_ended_node, stale_future_root_revision, context_mismatch, content_digest_substitution, canonical_definition_revision_shape_substitution, active_N_overflow, stale_before_track_revision, locked_selection, cap_overflow, conflicting_occurrence, malformed_uuid, orphan_history, invalid_retained_map, immutable_binding_corruption, concurrent_root_successor]
  considered_not_applicable: [WP_has_no_caller_time_or_deadline_field_and_reuses_existing_discrete_current_admission_fences, protocol_and_controller_installation_are_not_exposed_by_this_durability_seam]
one_invariant_per_negative_case: explicit_named_cases
independent_current_fact_sources: [existing_recovery_seal, registered_node_and_scope_assignment, leased_session_admission_guards, independent_root_progression_and_interpretation, immutable_semantic_source_lookup]
record_derived_matching_helper:
  allowed_for_positive_happy_path: false
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: reviewed
  protocol_versions: NOT_APPLICABLE_no_protocol_change
  direct_and_reconciled_paths: actual_PG17_6_PASS
  fenced_durable_writes: actual_PG17_6_PASS
  restart_retry_replay_concurrency_pg_reload: actual_PG17_6_PASS
  evidence: [prof1-final-native-structural-review.md, prof1-combined-tests/qualification.md, prof1-binding-integrity-structural-review.md]
finding_dispositions:
  p0_p1_accepted_and_repaired: [P1_all_cause_retained_binding_integrity_six_actual_RED_mutations_then_GREEN]
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: [DOC_1_fixed_module_and_legacy_reconcile_scope, precise_integrity_error_oracles_fixed, corrupted_snapshot_no_write_oracles_fixed]
```

## Validation and limits

PROVEN: unchanged SQL SHA9bde3bfc has161/161 actual PG17.6 cases, seven legacy kinds and a two-connection root race; two scratch SQLx wrappers independently passed. Final canonical support now runs11/11 actual PG cases through EACH authority/progression wrapper;20/20 native model/request/read/map tests including the TEMP PG corruption decoder pass, plus1/1 separately qualified public recovery-provenance test. The11th case additionally exercises the actual independently retained exclusive recovery1→2 after a public Migration, foreign lineage rejection, successor-admission rollback on unavailable/bad maps, exact idempotence and generation2 read. Earlier supplemental fixture borrow failure was repaired without changing production code. Final exact source passed fmt, locked all-target clippy with warnings denied and full57suites/16890PASS/7ignored. General package runs without the PostgreSQL environment; actual PG qualification is recorded separately. Governance validator and36 tests pass. Formal exact-head review/CI/MQ remain CP-owned after authorized publication/freeze.

Verified private-gate P1: Training/PerkSelection binding hash/version/digest corruption passed retained read/global open because only Migration bindings were reconstructed. Actual RED executed/restored all six one-byte faults, then the minimal codec fix re-encodes ALL causes;11/11 final GREEN in both wrappers includes sparse same-track predecessor revision2 at global revision3. Immutable historical predecessor is an intent field, never current authority. Existing seven kinds and migration mapping checks are unchanged. Independent source repair review found no further findings. Old ambiguous filter/cache runs, malformed COPY-only fixtures and superseded pre-map qualifications are not final evidence.

SQL is678 handwritten lines. Native follow-ons and test precursors were prepared as bounded logical batches; canonical support is1954 handwritten lines after verified repairs and separate binding/recovery regression batches. The complete coupled delta exceeds the ordinary500-line publication batch; Owner-directed draft publication preserves this complete coupled proposal; no generated-data waiver is asserted. Coordinator must assess the coupled integration batch during required review. Tests use fixed independently seeded content context; they qualify retained map consumption, not a production content-generation transition. Production content adapter, ordered PROF-2/PZ/effect hooks and wire children remain outside A1/A2's allocated activation scope. D283c compatible-refresh decision is pending; stale revision training/selection refuses. PROFICIENCY1B operations remain NOT_ADMITTED. Whole playable WP/Gem completion is not asserted.

## Next action

The final exact remote head is recorded in PR1532 FREEZE_SHA and #162. Coordinator owns required independent exact-head review, CI and integration/MQ. No frozen upstream branch writes or worker review/merge triggers.

## PR closeout

- PR:1532, OPEN/DRAFT, merge not requested.
- Authoring and source-pinned local qualification complete; archive moves in the final authoring commit.
- Final head: external FREEZE_SHA receipt; this commit cannot contain its own SHA.
- Independent exact-head review/CI: pending, coordinator-owned.
- Protected integration and runtime activation: pending, not asserted.
- Evidence and unapplied Ink/Gem proposals: `docs/agents/evidence/wp-gem-completion-20261002/README.md`.

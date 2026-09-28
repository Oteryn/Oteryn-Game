# OTV2-20260928-dur03-maxima-death-identity-decision

```yaml
task_id: OTV2-20260928-dur03-maxima-death-identity-decision
title: DUR-03 resource maxima (A5) and creature-death identity (A4) decision
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1079
base_sha: 57a0fc76f9f4812c1e66a18560ff14d05ef36579
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_DUR03_RESOURCE_MAXIMA_AND_CREATURE_DEATH_IDENTITY_DECISION_2026-09-28.md
  - docs/agents/tasks/active/OTV2-20260928-dur03-maxima-death-identity-decision.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md   # §39.3 restart clause (Codex P1 on 5c0f994)
  - docs/architecture/VSL-COMBAT-01_MINIMAL_COMBAT_DEATH_LOOT_CONTRACT_CANDIDATE.md   # recovery property (same)
  - docs/agents/tasks/active/OTV2-20260928-character-composition-closeout.md   # archive move after #1072
  - docs/agents/tasks/archive/OTV2-20260928-character-composition-closeout.md
public_contracts:
  - DUR-03
  - VSL-COMBAT-01
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records the durable text for the owner decisions D50-D52, posted on #162
(comment 5866894112, which supersedes the A5 values of 5866460480):

- A5: the DUR03-RL-01..08 hard maxima, RL-07 per-field byte bounds, and the RL-08 retry ceiling.
- A4: the restart-stable `CreatureDeathOccurrenceRef` key, the derived loot MINT cause and XP
  occurrence, and the live-generation rule. Uncommitted descendants are dropped on a generation
  change and never duplicated.

No registry row, migration or code changes here. DUR-03 §39.3 and VSL-COMBAT-01 gain the D52
terminal rule for restart. B4 (#513) registers the values, and stage D
implements A4.

## Architecture and source of truth

- `PROVEN`: Durability topology packet :325-344 and :409-413 (hard-maximum rule); B2 evidence
  (#1031); `dur03_native_one_item_audit.rs` :181-182 (candidate 4,096 B cap), :906 and :931-934
  (fixed retry loop); VSL-COMBAT-01 :120-151 and :505; DUR-03 :885-897; `runtime_actor_carrier.rs`
  :132-138, :373-382 and :1310-1317; `0003_runtime_scope_assignment.sql` :398-403;
  `character_progression.rs` :31-39; DUR-01 :109 and :361.
- `DERIVED`: the worst-case protobuf arithmetic for the final schema in the normative `EventEnvelope`
  (§3.2 of the decision): MINT 7,167 B, TRANSFER 8,471 B, and 8,603 B once `typed_position` is
  defined; overhead at most 1,038 B.
- `UNKNOWN`: stage C PostgreSQL isolation and deadlock behaviour; loot tables; XP values.

## High-risk authority/recovery qualification

Applicable at design level to A4, which defines which evidence authorizes a loot MINT or XP award
after a creature death and how recovery treats pending descendants. The negative cases below bind
the stage D allocation. A5 selects numeric bounds only; its boundary tests bind B4, and RL-08 is re-decided in C.

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - J1 one death key per actor lifecycle generation; a replayed lethal effect makes no second death
  - J2 a descendant commits only while the death's ownership generation is the current assignment held by the current node incarnation
  - J3 the same death key with a different lethal-effect or revision binding is an integrity CONFLICT
  - J4 the loot MINT cause is the full typed tuple; no hash-only equality
  - J5 the XP occurrence is a server-minted UUIDv7, reused for every retry in the same generation
  - J6 a despawn or scope retirement creates no death key
consumer_boundaries:
  - loot MINT to Ground (DUR-03)
  - XP award (P03 commit_character_experience)
  - retry after an ambiguous commit in the same generation
  - restart, crash or scope move between death commit and descendant commit
mutation_operators:
  applicable:
    - stale generation (ownership generation moved, node incarnation stale, actor slot recycled)
    - mismatched identity or binding (same key, other lethal effect or revision context)
    - provenance substitution (caller-supplied UUID or label as a MINT cause)
    - replay and concurrency (duplicate lethal effect, retry after an ambiguous commit)
  considered_not_applicable:
    - "time: no death identity field is time-based (VSL :136)"
one_invariant_per_negative_case: true
negative_cases_required_of_implementation:
  - J1 replayed lethal effect -> the existing death is returned, no second key
  - J2 generation changed before the descendant commits -> refused, nothing minted or awarded
  - J3 same key with a different binding -> CONFLICT, nothing written
  - J4 a MINT with a caller-supplied cause -> rejected
  - J5 a retry in the same generation -> the same XP receipt, no second award
  - J6 administrative despawn -> no death key, no loot, no XP
positive_cases_required_of_implementation:
  - death then loot MINT and XP award commit in the same generation
  - a retry after an ambiguous commit returns the first outcome
independent_current_fact_sources:
  - game_runtime_scope_assignments plus node incarnation proof
  - the owner's in-memory committed death record
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: "P03 fence and #1033 fence are the references; no new fence"
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: "same-generation retry only; no cross-generation reconciliation by design (D52)"
  fenced_durable_writes: "loot MINT and XP receipt only"
  restart_retry_replay_concurrency_pg_reload: "covered by J1, J2 and J5"
  evidence:
    - apps/game-server/src/foundation/runtime_actor_carrier.rs
    - apps/game-server/src/durability/character_progression.rs
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - "Codex 4120451420 on 5c0f994 (crash-loss rule conflicted with the DUR-03 restart clause): DUR-03 §39.3 and VSL-COMBAT-01 amended"
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred:
    - "Codex 4120451429 on 5c0f994 (maxima measured on the candidate shape, not the full loot cause): recomputed"
    - "Codex 4120510110 on 6238499 (envelope sized on the evidence wrapper, not the normative EventEnvelope): recomputed; TRANSFER exceeded 8,192 B, so the owner raised the envelope cap to 9,216 B"

```

## Acceptance criteria

- [ ] The decision document is on an exact frozen head with passing governance and repository policy.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Registry rows, the `.proto`, examples, migrations and runtime code.
- Loot tables, XP values, inventory capacity and TRANSFER admission.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.
- The #1072 closeout record is archived with terminal integration evidence (`9fa52e3`).

## Context checkpoint

```yaml
last_progress: authored; PR #1079 open
status: validating
branch: claude/gifted-rubin-a0axzx
pr: 1079
owner_action_required: null
blocker: null
next_action: exact-head review and Merge Queue integration of #1079
```

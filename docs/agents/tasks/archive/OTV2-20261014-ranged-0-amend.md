# OTV2-20261014-ranged-0-amend

```yaml
task_id: OTV2-20261014-ranged-0-amend
title: "RANGED-0: amend the decision after the four-lane review"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ranged-0-amend-20261014
pr: 2111
head_sha: "exact frozen head in the report to the control plane"
final_head_sha: "exact frozen head in the report to the control plane"
owner: claude-code worker for control plane session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-14
updated_at: 2026-10-14
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/reviews/OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md
  - docs/architecture/reviews/OTERYN_GAME_ITEM_MOVE_WIRE1_EQUIP_AND_DROP_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_MOVE_RL11_VISIBILITY_DECISION_2026-09-28.md
  - docs/architecture/reviews/OTERYN_GAME_STACK0_PARTIAL_COUNTS_AND_STACK_MERGES_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_CREATURE_MOVE1_STEPS_PATHS_AND_GOALS_PACKET_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md
  - docs/agents/tasks/archive/OTV2-20261014-ranged-0-amend.md
public_contracts:
  - "Composition, ITEM-MOVE-WIRE-1, MOVE-RL11 and ATTACK-0 pending RANGED-0 notes reconciled to the decision (allocated TransactionId, stack_ordinal, no ordinal change, bounded chase): persistence, movement and combat review"
  - "DUR-03 pending RANGED-0 notes (section 15, 39.1, 39.3) edited to WeaponUseAggregateV1, still pending: persistence review"
  - "CREATURE-AI-0 RANGED0-RL-03 chase amendment kept consistent with the server-driven chase of decision section 8 (owner decision 2026-10-14): movement review"
  - "Decision section 6.2 restart reconciliation and section 5 per-role hit chance: persistence and combat review"
  - "Decision §6.4 WeaponUseAggregateV1 (DUR-03 §15, §39.1, §39.3 amendment text, active on acceptance only): persistence review"
  - "Decision §6.2 allocated UUIDv7 TransactionId, server-originated fence, composition rule 1 scope: persistence review"
  - "Decision §6.1.2 Ground order is the existing stack_ordinal (no migration): persistence review"
  - "Decision §5 and §4 hit_chance semantics, cap rows, AutoAttack routing, SwingRngPurpose: combat review"
  - "Decision §8 chase is server-driven with bounded searches (owner decision 2026-10-14): movement review"
  - "Decision §7 missile stream waits for SPELL-PRESENT-0 registration: protocol review"
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- The RANGED-0 decision is amended to close every blocker of the 2026-10-14 four-lane review (table in decision §14). Status remains CANDIDATE; the four lanes re-run on the amended head. Text only: no code, contract file, migration or protocol change.

## Architecture and source of truth

- PROVEN: blockers cited from code (`target.rs`, migration 0034, `spell_item_transaction.rs`, DUR-03 §20) were checked against the repository before the text changed.

## High-risk authority/recovery qualification

The diff is text only and installs no code path, but it amends the PREPARE/COMMIT fencing and persisted-receipt restart rules (decision §4.2, §6.2, §6.4) that RANGED-1 will implement. The qualification below is therefore completed for the contract text; RANGED-1 carries it into code and tests. Acceptance still needs all four independent lanes.

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - "TransactionId is an allocated UUIDv7 frozen at PREPARE; it is never derived from the swing tuple."
  - "Restart order: the claim commits under the character_root lock and advances the scope and CharacterLease/GameSession generation before any receipt read, and advances only the generation that changed (a character reconnect advances the CharacterLease/GameSession generation; a scope-owner replacement advances the scope generation through the scope-assignment authority, never both on the claim's own authority); only then is the newest receipt read (LIMIT 1) and a new TransactionId allocated."
  - "At most one outstanding ambiguous TransactionId per character: after the 2,000 ms window the slot is freed for every use except another weapon swing."
  - "A replay or reconciliation read returns the first outcome and credits nothing."
  - "Weapon swings stay refused (fail closed) while the restart read fails or a swing is unobserved or ambiguous."
consumer_boundaries:
  - "RANGED-1 (WeaponUseAggregateV1 rows, receipt, audit tables; the 7,936 payload and 9,216 envelope caps)"
  - "STACK-1 (Ground order is the existing stack_ordinal, no merge onto a legacy NULL top)"
  - "CHASE-1 and CREATURE-MOVE-1 (bounded server-driven chase)"
mutation_operators:
  applicable:
    - "stale or missing generation at PREPARE and at COMMIT"
    - "duplicate or reordered TransactionId delivery"
    - "receipt read failure or lost commit acknowledgement after an owner loss"
    - "second weapon swing while a swing is ambiguous"
  considered_not_applicable:
    - "channel-transfer fence changes: unchanged, the slot still waits for its outcome"
one_invariant_per_negative_case: true
independent_current_fact_sources:
  - "persisted receipt row, live CharacterLease/GameSession generation, migration 0034 stack_ordinal"
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: "DUR-03 section 15, 39.1, 39.3, the composition decision, ITEM-MOVE-WIRE-1, MOVE-RL11 and STACK-0 reconciled in this PR"
  protocol_versions: "no wire change; protocol-oteryn unchanged"
  direct_and_reconciled_paths: "direct commit and restart reconciliation both specified in section 6.2"
  fenced_durable_writes: "the receipt and the aggregate commit share one transaction under the fence (section 6.4)"
  restart_retry_replay_concurrency_pg_reload: "section 6.2 (a)-(e) and section 4.2; the negative tests are RANGED-1 scope and listed in the decision"
  evidence:
    - "decision sections 4.2, 6.2, 6.4 and the section 14 blocker table"
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - "Codex P1 on b07ed26f (R7 owner acceptance, LIMIT 1 completeness): repaired in c5d78959"
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred:
    - "Codex P2 on b07ed26f and c5d78959 (critical-charm hook, wand swings): fixed"
    - "Codex P1 x2 on 7a07cb26 (archive invariant recombined the generations; STACK-1 still depended on RANGED-1): fixed"
    - "Codex P1 on e5e6b0bb (separate scope-owner and character-session recovery cases): fixed"
    - "Codex P1/P2 on ed1d264a (CREATURE-MOVE-1 packet order reconciled; oldest-waiting-first fairness inside the chase reserve): fixed"
```

## Acceptance criteria

- [x] Each review blocker maps to a section (decision §14).

## Excluded scope

- No code, content, migration or protocol change; no acceptance.

## Validation

### Focused

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: pass

### Component/integration

- `NOT_APPLICABLE`: documentation only.

### E2E

- `NOT_APPLICABLE`: documentation only.

### Exact-head CI

- final head: the PR head
- result: the PR checks on that head

## Self-review

- exact head: the frozen head
- method/reviewer: task writer.
- material findings: none open
- verdict: ready for the four-lane re-review

## Independent review

- required: the control plane re-runs combat, persistence, movement and protocol lanes.
- verdict: pending.

## PR and closeout

- changed-file review: documentation paths only
- unresolved review threads: none
- protected auto-merge: control plane
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: amendment pushed
status: completed
branch: claude/ranged-0-amend-20261014
pr: 2111
blocker: null
next_action: control plane re-runs the four lanes
```

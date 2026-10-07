# OTV2-20261006-quest-talk-npc-candidates

```yaml
task_id: OTV2-20261006-quest-talk-npc-candidates
title: Qualify source talk NPC dialogue candidates
mode: AUDIT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/quest-talk-npc-candidates-20261006
pr: 1897
base_sha: f6894e793c162d9d8a43578332f3a6f77d2936a3
head_sha: f98414580a1b8914c78d16ffd527b1a5e9161321
final_head_sha: null
final_head_frozen_at: null
owner: chatgpt-quest-completion
created_at: 2026-10-06
updated_at: 2026-10-07
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/quest-authoring/quest_talk_npc_candidates.py
  - tools/content-schema/quest-authoring/test_quest_talk_npc_candidates.py
  - tools/content-schema/quest-authoring/samples/server-completion/talk-npc-candidates.json
  - docs/agents/tasks/archive/OTV2-20261006-quest-talk-npc-candidates.md
public_contracts: []
depends_on: []
blocks:
  - NPC-QUEST-1
cross_repository_coordination_id: 1622
external_repositories: []
```

## Outcome

Qualified source-derived Quest `talk` stages against existing canonical NPC and Dialogue identities, fail-closed and without selecting dialogue branches or admitting runtime dispatch.

Population at candidate freeze:

- source-derived talk stages audited: **250**
- `EXACT_NPC_WITH_DIALOGUE`: **174 stages**
  - **95 quests**
  - **129 unique canonical NPCs**
  - **129 unique canonical Dialogues**
- `EXACT_NPC_NO_DIALOGUE`: **23**
- `AMBIGUOUS_MULTIPLE_NPCS`: **36**
- `NO_EXACT_NPC`: **17**
- runtime bindings: **0**
- selected dialogue branches: **0**



### Current-main refresh

After merging current main, three newly lowered talk stages entered the source-derived binding plan:
Barbarian Arena s1 (Halvar) and Rift Warrior Outfits s3/s5 (Cledwyn).
All three resolve to one exact canonical NPC+Dialogue while the other stage targets
(Greenhorn, Silver Token) remain non-NPC targets. This moves the deterministic
population from 247 to 250 and exact NPC+Dialogue candidates from 171 to 174;
the other classification counts remain unchanged.

## Architecture and source of truth

**PROVEN:** matching uses only canonical NPC key tails and existing NPC `source_bindings[].external_id` where `disposition=EXACT`, followed by deterministic punctuation/underscore/case normalization.

**PROVEN:** no fuzzy matching, display-name guessing, multi-NPC selection, dialogue-branch selection or runtime admission is performed.

**PROVEN:** canonical NPC/Dialogue identity is insufficient for native Quest progression. Runtime admission remains owned by NPC talk / NPC-QUEST work.

## High-risk authority/recovery qualification

```yaml
applicable: false
model: NOT_APPLICABLE
authority_invariants: []
consumer_boundaries: []
mutation_operators:
  applicable: []
  considered_not_applicable:
    - Offline identity qualification only; no production mutation or authority-bearing recovery.
one_invariant_per_negative_case: not_applicable
independent_current_fact_sources: []
record_derived_matching_helper:
  allowed_for_positive_happy_path: not_applicable
  forbidden_for_negative_authority_or_provenance_cases: not_applicable
finding_family_sweep:
  sibling_apis: not_applicable
  protocol_versions: not_applicable
  direct_and_reconciled_paths: not_applicable
  fenced_durable_writes: not_applicable
  restart_retry_replay_concurrency_pg_reload: not_applicable
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [x] All source-derived talk stages in scope are classified deterministically.
- [x] Exact NPC+Dialogue candidates retain null selected branch and null native dispatch binding.
- [x] Ambiguous, missing-dialogue and no-exact-match rows remain held.
- [x] Dedicated generator/test artifacts are deterministic.
- [x] No runtime/playability claim is made.
- [x] Task record is archived before final candidate freeze.

## Excluded scope

- Dialogue branch selection.
- NPC talk runtime execution.
- Durable QuestState transition request.
- Fuzzy or inferred NPC identity matching.
- Runtime activation.

## Implementation / findings

Exact matching contract:

1. canonical NPC identity key tail;
2. existing exact NPC external source binding;
3. deterministic punctuation/underscore/case normalization only.

Representative evidence:

- A Piece of Cake / Biff the Baker -> exact NPC + Dialogue candidate.
- An Interest In Botany / Rabaz -> exact NPC while Botany Almanach is not coerced into NPC identity.
- Between the Lines / Phillip + Wyrdin -> ambiguous, held.
- Barbarian Test / Sven -> exact NPC but no canonical Dialogue, held.
- 25 Years of Tibia / Lord Retro -> no exact canonical NPC, held.

## Validation

### Focused

- generator: PASS, **250** source-derived talk stages
- exact NPC+Dialogue candidates: **174**
- dedicated unittest: **8/8 PASS**
- generator `--check`: PASS
- committed packet byte-for-byte drift check: PASS
- `git diff --check`: PASS
- `python -m unittest discover -s tools/agents/tests`: **59/59 PASS**
- `python tools/agents/validate_governance.py`: PASS (re-run after recording this validation evidence)

### Component/integration

- runtime integration: NOT_APPLICABLE; this task only qualifies identity candidates.

### E2E

- NOT_APPLICABLE; no executable NPC/Quest binding is admitted.

### Exact-head CI

- final exact head is recorded by PR #1897 after the archive closeout commit; the task file intentionally does not move the frozen head to self-record its own SHA.

## Self-review

- method/reviewer: implementing agent
- material findings: no identity promotion beyond exact accepted aliases
- verdict: PASS

## Independent review

- required: YES
- method/auditor: Codex PR review
- material finding: P1 task record remained active after implementation
- disposition: fixed by archiving this record with completed status before final freeze
- verdict: pending exact-head rereview / thread resolution

## PR and closeout

- changed-file review: bounded to generator, test, generated candidate packet and archived task record
- unresolved review threads: expected 0 after exact-head review thread resolution
- related/superseded PRs: none
- protected auto-merge: pending exact-head required checks
- merge commit/result: pending
- ownership release: yes after merge

## Context checkpoint

```yaml
last_progress: candidate population qualified; review closeout fixed by task archival
status: completed
branch: codex/quest-talk-npc-candidates-20261006
head_sha: f98414580a1b8914c78d16ffd527b1a5e9161321
pr: 1897
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending exact-head rerun
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: final exact-head validation and protected merge queue
```

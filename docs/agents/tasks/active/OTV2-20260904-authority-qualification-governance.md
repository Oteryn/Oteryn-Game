# OTV2-20260904-authority-qualification-governance

```yaml
task_id: OTV2-20260904-authority-qualification-governance
title: Converge high-risk authority qualification
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/authority-qualification-278
pr: null
issue: 278
parent_issue: 277
base_sha: d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Pro implementation worker
created_at: 2026-09-04T13:50:00Z
updated_at: 2026-09-04T13:50:00Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - apps/game-server/AGENTS.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/agents/prompts/OTV2_SOL_DURABILITY_LEAD.md
  - docs/agents/tasks/active/OTV2-20260904-authority-qualification-governance.md
public_contracts: []
depends_on:
  - issue: 277
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - repository: Oteryn/Oteryn
    revision: 0c493896040072badeff1f333eb83d7114a993ff
    paths:
      - docs/governance/AI_REVIEW_POLICY.md
      - docs/agents/contracts/PERSISTENT_AUTONOMOUS_CONTINUATION_POLICY.md
      - ecosystem/agent-execution-routing-policy.json
```

## Outcome

Game high-risk authority/recovery tasks use a pre-freeze invariant-and-boundary discipline that detects whole finding families before qualification, while preserving the current META-owned advisory AI-review model and non-self-invalidating freeze semantics.

## Architecture and source of truth

- `PROVEN` — protected Game `main` at admission is `d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d`.
- `PROVEN` — parent programme #277 and allocation #278 authorize only the four paths listed above; none overlaps the 12 paths owned by active Issue #250 / PR #252.
- `PROVEN` — META `AI_REVIEW_POLICY.md@0c493896040072badeff1f333eb83d7114a993ff` makes external AI review advisory and rejects fingerprints, attestations and a second required status.
- `PROVEN` — the current Game task template already forbids moving a frozen head only to copy SHA/run/review/READY evidence.
- `DERIVED` — executable test architecture, not a natural-language semantic parser, is the durable enforcement location for the future authority matrix. This task adds concise governing expectations only.

## High-risk authority/recovery qualification

```yaml
applicable: GOVERNANCE_PROCESS_ONLY
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - identity_and_binding
  - current_liveness_and_authority
  - temporal_and_provenance
consumer_boundaries:
  - prepare_authorization
  - commit_authorization
  - controller_installing_reconciliation
  - compatibility_reconciliation
  - typed_reconciliation
mutation_operators:
  - exactly_one_invariant_changed
independent_current_fact_sources:
  - required_by_governing_text_for_future_runtime_tasks
record_derived_matching_helper:
  allowed_for_positive_happy_path: test_only
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: required_when_applicable
  protocol_versions: required_when_applicable
  direct_and_reconciled_paths: required_when_applicable
  restart_retry_replay_concurrency_pg_reload: required_when_applicable
  evidence: []
finding_dispositions:
  p0_p1_verified_repair_or_rejection: pending_review
  p2_fixed_accepted_or_deferred: pending_review
```

No runtime authority is granted by this documentation-only task.

## Acceptance criteria

- [x] `apps/game-server/AGENTS.md` distinguishes immutable expected evidence from independent current authority.
- [x] High-risk tasks use `AuthorityInvariant × ConsumerBoundary × MutationOperator` and one-invariant negative mutations.
- [x] Negative authority/provenance tests are forbidden from using record-derived matching-current helpers.
- [x] Finding-family sweep precedes material freeze and another deep review after material repair.
- [x] P0/P1 and P2 have explicit evidence-based dispositions without making AI merge authority.
- [x] Existing post-freeze bookkeeping prohibition remains intact.
- [x] No natural-language governance parser, fingerprint, attestation plane or new required status is introduced.
- [ ] Exact-head Agent Governance, Architecture and canonical `game-gate` pass.
- [ ] One independent deep review of the stable governance candidate has no unresolved actionable finding.

## Excluded scope

No runtime, test harness, PostgreSQL, workflow, ruleset, protected setting, production, secret, Platform, Atlas or META mutation. Do not edit the active #252 task record. Do not merge directly.

## Implementation / findings

- Extended the nearest game-server instructions with the authority-invariant model, independent-current-fact boundary, pre-freeze family sweep and evidence-based finding disposition.
- Extended `TASK_TEMPLATE.md` with an applicable-or-`NOT_APPLICABLE` high-risk authority/recovery section while preserving the existing final-head semantics.
- Updated the Durability Lead prompt to version 1.3 and added the same pre-freeze discipline plus an explicit authority-qualification handoff block.
- Deliberately did not add a semantic validator for natural-language authority prose; future executable coverage belongs in Issues #280–#282.
- The exact candidate SHA is established by authoritative branch/PR readback after this commit exists and is not self-embedded here.

## Validation

### Focused

- command/run: exact four-path diff inspection and Markdown/YAML-block self-review
- result: PASS before publication

### Component/integration

- command/run: `python tools/agents/validate_governance.py`; `python tools/repository/validate_repository_policy.py`
- result: pending exact-head hosted execution

### E2E

- scenario: `NOT_APPLICABLE` — governance/documentation-only; no runtime behavior changes
- result: NOT_APPLICABLE

### Exact-head CI

- final head: established by authoritative GitHub readback after publication
- trigger source: Draft PR `pull_request`
- workflow/run/job: pending
- runner assignment: GitHub-hosted repository workflows
- classification: governance/documentation-only
- result: pending

## Self-review

- exact head: established by authoritative GitHub readback after publication
- method/reviewer: implementing agent, mandatory whole-diff review
- material findings: none before publication
- verdict: PASS_PENDING_EXACT_HEAD_READBACK

## Independent review

- required: YES — changes high-risk qualification governance
- exact head: pending authoritative readback
- method/auditor: one independent deep review under current META policy
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: four allocated paths only
- unresolved review threads: pending
- related/superseded PRs: none
- protected auto-merge: disabled/not requested
- merge commit/result: pending control-plane integration
- ownership release: after protected-main terminal readback

## Context checkpoint

```yaml
last_progress: material governance candidate prepared without runtime or control-plane overlap
status: validating
branch: governance/authority-qualification-278
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: 2026-09-04T13:50:00Z
ci_trigger_source: pull_request_after_publication
ci_check_generation: pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: github_hosted_pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: qualify the authoritative read-back candidate through exact-head governance/game-gate, whole-diff self-review and one independent deep review; repair any actionable finding before READY_FOR_INTEGRATION; do not merge
```

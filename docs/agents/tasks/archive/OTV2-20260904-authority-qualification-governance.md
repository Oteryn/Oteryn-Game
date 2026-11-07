# OTV2-20260904-authority-qualification-governance

```yaml
task_id: OTV2-20260904-authority-qualification-governance
title: Converge high-risk authority qualification
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/authority-qualification-278
pr: 286
issue: 278
parent_issue: 277
base_sha: d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d
head_sha: null
final_head_sha: null
final_head_frozen_at: 2026-09-04T14:38:00Z
owner: ChatGPT GPT-5.6 Pro implementation worker
created_at: 2026-09-04T13:50:00Z
updated_at: 2026-09-04T14:38:00Z
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

Game high-risk authority/recovery tasks use a pre-freeze executable model of `AuthorityInvariant × ConsumerBoundary × MutationOperator` that includes all production mutations consuming current fence evidence, enumerates concrete mutation operators, expands accepted finding families before re-review, and preserves a frozen candidate when an advisory report is verified incorrect.

## Architecture and source of truth

- `PROVEN` — protected Game `main` at admission is `d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d`.
- `PROVEN` — parent programme #277 and allocation #278 authorize exactly the four listed paths; none overlaps the 12 paths owned by active Issue #250 / PR #252.
- `PROVEN` — META `AI_REVIEW_POLICY.md@0c493896040072badeff1f333eb83d7114a993ff` makes external AI review advisory and rejects fingerprints, attestations and a second required status.
- `PROVEN` — current Game task-template semantics forbid moving a frozen head only to copy SHA/run/review/READY evidence.
- `DERIVED` — executable test architecture, not a natural-language semantic parser, is the future enforcement location for the authority matrix. This task establishes concise governing expectations only.

## High-risk authority/recovery qualification

```yaml
applicable: GOVERNANCE_PROCESS_ONLY
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - identity_and_binding
  - current_liveness_and_authority
  - temporal_and_provenance
consumer_boundaries:
  - every_production_mutation_consuming_current_fence_evidence
  - prepare_authorization
  - commit_authorization
  - controller_installing_reconciliation
  - compatibility_reconciliation
  - typed_reconciliation
mutation_operators:
  applicable:
    - missing_fact
    - stale_fact_or_generation
    - mismatched_identity_or_binding
    - expired_future_or_non_monotonic_time
    - provenance_substitution
    - boundary_specific_replay_or_concurrency
  considered_not_applicable: []
one_invariant_per_negative_case: true
independent_current_fact_sources:
  - required_by_governing_text_for_future_runtime_tasks
record_derived_matching_helper:
  allowed_for_positive_happy_path: test_only
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: required_when_applicable
  protocol_versions: required_when_applicable
  direct_and_reconciled_paths: required_when_applicable
  fenced_durable_writes: required_when_applicable
  restart_retry_replay_concurrency_pg_reload: required_when_applicable
  evidence:
    - review PRR_kwDOT8SzxM8AAAABMNHSgg
    - threads PRRT_kwDOT8SzxM6fUpbL PRRT_kwDOT8SzxM6fUpbW PRRT_kwDOT8SzxM6fUpba
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - include_fenced_durable_writes_and_every_authority_consuming_mutation
    - preserve_frozen_head_after_verified_rejection
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred:
    - fixed_enumerate_concrete_mutation_operators
```

No runtime authority is granted by this documentation-only task.

## Acceptance criteria

- [x] Nearest game-server instructions distinguish immutable expected evidence from independent current authority.
- [x] Applicability includes every production mutation consuming current session, lease, generation, authority or other fence evidence.
- [x] The model enumerates concrete mutation operators, with explicit `NOT_APPLICABLE` evidence where appropriate, and still changes exactly one invariant per negative case.
- [x] Negative authority/provenance tests are forbidden from using record-derived matching-current helpers.
- [x] Finding-family sweep includes fenced durable writes and applicable sibling/version/direct/reconciled/restart/replay/concurrency/PG surfaces.
- [x] A verified rejection preserves the frozen candidate and representative review; only an accepted/verified material finding supersedes the generation.
- [x] P0/P1 and P2 have explicit evidence-based dispositions without making AI merge authority.
- [x] Existing post-freeze bookkeeping prohibition remains intact.
- [x] No natural-language governance parser, fingerprint, attestation plane or new required status is introduced.
- [ ] Exact repair-head Agent Governance, Architecture and canonical `game-gate` pass.
- [ ] One independent re-review of the stable repaired candidate has no unresolved actionable finding.

## Excluded scope

No runtime, test harness, PostgreSQL, workflow, ruleset, protected setting, production, secret, Platform, Atlas or META mutation. Do not edit the active #252 task record. Do not merge directly.

## Implementation / findings

### Initial candidate

- Initial material candidate: `b28605bb8dbb76cfdf4d204bff6daf12132ede93`.
- Exact-head validation passed: Rust `33880654197`, Agent Governance `33880654212`, Architecture `33880654253`, Merge Gate `33880654221`.
- Independent review `PRR_kwDOT8SzxM8AAAABMNHSgg` reviewed that exact candidate and produced three accepted findings:
  - P1 `PRRT_kwDOT8SzxM6fUpbL` / `3934856323`: applicability omitted ordinary fenced durable writes and other authority-consuming production mutations;
  - P1 `PRRT_kwDOT8SzxM6fUpbW` / `3934856334`: advisory P0/P1 labeling could thaw a frozen candidate even after the report was verified wrong;
  - P2 `PRRT_kwDOT8SzxM6fUpba` / `3934856340`: the third matrix dimension lacked concrete mutation-operator enumeration.

### Family repair

- The accepted findings superseded `b28605bb...`.
- `apps/game-server/AGENTS.md`, `TASK_TEMPLATE.md` and Durability Lead prompt are repaired consistently rather than patching one sentence:
  - applicability now includes every production mutation gated by current session/lease/generation/authority/fence evidence;
  - concrete operators are enumerated: missing, stale, mismatched, temporal, provenance-substitution and boundary-specific replay/concurrency;
  - `one_invariant_per_negative_case` remains a separate isolation constraint;
  - family sweep explicitly includes fenced durable writes;
  - verified rejection preserves freeze and prior review; only an accepted/verified material finding supersedes and requires repair/re-review;
  - handoff finding dispositions distinguish accepted-and-repaired from rejected-with-evidence.
- Prompt version advances from 1.3 to 1.4.
- The exact repair commit is established by authoritative branch/PR readback after publication and is not self-embedded here.

## Validation

### Focused

- command/run: exact four-path diff inspection; cross-surface terminology/finding-family self-review
- result: PASS before repair publication

### Component/integration

- command/run: `python tools/agents/validate_governance.py`; `python tools/repository/validate_repository_policy.py`
- result: pending exact repair-head hosted execution

### E2E

- scenario: `NOT_APPLICABLE` — governance/documentation-only; no runtime behavior changes
- result: NOT_APPLICABLE

### Exact-head CI

- final head: established by authoritative GitHub readback after repair publication
- trigger source: Draft PR #286 synchronize
- workflow/run/job: pending repair generation
- runner assignment: GitHub-hosted repository workflows
- classification: material high-risk governance repair
- result: pending

## Self-review

- exact head: established by authoritative readback after repair publication
- method/reviewer: implementing agent, mandatory whole-diff/finding-family review
- material findings: all three first-review findings repaired consistently; no additional pre-publication finding
- verdict: PASS_PENDING_EXACT_HEAD_VALIDATION

## Independent review

- required: YES — accepted material repair invalidates the first review as final evidence
- initial exact review: `PRR_kwDOT8SzxM8AAAABMNHSgg` on `b28605bb8dbb76cfdf4d204bff6daf12132ede93`
- repaired exact head: pending authoritative readback
- method/auditor: one independent deep re-review under current META policy
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: exactly four allocated paths
- unresolved review threads: three accepted findings remain unresolved until exact repair-head validation and re-review
- related/superseded PRs: none
- protected auto-merge: disabled/not requested
- merge commit/result: pending control-plane integration
- ownership release: after protected-main terminal readback

## Context checkpoint

```yaml
last_progress: accepted two P1 and one P2 were expanded into one cross-surface governance-family repair
status: validating
branch: governance/authority-qualification-278
head_sha: null
pr: 286
final_head_sha: null
final_head_frozen_at: 2026-09-04T14:38:00Z
ci_trigger_source: pull_request_synchronize_after_repair
ci_check_generation: repair_pending
ci_checks_for_current_head: 0
ci_run_ids:
  - 33880654197_INITIAL_SUCCESS
  - 33880654212_INITIAL_SUCCESS
  - 33880654253_INITIAL_SUCCESS
  - 33880654221_INITIAL_SUCCESS
ci_job_ids: []
runner_assignment_state: github_hosted_repair_pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish the four-path family repair, verify exact branch/PR head and changed paths, then require exact-head governance/game-gate, whole-diff self-review and one independent re-review before resolving findings or READY_FOR_INTEGRATION; do not merge
```

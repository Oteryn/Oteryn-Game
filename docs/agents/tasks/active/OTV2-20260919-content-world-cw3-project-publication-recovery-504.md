# OTV2-20260919-content-world-cw3-project-publication-recovery-504

```yaml
task_id: OTV2-20260919-content-world-cw3-project-publication-recovery-504
title: Durable canonical World-project publication and recovery
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw3-project-durable-cleanup-504
pr: pending
base_sha: 54869eb9db46d83beaaadfc557270d58a39cc6dd
head_sha: pending
final_head_sha: null
final_head_frozen_at: null
owner: cw3-publication worker under issue #162 control plane
created_at: 2026-09-20
updated_at: 2026-09-20
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/project_fs.rs
  - apps/game-server/tests/content_world_project_publication.rs
  - docs/agents/tasks/active/OTV2-20260919-content-world-cw3-project-publication-recovery-504.md
public_contracts:
  - docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V1_DECISION.md
depends_on:
  - OTV2-20260919-content-world-cw3-project-filesystem-capture-504
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Linux authors can save a complete canonical editable project through a durable whole-root
transaction. Readers observe either the prior validated root or the replacement validated root;
recovery retains the prior root as a backup and fails closed on unplanned or substituted cleanup
entries. Non-Linux targets return `UnsupportedPlatform` before filesystem access.

## Architecture and source of truth

- `PROVEN`: CW1 section 8 requires one complete validated revision or the previous revision,
  never a partially authoritative mixture, and requires the previous revision and pre-migration
  backup to remain recoverable.
- `PROVEN`: protected main at the allocated base already contains the strict canonical writer,
  parser, Linux capture adapter and protected CW2 B4 evidence import.
- `PROVEN`: issue `#162` control-plane release comment `5747852252` allocates this exact branch,
  base and three-path custody.
- `DERIVED`: an append-only, identity-bound journal plus Linux atomic root exchange is the smallest
  existing-dependency mechanism satisfying CW1 without creating another parser or loader.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - durable journal identity and checksum chain
  - exact parent/root/child filesystem identity and ordinary entry type
  - bounded complete previous and replacement tree roles
consumer_boundaries:
  - interrupted-save recovery
  - prior-backup retirement before a later save
mutation_operators:
  applicable:
    - missing planned entry after interruption
    - unplanned addition
    - root or child identity substitution
    - symlink, hardlink or special-file substitution
    - journal tamper or truncated suffix
    - replay and repeated recovery
    - same-content previous and replacement revisions
  considered_not_applicable:
    - session generation, lease, credential and database authority; this adapter has none
one_invariant_per_negative_case: pending final test sweep
independent_current_fact_sources:
  - opened no-follow entry metadata and filesystem identity rechecked against the durable journal
record_derived_matching_helper:
  allowed_for_positive_happy_path: false
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: capture, publish, recover
  protocol_versions: journal v2 only
  direct_and_reconciled_paths: initial install, exchange, backup install, rollback, committed cleanup
  fenced_durable_writes: file, directory, journal and parent synchronization
  restart_retry_replay_concurrency_pg_reload: restart/retry/replay applicable; concurrency races fail closed; PostgreSQL not applicable
  evidence: pending final validation
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - bounded cleanup replaces recursive remove_dir_all
    - same-content recovery uses identity roles rather than equal digests
    - cleanup retains planned type/identity and opens directories no-follow
    - partial cleanup recovery accepts only a provable subset of the durable plan
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [x] Canonical documents are validated before filesystem mutation and staged as one synchronized
  root before an atomic initial rename or replacement exchange.
- [x] The append-only journal binds raw names, entry types and filesystem identities before later
  mutation and is bounded from the caller's existing scan limits plus Linux `NAME_MAX`.
- [x] Successful replacement retains the prior root under the transaction's exact backup role.
- [ ] Crash-point, tamper, addition/substitution, resource-boundary and repeat-recovery tests pass.
- [ ] Focused tests, existing capture tests, lib tests, fmt and clippy pass on the frozen head.
- [ ] Independent exact-head material review is accepted by the control plane.

## Excluded scope

No project codec, importer, compiler, runtime loader, Cargo dependency, production limit, Windows
filesystem implementation or activation behavior changes in this task.

## Implementation / findings

The Linux adapter derives fixed sibling names from the raw project-root basename. A newly created,
single-link journal records a checksum-chained header, the complete previous-root deletion plan,
each stage identity immediately after creation, and explicit install phases. File and directory
data are synchronized before `StageReady`; replacement uses `RENAME_EXCHANGE`, then renames the old
root to the previous-backup role. Recovery classifies topology by recorded identities rather than
content digest.

Cleanup first proves that the current tree is a subset of the durable plan. It rejects additions,
type changes, hard links, special files and identity substitution before unlinking any admitted
entry. It retains the preflight tree and uses `open_dir_nofollow` while deleting. A crash between
entry creation and its durable plan record therefore preserves the old live root and reports an
explicit conflict; recovery never creates a fresh ownership plan for residue.

## Validation

### Focused

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server --test content_world_project_publication`
- result: PASS for initial, replacement, same-content repeat recovery and pre-I/O invalid limit;
  adversarial/crash matrix still pending.

### Component/integration

- command/run: pending final affected test sweep
- result: pending

### E2E

- scenario: local filesystem transaction evidence only; no runtime activation is in scope
- result: pending final transaction matrix

### Exact-head CI

- final head: pending
- trigger source: pending control-plane draft PR
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent
- material findings: pending adversarial whole-diff review
- verdict: pending

## Independent review

- required: YES; persisted recovery and destructive cleanup interpretation are material
- exact head: pending
- method/auditor: parent control plane
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none known
- protected auto-merge: parent control plane only; pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: core journal/publication/recovery mechanism compiles and initial focused tests pass
status: implementing
branch: agent/content-world-cw3-project-durable-cleanup-504
head_sha: pending first coherent checkpoint
pr: pending
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: add deterministic crash-state and exact-plan adversarial coverage
```

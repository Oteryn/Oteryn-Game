> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #692 merged as `f1f16e8a310835fedf8662f53208e47a07ca5a54`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260919-content-world-cw3-project-publication-recovery-504

```yaml
task_id: OTV2-20260919-content-world-cw3-project-publication-recovery-504
title: Durable canonical World-project publication and recovery
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw3-project-durable-cleanup-504
pr: 692
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
  - one pinned and locked parent capability throughout transaction validation and mutation
  - one pinned root capability throughout each journal-role verification
  - every canonical child consumed by role capture matches its durable parent/name/type/identity
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
one_invariant_per_negative_case: complete for the applicable filesystem mutation matrix
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
  evidence:
    - 12 private crash/concurrency/resource/capability unit tests in project_fs.rs
    - 10 public publication/recovery integration tests
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - bounded cleanup replaces recursive remove_dir_all
    - same-content recovery uses identity roles rather than equal digests
    - cleanup retains planned type/identity and opens directories no-follow
    - partial cleanup recovery accepts only a provable subset of the durable plan
    - initial and backup install now use RENAME_NOREPLACE after self-review found replacement-permitting rename
    - previous tree and root digest are reverified immediately before root exchange
    - transaction validation stays on the locked parent capability after ambient rename or substitution
    - committed, installed and previous roles require a complete canonical capture, not only identity and project.json digest
    - recovery synchronizes every observed renamed topology before appending the corresponding durable phase
    - role verification binds the identity plan, canonical bytes and journal commitment to one opened root
    - plan-aware capture rejects canonical child rebinding before and after byte consumption
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred:
    - caller byte/count limits are checked over borrowed canonical documents before cloning them
```

## Acceptance criteria

- [x] Canonical documents are validated before filesystem mutation and staged as one synchronized
  root before an atomic initial rename or replacement exchange.
- [x] The append-only journal binds raw names, entry types and filesystem identities before later
  mutation and is bounded from the caller's existing scan limits plus Linux `NAME_MAX`.
- [x] Successful replacement retains the prior root under the transaction's exact backup role.
- [x] Crash-point, tamper, addition/substitution, resource-boundary and repeat-recovery tests pass.
- [x] Focused tests, existing capture tests, lib tests, fmt and clippy pass locally.
- [ ] Independent exact-head material review is accepted by the control plane.

## Excluded scope

No project codec, importer, compiler, runtime loader, Cargo dependency, production limit, Windows
filesystem implementation or activation behavior changes in this task.

## Implementation / findings

The Linux adapter derives fixed sibling names from the raw project-root basename and holds an
exclusive nonblocking lock on the opened parent during publication or recovery. A newly created,
single-link journal records a checksum-chained header, the complete previous-root deletion plan,
each stage identity immediately after creation, and explicit install phases. File and directory
data are synchronized before `StageReady`; replacement uses `RENAME_EXCHANGE`, then renames the old
root to the previous-backup role. Recovery classifies topology by recorded identities rather than
content digest. Every transaction capture uses the original opened parent capability even if the
ambient pathname is renamed and replaced. Installed, committed and previous roles are accepted
only after a complete canonical capture plus the identity plan and journal-bound root digest. Each
role verification opens the named root once, performs all tree and canonical-byte checks through
that handle, derives the root commitment from those captured bytes, and finally requires the role
name still to bind the opened root identity. Every canonical directory component and file opened by
that capture must match the durable plan's parent identity, raw name, kind and filesystem identity;
the complete plan is checked again through the pinned root after byte consumption.

Recovery synchronizes an already-observed initial rename, root exchange or backup rename before it
advances the journal phase. Canonical document count, individual byte length and checked aggregate
bytes are admitted over borrowed data before the publication path clones the document map.

Cleanup first proves that the current tree is a subset of the durable plan. It rejects additions,
type changes, hard links, special files and identity substitution before unlinking any admitted
entry. It retains the preflight tree and uses `open_dir_nofollow` while deleting. A crash between
entry creation and its durable plan record therefore preserves the old live root and reports an
explicit conflict; recovery never creates a fresh ownership plan for residue.

## Validation

### Focused

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server --test content_world_project_publication`
- result: PASS, 10/10. Covers initial/replacement/same-content, repeat recovery, partial cleanup,
  unknown addition, root/child substitution, symlink/hardlink/FIFO, journal tamper/torn suffix,
  sparse journal admission, same-identity/length child mutation, and exact/max+1/overflow resource
  boundaries.

### Component/integration

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server --test content_world_project_fs`; `cargo +1.94.0 test --locked -p oteryn-game-server --test content_world_project`; `cargo +1.94.0 test --locked -p oteryn-game-server --lib`; `cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings`; `cargo +1.94.0 fmt --all -- --check`; `python3 tools/agents/validate_governance.py`; `git diff --check`
- result: repaired candidate PASS — existing capture 9/9, project 19/19, lib 456/456 including
  private publication family 12/12, strict Clippy, formatting, governance (26 documents, 9 lanes)
  and diff validation.

### E2E

- scenario: deterministic local filesystem interruption at plan, stage-ready, atomic install and
  backup-install boundaries; repeat recovery validates the same public capture consumer
- result: PASS; no runtime activation is in scope

### Exact-head CI

- final head: pending
- trigger source: pending control-plane draft PR
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: `edb8dc6da8bd1c3353d8a61f07114b5a31b93ce9`
- method/reviewer: implementing agent
- material findings: accepted/repaired — replacement-permitting initial/backup rename,
  pre-exchange previous-tree revalidation, competing publication serialization, parent capability
  escape, incomplete committed-role validation, recovered-rename synchronization and pre-admission
  clone
- verdict: implementation now uses NOREPLACE, revalidates the previous tree/digest immediately
  before exchange, holds transaction validation on one parent capability, requires complete
  canonical role captures, fences recovered phase advances with parent sync, admits caller limits
  before clone, and fails a competing operation before journal interpretation

## Independent review

- required: YES; persisted recovery and destructive cleanup interpretation are material
- exact heads: initial review `edb8dc6da8bd1c3353d8a61f07114b5a31b93ce9`; repaired-head
  rereviews `3636d9cacf7718496a62940617cf079d868d0349` and
  `2a21f5e0857491f76b07b5fb6979b95dd372b8ac`
- method/auditor: independent Sol High whole-diff security/durability review, control-plane evidence
  comments `5748015463` and `5748112226`
- material findings: P0 0; P1 3 accepted — ambient parent reopen escaped the pinned capability,
  committed recovery validated only `project.json`, and recovered rename topologies advanced journal
  phases without first synchronizing the parent; P2 1 accepted — canonical documents were cloned
  before caller byte/count admission. The first four findings are confirmed closed. Re-review found
  one additional P1: role verification reopened the role name between identity-plan, canonical-byte
  and root-commitment checks, permitting exchanged roots to supply different evidence.
- verdict: additional P1 repaired locally with one pinned root handle and deterministic exchanges at
  both former boundaries. Final rereview found the coupled child-rebinding form: capture could consume
  a substituted canonical child between plan checks. Plan-aware capture plus a post-capture plan
  check and deterministic child exchanges repair it locally; new exact-head targeted material
  rereview is required before integration

## PR and closeout

- changed-file review: exact three allocated paths confirmed locally and on PR #692
- unresolved review threads: pending
- related/superseded PRs: none known
- protected auto-merge: parent control plane only; pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: final child-rebinding P1 fixed; publication 10, capture 9, project 19, lib 456, clippy/fmt/governance pass
status: ready
branch: agent/content-world-cw3-project-durable-cleanup-504
head_sha: external_pr_evidence_after_final_publish
pr: 692
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
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish one immutable head and request parent-owned targeted exact-head rereview
```

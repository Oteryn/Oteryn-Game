# OTV2-20261015-oracle-arm64-deploy

```yaml
task_id: OTV2-20261015-oracle-arm64-deploy
title: Oracle ARM64 runner and deployment
mode: MIGRATE
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 2217
pr: null
base_sha: 77af046ce14c81df6ed6cab03ae693a68acff32c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex Oracle migration task
created_at: 2026-10-15T21:31:19Z
updated_at: 2026-10-15T22:08:00Z
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/oracle-operations.yml
  - deploy/oracle/**
  - docs/agents/tasks/active/OTV2-20261015-oracle-arm64-deploy.md
public_contracts: []
depends_on: [Oteryn/Oteryn-Platform#1639]
blocks: []
cross_repository_coordination_id: Oracle migration 20261015
external_repositories: [Oteryn/Oteryn-Platform]
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2218](https://github.com/Oteryn/Oteryn-Game/pull/2218), [#2220](https://github.com/Oteryn/Oteryn-Game/pull/2220), [#2222](https://github.com/Oteryn/Oteryn-Game/pull/2222). Game Oracle workflow/runner source delivery and executable identity-binding repair are merged. Evidence: `deploy/oracle/README.md`.

Remaining: Physical main-only host acceptance and real incarnation/readiness remain under Issue #2217; source delivery is not operational acceptance. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

## Outcome

Governing Issue: https://github.com/Oteryn/Oteryn-Game/issues/2217.
The owner explicitly requested moving the Game and Platform runners/workflows to
Oracle after discussing the two repositories and exact Synology dependencies.
The existing Game runner remains disabled on NAS. Its replacement is registered
in game-runners, restricted to this repository, retaining oteryn-game and adding
oteryn-oracle/ARM64. No gameplay, protocol, schema or content changes are owned.

## Architecture and source of truth

PROVEN: installed runtime uses protected Game 77af046ce14c81df6ed6cab03ae693a68acff32c,
source World content and canonical identities, existing source trust and independent
floors. A root-owned durable incarnation controller issues/reconciles authorization,
registration, assignment and readiness. Docker never replays spent authorizations.
PROVEN: both new runners are registered offline; no runner has a Docker socket.
Deployment adapter, private host configuration and backup controller are owned by
Platform #1639 under deploy/oracle. Game owns its workflow and operational boundary.
UNKNOWN: real workflow deployment acceptance and final native entry.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - exact current protected-main SHA with successful Game aggregate gate
  - root-owned component-bound forced SSH command
  - running incarnation equals independently retained ready cycle
consumer_boundaries:
  - Game workflow to host forced command
  - host deploy to existing durable incarnation controller
mutation_operators:
  applicable: [missing, stale, mismatched_identity, provenance_substitution, replay, concurrency]
  considered_not_applicable: [time_expiry_no_time_based_deploy_grant]
one_invariant_per_negative_case: true
independent_current_fact_sources: [GitHub protected main, GitHub Actions check runs, root controller state, Docker inspect]
record_derived_matching_helper:
  allowed_for_positive_happy_path: false
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: component key cannot choose other component or arbitrary commands
  protocol_versions: unchanged
  direct_and_reconciled_paths: existing controller retained unchanged
  fenced_durable_writes: existing controller retained unchanged
  restart_retry_replay_concurrency_pg_reload: shared host deployment lock and existing durable controller
  evidence: [Platform deploy/oracle/test_actions_deploy.py]
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [ ] Protected main workflow proves expected runner, ARM64, uid1001 and no Docker socket.
- [ ] Host accepts only current protected main after its successful game-gate.
- [ ] Native build/deployment preserves host secrets, content, custody and controller.
- [ ] Real new incarnation reaches readiness; native entry remains separate proof.

## Excluded scope

No Game protocol or gameplay changes, no Tunnel, no trust bootstrap/reset, no
stale NAS writer restart, no arbitrary runner shell/root/Docker access.

## Validation

- python tools/repository/validate_repository_policy.py: PASS
- python -m unittest discover -s tools/repository -p 'test_*.py': PASS (88 tests)
- python tools/agents/validate_governance.py: PASS
- python -m unittest discover -s tools/agents/tests: PASS (124 tests)
- Exact source candidate 26954117868d25f31575f3a5e1d3bd8f19405c69: game-gate PASS; independent Codex review completed with no findings (summary comment 6203346942).
- Governed queue request 6203757391: executor run 38818727492/job 116709781838 succeeded; receipt 1bc51b72-cf9b-4b01-bd43-67be9149d709 sequence 1/2 readback verified.
- Real merge_group run 38818782335 and game-gate check 116711350839: PASS at fb11de5c9845e4f3eacc0173074d9927834164ae; protected-main readback matched.
- Protected-main governance subsequently detected the terminal active PR reference; this operational packet repair removes it while preserving the incomplete physical acceptance boundary.

Review finding 4288835355 accepted: add the negative Docker capability probe
before SSH, covering custom socket/context and DOCKER_HOST endpoints as well as
the canonical socket. Apply the same guard to the sibling Platform workflow.

## Operational continuation after source delivery

Delivery PR #2218 owns the repository workflow step. The task continues under
open Issue #2217 for main-only host acceptance, so it has no canonical active
implementation PR after source delivery. Keep this operational packet waiting
until the protected workflow, new incarnation and native entry are qualified,
then archive it with the accepted post-deployment closeout exception. This avoids
leaving a terminal PR as a live canonical dispatch target. Host implementation
is Platform #1644; the separately authorized server contract repair is #2219.

Source delivery PR #2218 merged at fb11de5c9845e4f3eacc0173074d9927834164ae;
operational acceptance remains open under #2217. No source writer is allocated
by this waiting packet. The exact queue receipt UUID and sequence1/2 readback
were verified before protected-main readback.

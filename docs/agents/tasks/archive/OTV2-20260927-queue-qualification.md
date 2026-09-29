---
task_id: OTV2-20260927-queue-qualification
title: Re-qualify the physical server qualifications on the Merge Queue candidate
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: bab42d5c
branch: agent/queue-qualification-20260927
issue: 162
jira: KAN-13
allocation_comment: 5860126133
owned_paths:
  - .github/workflows/merge-group-gate.yml
  - tools/repository/validate_repository_policy_core.py
  - tools/repository/test_validate_merge_group_pg_sim.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/archive/OTV2-20260927-queue-qualification.md
---

# Re-qualify the physical server qualifications on the Merge Queue candidate

This is an owner decision made in this session. It follows #1027, which requires node boot and Server Seam on the PR head.

## Defect

The Merge Queue gate never re-ran the physical qualifications on the synthetic `merge_group` candidate. A batched or rebased candidate that differs from every individually qualified PR head was therefore integrated without them.

## Outcome

- The queue `candidate` job adds a `server_qualification` output. It computes the output with the protected-base `server_qualification_required` (from #1027) over the exact queue diff. The selection fails closed: import errors, diff errors and invalid evidence all select it.
- New jobs `Merge Queue / Node boot against the real Platform` and `Merge Queue / Server Seam over TCP+TLS` run on the exact queue head with the pinned Platform producer.
- `game-gate` requires both jobs unless the selection is explicitly `false`.
- Pins and tests are updated:
  - the policy pin for the queue blob;
  - the canonical job list;
  - the job fragments;
  - the PG-SIM regressions (fan-in, skipped selection, fail-closed missing selection).
- The protected-base merge-authority audit approves the new queue blob and `game-gate` `needs` line in a separate owner-authorized stage-A rotation.

Excluded: no change to the PR gate, the classifier or the required-status configuration.

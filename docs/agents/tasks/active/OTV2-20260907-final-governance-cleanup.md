# OTV2-20260907-final-governance-cleanup

```yaml
task_id: OTV2-20260907-final-governance-cleanup
title: Remove retired review adapter and terminal prompt dispatch
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
issue: 388
base_branch: main
branch: governance/final-cleanup-388
pr: 389
base_sha: a793457cf3001df37109acb2c4b4a772b53db97a
owner: final-cleanup-coordinator
created_at: 2026-09-07T16:30:37Z
updated_at: 2026-09-07T17:20:00Z
execution_policy: continuous_progress
owned_paths: []
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Outcome and authority

Issue #388 owns this bounded implementation; Oteryn/Oteryn#176 owns organization closeout. The META v3 binding remains unchanged. The candidate removes the obsolete Game-local Codex review controller/adapter/JSON, keeps one real governance validator entry point, retires terminal one-shot prompt dispatch without deleting historical prompt files, and removes active dependencies on the retired Superpowers planning process.

The #364 remediation programme retains its own product/control-plane scope. Its protected programme document states that the present #364 writer owns only its programme file and task packet and that prospective WP1 paths are not admitted; this cleanup therefore does not take over an active WP1 implementation lease.

## Acceptance and validation

Required candidate validation: canonical governance validator; lifecycle discovery including the injected failing-assertion canary; inherited META adoption tests; repository policy; applicable semantic audit; `git diff --check`; complete-diff self-review; exact-head GitHub CI and normal protected Merge Queue. Runtime E2E and production/recovery qualification are `NOT_APPLICABLE` because this change contains no runtime, persistence, deployment, workflow, ruleset, required-gate or production mutation.

The owner explicitly authorized the final review to be performed in this execution rather than requiring a separate child-model reviewer. No missing child-model surface is a blocker.

## Excluded scope

No product implementation, Cargo/shared registries/contracts, other task leases, workflows, required gates, maintenance controls, credentials or production. Active #364 remediation, #308/W6 and product lanes remain separate.

## Context checkpoint

Candidate assembled from protected `main@a793457cf3001df37109acb2c4b4a772b53db97a`. Terminal preparation Issues #93-#97 and #179 delivery evidence were re-read from live GitHub before lifecycle retirement. Mutation ownership is limited to PR #389 until protected integration/readback.

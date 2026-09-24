# WP1 required-gate credibility repair — 2026-09-07

Authority: #162 / #364 / #308 comment `5574487313`.
Admission: protected `main@1b41d485cc4bf126d2a9e5fe9717cc8530ece3d5`.
Branch: `ci/wp1-required-gate-credibility-364`.

## Goal

Close only remediation F01/F02 by making the existing required PR/Merge Queue
control plane truthfully fail on early native Windows command failure and by
executing the real lifecycle regression in the PR/MQ paths that feed
`game-gate`.

## Sequence

1. **RED:** change only `test_validate_merge_group_pg_sim.py` plus task/plan.
   Require the final MQ gate contract that current protected main does not yet
   satisfy. Publish Draft PR and preserve the expected exact-head failure.
2. **GREEN candidate:** minimally modify `merge-group-gate.yml`; keep every
   existing command/job/fan-in and add only fail-closed PowerShell semantics plus
   lifecycle regression execution. Extend the queue regression with positive and
   four-position injected-native-failure controls and actual lifecycle execution.
3. Read the resulting exact Git blob. Bind that exact blob in
   `validate_repository_policy_core.py` and queue regression. PR gate remains
   unchanged unless evidence proves necessary; its current regression already
   imports the queue test.
4. Freeze/review the material gate candidate. Do **not** integrate it while the
   protected audit still approves only the old blob.
5. Create a separate protected-base pin-rotation PR for
   `merge-authority-audit.yml`, binding only the precomputed future gate blob.
   Preserve self-modification refusal and all audit protections. Independent
   exact-head review + normal protected integration/readback are required.
6. Reconcile the unchanged gate candidate to protected main, requalify, obtain
   independent exact-head review, use normal FULL Merge Queue, and read back
   protected source.
7. Archive the WP1 task and release all special path custody.

## Negative controls

- native failure injected independently at Windows command positions 1–4 must
  terminate the modeled block before later commands can mask it;
- lifecycle assertion failure must make the real regression command nonzero and
  must therefore fail the governance/candidate job feeding `game-gate`;
- mutations adding `continue-on-error`, skip conditions, early exit or replacing
  required PG/SIM commands remain rejected;
- missing/cancelled/skipped aggregate dependency remains rejection.

## Positive controls

All existing build, Clippy, client smoke, synthetic harness, simulation,
PostgreSQL, Linux, CodeQL, dependency review and supply-chain work remains and
passes on valid source. Lifecycle regressions pass unmodified on valid source.

## Explicit non-goals

No CI-cost routing change, ruleset/status/MQ setting change, protected-audit
bypass, workflow permission change, runtime/product/Cargo/registry mutation or
release of WP2/WP3/WP4/Server Seam/production.

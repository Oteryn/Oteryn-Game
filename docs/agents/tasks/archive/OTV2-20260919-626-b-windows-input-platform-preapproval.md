> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #678 merged as `8f07f31289719cb4f7247f6d34a2f6c8061b3e6a`, and the canonical task branch is deleted. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260919-626-b-windows-input-platform-preapproval

```yaml
task_id: OTV2-20260919-626-b-windows-input-platform-preapproval
title: Preapprove Finding B Windows input-platform Merge Queue blob
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/626-b-windows-input-platform-preapproval
pr: null
issue: 626
parent_issue: 162
base_sha: c7688069bc22ac3cde46e48e6b05d8051418fed1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: impl qa
created_at: 2026-09-19T16:21:33Z
updated_at: 2026-09-19T16:21:33Z
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-authority-audit.yml
  - docs/agents/tasks/active/OTV2-20260919-626-b-windows-input-platform-preapproval.md
public_contracts:
  - protected-base preapproval of the exact future Merge Queue gate blob
depends_on: []
blocks:
  - "#626 Finding B Stage B"
cross_repository_coordination_id: null
external_repositories: []
```
## Outcome

Preapprove exactly the future Merge Queue workflow bytes required for #626 Finding B without modifying the Stage-B workflow itself. The protected-base audit pin changes from the current Merge Queue blob to the deterministic future blob that adds the existing input-platform package tests to the Windows queue job.

## Architecture and source of truth

- PROVEN: allocation authority is #162 comment `5743135862`.
- PROVEN: allocated base is `c7688069bc22ac3cde46e48e6b05d8051418fed1`.
- PROVEN: current audit blob at allocation is `ef8cb59eab5402b8574644cf14b17af8a6dbe9a8`.
- PROVEN: current Merge Queue gate blob is `df22c9a40847ce759337ab63c84a28e1180892ba`.
- PROVEN: protected `main` later advanced path-disjointly to `c2b755b3ff5996a4f18001bbf9f935c5dd59b3b6`; neither Stage-A workflow blob changed.
- DERIVED and reproducibly checked: intended future Merge Queue gate blob is `ad439cf3b04aaea084521f7be37761d3b1458cc5`.

## Deterministic future-blob construction

Starting from the exact current `.github/workflows/merge-group-gate.yml` bytes, insert exactly one unconditional step in the existing `rust_windows` job immediately before the deterministic simulation step:

```yaml
      - name: Test Windows input platform
        shell: pwsh
        run: cargo +1.94.0 test --locked -p oteryn-input-platform --target x86_64-pc-windows-msvc
```

No other future byte changes are admitted by this Stage-A task.
The Git object identity is computed from the resulting UTF-8 bytes using the canonical Git blob formula:

```text
SHA1("blob " + decimal_byte_length + NUL + exact_bytes)
```

Reproducibility control: applying the same computation to the unchanged current workflow yields exactly the repository-reported current blob `df22c9a40847ce759337ab63c84a28e1180892ba`; applying it after only the accepted insertion yields `ad439cf3b04aaea084521f7be37761d3b1458cc5`.

## Invariant-preservation checklist

- preserve every existing job and job dependency;
- preserve all permissions and action pins;
- preserve merge-group identity and lane selector/routing behavior;
- preserve Linux workspace and configured PostgreSQL 17.6 evidence;
- preserve Windows client build, strict Clippy, release smoke and synthetic harness;
- preserve Windows simulation-determinism execution;
- preserve supply-chain, dependency review and CodeQL behavior;
- preserve aggregate validation and stable `game-gate` fan-in;
- do not include the CodeQL 4.38.0 update;
- do not perform unrelated cleanup.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` to runtime authority/recovery APIs. This task rotates one protected control-plane preapproval pin only; it performs no production mutation, persistence recovery, PREPARE/COMMIT, controller restoration, session replacement or live-data operation.

## Stage-B boundary

**Stage B is NOT allocated by this task.** In particular this task does not write `.github/workflows/merge-group-gate.yml`, `.github/workflows/merge-gate.yml`, `.github/workflows/rust.yml`, repository policy validators/tests, or the build/test matrix. Those remain read-only until #162 performs a fresh post-integration custody reconciliation and explicit Stage-B release.
## Acceptance criteria

- [x] Exact two-path write custody only.
- [x] Future Merge Queue bytes are deterministic and reproduce the current blob before the intended insertion.
- [x] Audit pin is changed only to the exact derived future blob.
- [ ] Repository policy/governance/focused regressions pass on the candidate.
- [ ] `git diff --check` and exact-path readback pass.
- [ ] Whole-diff implementer self-review has zero material findings.
- [ ] Independent CONTROL/HIGH exact-head review has zero unresolved material findings.
- [ ] Applicable exact-head hosted CI is complete.

## Validation

Focused commands for this Stage-A control-plane generation:

```text
python tools/agents/validate_governance.py
python tools/repository/validate_repository_policy.py
python tools/agents/tests/test_governance_lifecycle_discovery.py
python tools/repository/test_validate_merge_group_pg_sim.py
git diff --check
```

The protected-base `Merge authority audit` intentionally rejects a candidate that modifies `.github/workflows/merge-authority-audit.yml` itself. That self-audit outcome must remain truthful and is not to be weakened, skipped or relabelled green.

## Excluded scope

No Stage-B workflow activation, no CodeQL-version update, no runtime/Cargo/persistence/product change, no protection/ruleset mutation, and no merge/enqueue by this worker.

## Independent review

Required: **YES — CONTROL/HIGH whole-diff review** on the immutable exact head because this rotates a protected Merge Queue authority pin. Review evidence belongs on the PR/check generation after the final commit exists.

## Context checkpoint

```yaml
last_progress: deterministic future Merge Queue blob derived and Stage-A pin prepared
status: validating
branch: agent/626-b-windows-input-platform-preapproval
head_sha: null
pr: null
final_head_sha: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
blocker: null
next_action: run deterministic local validation, freeze one exact head, publish by normal non-force Git push, then qualify PR CI and independent review
```

Refs #162 #626.

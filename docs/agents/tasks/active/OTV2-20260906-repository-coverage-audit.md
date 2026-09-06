# OTV2-20260906-repository-coverage-audit

```yaml
task_id: OTV2-20260906-repository-coverage-audit
title: Audit repository coverage and publish verified evidence
mode: AUDIT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: audit/repository-coverage-20260906
issue: 359
pr: 360
admission_main_sha: 7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3
base_sha: 7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3
head_sha: null
final_head_sha: null
owner: repository-coverage auditor
created_at: 2026-09-06
updated_at: 2026-09-06
execution_budget_minutes: 120
large_budget_reason: historical field only; current main supersedes fixed wall-clock stop semantics
owned_paths:
  - docs/agents/reports/repository-audit-20260906/
  - docs/agents/tasks/active/OTV2-20260906-repository-coverage-audit.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Outcome and current authority

Owner-requested audit and repository publication, tracked in #359 and PR #360. The report records all 803 pinned source entries, 32 audit controls, 17 findings, fresh hosted executions and their remaining limitations. Inventory/static scanning is not full semantic review. The open Issue must not be closed merely by integrating this report while its applicable semantic/runtime controls remain incomplete.

The audited product remains pinned at `7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3` / tree `359a52348bfdf8088a7cd456f4015b05279721b6`. Protected main later advanced to `b008614881fcc74f09e55e4d1b9e6c64ece04ce9` only through three agent-instruction documents. The audit branch adopts that instruction delta by normal merge-up; it does not silently redefine the pinned product snapshot.

## Completed evidence

- Complete Git blob/size identity and full-text census: 803 files; 557 Markdown; 102 Rust; 53 Python; all 17 workflows.
- Actual hosted Cargo metadata: 21 workspace packages, 43 internal edges, 369 resolved packages.
- Run 34049229486: Linux release all-targets/all-features workspace build and tests with actual PostgreSQL 17.6; exact release bootstrap/harness invocation. 718 passed test executions, not unique scenarios.
- Run 34049958965: original-test LLVM export with retained limitations; separate actual Rust parser/PKCE and SQL probes; Windows exact release, 33 original component tests and a real production DX12 first-frame probe.
- Run 34050635585: standalone Rust 11 tests PASS; Python 3.12.14 26 of 27 commands PASS. The original lifecycle suite raises three setup errors; this audit run remains FAILURE and the failure is retained.
- Run 34056706385: native libFuzzer/ASan continuation against the exact pinned product. Wire parser: 69,627,322 units in 301 seconds, 5,819 new units, no crash. Content parser: 4,757,388 units in 301 seconds, 1,511 new units, no crash. Full bounded interpretation and artifact digests are retained in `CONTINUATION.md`.
- README, per-file coverage definition/reconstructor, findings, controls, compact raw reproduction evidence and exact run/artifact/binary hashes are retained in the report directory. Full original archives and generated CSV accompany the owner package.

## Publication scope

Temporary audit observers existed only on this task branch to collect bounded evidence. They never became required checks or standing merge authority. The current checkpoint removes `.github/workflows/repository-audit-evidence-20260906.yml` from the branch tree after the successful native fuzz run; its historical commits and Actions runs remain immutable provenance.

Active product/CI allocations remain untouched. No runtime, Cargo, migration, protocol, registry, canonical workflow, ruleset, Merge Queue, deployment, live-data, secret or external-repository product write is part of this checkpoint. Audit probe sources under the report directory are evidence, not product fixes or canonical test targets.

## Acceptance and residual controls

- [x] Complete pinned file inventory with distinct identity/static/focused-semantic coverage.
- [x] Exact command, environment, run/job, checksum and outcome evidence for executed checks.
- [x] Findings distinguish defects, recommendations, readiness gaps and measurement limitations.
- [x] Previous report reconciliation: #353 is implemented through #358, not allocation-only.
- [x] Every audit control has an explicit status and missing-evidence statement.
- [x] Bounded native coverage-guided fuzzing executed for wire and content parser targets; no crash in either five-minute campaign.
- [ ] Resolve all remaining semantic, runtime, external-source, performance, operations and distribution controls in controls.json.
- [ ] Genuinely independent semantic review where required; none is claimed for this audit's own work.
- [ ] Final protected integration, if separately selected by the owning repository control plane; saving this report does not authorize a bypass.

## Validation and self-review

The evidence verifier reconstructs the exact 803-row CSV from the original Git tree and checks its SHA256. Downloaded fuzz artifacts were independently hashed after download and matched the Actions artifact digests. The bounded fuzz run is stronger parser robustness evidence but not exhaustive or release-equivalent execution.

Current upstream instruction changes were read before this checkpoint. They explicitly supersede historical fixed 60/120-minute stop semantics; the budget field above is retained only as provenance and does not stop productive continuation.

## Context checkpoint

```yaml
status: waiting
last_progress: native wire/content libFuzzer campaigns passed; current main instruction delta reconciled; temporary audit workflow removed from checkpoint tree
owner_action_required: null
blocker: applicable residual controls remain unverified; report publication is not a 100-percent semantic certificate
next_action: continue the explicitly listed residual controls under current main authority without repeating completed inventory, release builds, PostgreSQL execution or the completed five-minute parser fuzz campaigns
```

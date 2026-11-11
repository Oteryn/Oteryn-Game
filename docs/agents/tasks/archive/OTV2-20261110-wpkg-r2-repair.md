# OTV2-20261110-wpkg-r2-repair

```yaml
task_id: OTV2-20261110-wpkg-r2-repair
title: WPKG-R2 repair, runtime qualification and native World completion
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/pr3076-complete-e48f
pr: 3106
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: PR3106 repair lead; sole publisher and review dispatcher in comment 6786929490
created_at: 2026-11-10T15:59:04Z
updated_at: 2026-11-11T10:14:39Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20261110-wpkg-r2-repair.md
  - docs/agents/tasks/archive/OTV2-20261110-wpkg-r2-repair.md
public_contracts:
  - docs/contracts/protocol-oteryn/v1/cast/cast-v1.md
  - docs/contracts/S17_CAST_CONSENT_PERSISTENCE.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implementation and authoring complete; integration and post-merge acceptance
pending. This is the task's final authoring record for PR #3106, not a claim that
the final candidate has passed CI, merged, or released ownership. The exact
bounded file allocation and remaining acceptance belong to live control
[6786929490](https://github.com/Oteryn/Oteryn-Game/pull/3106#issuecomment-6786929490);
the two owned paths here record only the final documentation move. Source
allocations remain those published in that control, with no additional writes
authorized by this record.

## Architecture and source of truth

- PROVEN: bound META policy `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`,
  version 3.1.0; Game remains the sole product write authority.
- PROVEN: the lead alone publishes and dispatches Codex review. Disjoint worker
  allocations, expected-head publication and exact readback precede FREEZE_SHA.
  A material repair requires explicit return to AUTHORING and fresh qualification.
- PROVEN: Game's override in `docs/agents/AGENTS.md` selects squash merge and the
  full post-merge gate on `main`; protection bypass and self-audit bypass are excluded.
- PROVEN: live PR/check/control receipts govern current state. A commit cannot
  name its own SHA; final head, review and run bindings are recorded externally.

## Retained history

| Cohort | Verified disposition | Evidence boundary |
|---|---|---|
| Original collector repair, #3042 | Externally merged as `f01f18582443f482b7adcfaf32cae3f9d3616876` | Historical control 6767805546; that merge did not establish successful full Node or post-merge qualification. |
| Separate protected pin rotation, #3103 | Merged as `2520d704c05f67d3f63c3637e62f8a8ae14fd304` | Approved workflow blob `f574c67416ff8323327cfc6ac0e5829a1c9cba92`; audit self-modification refusal retained. |
| Grouped P2/P3 follow-up, #3108 | Externally merged as `edf8272737d66bd050519de6c19e32dbf6ad0ddb` | Collected mechanical follow-up; historical backlog 6766071958 retains explicit deferrals. |
| Runtime, CI ledger and native-content continuation, #3106 | Final authoring record; acceptance pending | Canonical branch `codex/pr3076-complete-e48f`, control 6786929490. |

Earlier fixture, cast, collector and scheduling receipts remain in #3042 and
the prior revisions of this record. Their results qualify their own SHAs only.

## Implementation / findings

- PROVEN: valid PR file-count drift discards transported scope evidence. Exact
  protected-base Git recovery may classify lanes; unavailable recovery selects
  FULL. Repository, state, head and base identity refusals remain strict.
- PROVEN: the CI success-ledger repair requires a completed successful execution;
  a failed Clippy step cannot create reusable success evidence. Approved workflow
  pinning remains separate from the workflow's own protected audit.
- PROVEN: the runtime continuation includes the reviewed admission/recovery and
  foreground-holder work; failure diagnostics use closed labels and registered
  event fields. Diagnostics do not establish or replace a runtime success result.
- PROVEN: the PostgreSQL ancestry fixture observes its own command through
  two-stage PID/transaction barriers, retains the competing NOWAIT lock assertion,
  and rejects an unrelated blocked reader. Clock qualification retains the 75 s
  advancement, 8 s owed-delta and 120 s outer budgets and all ten actions.
- PROVEN: cleanup accepts only the defined accepted/rejected auxiliary TURN
  reply pairs; it still requires a final accepted logout. The five real TLS
  regressions and three selected clock regressions passed on predecessor `cd6d149a`.
- PROVEN: main's three content failures exposed noncanonical World bytes, four
  previously admitted Djinn services and the additional Thaian import batch.
  The repair uses the native writer on an explicit package, preserves the complete
  source cohort, and retains strict document, identity and service assertions.
  Standalone Thaian completion and normal regeneration both require native output.
- PROVEN on predecessor `f959dfb444e09eae82f8b592750c5d388ee19acd`: hosted run
  42994570833 reproduced two native eleven-document packages, retained identical
  bytes on a third completion, and generated the three native pin outputs. The
  package tree is `74ff1708c9f85dbe20997265430fff5e5b157fa7c47dbfc4e28cff209bfe8b13`;
  the native bundle digest is
  `61357aebac5cd74b2f0bea6c920455940c6488a609f4b64e1f492a135113d7ab`.
  The source checkpoint still failed its tracked-package, focused-test and pin
  comparisons; its oversized focused log was unavailable. Final committed-source
  qualification remains pending and must pass every required check.

## High-risk authority/recovery qualification

Current node, session-generation, binding, lease and durable-root fences remain
the authority boundary. Historical terminal evidence cannot reacquire authority.
Cast retry remains bounded and rechecks node authority, bans and remaining lifetime.
The final review must retain the inherited cohort's negative, restart, retry,
replay, concurrency and PostgreSQL cases, plus the changed fixtures' real dispatch
and TLS evidence. No weaker assertion, added retry budget or ignored test is an
accepted repair. Exact case/run receipts remain pending for the final head.

## Acceptance criteria

The owner's later instruction to obtain **all tests green and merge** supersedes
this record's former blanket statement that Node is informational and the older
Node exception cited from comment 6745293631. Workflow classification alone does
not waive the owner's requested Node, integration or post-merge outcome.

- [ ] Final unchanged PR head: all required checks, `game-gate`, `heavy-gate` and
  all 24 PostgreSQL partitions pass with actual execution evidence.
- [ ] Node reaches `NODE_BOOT_RESULT=NODE_BOOT_PASS` with its required stages and
  the scenario verdicts required by live control 6786929490. Retain the actual
  clock, ranking and disconnect/resume row evidence. Explicitly nonblocking
  full-row coverage gaps remain reported gaps; this record adds no `W20b.full`
  PASS requirement. Real failing Node jobs or required stages remain blockers.
- [ ] Hosted **WorldProject canonical package**
  (`.github/workflows/world-project-package.yml`) reproduces two fresh completed
  packages, proves idempotence and exact eleven-document equality, and passes
  all three repaired integration targets and its existing checks.
- [ ] Native `pin-refresh` outputs are published and the final source passes
  `pin-check`; a runner's uncommitted generated files do not qualify the source SHA.
- [ ] The lead closes the complete changed-file review and the required Codex
  review binds the final material delta with no open concrete P0/P1 finding.
- [ ] Protected squash integration is eligible; the resulting `main` commit passes
  its full post-merge gate, including integration, Windows and dedicated seams.
- [ ] Residual P2/P3 findings have explicit grouped dispositions; ownership is released.

## Reference parity

NOT_APPLICABLE: repair of existing behavior, source serialization and qualification.
No new mechanic, reference value or public protocol change is asserted.

## Validation

Local checks below passed on this authoring composition; governance checks are
repeated after the archive move before publication. Hosted results remain pending
on the final SHA. The user requested light local checks and hosted heavy/database
validation. Previous-head passes remain historical evidence, not final acceptance.

- `python tools/agents/validate_governance.py` — PASS; 22 required policy documents and 9 lanes.
- `python -m unittest discover -s tools/agents/tests` — OK; 129 tests.
- `python tools/repository/validate_repository_policy.py` — PASS; 23 files and 70 workflows.
- `python -m unittest discover -s tools/repository -p 'test_job_result_key.py'` — OK; 31 tests.
- `python tools/repository/test_classify_pr_test_lanes.py` — PASS; 12 routing cases.
- `python tools/repository/test_validate_pr_gate_pg_sim.py` — PASS; 10 placement tests, routing and fan-in checks, 47 workflow mutations and four native failure positions with PowerShell 7.7.0 on Unix.
- `python tools/ci/test_merge_gate_scope.py` — OK; 3 tests.
- `python tools/qualification/node_boot/test_diagnostics.py` — OK; 9 tests.
- `python -m unittest discover -s tools/content-migration -p 'test_native_world_canonicalization.py'` — OK; 7 tests after materializing the unchanged sparse-checkout source packet.
- `python tools/content-migration/test_regenerate_content.py` — PASS; 15 checks.
- `git diff --check` — PASS. Complete candidate diff inspection: source, fixture,
  exact native-output hashes and main-adoption boundaries reviewed; final tree
  identity is recorded externally after publication.
- `python tools/content-migration/test_native_profile_composition.py` — OK; 6 tests.
- Pinned Rust 1.99.1 `rustfmt --edition 2024 --check` for the three changed content
  tests — PASS. Workflow YAML, 11 shell blocks and embedded Python syntax — PASS.
- `cargo fmt --all --check`; affected-package Clippy/tests, including dispatch's
  `--all-features`, and architecture workspace validation — pending; hosted execution.
- `cargo test --locked -p oteryn-game-server --test content_npc_catalogue` — pending; hosted execution.
- `cargo test --locked -p oteryn-game-server --test content_npc_service_model` — pending; hosted execution.
- `cargo test --locked -p oteryn-game-server --test content_world_project_repository` — pending; hosted execution.
- WorldProject canonical package native writer/regeneration, eleven-document
  equality and regenerated-versus-committed pin equality — pending; hosted
  execution through `.github/workflows/world-project-package.yml`.
- Native bundle `pin-check` and its exact command/job receipt — pending in the
  selected World bundle qualification lane.
- PostgreSQL, Node, full integration and post-merge run/job manifest — pending.

## Self-review

The lead inspected the bounded source/test/workflow delta and exact native data
manifest. Independent review verified all eleven package hashes and three native
pin outputs, preserving entry_start and strict comparison results. Main's protected
audit is deliberately not copied into the PR delta: its self-modification refusal
remains intact, and prospective squash must retain main's approved audit blob.
Final remote tree readback and hosted acceptance remain pending in live control.

## Independent review

Required: YES. The historical clean review of `29bb` and earlier #3042 receipts
are retained only for their actual scope. The final material delta needs its
own applicable review receipt; exact reviewed SHA, findings and disposition are
pending in control 6786929490. Only the lead dispatches under the bound review
policy; no duplicate request or unapproved extra round is implied by this record.

## PR and closeout

- Canonical PR: https://github.com/Oteryn/Oteryn-Game/pull/3106.
- Final FREEZE_SHA and review/CI receipt: pending in live control 6786929490.
- Merge commit/result: squash merge of #3106 — pending, not yet claimed.
- Post-merge protected-main readback and full gate: pending.
- Ownership release: pending after verified integration and post-merge acceptance.
- Archive placement closes authoring under `tasks/archive/README.md`. It does
  not make an unmerged repair accepted. Do not move the frozen head merely to
  copy later CI/review state into this file.

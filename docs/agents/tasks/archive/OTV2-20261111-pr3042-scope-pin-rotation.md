# OTV2-20261111-pr3042-scope-pin-rotation

```yaml
task_id: OTV2-20261111-pr3042-scope-pin-rotation
title: Preapprove the bounded PR3042 scope-recovery workflow
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/pr3042-scope-pin-proposal-20261111
pr: 3103
issue: 3042
base_sha: c255c6b88dff9467deae5dfc7cd0ff1a0c8fb75f
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: PR3042 repair lead; sole publication and review dispatcher
created_at: 2026-11-10T23:21:46Z
updated_at: 2026-11-11T05:33:42Z
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-authority-audit.yml
  - docs/agents/tasks/archive/OTV2-20261111-pr3042-scope-pin-rotation.md
public_contracts:
  - protected merge-authority approved workflow blob
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Rotate the protected audit pin to exactly the reviewed PR3042 merge-gate blob,
without changing that workflow in this PR. The proposal preceded PR3042's merge;
the same workflow is now on main. Canonical rotation PR #3103 closes this task.
This record is prepared for archive in its final authoring commit, as required
by `docs/agents/tasks/archive/README.md`; merge and post-merge acceptance remain
pending. The tracking issue remains PR3042's existing issue record.

## Architecture and source of truth

- PROVEN: META3.1.0 is bound to Oteryn/Oteryn commit
  `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.
- PROVEN: main is `c255c6b88dff9467deae5dfc7cd0ff1a0c8fb75f` at proposal
  preparation. Its audit approves merge-gate blob `2480e2eb9fb73d9af246188cde918359a260de94`.
- PROVEN: main readback at 2026-11-11T05:33:42Z is
  `f01f18582443f482b7adcfaf32cae3f9d3616876`, the squash of PR3042. The base
  metadata above records the historical proposal base, not current main.
- PROVEN: merged PR3055, commit `82ff2d753d26fa35fdc6686c07fffec7e545ddc5`,
  changed the cargo-deny step and the merge-gate blob from2480e2eb to
  `23a414176bb1c889de9f9477bf17c5c88d28dcbf`, leaving the audit unchanged.
  Its PR explicitly excluded `merge-authority-audit.yml`.
- PROVEN: frozen PR3042 candidate `69a60e191c9347e1a8978dd0cbcb2ae28b89c95b`
  contains the bounded, independently inspected workflow repair. Its exact blob
  is `98ae5ef50aee3e6839187d7edcfa550ee18b5b10`; this proposal preapproves
  those bytes, not an unspecified later edit.
- PROVEN: the current owner request covers PR3042 repair, subagents, frequent
  pushes, Codex review, all tests passing and eventual merge. This record does
  not invent a separate approval receipt for the protected-audit rotation.
- PROVEN: control comment 6782243515 records the lead's current authority and
  sole publication ownership, including the return to AUTHORING for this
  record-only closeout. No separate historical rotation approval is fabricated;
  integration remains subject to `docs/repository/GITHUB_GOVERNANCE.md`.

## High-risk authority/recovery qualification

NOT_APPLICABLE to the production session/lease/recovery matrix: no runtime,
persisted state, controller, protocol or live account is changed. This is a
protected CI control-plane change and still needs its deterministic guard
validation, exact-head evidence and independent deep review.

## Acceptance criteria

- [x] Exactly the approved merge-gate pin changes from2480e2eb to98ae5ef5.
- [x] Group pin7b7f36b1, agent pin42d84c90, self-modification refusal, exact
  head/base checks, read-only permissions and all other guards remain unchanged.
- [x] Repository/governance and relevant adversarial checks pass; the target
  blob identity and inert audit Python compilation are independently verified.
- [x] Codex receipt 6782300548 reports no major issues on published head `07909fea`;
  its reviewed audit blob remains unchanged in this record-only successor.
- [ ] The new frozen candidate passes its required qualification; the expected
  self-audit rejection remains disclosed and is not a green audit or a bypass.
- [ ] Normal squash, protected-main pin readback and post-merge qualification
  complete under the live control record after main runtime health is restored.

## Reference parity

NOT_APPLICABLE: CI preapproval changes no mechanic or reference gameplay value.

## Excluded scope

No merge-gate activation, group/agent pin change, self-authorization, weaker
guard, bypass, required-check change, runtime change or deployment. The older
collector audit/group pair must not replace current main's pair.

## Implementation / findings

The proposal changes only `EXPECTED_MERGE_GATE_BLOB` and this task record.
The reviewed group pin remains `7b7f36b197a72a4317d6318b1cc4ec4ab9b602f6`.
The candidate scope repair retains live open/same-repository/main-base/exact-head
fences, rejects malformed counts, and invalidates transported file evidence on
summary drift so trusted-base routing uses immutable Git evidence or FULL.

The audit intentionally rejects a PR that changes the audit itself. That
expected signal remains intact and is not a passing test, a green audit, or
permission to bypass controls. The exceptional rotation process is documented
in `docs/agents/tasks/archive/OTV2-20260904-merge-group-audit-pin-rotation.md`.
This proposal therefore cannot claim every check is green before integration.

## Validation

### Final-authoring record change

- `python tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python -m unittest discover -s tools/agents/tests`: PASS, 129 tests on that tree.
- `git diff --check`: PASS; only this record moves and changes.

### Historical focused evidence on `07909fea`

- PASS: one-line pin-only diff and recomputed target Git blob98ae5ef5. Replacing
  only the new pin with the old pin reconstructs the exact main audit bytes.
- PASS: exact audit Python compiles; an inert API-fixture probe runs one approved
  candidate and13 negative cases, including self-modification, moved/foreign
  scope, incomplete enumeration, unapproved/missing/nonregular workflows and
  changed group bytes. This is a component check, not a live GitHub audit.
- `python tools/repository/validate_repository_policy.py`: PASS,23files/68workflows.
- `python tools/repository/test_merge_authority_applicability.py`: PASS,1test.
- `python -m unittest discover -s tools/repository -p test_validate_pr_gate_pg_sim.py`:
  PASS,10tests; `git diff --check`: PASS.
- Full `test_validate_merge_group_pg_sim.py` and actionlint were not run locally:
  PowerShell/actionlint are unavailable. The group workflow is unchanged;
  required hosted qualification remains pending. No heavy or database test ran.

### Component/integration

The published `07909fea` candidate received the independent review recorded below.
This record-only successor requires its own published freeze and required CI;
historical component checks are not relabelled as execution on the new SHA.

### E2E

NOT_APPLICABLE: this proposal does not activate a workflow or change runtime.

### Exact-head CI

PR: https://github.com/Oteryn/Oteryn-Game/pull/3103. The successor's final head,
run/job IDs and results remain pending publication in control comment 6782243515.
The expected protected-base self-modification rejection remains distinct from
ordinary required-check results and protected-main post-merge audit success.

## Self-review

The original implementing worker inspected both proposal files and proved that
replacing only the new pin reconstructs the historical base's audit bytes.
The final-authoring worker inspected the record move and verified that audit
blob `5f5632ece0d2b5a3326bb246355f4d33e2a15293` remains byte-identical. This is
not a claim that the successor has passed hosted qualification or merged.

## Independent review

Required: YES, protected merge-authority change. Codex
[receipt 6782300548](https://github.com/Oteryn/Oteryn-Game/pull/3103#issuecomment-6782300548)
reports no major issues on `07909feac18b9851bd920f2c4b5ea721198038a6`.
Earlier source inspection by `review_fixture` found no P0/P1 and exercised
old-pin refusal, new-pin acceptance and8 adversarial refusals.

This final-authoring delta changes only the record; the reviewed workflow blob
is unchanged. The receipt retains its scope on `07909fea` and is not presented as
a review execution on the successor. No duplicate paid review is requested.

## PR and closeout

Canonical PR: #3103. Implementation and local record validation are complete;
the archive is delivered to main only if this closing PR merges. Root retains
publication and integration ownership under control comment 6782243515.

- Merge commit/result: squash merge of #3103 (pending).
- Final exact-head required CI, protected-main pin readback and post-merge
  qualification: pending; integration waits for restored main runtime health.
- Ownership release: pending successful integration and post-merge verification.

PR3042 already merged as f01f1858; this rotation does not retroactively qualify it.

## Context checkpoint

```yaml
last_progress: reviewed workflow preserved; closing record prepared in final authoring
status: completed
branch: codex/pr3042-scope-pin-proposal-20261111
head_sha: null
pr: 3103
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
next_action: lead publishes and freezes the record-only successor for its required CI
```

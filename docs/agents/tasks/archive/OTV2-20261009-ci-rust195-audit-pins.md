# OTV2-20261009-ci-rust195-audit-pins

```yaml
task_id: OTV2-20261009-ci-rust195-audit-pins
title: Preapprove exact Rust 1.95 gate blobs
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: 1e77ca22b4fcb7869f8f1d169f4e12bc749b295d
branch: chore/ci-rust195-audit-pins-20261009
issue: 1927
pr: 1943
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: root client/CI task, sole publisher and review dispatcher under explicit owner instruction
created_at: 2026-10-09
updated_at: 2026-10-09
owned_paths:
  - .github/workflows/merge-authority-audit.yml
  - docs/agents/tasks/active/OTV2-20261009-ci-rust195-audit-pins.md
  - docs/agents/tasks/archive/OTV2-20261009-ci-rust195-audit-pins.md
depends_on: []
blocks: [OTV2-20261009-ci-rust195-workflows]
external_repositories: []
```

## Authority and lifecycle

The owner answered "Tak, przygotuj PR-y CI i przegląd" after reviewing the
21-path Rust 1.95 proposal, its two separate PRs and required independent review.
The root task is the sole publisher and review dispatcher for these bounded
candidates. A subagent prepares isolated local source and evidence only.
This grants preparation/publication/review, not merge, activation, deployment,
production, credential, ruleset or database mutation. No audit exception is
self-granted. No unrelated client or server edits are included.

Canonical draft PR #1943 is open against main. This record is archived in the
final AUTHORING successor, as required by tasks/archive/README.md. Its completed
status closes the authoring record only; review and integration are pending.
Root publishes the record-only successor and records the exact read-back remote
freeze outside this commit before candidate-specific validation/review.
Independent deep review is required and remains pending; no local check is
represented as that independent review or protected integration.

## Prepared change

Proposal source: PR #1942 at 0c3e9287eb208aad65714109874b1567f3a430a3,
`docs/repository/proposals/client-rust-195-20261009/01-audit-pin-rotation.patch`.
Trusted main was live-read as the base SHA above before creating this isolated
branch. The changed audit bytes must equal Git blob
`39e13fdf3d86086e815710a38e4689983542faad`.

Rotate only the future PR gate pin to
`4abf4feb48ab406eb09b7fa7c142c8edaa7d82a5`, the future MQ gate pin to
`2fe13184cae1982f7d9d605794bb98ecde760061`, and four command assertions
from Rust 1.94.0 to 1.95.0. Preserve agent-governance pin, inert-data admission,
main/head/repository fences, audit self-edit rejection, permissions, action pins,
forbidden behaviors and normal Merge Queue. No gate is activated by this PR.

## Validation

- `python tools/repository/validate_repository_policy.py`: PASS.
- `python tools/agents/validate_governance.py`: PASS.
- `python -m unittest discover -s tools/agents/tests`: PASS, 59 tests.
- `python tools/repository/test_merge_authority_applicability.py`: PASS.
- `python tools/repository/test_agent_governance_pr_targets.py`: PASS, 16 tests.
- `python tools/repository/test_validate_pr_gate_pg_sim.py`: PASS, 31 canonical
  regressions plus queue/routing checks, using official PowerShell 7.5.4 for all
  four native failure positions and 28 workflow mutation cases.
- Official actionlint 1.7.7 on the changed audit workflow: PASS.
- Exact proposal bytes, audit Git blob and paired actual future gate/audit
  inert-data fixtures: PASS; six fail-closed canaries PASS.
- `git diff --check`: PASS.

Both tool archives were SHA-256 checked against their official release lists.
These are local authoring checks. The paired future fixture does not make the
current protected audit green: its intentional self-edit rejection was separately
exercised and retained. Hosted exact-head CI and independent review remain
pending after remote freeze.

## Dependency and owning decision

The unchanged protected-base workflow deliberately rejects a candidate that
changes the audit itself. This expected self-edit failure remains visible.
Before any governed integration, owner authorization must be bound to the
frozen PR/head and confined to that intentional audit rotation; every other
qualification and independent deep review must pass. The preparation/review
grant above does not grant that exception or any merge authority.

After separately authorized normal MQ integration and protected-main readback,
the dependent migration requires its ordinary successful protected-base audit.
No exception is transferred to that second PR or a materially changed head.

## PR and authoring closeout

Canonical draft: https://github.com/Oteryn/Oteryn-Game/pull/1943.
The source audit blob remains exactly the prepared value above; this final
authoring successor changes only this record and its active-to-archive location.
The frozen SHA is recorded outside the commit on the PR/coordination issue.
Independent deep review and hosted final-head qualification remain pending.
Merge commit/result is pending; a future squash merge of #1943 requires separate
owner authorization, normal MQ proof and protected-main readback. No integration,
production activation or ownership-release result is claimed by this archive.

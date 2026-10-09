# OTV2-20261009-ci-rust195-workflows

```yaml
task_id: OTV2-20261009-ci-rust195-workflows
title: Align active Rust CI and reviewed expectations to 1.95
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: 1e77ca22b4fcb7869f8f1d169f4e12bc749b295d
branch: chore/ci-rust195-workflows-20261009
issue: 1927
pr: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: root client/CI task, sole publisher and review dispatcher under explicit owner instruction
created_at: 2026-10-09
updated_at: 2026-10-09
owned_paths:
  - .github/workflows/g4-canonical-worldproject-package-seed.yml
  - .github/workflows/g4-item-binding-pilot.yml
  - .github/workflows/gameplay-server-seam.yml
  - .github/workflows/item-content-continuity.yml
  - .github/workflows/item-content-promotion.yml
  - .github/workflows/item-content-verification.yml
  - .github/workflows/merge-gate.yml
  - .github/workflows/merge-group-gate.yml
  - .github/workflows/native-entry-room-qualification.yml
  - .github/workflows/node-boot-qualification.yml
  - .github/workflows/rust-cache-pilot.yml
  - .github/workflows/rust.yml
  - .github/workflows/synology-game-deploy.yml
  - .github/workflows/worldproject-v2-full-cardinality-scale.yml
  - .github/workflows/wp5-s3a-real-interop.yml
  - .github/workflows/wp5-s3b-composition.yml
  - tools/repository/test_validate_merge_group_pg_sim.py
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - tools/repository/validate_pr_gate_pg_sim.py
  - tools/repository/validate_repository_policy_core.py
  - docs/agents/tasks/active/OTV2-20261009-ci-rust195-workflows.md
  - docs/agents/tasks/archive/OTV2-20261009-ci-rust195-workflows.md
depends_on: [OTV2-20261009-ci-rust195-audit-pins]
blocks: [PR-1942-native-client-CI-qualification]
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

Current state is AUTHORING, with no PR number or remote freeze yet. Root opens
the main-targeted draft, records its positive PR number, completes the final
authoring/archive step required by tasks/archive/README.md, then freezes the
exact read-back remote head before candidate-specific validation/review.
Independent deep review is required and remains pending; no local check is
represented as that independent review or protected integration.

## Prepared change

Proposal source: PR #1942 at 0c3e9287eb208aad65714109874b1567f3a430a3,
`docs/repository/proposals/client-rust-195-20261009/02-toolchain-migration.patch`.
Trusted main was live-read as the base SHA above before creating this isolated
branch. The 20 active source paths preserve that prepared proposal plus the two
canonical evidence-job hash rotations required by the same toolchain-only change.

Align 16 workflows and four current validators/regression expectations to
Rust 1.95.0. Rotate only exact reviewed contract hashes whose text changes.
Preserve triggers, permissions, action pins, scope/final validation, required
checks, fail-closed authorization, negative tests and normal Merge Queue.

The exact new PR gate blob is
`4abf4feb48ab406eb09b7fa7c142c8edaa7d82a5`; the new MQ gate blob is
`2fe13184cae1982f7d9d605794bb98ecde760061`. Post-merge Rust workflow
SHA-256 is `942ddf0ec317633a20113e89a91139dc56aa75630ffce28d2203437b7443039b`.
Canonical PR evidence-job SHA-256 pins are
`1172ab6117f12a0948698d69c22d4af0accdde15ddeb820455112ea03484b570`
for Rust Linux and
`9d09c490c39d26abcc98e6953646149055142a36a700b31de43e2f418e4926bf`
for Rust Windows. The original reviewed job bytes match their old hashes;
each new job differs only by the explicit 1.94.0-to-1.95.0 replacement.
This branch does not change the audit workflow, Rust dependencies, Cargo.lock,
MSRV declarations, world content/pin or runtime source.

## Validation

- `python tools/repository/validate_repository_policy.py`: PASS, including the
  full wrapper's canonical PR Linux/Windows evidence-job hashes.
- `python tools/agents/validate_governance.py`: PASS.
- `python -m unittest discover -s tools/agents/tests`: PASS, 59 tests.
- `python tools/repository/test_validate_pr_gate_pg_sim.py`: PASS, 31 canonical
  regressions plus queue/routing checks, using official PowerShell 7.5.4 for all
  four native failure positions and 28 workflow mutation cases.
- Official actionlint 1.7.7 on all 16 changed workflows: PASS, with only the
  existing `oteryn-game` self-hosted runner label declared in local configuration;
  no error patterns were ignored.
- Exact proposal bytes plus the two evidence-job rotations, Git blobs/contract
  hashes and paired actual future gate/audit inert-data fixtures: PASS; six
  fail-closed canaries PASS.
- `git diff --check`: PASS.

Both tool archives were SHA-256 checked against their official release lists.
These are local authoring checks. Paired future compatibility does not make the
current protected-base audit green: the old-pin dependency was separately
exercised and retained. Hosted exact-head CI and independent review remain
pending after remote freeze.

## Dependency and qualification boundary

This main-targeted draft depends on the separate audit-pin rotation reaching
protected main. The current protected-base audit rejects the changed PR/MQ
blobs until that predecessor integrates. This is an expected dependency, not
a green audit or an exception request for this PR. After prerequisite readback,
reconcile main without force/reset/rebase, freeze the final candidate, and
require the normal successful audit plus all other qualification.

Publication and review are authorized. Merge, activation, production operation,
protection changes and an audit exception for this migration are not authorized.


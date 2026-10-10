# OTV2-20261009-ci-rust195-workflows

```yaml
task_id: OTV2-20261009-ci-rust195-workflows
title: Align active Rust CI and reviewed expectations to 1.95
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: 1e77ca22b4fcb7869f8f1d169f4e12bc749b295d
branch: chore/ci-rust195-workflows-20261009
issue: 1927
pr: 1944
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
  - apps/client/installer/ci-installer.ps1
  - tools/qualification/native_entry_room/run.sh
  - tools/qualification/node_boot/run.sh
  - tools/qualification/spells/run.sh
  - tools/qualification/wp5_s3a/run.sh
  - tools/qualification/wp5_s3b/run.sh
  - docs/agents/tasks/active/OTV2-20261009-ci-rust195-workflows.md
  - docs/agents/tasks/archive/OTV2-20261009-ci-rust195-workflows.md
depends_on: [OTV2-20261009-ci-rust195-audit-pins, PR-1943]
blocks: [PR-1942-native-client-CI-qualification]
external_repositories: []
```

## Authority and lifecycle

The owner answered "Tak, przygotuj PR-y CI i przegląd" after reviewing the
21-path Rust 1.95 proposal, its two separate PRs and required independent review.
The root task is the sole publisher and review dispatcher for these bounded
candidates. A subagent prepares isolated local source and evidence only.
After the initial review identified P1 inline finding `4230087890`, root explicitly
returned #1944 to AUTHORING and authorized the six directly reachable helper
repairs below, the regression in an already owned test file, and this record.
This expands the original 20-path migration source scope to 26 source paths;
the separate audit proposal remains unchanged.
This grants preparation/publication/review, not merge, activation, deployment,
production, credential, ruleset or database mutation. No audit exception is
self-granted. No unrelated client or server edits are included.

Canonical draft PR #1944 is open against main and depends on audit-pin PR #1943.
This record is archived in the final AUTHORING successor, as required by
tasks/archive/README.md. Its completed status closes the authoring record only;
re-review and integration are pending. Root publishes the qualified successor and
records the exact read-back remote freeze outside this commit before validation
and independent review.
Independent deep re-review is required after the material P1 repair; no local check is
represented as that independent review or protected integration.

## Prepared change

Proposal source: PR #1942 at 0c3e9287eb208aad65714109874b1567f3a430a3,
`docs/repository/proposals/client-rust-195-20261009/02-toolchain-migration.patch`.
Trusted main was live-read as the base SHA above before creating this isolated
branch. The original 20 active source paths preserve that prepared proposal plus the two
canonical evidence-job hash rotations required by the same toolchain-only change.
The review repair adds six CI-invoked helper paths and strengthens the regression
in the already owned `test_validate_pr_gate_pg_sim.py`.

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
- `python tools/repository/test_validate_pr_gate_pg_sim.py`: PASS, 32 canonical
  regressions plus queue/routing checks, using official PowerShell 7.5.4 for all
  four native failure positions and 28 workflow mutation cases.
- Official actionlint 1.7.7 on all 16 changed workflows: PASS, with only the
  existing `oteryn-game` self-hosted runner label declared in local configuration;
  no error patterns were ignored.
- Exact proposal bytes plus the two evidence-job rotations, Git blobs/contract
  hashes and paired actual future gate/audit inert-data fixtures: PASS; six
  fail-closed canaries PASS.
- Actual helper/caller PowerShell AST parsing and fixture-build failure
  propagation: PASS (two parses, native exit 23 becomes caller exit 1).
- Five shell parses, 17 actual preflight refusals and four native Cargo spell
  mode failure canaries: PASS; source hashes bind the local evidence runner.
- Rust 1.95 `cargo check --locked -p oteryn-game-server --lib --bins` plus the
  three real composed/native-room integration targets: PASS, without linking
  binaries or starting services. Full no-run build is compiler-SIGKILL blocked.
- Rust 1.95 `cargo check --locked -p oteryn-synthetic-client-harness --all-targets`:
  PASS for the spell helper's client/live target.
- Base-to-candidate and staged `git diff --check`, plus `git show --check`: PASS.

Both tool archives were SHA-256 checked against their official release lists.
These are local authoring checks. Paired future compatibility does not make the
current protected-base audit green: the old-pin dependency was separately
exercised and retained. Hosted exact-head CI and independent re-review remain
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

## P1 helper toolchain repair

Review finding `4230087890` showed that installing Rust 1.95 in CI did not change
explicit `cargo +1.94.0` selections inside invoked helpers. The installer fixture
build, node boot and S3-A/S3-B helpers now select `+1.95.0` explicitly. A bounded
caller sweep also found plain Cargo in `native_entry_room/run.sh` and the nested
`spells/run.sh`: this branch's unchanged repository toolchain pins 1.94, so these
helpers also require an explicit selector. All six repairs change command
selection only; arguments, topology, producer/image/artifact pins, source fences,
cleanup and error handling remain unchanged.

The regression reads actual active workflows, follows their literal helper
references and conditional nested helper modes, and validates Cargo selection at
each reached command. Twelve mutations of actual helper bytes (a stale selector
and an implicit selector in each helper) and one additional stale nested helper
must be rejected. The manual `login_local/run.sh` is not reachable from active
workflow callers in this sweep; its pinned container image is outside this repair.
The old Rust prerequisite text in `wp5_s3b/README.md` and `spells/README.md` is
reported as retained documentation and remains untouched.

Actual PowerShell AST parsing and fixture-build failure propagation, shell parsing,
and full-script preflight refusal canaries qualify the preserved error boundaries
without starting services. Real Rust 1.95 target typechecking qualifies the helper
targets; full no-run code generation was resource-blocked by compiler SIGKILL in
this 8 GiB environment. These local checks do not establish Windows installer execution or
successful real composed Platform/server runtime qualification.

The existing spell qualification Python suite reports 13 PASS / 1 FAIL: its
stager-equality assertion at `test_qualification_tools.py:111` compares the node's
field set containing `loot_tables` with the deployment stager's smaller `FIELDS`.
The exact same mismatch is proven from trusted base `1e77ca22`, and the stager,
test and node field selection are unchanged by this repair. Canonical node boot
uses its own embedded stager; canonical Server Seam reads the manifest directly.
Neither depends on deployment `FIELDS` or this static test. This is a separately
retained deployment/test staging gap; the failing suite is not claimed as PASS.

## PR and authoring closeout

Canonical draft: https://github.com/Oteryn/Oteryn-Game/pull/1944.
Prerequisite: https://github.com/Oteryn/Oteryn-Game/pull/1943.
The 26-source-path candidate includes the complete P1 helper repair and its
actual-callgraph negative regression. The audit workflow and reviewed canonical
PR/MQ gate blobs remain unchanged by this repair. The frozen SHA is recorded
outside the commit on the PR/coordination issue. Independent deep re-review and hosted final-head
qualification remain pending. Merge commit/result is pending; a future squash
merge of #1944 requires separate owner authorization, prerequisite main readback,
ordinary green protected audit and normal MQ proof. No integration, production
activation or ownership-release result is claimed by this archive.

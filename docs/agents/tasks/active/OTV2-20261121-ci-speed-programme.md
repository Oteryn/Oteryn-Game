# OTV2-20261121-ci-speed-programme

```yaml
task_id: OTV2-20261121-ci-speed-programme
title: Measure and shorten CI compilation and PostgreSQL qualification
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/ci-bottlenecks-single-pr-20261121
issue: 3352
pr: null
base_sha: 984aefe4b93e1849fe92764e62ae26a66984fcd6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT standalone CI programme coordinator, issue 3352
created_at: 2026-11-21T09:29:01Z
updated_at: 2026-11-21T17:42:21Z
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/
  - .claude/skills/oteryn-dev/references/ci-merge.md
  - .claude/skills/oteryn-dev/references/local-checks.md
  - tools/ci/
  - tools/repository/
  - tools/content-migration/
  - tools/qualification/native_entry_room/
  - tools/qualification/node_boot/
  - tools/qualification/game_platform/
  - tools/qualification/wp5_s3b/run.sh
  - apps/game-server/tests/world_bundle_artifact_boot.rs
  - docs/agents/tasks/active/OTV2-20261121-ci-speed-programme.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Issue #3352 owns this programme. The first wave landed in several PRs; the owner
requires every second-wave repair in one integrated PR (2026-11-21). Make small changes cheaper to qualify,
measure actual edited Rust builds, and remove redundant migration embedding.
Preserve test coverage, source provenance, canonical PostgreSQL 18.6, required
checks and protected-main integration. No achieved latency target is claimed.

## Second wave: one integrated candidate

- Owner request: repair all audited CI bottlenecks in **one PR**; prior merge and
  force-merge authorization remains in the session. This is standalone CI work,
  not quest automation or a change to that programme's control plane.
- Integrated base: `984aefe4b93e1849fe92764e62ae26a66984fcd6`; sole writer:
  `chatgpt/ci-bottlenecks-single-pr-20261121`. State: **AUTHORING**, no frozen head yet.
  The earlier f067-based placeholder branch has no material commit or PR. This
  fresh candidate incorporates #3361 and preserves #3362's two-job build limit;
  all candidate-specific local checks are rerun on the integrated tree.
- Publication: native Git dry-run returned `fatal: unable to get password from
  user`. Use the already selected META 3.1.0 API-native new-candidate route with
  a complete bounded delta, sole-parent commit, fresh expected-head lease,
  non-force ref update and readback. No Remote Desktop publication workaround.

| Work packet | Owner | Implementation and preserved contract |
| --- | --- | --- |
| A: compiler and evidence | audit_compile_cache | Exact regression-to-compile-shard resolver; canonical shared Dispatch archive, current-run/head proof and complete result collector; measured PG duration refresh. |
| B: World and native tools | audit_pg_timing | Combined compatible World tools; bounded migration checks; native executable reuse and cleaned-up fallback; shared release World compiler. Real materialization, pin checks and all qualification cases remain. |
| C: qualification executables | audit_postmerge_failures | Shared Platform binary; independent Server Seam producer; correct spawn/clock features; per-archive fast workers and older-head compatibility. Fresh databases, keys and real Platform issuance remain per consumer. |
| Integration and publication | coordinator | Workflow wiring, package ledger completeness, protected pin rotation, one integrated review and CI, exact-head squash/readback. |

The reported offline-death job in run `44919024484`, job `136890762651`,
failed after 57m02s. Its downloadable log was unavailable; no OOM cause is
claimed. The new workflow measures compilation and the exact PG-backed test
separately and retains both diagnostics. Source inventory resolves the selected
case instead of hard-coding a shard number.

The final production smoke build is deliberately retained. Its package-scoped
feature graph differs from the workspace build with examples/dev dependencies;
reusing that workspace binary would change production qualification.

The protected-base audit rejects changes to its own file by design. This batch
requires a real pin rotation with the full candidate independently reviewed;
no green audit result, bypass actor, relaxed required check, or authority receipt
may be fabricated. Any rejection remains visible. No latency or monetary saving
is claimed until the integrated head has run; compare both critical-path time
and summed runner time, including artifact transfer and fallback builds.

Acceptance still pending: exact-head workflow validation, full integrated review,
real PostgreSQL/Node boot qualification and post-merge observation. Locally
unavailable live services are CI obligations, never recorded as passing tests.

Known baseline failures are separate from build optimizations: main run
`44920758625` at `f067b43cfde9c2559454f4b78e1763ac6bfc11c0` failed Node
`g-core-mp` with admission-journal holder contention and PostgreSQL partition 4
with a transaction timeout. No runtime repair or measured improvement is claimed
by the artifact changes. Independent review must include the full protected
workflow/pin delta and the older-head/partial-rerun compatibility paths.

### Second-wave local validation on the integrated tree

- Complete repository-tool suite: 278/278 tests, zero failures/errors/skips.
  The final run observed no source changes. The old heavy-gate fixtures were
  updated for the actual canonical producer and new dependencies; all prior
  failure/skip cases remain and the focused 26-test suite passes.
- Governance: 135 tests and validation pass; repository policy passes for
  23 files and 78 workflows.
- World/native helpers: 51 tests pass. Node prebuilt: 15; Platform: 13;
  Node diagnostics: 38; Node matrix: 16. Shell syntax checks pass.
- Workspace `cargo +1.100.0 fmt --all --check` and actionlint 1.7.12 on all
  13 changed workflows pass.
- Full merge-group contract passes, including 54 workflow mutations, four
  PowerShell native-failure positions, aggregation and protected routing.
  Nine local fixtures of the actual protected audit pass. These fixtures are
  not a claim that the live self-modification audit will pass.
- Derived pin freshness and whitespace checks pass. The reliability checklist
  selects CI control-plane, Node harness and governance checks; all listed
  local commands have now run.
- Not run locally: full project Rust compilation, live PostgreSQL/Platform and
  the World artifact boot integration target. No Cargo target/registry cache
  or local PostgreSQL route was available. These remain CI obligations.
  Manual integration dispatch also requires an authenticated execution route;
  the available cloud browser showed GitHub signed out.

These results qualify the authoring tree for publication and independent review;
they do not establish hosted CI success, deployment or performance savings.

## First-wave architecture and source of truth

- PROVEN: admitted main is `b20391213cde54511233bd02fae3370e019c7bf4`.
- PROVEN: META policy binding is the repository's existing immutable 3.1.0 binding.
- PROVEN: `GAME_MIGRATOR` already owns the canonical embedded migration chain.
- PROVEN: PG producer emits five archive variants and bare filenames in SHA256SUMS;
  the rerun consumer selected only e2e/lib and expected a leading `./`.
- DERIVED: removing duplicate embedding can reduce compiler work; the speedup is
  UNKNOWN until representative compilation measurements exist.
- UNKNOWN: warm-builder end-to-end improvement, RAM high-water mark and PG 19
  benefit on the accepted workload. Experiments must not change qualification pins.

## High-risk authority/recovery qualification

The second wave changes shared CI artifact custody and protected workflow pins.
Required qualification includes negative source/digest/inventory/attempt tests,
the actual protected-base audit behavior, all gate contract cases and an
independent exact-head review. It changes no production authority or schema.

First-wave scope: NOT_APPLICABLE; that batch changes diagnostic CI and reuses the identical
existing migrator. It does not change a production authority/fence operator,
migration SQL, durable-value rules or accepted merge eligibility. Archive source
identity, integrity, inventory ownership and actual runtime selection have
negative regression coverage in `tools/ci/test_pg_test_rerun.py`.

## Acceptance criteria

- [ ] Rerun every selected PG test exactly once per pass across all five archive
  variants, including legacy single-lib archives, with exact run/head custody.
- [ ] Reject bad checksums, wrong head/workspace, malformed/missing bundles,
  duplicate artifact identities and absent or unexpected actual nextest matches.
- [ ] Replace the 31 duplicate game-server migration embeds with the canonical
  migrator; compile affected targets and qualify migration refusal/history cases.
- [ ] Compare real edited builds with retained Cargo incremental targets and
  empty targets backed by isolated sccache; record source trees and exact patch.
- [ ] Qualify each final PR head through required checks and independent review;
  verify the post-merge full gate before claiming integration complete.
- [ ] Use observed timings, memory and storage to select further build/DAG work.

## Reference parity

NOT_APPLICABLE: build/CI infrastructure; no new player mechanic or reference value.

## First-wave excluded scope

No PostgreSQL version or durability change, skipped assertions, weaker provenance,
protection bypass, paid runner purchase, unrelated PC cleanup or main workflow
pin rotation is part of the initial three branches. Later protected workflow
rotation requires its separately prepared exact audit/activation contract.

## Implementation / findings

One writer per branch. Workers prepare uncommitted changes and release their
worktree; the coordinator reviews, publishes, freezes and qualifies candidates.
Branches: `chatgpt/ci-pg-flow-20261121`,
`chatgpt/ci-migrator-reuse-20261121`, `chatgpt/ci-build-deltas-20261121`.
This card remains active across those PRs and later measured programme work.

Native local Git publication was attempted with a non-mutating dry run and failed
because no GitHub credentials were available. The coordinator selects the bound
META API-native new-candidate route: complete bounded tree, one sole-parent
successor, fresh expected-head lease, one non-force ref movement, live readback,
then fresh freeze/CI/review. This does not preserve any earlier candidate identity.

## First-wave validation history

The first wave is merged: #3353 (PG reruns), #3354 (real-edit benchmark),
#3355 (per-archive PG scheduling and M23), #3356 (shared embedded migrations).
The exact heads, merge commits, focused checks, actual PG jobs and review
receipts are retained in issue #3352. Those historical results are not reused
as qualification of this second-wave candidate. The programme remains open:
real-edit measurements, persistent-builder qualification, the 10–15-minute
latency goal and measured RAM/cost savings have not been established.

## Self-review

Coordinator reviews the full bounded delta and archive/selection failure paths
before publication. Worker summaries do not replace source inspection.

## Independent review

Required on each separately merging PR under OWNER_FUNDED_AI_POLICY.md. Root is
the unique review dispatcher after a live same-head de-duplication check. Pending.

## PR and closeout

Canonical PRs are created after initial candidate publication and linked in #3352.
Programme and ownership remain active. No merge, post-merge result, achieved
speedup or completion is claimed by this authoring record.

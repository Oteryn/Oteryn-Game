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
pr: 3367
base_sha: 5695f7b8af567086efa11bdeb47a2fd609f24c2c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT standalone CI programme coordinator, issue 3352
created_at: 2026-11-21T09:29:01Z
updated_at: 2026-11-21T19:08:04Z
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
- Integrated main: `5695f7b8af567086efa11bdeb47a2fd609f24c2c`; sole writer:
  `chatgpt/ci-bottlenecks-single-pr-20261121`, PR #3367. State: **AUTHORING**
  for a successor of `b75d1098e052af3f17d5b8d4163ae79eb9b88f1f`.
  Root ended that freeze at 19:08 UTC to integrate main's named #3366 fix:
  the revert of #3361's confirmed field-cursor PG regression. Preserve #3365's
  narrower auxiliary-workflow routing and register the new reusable fast
  workflow as a canonical control. Its real routing test reproduced the
  missing registration before passing with the one-line repair. #3362's
  two-job build limit and all five earlier CI repairs remain included.
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

Offline-death run `44919024484`/job `136890762651` failed after 57m02s;
its final log was unavailable, so the cause is unknown. Resolve the exact
case's shard from source and retain separate compile/test resource diagnostics.

The final production smoke build is deliberately retained. Its package-scoped
feature graph differs from the workspace build with examples/dev dependencies;
reusing that workspace binary would change production qualification.

The protected-base audit rejects changes to its own file by design. This batch
requires a real pin rotation with the full candidate independently reviewed;
no green audit result, bypass actor, relaxed required check, or authority receipt
may be fabricated. Any rejection remains visible. No latency or monetary saving
is claimed until the integrated head has run; compare both critical-path time
and summed runner time, including artifact transfer and fallback builds.

Acceptance pending: final-head CI/review and post-merge observation. First-head
review was clean. On b75, fast 44936586415 passed all 25 workers and collectors;
content took 7m38, retained PG 13m28, native passed 3 tests, manual integration
passed 2043+38 tests and WorldProject passed 55 Rust+15 Python tests in 23m53.
Two actual PG archives were reused without compilation. These results qualify
only b75; the main-integrated successor needs fresh qualification and review.

Prior main `44920758625` failed Node admission contention and MAX501; the first
PR run repeated MAX501 with three competing tests. On b75, PG job136951451690
passed 219/219, including MAX501 with no overlapping sibling test. Slot
reservation preserves the 2000ms contract and 501-item atomicity. The final
successor needs its own qualification; no SQL or Node runtime repair is claimed.

### Second-wave local validation on the integrated tree

- Repository-tool suite: 283/283 tests, zero failures/errors/skips.
  The earlier integrated run had 278 tests. The old heavy-gate fixtures were
  updated for the actual canonical producer and new dependencies; all prior
  failure/skip cases remain and the focused 26-test suite passes.
- Governance: 135 tests and validation pass; repository policy passes for
  23 files and 78 workflows.
- World/native helpers: 51 tests pass. Node prebuilt: 15; repaired Platform: 16;
  Node diagnostics: 38; Node matrix: 16. Shell syntax checks pass.
- Content aggregate: five tests pass, including actual shell failure propagation;
  the 40 checks and every original qualification command remain preserved.
- Real nextest/mold protocol, MAX501 scheduling and linker installer reproduce the
  first-head errors and pass with the repairs; artifact custody remains strict.
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
  Manual integration dispatch uses the owner-authorized Molehill GitHub CLI
  after exact-head verification; no Git publication uses that route.

Local results qualify authoring only; immutable hosted evidence is in #3352.

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

First-wave publication receipts remain in issue #3352. The current route and
candidate lifecycle are recorded above.

## First-wave validation history

Merged first wave: #3353 (PG reruns), #3354 (benchmark), #3355 (PG scheduling/M23),
#3356 (shared migrator). Exact evidence remains in #3352, not qualification of
this candidate. The programme stays open for real-edit/persistent-builder
measurements, the 10–15-minute goal and measured RAM/cost savings.

## Self-review

Coordinator reviews the full bounded delta and archive/selection failure paths
before publication. Worker summaries do not replace source inspection.

## Independent review

Root is the sole dispatcher under OWNER_FUNDED_AI_POLICY.md. First-head review
completed clean (PR #3367 comment 7066802278). A material repair needs the second
and final independent delta review after local validation and successor freeze.

## PR and closeout

Canonical PRs are created after initial candidate publication and linked in #3352.
Programme and ownership remain active. No merge, post-merge result, achieved
speedup or completion is claimed by this authoring record.

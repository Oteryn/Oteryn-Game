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
updated_at: 2026-11-21T20:00:58Z
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
  for a successor of `448efc0582425c22bd5b5daab7b64cee45475d7c`.
  Root ended the freeze at 20:00:58 UTC after an actual-workflow reproduction
  showed three new exact references defeating #3365's auxiliary narrowing.
  The repair recognizes only those complete fixed-reference lines. Exact-file
  consumers, unknown literals, globs and dynamic references remain FULL.
  Integrated #3366's field-cursor revert, #3362's two-job limit, canonical
  fast-producer registration and all five earlier CI repairs remain intact.
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

Acceptance pending: successor qualification and post-merge observation. On 448,
fast 44941494744 passed all 25 workers/collectors in 19m16; content 44941494805
took 7m50, retained PG 44941494715 took 13m02, WorldProject 44941495199 took 24m15.
Native 44941494679 passed 3 tests; manual 44941617744 passed 2043+38 tests.
PG 44942645764 reused 2/3 eligible archives and all 14 reused workers passed;
other PG/Node workers were still running. These results qualify only 448.
QuestPC8 varied from 19m36 on b75 to 11m12.6 on 448; neither is a fixed floor.

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

## First-wave source of truth

Issue #3352 retains the admitted main, five-variant PG archive repair and shared
`GAME_MIGRATOR` evidence. Warm-builder improvement, RAM high-water mark and PG 19
benefit remain unknown; experiments must preserve qualification pins.

## High-risk authority/recovery qualification

The second wave changes shared CI artifact custody and protected workflow pins.
Required qualification includes negative source/digest/inventory/attempt tests,
the actual protected-base audit behavior, all gate contract cases and an
independent exact-head review. It changes no production authority or schema.

First-wave scope: diagnostic CI and identical migrator reuse; negative archive
custody regressions remain in `tools/ci/test_pg_test_rerun.py`.

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

Root is the sole dispatcher under OWNER_FUNDED_AI_POLICY.md. Both authorized
reviews completed clean: first 7533 (comment 7066802278), then 448
(comment 7068364491, 19:54:29 UTC). The new bounded routing repair is outside
those exact heads; it receives direct source review and behavioral validation.
No third paid review is authorized or dispatched. The bound META review policy
requires re-review only for a material risk-bearing delta; any such outstanding
requirement must be reported rather than inventing review coverage.
Coordinator assessment: this exact-line parser correction restores the existing
auxiliary route without changing canonical dependencies, selected test inventories,
permissions, gates or artifact custody. Actual-workflow RED/GREEN tests preserve
FULL for exact consumers, unknown literals, globs and dynamic paths; the previous
material review remains representative. No new material review is required.

## PR and closeout

Canonical PRs are created after initial candidate publication and linked in #3352.
Programme and ownership remain active. No merge, post-merge result, achieved
speedup or completion is claimed by this authoring record.

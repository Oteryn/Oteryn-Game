# OTV2-20261121-ci-speed-programme

```yaml
task_id: OTV2-20261121-ci-speed-programme
title: Measure and shorten CI compilation and PostgreSQL qualification
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/ci-warmup-balance-20261122
issue: 3352
pr: null
base_sha: dc0ea80cb74e19d5d21a5907895aead945270263
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT standalone CI programme coordinator, issue 3352
created_at: 2026-11-21T09:29:01Z
updated_at: 2026-11-22T08:38:48Z
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

## Third wave: measured cache and scheduling follow-up

The owner renewed execution and push authorization on 2026-11-22 and made
Molehill-PC available. PR #3367 is already merged as
`6eda3a4af6d1492baace7b26109c23cdddfbf7a4`; its exact-head Fast and actual
36-partition PostgreSQL gates passed. Current work is one new CI-only batch
from `dc0ea80cb74e19d5d21a5907895aead945270263`, not a continuation of writes
to the merged PR. Main advanced during initial clone; this actual checkout base
includes #3369/#3387. The measured run/benchmark source remains the earlier
accepted main `be364525d6965a900b1978f2f34e6bd6c98bd15c`; all ten task paths
are unchanged between those bases. Root owns publication, review and integration; workers own
disjoint cache, PG timing and Windows verification slices.

- The next cache consumer, run44957878502/job137013731566, restored the
  944,217,092-byte bounded archive in12s. Nevertheless, on current main
  run44996564744/job137129055003 the warmer took30m35; its no-run stage linked
  264 executables, including186 integration targets, before trimming removes
  all workspace products. Restrict that cache-only command to library tests
  and use the existing pinned mold linker. Keep all actual qualification tests,
  workspace/dev-dependency feature selection, package-specific warmups and the
  4GiB cap. Whole-workspace fingerprint equivalence is not claimed.
- Refresh the timing table with all7612 unique PASS results from the36 current
  main PG partitions in run44996564771:605 to912 estimates,307 additions and20
  increases, no removals/decreases. The real planner's scheduling model changes
  the longest worker from1200.608 to1016.955s on main and1412.049 to1266.805s
  on the independent PR3319 sample. Those are simulations at fixed observed
  case times, not measured new-run savings. Keep15/7/7/5/2 workers: reallocating
  them did not improve either model's maximum after the timing refresh.
- Windows run44996564771/job137129526796 spent22m54s on release compilation
  before failing strict Clippy. Move the same Clippy command before the same
  release command. Actual-step PowerShell probes must preserve success and
  stop immediately on lint/build/smoke failures.
- Run the existing real-edit benchmark against accepted main in an isolated
  Molehill checkout, with bounded CPU/disk/time use and a private target/cache.
  Record cold, no-op and leaf-edit cases separately; do not infer whole-CI,
  remote-cache, monthly-cost or aggregate-RAM gains from a server-library test.

Native Git dry-run publication failed because this isolated environment has no
GitHub credentials. Root selects the already authorized META3.1.0 API-native
new-candidate route for this exclusive branch: one complete bounded tree,
one sole-parent successor, fresh predecessor, non-force update and exact readback.
Remote Desktop is used only for isolated measurement, never publication.
Preserve PostgreSQL18.6, required checks, test assertions and source provenance.
The generated protected pin rotation still needs independent review; its
protected-base self-audit rejection is not a fabricated passing check.

Current main's full gate44996564771 is red outside PG: Windows dead code,
four client integration failures and Node rows. Runtime repairs in active
PRs #3388/#3389 belong to their existing owners and are not duplicated.
Keep this programme open: the whole-CI10–15min goal remains unproven, and one
unchanged end-to-end campaign alone took1016.955s on the measured main.

## Previous-wave evidence

Detailed first- and second-wave authoring history remains in issue #3352 and
this record at immutable commit
`be364525d6965a900b1978f2f34e6bd6c98bd15c` (before this continuation).
The first wave merged PRs #3353/#3354/#3355/#3356. The second wave merged
PR #3367 as `6eda3a4af6d1492baace7b26109c23cdddfbf7a4`.
Its frozen head `62b780d833b73f458d6bead7b3a955adddcddb76` passed Fast
run44953597836 in33m23 and actual PG run44955345659 in25m35, plus Content,
World, Native and retained regression qualifications. These results belong
to that previous head and are not qualification of the new candidate.

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

One writer per branch. Workers release bounded source changes; the coordinator
reviews, publishes, freezes and qualifies the complete candidate. First-wave
PRs #3353/#3354/#3355/#3356 and their evidence remain in #3352. This programme
stays open for edited-build/persistent-builder measurements and RAM/cost targets.

## Self-review

Coordinator reviews the full bounded delta and archive/selection failure paths
before publication. Worker summaries do not replace source inspection.

## Independent review

Root is the sole dispatcher under OWNER_FUNDED_AI_POLICY.md. A new PR gets
its own exact-head review after focused deterministic checks and initial
publication. The prior PR's two reviews and subsequent peer/runtime proofs
remain scoped to that earlier work; neither is a review of this continuation.
Current owner integration authorization persists, and required exact-head
checks remain mandatory. Do not change a frozen head without returning to
AUTHORING and requalifying the successor.

## PR and closeout

Canonical PRs are created after initial candidate publication and linked in #3352.
Programme and ownership remain active. No merge, post-merge result, achieved
speedup or completion is claimed by this authoring record.

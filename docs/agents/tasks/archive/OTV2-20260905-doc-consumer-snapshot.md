# OTV2-20260905-doc-consumer-snapshot

```yaml
task_id: OTV2-20260905-doc-consumer-snapshot
title: Restore reviewed document-consumer PR routing
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 309
pr: 310
base_sha: b9b1a4317858bffc25ad6af3cffcf7b5eff93445
admission_main_sha: b9b1a4317858bffc25ad6af3cffcf7b5eff93445
head_sha: 8a12e94693b51b344400da36627c33bbb181563c
final_head_sha: 8a12e94693b51b344400da36627c33bbb181563c
merge_sha: d89f063ea7ad7f1d8fa09688309c8d898fae856e
owner: null
original_owner: Codex-doc-snapshot
original_branch: ci/doc-consumer-snapshot
created_at: 2026-09-05T14:44:38Z
updated_at: 2026-09-07
owned_paths: []
public_contracts: []
blocks: []
external_repositories: []
```

## Outcome

Accepted implementation is protected; the outstanding natural-documentation
qualification is now directly observed. This terminal archive releases the
obsolete active record and preserves the original admission and implementation
history. It is not a new implementation, admission, ownership grant or product
acceptance. GitHub remains the live lifecycle authority.

## Original delivery and preserved evidence

PR #310 merged on 2026-09-05T15:19:07Z. Its final head is the one in the
terminal metadata, not the earlier head still mentioned in the PR body.
Original FULL PR `33973176570`, FULL MQ `33973968237`, protected push
`33974431227`, and #309/#308 review comments retain the accepted delivery.

The bounded original audit covered five new/changed server tests since #297:
authority_invariants, durability_postgres, server_ci_qualification and the two
authority support modules. It adopted only the reviewed all-consumer digest
`f8eed774249df64a5a64612b4a169a73bac093a7bcbfb21e59ea0e06dd2ddc26`
(124 entries), with unchanged non-server digest
`9f7aff4dc25c9c6561b77ea73342b675eeccb1d008ab9d1fbdbd504618ec5ab8`
(63 entries). Those values are historical, not current routing instructions.
Recorded RED/GREEN and modified/new-consumer, malformed/unknown/mixed/rename
negatives preserve the original fail-closed proof. No workflow was changed by
#310; standalone push optimization belongs to the separate #312 lineage.

The full original task text, test rationale, worker identity, historical 120-minute
budget and initial checkpoint are preserved by the immutable source link below.
The original six implementation/evidence paths are released, not transferred to
this archive. The static 100-candidate/507-run audit remains unchanged under
`docs/agents/evidence/OTV2-20260905-system-ci-impact/`.

Original complete task: [OTV2-20260905-doc-consumer-snapshot at protected a793457](https://github.com/Oteryn/Oteryn-Game/blob/a793457cf3001df37109acb2c4b4a772b53db97a/docs/agents/tasks/active/OTV2-20260905-doc-consumer-snapshot.md).
Original blob: `88fa104aac2113af1430f2f1f7637c8e6f321843`. Its initial waiting prose and budget are historical,
not an active dispatch or stop instruction.

## Observed post-deployment acceptance

The old wait for a natural qualifying documentation change is satisfied. These
are existing useful changes, not benchmark probes or no-op retriggers. The
original snapshot later became stale and correctly selected FULL; separately
reviewed #375 / #378 restored eligibility. Do not attribute the current tree to
the obsolete original digest or silently reactivate its implementation lease.

Protected successor #378: final head
`b20dfb4e476b694a3f5bd08b2c7d6ee64a9daf6c`, merge
`65a204ea284c54188b13c4c41cf7411b1a003c61`, FULL PR run `34107712276`,
FULL MQ `34108577361`, protected push `34109380351`. Its accepted review and
source audit remain in the archived `OTV2-20260907-doc-consumer-snapshot-refresh.md`.

| Existing sample | Exact head | Directly observed result |
| --- | --- | --- |
| #390 allocation PR, run 34145788453 | d383fd0ea4a77f61a2bcff0d480b58f10aae92f3 | Eight applicable jobs passed; four Rust jobs skipped with no assigned runner; game-gate passed. |
| #387 documentation push, run 34141454683 | a793457cf3001df37109acb2c4b4a772b53db97a | Classifier, policy/metadata and supply chain passed; Linux, PostgreSQL and Windows skipped with no assigned runner. |
| #387 normal FULL MQ, run 34140728064 | a793457cf3001df37109acb2c4b4a772b53db97a | Existing queue qualification succeeded; the push omission did not replace MQ. |

Actual push classifier job `101804136787` emitted at 2026-09-07T16:04:23Z:
`{"reason":"neutral-documentation","rust":false,"surface":"docs","windows":false}`.
Its exact protected checkout and coherent-output publication passed. The
post-merge governance `34141454583` and CodeQL `34141454572` also passed.
PR #390 remains independently review/integration-held; its successful routing
sample is not permission to integrate that allocation or activate WP2.

## Measurement method and limits

Use the sum of completed_at minus started_at for actually executed jobs;
exclude skipped jobs with no runner. This is unweighted runner-allocation time,
not billed minutes. Keep workflow elapsed separate from job sums and queue time.

- PR #390 executed-job durations: 4 + 43 + 54 + 13 + 9 + 5 + 4 + 2 = 134 seconds.
  Jobs: 101817428944, 101817461071, 101817461150, 101817461163,
  101817461188, 101817461282, 101817660460, 101817684753.
- #387 push executed-job durations: supply chain 51, classification 12,
  policy/metadata 24 = 87 seconds. Jobs: 101804136564, 101804136787,
  101804136817. GitHub run timing reports 53 seconds elapsed.
- Comparable historical documentation push timing endpoints report 502 seconds
  for `33969957232` and 550 seconds for `33973093609`. Original #308/#311
  evidence records 845 and 888 allocated seconds respectively; the latter
  includes 818 runtime seconds and 70 retained-check seconds. Those historical
  allocation figures are retained evidence, not a newly reconstructed job sum.

The observed elapsed differences are 449 and 497 seconds, but the changes use
different heads and environments. There is no controlled A/B, isolated causal
saving, monthly forecast or billing claim. Timing endpoints report zero billable
milliseconds, which must not be misrepresented as zero executed work or a price
saving. A FULL/server run still has the existing classifier dependency: original
post-integration FULL controls and the later #378 FULL control remain evidence;
no new claim of zero critical-path overhead is made.

## Closeout scope and remaining programme boundaries

Work's reserved lifecycle turn is recorded in #308 comment `5573827375`.
This closeout changes only the two #309/#311 active-to-archive task pairs, clears
their stale ownership and records the now-observed acceptance. It changes no
classifier, workflow, required check, ruleset, registry, Cargo, runtime or test.
No older implementation allocation or worker is restarted.

The old #308 first-wave measurements and both deliveries are reconciled; any
later unreviewed consumer change must still select FULL until a separately
reviewed refresh. No automatic snapshot maintenance is authorized.

This is not WP1/F01/F02 correctness acceptance. The frozen native-command
failure-propagation/lifecycle-gate repair, #390 review, WP2/#361, WP3, WP4,
WP5 and #247 remain separate. No production, consumer activation, external
repository, paid-AI or Remote Desktop authority follows from this archival.

## Validation and lifecycle

The source implementation's recorded tests, independent review and exact-head
CI remain bound to its original head; this archival does not pretend to rerun
or independently review that implementation. For this low-risk lifecycle-only
change: inspect the complete four-path diff, validate terminal records and exact
source identities, run normal canonical documentation checks, and integrate
through the unchanged protected Merge Queue. A new external AI review is not
required solely to archive already accepted delivery and observed measurements.
Runtime E2E is NOT_APPLICABLE to the archive; skipped runtime jobs are not E2E PASS.

The archive candidate itself is not protected completion until its own normal
PR/MQ merge and protected-main readback. Work closes the linked issues only after
that readback. Keep the closeout head/CI/merge evidence in GitHub, not a no-op
self-referential source commit. Programme #308 retains all unrelated obligations.

# OTV2-20261003-arch-rt-checkpoint-rows-1

```yaml
task_id: OTV2-20261003-arch-rt-checkpoint-rows-1
title: "ARCH-RT-CHECKPOINT-ROWS-1: the §12 checkpoint rows become TIMED-RT-1b's first item; RT-1b narrowed"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-rt-checkpoint-rows-1
issue: 1622
pr: 1692
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: Sol Supervising Architect
created_at: 2026-10-03
updated_at: 2026-10-03
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_TIMED_FORGE_PROF_PACKETS_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-arch-rt-checkpoint-rows-1.md
public_contracts: []
external_repositories: []
```

## Outcome

This task carries #1689's round-3 P1 4174803489. The control plane routed it to this follow-up
under D317. #1681 (RT-1a) was already in the Merge Queue, and it merged its checkpoint writer
without the §12 checkpoint and composed checkpoint shape rows.

- §2.3 makes both rows TIMED-RT-1b's first item. RT-1b's first commit registers them, with max and
  max+1 tests against RT-1a's merged writer, before any other RT-1b work. The rest of RT-1b is not
  reviewed until they are on its branch.
- §2.3's Rows list keeps the three expiry shapes. The fallback for `TIMEDITEM0B-RL-01/-02/-04/-05`
  now reads against `main` rather than an open #1681.

- **#1692 round 1 (Codex, `3d2428fc`).** P1 4174859933: RT-1a has no composed writer, because
  `commit_timed_checkpoint` writes a plain record with no build receipt. The control plane chose
  option (a):
  - The checkpoint row keeps writer-backed tests on RT-1a's writer.
  - The composed checkpoint and composed expiry burn rows get shape-level tests in RT-1b.
  - Their writer-backed tests are a merge condition of EXERCISE-1, which adds the composed
    writers.
- **RT-1b narrowing (control plane, folded in here so no second docs PR is needed).** §1.1 and
  §2.3 reduce RT-1b to four things:
  - 0058 expiry;
  - the 0011 and 0023 guard amendments, made in 0058;
  - the five §12 rows;
  - a `timed_item_host` lib wired at login and logout only.

  The respawn, arrival, slot, channel transfer and death call sites move to ITEM-MOVE-2a. The
  exercise binding and the composed writers move to EXERCISE-1.

This changes no code, no contract, no wire and no resource value.

## Validation

- `python3 tools/agents/validate_governance.py`: pass
- `python3 -m unittest discover -s tools/agents/tests`: 54 tests, OK
- `git diff --check`: pass

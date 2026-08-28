# OTV2-20260811-channel-selection-owner-baseline — Archive

```yaml
task_id: OTV2-20260811-channel-selection-owner-baseline
title: Persist previously accepted channel selection owner baseline
mode: COORDINATE
status: completed
repository: blakinio/Oteryn-v2
pr: 154
base_sha: 9a088675c8d9bec4e42036333076e24d4dc4785e
final_head_sha: 9a7476a3eda26aa7c949ae2488580977d7e586ab
merge_commit: 5631a9eecffbacdff101f3a2da39583c6028c80f
owner: released
source_type: USER_SOURCE
owner_decision_date: 2026-07-22
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
```

## Outcome

Persisted `docs/architecture/GAME-CHANNEL-01_CHANNEL_SELECTION_OWNER_BASELINE.md` as a narrow partial owner baseline covering only the previously accepted selection UX:

- one Oteryn login flow, not separate login servers per channel;
- channel context in character/world selection;
- automatic/recommended channel path;
- manual `Change Channel` choice retained.

Full `GAME-CHANNEL-01` remains **NOT ACCEPTED**. In-world switching semantics, party co-location, anti-hopping, PvP, capacity, event/reward and recovery policy remain unresolved or owned by other accepted baselines.

## Terminal evidence

- final PR head: `9a7476a3eda26aa7c949ae2488580977d7e586ab`;
- mandatory exact-head self-review `4903782624`: PASS, zero open material findings;
- Agent Governance `31467968234`: PASS;
- Dependency Review `31467968206`: PASS;
- CodeQL `31467968210`: PASS;
- component/integration/E2E: `NOT_APPLICABLE`;
- independent review: `NOT_REQUIRED` under current risk policy;
- automatic review found two P1 conversations on older candidate `c7299a8125`: mandatory decision-timing record and stale task checkpoint;
- the decision-timing P1 was repaired on the final head;
- the task-checkpoint P1 was already satisfied by later validating metadata and became outdated;
- both conversations were replied to and resolved before merge;
- squash merge: `5631a9eecffbacdff101f3a2da39583c6028c80f`.

## Preserved decision discipline

The owner baseline includes why the prior decision must be recorded now, named downstream consumers, cost of delay and evidence that could justify an explicit superseding proposal. Evidence cannot silently supersede the owner decision; a later conflicting choice must name this baseline and the exact superseded scope.

This archive/closeout releases task ownership only. It changes no architecture semantics beyond the already delivered PR #154 baseline.

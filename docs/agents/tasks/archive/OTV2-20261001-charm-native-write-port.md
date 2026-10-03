# OTV2-20261001-charm-native-write-port

```yaml
task_id: OTV2-20261001-charm-native-write-port
title: Native fenced Charm unlock and assignment port
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: codex/charm-native-durability-port-20261001
branch: codex/charm-native-write-port-v2-20261001
pr: 1507
issue: 162
base_sha: 667be3950e0bc64925d9e48370616a0dbe95fbb7
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, sole publisher; transport source preparer
created_at: 2026-10-01
updated_at: 2026-10-01
owned_paths:
  - apps/game-server/src/gameplay_transport/charm_native.rs
  - apps/game-server/tests/support/charm_state_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20261001-charm-native-write-port.md
public_contracts: []
depends_on: [CHARM-2, CHARM-3, 1501]
blocks: []
external_repositories: []
```

## Bounded result

NativeCharmProgressionPort implements the existing CharmProgressionPort and dispatches the
registered unlock/assign commands into the real DurabilityRoot. Session identity must match
the bound port. Current Content indices resolve to durable keys; current root revision is
read for a new command. Unlock expected_stage is checked against current state or retained
stage_before. Original occurrence revision is retained only as semantic binding, never as
current live authority. Existing durable rules map to existing typed wire dispositions.

AuthorityInvariant x ConsumerBoundary x MutationOperator: the caller independently supplies
current session/connection/lease/scope/node evidence; a new write checks it at the durable
boundary. Retained terminal outcomes may be returned after connection replacement, with no
write or restored authority. Changed command, key, race, expected stage or root binding rejects.
A bounded same-occurrence race recovery reuses only the original revision; it never reconstructs
live fields from receipts. Production guards, holder/receipt budgets, schema and wire IDs remain
unchanged. The complete source batch is532 changed Rust lines, including required actual SQL
dispatch/restart/concurrency/refusal oracles, with no dropped assertions.

## Validation and remaining work

Actual PostgreSQL17.6 suite792PASS. Two independently reconciled test roots concurrently submit
the same unlock: both receive Unlocked, with one receipt, one revision increment, stage1 and
one240-point debit. The first test attempt used one root's single ready SQL holder and correctly
failed admission; the fixture now follows the existing two-root owner pattern without increasing
capacity or changing runtime behavior. Tests preserve entire durable snapshots across replay,
changed payload, insufficient currencies, locked/already-assigned/stage-low cases, independently
wrong dispatcher identity, restart and stale connection. Retained raw count5/current wire3
regression from the repaired reader is preserved. Internal race-branch coverage is not claimed.

Workspace fmt, diff check, governance/lifecycle13PASS and strict workspace all-target Clippy PASS.
Real connection routing/capability advertisement,
qualified Content generations, commercial facts, paid unassign and combat activation remain
unfinished. The full nine-row parent stays IMPLEMENTING. Stack CI requires base main; CP owns
retarget/requalification after PR1501 integration. Root publishes guarded actual Git identity;
exact frozen head and independent review belong to the external packet. This child archive
reaches main only if PR1507 merges; a commit cannot contain its own final SHA.

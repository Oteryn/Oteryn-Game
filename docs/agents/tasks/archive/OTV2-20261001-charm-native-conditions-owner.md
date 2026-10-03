# OTV2-20261001-charm-native-conditions-owner

```yaml
task_id: OTV2-20261001-charm-native-conditions-owner
title: Condition transitions on the existing native actor owner
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: codex/charm-cleanse-owner-store-20261001
branch: codex/charm-native-conditions-owner-20261001
pr: 1488
issue: 162
base_sha: 40694195de1a9200f7e03041f7f6e6c197d7e68b
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, sole publisher; conditions source preparer
created_at: 2026-10-01
updated_at: 2026-10-01
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/runtime_actor_conditions.rs
  - apps/game-server/src/foundation/runtime_actor_conditions_tests.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/foundation/damage_contributors_tests.rs
  - apps/game-server/src/foundation/movement_static_kernel_structural_tests.rs
  - docs/agents/tasks/archive/OTV2-20261001-charm-native-conditions-owner.md
public_contracts: []
depends_on: [CONDITIONS-0, 1479]
blocks: []
external_repositories: []
```

## Bounded implementation

COND-1 stores the existing ConditionStore in each occupied actor slot, without a second map.
Prepared transitions are immutable expected evidence; commit independently resolves current
scope, target generation/session and the application source. One retained receipt makes the
same prepared plan a no-op; changed revisions cannot overwrite newer state. The occurrence
consumer must retain its original plan and must not replan historical occurrences.
Creature death clears conditions/immunity; control loss preserves them. Field provenance is
refused by this actor-source API. The 16-instance limit stays fixed. Lifecycle boxing reduces
the measured slot from168 to160 bytes; the exact-size assertion remains exact, with no padding.

AuthorityInvariant × ConsumerBoundary × MutationOperator covers target/source binding and
current liveness at prepare/commit: wrong source session, wrong owner scope, replaced target
generation, future/decreasing time, and identical prepared replay. Each negative preserves
unrelated valid facts. Persistence/restart grants are NOT_APPLICABLE: this is ephemeral actor
state, with no authority restoration, durable write, migration or protocol change.

## Validation and remaining work

Two native condition tests PASS. Full server library:1297PASS,2existing ignored. The first
full run exposed the stale168-byte assertion; the exact160-byte repair then passed the complete
suite. Workspace rustfmt, diff check and strict all-target Clippy PASS. Source budget580 changed
Rust lines across six source paths, including essential negative tests and two affected fixture
repairs; the tick/speed followup is excluded to keep this one module batch bounded.

The full nine-row Charm parent remains IMPLEMENTING. This child does not close COND-1 ticks,
SPEED-1 pacing or attack/Charm wiring, and no effect activation is changed. Root preserves the
local Git candidate identity via guarded normal push and remote readback. Final-head review/CI
belong to the external FREEZE_SHA packet. Protected integration is control-plane owned; this
stack targets frozen PR1479 and must be retargeted/requalified after that parent merges.
The archive reaches main only if PR1488 merges; a commit cannot contain its own final SHA.

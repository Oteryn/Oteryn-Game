# OTV2-20261001-charm-native-progression-snapshot

```yaml
task_id: OTV2-20261001-charm-native-progression-snapshot
title: Coherent fenced Bestiary and Charm progression snapshot
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/charm-native-progression-port-20261001
pr: 1490
issue: 162
base_sha: e225b3f76e152d195f75cb757b3d639a3ff5577a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, sole publisher; progression source preparer
created_at: 2026-10-01
updated_at: 2026-10-01
owned_paths:
  - apps/game-server/src/durability/charm_state.rs
  - apps/game-server/tests/support/charm_state_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20261001-charm-native-progression-snapshot.md
public_contracts: []
depends_on: [CHARM-0, CHARM-2, CHARM-3]
blocks: []
external_repositories: []
```

## Bounded implementation

The existing Character transaction checks recovery/current gameplay fences and locks the root
before reading Bestiary, Charm state, earning facts and entitlement at one CharacterRevision.
Expected revision is never current authority. Only bounded current-generation race keys are
probed by indexed lookups; historical counters remain stored. Reads validate content context.
Bestiary facts/read requests refuse more than1024 keys or duplicates; stored unlock/assignment
loaders fetch at most33 and refuse more than32 rather than truncating. No schema/wire allocation.

AuthorityInvariant × ConsumerBoundary × MutationOperator covers independent current identity,
session, connection generation, lease, scope and scope generation at the fenced read boundary.
Each isolated negative leaves unrelated facts valid; stale CharacterRevision and substituted
content refuse. Reads mutate no semantic state. Restart re-establishes reconciled authority
and returns the same snapshot, rather than promoting stored revision into live authority.

## Validation and remaining work

Actual configured PostgreSQL17.6:738testsPASS, including all eight Charm database scenarios,
the new snapshot/fence/content/bounds/restart cases and the bounded production facts unit test.
The focused snapshot case separately passed. The official CI-pinned image digest was obtained
from the public ECR mirror after DockerHub rate limiting. An initial noncanonical-port harness
refusal and an unconfigured PRE-ROUTING run are not qualification. Workspace rustfmt, diff check
and strict all-target Clippy PASS. Narrow dead-code allowances only support standalone suites
that path-load durability without these Charm cases. Source budget354 changed Rust lines.

The full nine-row Charm task remains IMPLEMENTING. This is the durability half of the native
progression port; client dispatch and the concrete port follow separately. Existing production
Bestiary facts remain Free/unpromoted until authoritative Premium/promotion/Expansion inputs
are delivered. No capability or effect activation, gameplay E2E or complete parity claim.
Root publishes the actual local Git identity via guarded normal push and exact remote readback.
Final-head independent review/CI belong to the external FREEZE_SHA packet; protected integration
is control-plane owned. This archive reaches main only if PR1490 merges; a commit cannot include
its own final SHA.

# OTV2-20261001-charm-native-damage-descendant

```yaml
task_id: OTV2-20261001-charm-native-damage-descendant
title: Sealed post-primary Charm damage descendant
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: codex/charm-ordered-effect-plan-20261001
branch: codex/charm-native-damage-descendant-20261001
pr: 1505
issue: 162
base_sha: c0008c9c28816ee8242315cb092fe765c229cf1c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, sole publisher; damage source preparer
created_at: 2026-10-01
updated_at: 2026-10-01
owned_paths:
  - apps/game-server/src/ability/commit.rs
  - apps/game-server/src/foundation/channel_owner_ability_commit_tests.rs
  - docs/agents/tasks/archive/OTV2-20261001-charm-native-damage-descendant.md
public_contracts: []
depends_on: [ABILITY-0, COMBAT-0, 1476, 1498]
blocks: []
external_repositories: []
```

## Bounded result and authority

Existing atomic commit_exact_owner_damage is unchanged. Its real owner-issued HP result mints
a sealed primary; the existing Charm evaluator can consume that result before preparing a
two-entry ordered root plan. The exact primary remains entry0 with its original binding; only
generated entry1 commits, with its own receipt. Combined prefix/child/HP binding obeys existing
4096-byte limits. Lethal/non-reducing primaries are terminal; child results retain CharmDamage
source and cannot be supplied as the sealed ordinary primary.

AuthorityInvariant x ConsumerBoundary x MutationOperator applies at child write: sealed parent
is immutable provenance only; current owner plus independently supplied attacker, lease and
command are required and rechecked by the native owner boundary. Exact target generation and
group/occurrence/revision/intent/prefix remain bound. Changed child semantics conflict. Sixteen
non-evictable receipts refuse a primary without mutation; with15, the accepted primary can be
evicted when the child commits. Retain the original child plan and replay it directly, never
reconstruct its prefix after eviction. No new receipt budget, global map or authority source.

Root inspected all552 changed Rust lines (197 implementation and355 native test changes).
The natural ordering prerequisite is PR1498; required owner/fence/capacity negatives remain
together here. Immutable matching fields do not substitute for current authority. Durable
restart is NOT_APPLICABLE to this ephemeral native receipt carrier.

## Validation and remaining work

Seven native tests PASS, including existing declared order plus real evaluator HP20→17 and
proc17→16, no-chain classification, original atomic-prefix replay, changed lineage/child,
lethal terminal, capacity/eviction replay, isolated current authority negatives and combined
byte rejection preserving the primary. Full server library1286PASS,2existing ignored; workspace
fmt, diff check, strict workspace all-target Clippy and governance/lifecycle13PASS. Three narrow
test-only dead-code annotations cover APIs executed in the native owner suite and excluded from
the standalone Ability fixture. Compiler argument-order and explicit-panic test diagnostics were
fixed before publication without weakening assertions. Actual PostgreSQL/E2E NOT_APPLICABLE.

Attack-cycle composition, qualified current Charm/equipment/mitigation facts, secondary target
selection, Carnage census and full runtime activation remain unfinished. The full nine-row parent
remains IMPLEMENTING. Root publishes actual Git identity with guarded normal pushes; exact
freeze, independent review and CI belong to the external packet. Governance accepts base=main
only; CP owns stack retarget/requalification after PR1498 integrates. This record reaches main
only if PR1505 merges; a commit cannot contain its own final SHA.

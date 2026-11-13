# OTV2-20261113-charms-leech-evidence

```yaml
task_id: OTV2-20261113-charms-leech-evidence
title: Global-reference Leech and passive spell critical corrections
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: fix/charms-leech-evidence-20261113
pr: null
base_sha: ce45063507c22a11e0db7e9c0ff86dacbe6ff834
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT standalone task writer
created_at: 2026-11-13T20:38:46Z
updated_at: 2026-11-13T20:38:46Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/combat-rules/src/resolution/equipped_offence.rs
  - crates/gameplay/dispatch/src/gameplay_transport/actor_spell_commit.rs
  - crates/gameplay/dispatch/src/gameplay_transport/actor_spell_commit_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/attack/charm_combat.rs
  - crates/gameplay/dispatch/src/gameplay_transport/attack/drain.rs
  - crates/gameplay/dispatch/src/gameplay_transport/attack/equipped_offence.rs
  - crates/gameplay/dispatch/src/gameplay_transport/bounty_talisman.rs
  - crates/gameplay/dispatch/src/gameplay_transport/item_spell_augments.rs
  - crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast.rs
  - crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast/prepare.rs
  - crates/gameplay/dispatch/src/gameplay_transport/native_companion_item_cast.rs
  - crates/gameplay/dispatch/src/gameplay_transport/native_world_item_cast.rs
  - crates/gameplay/dispatch/src/gameplay_transport/ordinary_combat.rs
  - crates/gameplay/dispatch/src/gameplay_transport/ordinary_combat/prepare.rs
  - crates/gameplay/dispatch/src/gameplay_transport/spell_leech_projection.rs
  - crates/gameplay/dispatch/src/lifted_tests/spell_magnitude_owner_tests.rs
  - crates/gameplay/progression/src/charm_effects.rs
  - crates/gameplay/progression/src/charm_effects/tests.rs
  - crates/gameplay/spell/src/spell/magnitude_owner.rs
  - crates/gameplay/spell/src/spell/native_combat.rs
  - docs/agents/evidence/OTV2-20261113-charms-leech-implementation.md
  - docs/agents/tasks/OTV2-20261113-charms-leech-evidence.md
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/parameter_spell.rs
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Correct direct-hit and direct-area Leech, preserving the actual physical/player commit and
replay boundaries; wire Low Blow and Savage Blow into qualified native/ordinary spell damage.
This is a bounded slice of the owner's six-mechanic request, not six completed mechanics.
Coordination: #1622, allocation `CHARMS-LEECH-EVIDENCE-20261113`.

## Architecture and source of truth

- `PROVEN`: existing Game runtime owns damage, player vitals, active creature policy and receipts.
- `DERIVED`: selected numerical Global model and its source-specific limits are listed in
  [implementation evidence](../evidence/OTV2-20261113-charms-leech-implementation.md).
- Reference research: #3177 at `97685d43a194010559a3a92821089067dbb147a3`.
- Existing external target date remains 2026-09-27. No migration/content/wire change.
- Hex/Cleanse condition changes already owned by #3176 are excluded from this branch.

## High-risk authority/recovery qualification

Applicable: numerical gains enter existing fenced player-vitals replacements.

- Authority: current controlled `ExactActorRef`, `GameSessionId`, attacker Character/lease,
  qualified creature incarnation and existing batch/player revisions; none is replaced by a
  string, history report, source filename, branch name or client-supplied damage.
- PREPARE/COMMIT: qualified numerical projection is retained by the prepared cast; receipt
  replay cannot select a fresh gain. The original joined physical/player install remains the
  only mutation boundary, under held owners. No added external side effect follows commit.
- Reconciliation: persisted binding format and deferred parent binding stay unchanged; source
  JSON is not rewritten. Existing stale-state rejection, receipt identity and replay rules apply.
- Negative tests: stale player revision aborts both damage and gain; duplicate/unapplied receipt
  grants no additional HP/MP; wrong-race charm does not alter the other target's rate.
- Live-control changes or ledger promotion: not authorised by this task. No deployment or merge.

## Validation and current gate

Red failures were observed for fractional rounding, overkill and missing passive AoE criticals;
those targeted tests subsequently passed. Full final unit/lint/CI/review qualification is ongoing.
No PostgreSQL success is inferred from route-less tests. Required before merge: exact-head
`game-gate`, all selected real PostgreSQL partitions and independent Codex review.

## Remaining work

Finish exact-tree validation, publish the isolated branch/PR with this evidence, run independent
review and record checks. Preserve the six-topic remaining-boundary table rather than claiming
full parity: native Carnage spell kills, replay-safe zero-damage minor contact, ranged off-target
execution and unproven Global edge conditions are not completed by this numerical slice.

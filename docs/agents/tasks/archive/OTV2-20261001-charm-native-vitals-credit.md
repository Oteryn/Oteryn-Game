# OTV2-20261001-charm-native-vitals-credit

```yaml
task_id: OTV2-20261001-charm-native-vitals-credit
title: Native player health and mana credit consumers
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/charm-native-vitals-credit-20261001
pr: 1482
issue: 162
base_sha: e225b3f76e152d195f75cb757b3d639a3ff5577a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, sole publisher; formula_defense prepares source
created_at: 2026-10-01
updated_at: 2026-10-01
owned_paths:
  - apps/game-server/src/spell/cast.rs
  - apps/game-server/src/spell/cast_tests.rs
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - docs/agents/tasks/archive/OTV2-20261001-charm-native-vitals-credit.md
public_contracts: []
depends_on: [CHARM-0, SPELL-D2, SPELL-D3]
blocks: []
external_repositories: []
```

## Bounded result

Resolved health and mana credits mutate the existing native PlayerSpellState through
ChannelSpellStates and its current actor/session validation and compare-commit.
Positive credits cap at the current maxima and advance one revision; zero/full-pool credits
do not write. Checked arithmetic refuses corrupt pools and revision exhaustion.

This child supplies two native mutation consumers, not complete leech/inversion gameplay.
The owning combat caller must resolve the accepted formula and suppress occurrence replay.
Its absence is explicit in narrowly scoped preparation-only dead-code annotations; no effect
activation or alternative occurrence authority is introduced. The full nine-row implementation
task remains active in PR1479 and #162. This single-PR child record does not close that parent.

## AuthorityInvariant × ConsumerBoundary × MutationOperator

Both health and mana commit boundaries independently resolve current authority through
ChannelRuntimeV1.player_control_facts before reading and before compare-commit.
No immutable credit magnitude reconstructs current actor/session authority.

- Identity/binding: independently valid foreign session refuses both credit boundaries.
- Current liveness/generation: ended exact actor refuses; replacement actor generation remains
  fresh and cannot make the old actor usable. Missing native state refuses.
- Revision integrity: successor revision is checked; exhausted revision refuses positive credit.
- Pool integrity: current value above its authoritative maximum refuses.
- Replay: caller-owned occurrence suppression is a documented prerequisite, not falsely proved
  by repeatedly invoking a primitive whose credit magnitude carries no occurrence identity.
- Time/provenance/SQL/reload: NOT_APPLICABLE to this runtime-local magnitude consumer; it accepts
  no temporal field, grants no controller authority and writes no durable state.

## Validation and closeout

Seven new source tests cover pure successor limits and actual native owner mutations/fences.
Self-review verified real fixture maxima185HP/90mana and exura cost20, exact replacement identity,
and unchanged unrelated vitals/cooldowns. Source preparation is bounded263 added Rust lines.

Local RED/GREEN, build, rustfmt and Clippy: UNAVAILABLE_EXECUTOR (managed environment failed).
Fresh exact-head CI and independent review are pending; source byte readback and the complete
owned delta are verified in the external FREEZE_SHA packet. No inherited PR1476/1479 evidence,
PostgreSQL/client/E2E or full Charm completion is claimed. The archive reaches main only if PR1482
merges; protected integration remains control-plane owned. A commit cannot store its own SHA.

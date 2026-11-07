# OTV2-20261004-entity-client-1

```yaml
task_id: OTV2-20261004-entity-client-1
title: "ENTITY-CLIENT-1: capability 6 entity snapshot and delta in the client session and the live harness"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/entity-client-1-20261004
issue: 1622
pr: 1760
head_sha: "exact frozen head in the FREEZE entry to the control plane"
final_head_sha: "exact frozen head in the FREEZE entry to the control plane"
owner: claude-code worker for control plane session_013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - crates/session/src/**
  - tools/dev-client/src/**
  - tools/synthetic-client-harness/src/live/**
  - docs/agents/tasks/archive/OTV2-20261004-entity-client-1.md
public_contracts: []
depends_on: [SESSION-PUSH-1]
blocks: [CHAT-CLIENT-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Packet: `docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_CORE_CLIENT_PACKETS_2026-10-04.md` §2.2, §1.3.

- `CLIENT_SUPPORTED_CAPABILITIES` is `[6, 13]`, added in the same change that decodes the domain-1
  type 2 snapshot and delta. Capability 6 requires nothing, so the set stays closed under the
  registry's `requires` (test `the_advertised_set_is_closed_under_the_registry_requires`).
- `Session::admit` decodes the type 2 snapshot when capability 6 is selected (#1734 P2) and keeps
  the entities in a `WorldEntities` store keyed by `EntityRef` (identity and generation).
  A type 2 snapshot without capability 6, and a type 1 snapshot with it, fail closed with
  `UnregisteredSnapshotType`. Without capability 6 the v1 path is unchanged.
- A type 2 delta goes through the same domain store (revision, registered type, content generation).
  `enter` of a stored identity, `update` or `leave` of an entity that is not stored, more than 256
  stored entities, or an own actor that is gone or not at the header `actor_position` poison the
  session (`EntityStoreInconsistent`). The own actor is therefore read from the header and from its
  PLAYER entity, and the two agree on every applied delta and at join (the protocol codec checks
  the snapshot). A type 2 delta without capability 6 is an unregistered type.
- Events: `SessionEvent::WorldSpatialEntities` carries the delta. A step still returns
  `world_spatial_delta` (shape unchanged): with capability 6 it is built from the claimed delta's
  header.
- `dev-client` mirrors the new error, re-exports the entity types and adds `world_entities()`.
- Harness: `RenderModel` mirrors the entities; draws `C` creature, `N` npc, `P` player, `x` corpse,
  `i` ground item, one top entity per tile, own actor `@`. A click on a non-door tile selects the
  top entity (creature, npc, player, corpse, item), shown in the status line and cleared when the
  entity leaves. Selection is local; no command is sent.
- Tests: snapshot at 0, 1 and 256 entities; type 2 refused without capability 6; delta
  enter/update/leave; step claim; poisoning cases; store at 256 and 257; harness appear, move and
  disappear through pushed deltas, glyphs, selection order.
- No protocol, registry, proto, server or `apps/client` file touched. `apps/game-server` does not
  offer capability 6 yet (VIS-3), so the Server Seam qualification is unaffected and was not
  required by this packet. No live-play evidence yet: it follows VIS-3.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-session -p oteryn-dev-client -p oteryn-synthetic-client-harness --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-session -p oteryn-dev-client -p oteryn-synthetic-client-harness --quiet`: pass
- `cargo check --locked -p oteryn-game-server --tests`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

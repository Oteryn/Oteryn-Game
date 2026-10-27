# OTV2-20261027-quest-callbacks-3

```yaml
task_id: QUEST-CALLBACKS-3
title: Complete bounded quest door, item-target and storage callbacks
type: mechanic
mode: IMPLEMENT
status: investigating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/quest-callbacks-3
pr: null
owned_paths:
  - content/quests/world-callbacks.json
  - crates/gameplay/quests/src/quest/world_callbacks.rs
  - crates/gameplay/world/src/map/facts/quest_callbacks.rs
  - crates/gameplay/world/src/map/floor/quest_callback.rs
  - crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks.rs
  - crates/gameplay/durability/src/durability/quest_world_callback.rs
  - crates/gameplay/durability/migrations/0265_quest_callback_classes.sql
  - tools/content-migration/quest_callback_profiles.py
depends_on: [QUEST-CALLBACKS-1, QUEST-CALLBACKS-2]
reserved_ids:
  migration: 0265
  capability: 135
  command: 156
  mechanic: 0370
```

## Goal

Close the remaining bounded callback classes identified by
`PARITY-QUEST-CALLBACKS-1` without embedding Lua: source-qualified quest doors,
single-item-on-target interactions, and storage-predicated use/access callbacks
execute through the existing `Use` route and quest receipt owner. Preserve
`WorldId`/`ChannelId`, session-generation fencing, replay safety, and atomic item
custody. Reserve the identifiers above only when the implementation actually
needs that registry or migration; do not create empty registrations.

## Scope and callback inventory

| Callback class | Reference evidence (Canary pin `47dfd51f45280a59a1d3e50ba7edd573d7234446`) | Oteryn owner |
|---|---|---|
| Quest/key/level/custom door predicate and pass-through | `data/scripts/actions/doors/quest_door.lua:12`; `key_door.lua:28`; `level_door.lua:12`; quest case `data-otservbr-global/scripts/quests/the_travelling_trader/actions_outlaw_camp_door.lua:1` | map state/collision in `gameplay/world`; admission and real `Use` routing in `gameplay/dispatch`; quest predicates in `gameplay/quests` |
| Single item used on a target, with consume/transform and quest transition | `data-otservbr-global/scripts/quests/the_cursed_crystal/actions_MedusaOil.lua:9`; `the_thieves_guild_quest/actions_fish_napping_key.lua:3`; `the_postman_missions_quest/actions_package.lua:1` | callback schema/evaluation in `gameplay/quests`; item custody in `gameplay/durability`; target and map effects in `gameplay/dispatch`/`gameplay/world` |
| Storage-gated or storage-mutating use/access callback (no lever or step-in requirement) | `data-otservbr-global/scripts/actions/other/others/quest_system2.lua:359`; `quests/the_thieves_guild_quest/actions_fish_napping_door.lua:1`; `quests/the_tainted_soul/actions_star_herb.lua:6` | predicates/transitions in `gameplay/quests`; durable per-character receipt/state in `gameplay/durability`; command orchestration in `gameplay/dispatch` |

These are the remaining action shapes, not permission to port every script in
the parity inventory. Select at least one representative Canary-backed case per
row, reconcile any Canary/Crystal conflict explicitly, and keep content bounded.

## Non-goals

- Do not redo QUEST-CALLBACKS-1 reward chests or QUEST-CALLBACKS-2 levers,
  step-in triggers, altar/multi-item offerings, or their five existing profiles.
- No general Lua runtime, arbitrary callback DSL, NPC dialogue, creature death,
  global-event scheduler, raid/boss framework, or wholesale quest completion.
- No client/protocol change, cross-channel map mutation, unfenced character
  write, speculative registry entry, or claim that all source quests are playable.
- Container/random/written/achievement rewards remain in their owning slices.

## Reference evidence and architecture

- **PROVEN:** the 2026-10-25 parity inventory marks keyed/level/quest/custom
  doors partial because only plain door toggling is routed, and its quest census
  shows action callbacks still unproven.
- **PROVEN:** QUEST-CALLBACKS-1 (`9eb768ce`, PR #2446) owns generic once-only
  reward chests; QUEST-CALLBACKS-2 (`f40562e5`, PR #2484) owns lever, step-in and
  multi-item offering callbacks.
- **DERIVED:** the three rows above are the smallest remaining reusable action
  classes evidenced by the gap census. A source-specific behavior outside them
  is a new card, not an extension of this one.
- Crystal revision `201189f871d23597b82caf88fd611454a2ddefb8` is comparison
  evidence. Canary remains the default target; conflicts must be recorded.

## Required reference parity table

The implementation PR must replace this planning table with one row per admitted
profile. Each row must name exact Canary lines, optional Crystal difference,
selector/predicate, before/after item and storage state, map/collision effect,
refusal, Oteryn source path, and exact table-driven test name. No row may say
“same as source” without asserted values.

| Behaviour | Reference | Expected Oteryn result | Required test |
|---|---|---|---|
| Door predicate | door sources above | denied state is unchanged; admitted state passes/toggles only in the addressed channel | real-dispatch PostgreSQL case |
| Item on target | Medusa oil/key/package source | exact target required; consumption/transform and transition commit once | success, wrong target, retry, reconnect |
| Storage use/access | quest_system2/fish door/star herb | exact range/equality predicate; one fenced transition and source message/effect class | lower/upper boundary and stale-generation cases |

## Acceptance and validation

- [ ] Schema rejects ambiguous selectors, unbounded effects and missing source
  witnesses; content loader and migration/profile generator agree.
- [ ] Each admitted profile has positive, refusal, duplicate/replay, reconnect,
  stale-generation and PostgreSQL reload coverage through the production route.
- [ ] Item and storage mutation is atomic; a failed map effect or custody check
  leaves both unchanged. Door/map effects remain channel-local.
- [ ] Run `cargo fmt --all -- --check` and strict Clippy for quests, world,
  durability and dispatch.
- [ ] Run focused quest/world callback unit tests and the mounted PostgreSQL
  `quest_world_callbacks` cases with `OTERYN_TEST_POSTGRES_ADMIN_URL`.
- [ ] Run `python tools/content-migration/quest_callback_profiles.py --check`,
  `oteryn-world-bundle-compiler pin-check .`, governance validation, and the
  relevant capability/command/mechanic registry checks if an ID is consumed.
- [ ] Attach the completed parity table and exact command/test receipts to the PR;
  update `MECHANICS_STATUS.md` only in the implementation closeout.

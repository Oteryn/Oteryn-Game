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

## 2026-10-27 validation and gap pass

Local authoring only on `claude/happy-davinci-ijz3wy-qcb3`; the operator owns
publication. No commit, staging, push, protected-file or reference-document edits.

The three new callbacks now share the existing consumption receipt path with
altar offerings. All inputs and replacement cardinalities are checked before
writes; every actual UPDATE/DELETE must affect exactly one row. `Ok(false)` from
`consume` drops the caller's uncommitted SQLx transaction (rollback), rather than
committing it. Migration **0268** is necessary: migration 0218 originally required
a Ground row and rejected every carried-item consumption and transform. The new
migration preserves those Ground checks and adds exact carried custody and after
proofs, using the same transaction ID and quest receipt. The offerings path has no
separate operation or outbox row; its immutable consumption row plus deferred
quest receipt are the existing proof owner. Lifecycle **2 = RETIRED**, quantity 0
(migration 0011; DUR-03 §11.4), not deletion of the instance identity.

Carried lookup includes equipment slots, the backpack itself, main entries and
nested/closed bags, bounded to depth 8 and 500 tree items. Container descendants
and special item states cannot be silently destroyed. The generic quest writer
also refuses item-on-target callbacks without their physical context. Callback
storage keys and quest identities now reuse the source-lowered NPC tracks, rather
than creating unreachable parallel tracks. Rejected USE callbacks send their
source-authored failure message.

### Parity evidence

Both repositories were read at the immutable pins below. Original Lua files are
reference evidence only and are not redistributed with this change.

| Callback | Canary `47dfd51f` | Crystal `201189f8` | Asserted local behavior / limitation |
|---|---|---|---|
| outlaw-camp-door | `data-otservbr-global/scripts/quests/the_travelling_trader/actions_outlaw_camp_door.lua:3-9,14` | `data-global/scripts/quests/the_travelling_trader/actions_outlaw_camp_door.lua:3-9,14`; same predicate/message | AID 1108, 1642→1643 only at Mission02=3; otherwise exact refusal message, no receipt. Source also teleports onto the door tile: full pass-through parity remains unverified. |
| fish-napping-key | `data-otservbr-global/scripts/quests/the_thieves_guild_quest/actions_fish_napping_key.lua:5-9,16` | `data-global/scripts/quests/the_thieves_guild_quest/actions_fish_napping_key.lua:3-7,14`; **target AID 41714**, versus Canary **51394** | Canary AID retained; i7934 ×1 retired, Mission06 stays 2, teleport to (32359,32788,-6), immutable proof and exact-cause replay. Source monster-say line is not projected by this bounded callback shape. |
| travelling-trader-delivery | `data-otservbr-global/scripts/actions/other/others/quest_system2.lua:20-27,378-380,445,475` | `data-global/scripts/actions/other/others/quest_system2.lua:20-27,378-380,456,486` | UID 3020, Mission02 3→4; other stages refused. Source also grants item 145 before updating storage; reward delivery parity is not claimed by this storage-only profile. |
| behemoth | `data-otservbr-global/scripts/quests/behemoth/action-lever.lua:1-28`; Not verified in this pass | `data-global/scripts/quests/behemoth/action-lever.lua:1-28`; Not verified in this pass | Existing literal-use wall/collision regression retained. |
| giant-smithhammer | `data-otservbr-global/scripts/quests/giant_smithhammer/lever.lua:1-18`; Not verified in this pass | `data-global/scripts/quests/giant_smithhammer/lever.lua:1-18`; Not verified in this pass | Existing literal-use wall/collision regression retained. |
| liquid-black-shortcut | `data-otservbr-global/scripts/quests/liquid_black/movements_shortcut.lua:1-23`; Not verified in this pass | `data-global/scripts/quests/liquid_black/movements_shortcut.lua:1-23`; Not verified in this pass | Existing storage/refusal/teleport regression retained. |
| paradox-skulls | `data-otservbr-global/scripts/quests/the_paradox_tower/movement_sacrifice_skulls.lua:1-41`; Not verified in this pass | `data-global/scripts/quests/the_paradox_tower/movement_sacrifice_skulls.lua:1-41`; Not verified in this pass | Existing atomic four-offering, replay and interrupted-projection regressions retained. |
| hellgate-bridge | `data-otservbr-global/scripts/quests/others/actions_hellgate_lever.lua:1-34`; Not verified in this pass | `data-global/scripts/quests/others/actions_hellgate_lever.lua:1-34`; Not verified in this pass | Existing bridge replacement/collision regression retained. |

The new real-dispatch test is
`quest_world_callbacks_qcb3_real_dispatch_actions`; source setup uses the existing
production NPC transition catalogue. Existing cases were moved unchanged into
sibling modules to keep every touched Rust file below 600 lines.

### Validation

Exact commands, outcomes and counts are retained in `QCB3-VAL-FINAL.md` at the
worktree root. PostgreSQL runs use the absolute queued cargo wrapper and only
callback filters. Database connection values are never emitted. Source reference
reads used public immutable raw GitHub URLs only.

| Check | Receipt |
|---|---|
| quests unit tests | 46 passed, 0 failed |
| dispatch `quest_world_callbacks` (live PG) | 7 passed, 0 failed |
| dispatch `gameplay_transport::world_map::` | 30 passed, 3 existing ignored |
| durability `quest_state` | 4 passed, 0 failed |
| strict Clippy, three packages, all features/targets | passed |
| cargo fmt and explicit mounted-module rustfmt checks | passed |
| quest Python suite | 1001 passed, 8 existing skips, 0 failed |
| callback Item admission Python tests | 3 passed, 0 failed |
| content regeneration and tree equivalence | passed without staging |
| world pin check | passed: 1 world, 30,923,832 bytes |
| governance suite | 129 passed, 0 failed |

### Remaining

- Donor source verification failed on a retained tree-count mismatch; Item/world
  Python checks needing `rfc3339-validator` or `zstandard` could not run. The
  owner forbids dependency downloads, so these remain for the qualified CI image.
- Database one-for-one source-item replacement after-proofs are tested, but no
  admitted callback currently exercises a committed source-item transform. The
  target appearance swap plus nested key consumption/teleport has real dispatch
  coverage. See the final report for exact qualification limits.
- `regenerate_content.py --resolve` stages files internally, forbidden by the
  owner's hard rule. Run its non-staging regeneration mode here; the operator
  owns the final staged pin refresh.
- The source-parity limitations above are explicit, not claims of full quest
  completion. No extra reward, speech or general Lua mechanism is introduced.
- External independent exact-head review and CI remain with the operator's
  publication lifecycle; this worker does not publish or trigger paid review.

# OTV2-20261031-monsterai

```yaml
task_id: OTV2-20261031-monsterai
title: Data-driven monster targeting, fleeing and attacks
type: mechanic
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/monsterai
pr: 2766
base_sha: c81c2c9b4333a7c53e362a9bfe32549ef1040ace
head_sha: null
final_head_sha: null
owner: monsterai-task-owner
created_at: 2026-10-31T11:11:44Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/monster-ai/
  - crates/gameplay/combat/
  - crates/gameplay/spell/
  - crates/gameplay/dispatch/src/gameplay_transport/
  - crates/gameplay/dispatch/src/lifted_tests/
  - crates/gameplay/dispatch/src/lib.rs
  - crates/gameplay/domain/src/domain/melee_definition.rs
  - crates/gameplay/entities/src/carrier/
  - crates/gameplay/entities/src/carrier.rs
  - crates/gameplay/durability/src/durability/creature_source_items.rs
  - crates/gameplay/durability/src/durability/native_world_read.rs
  - crates/gameplay/content/src/content/native_gameplay.rs
  - crates/gameplay/content/src/content/native_gameplay/test_support/activation_fixture.rs
  - tools/content-migration/bind_monster_source_definitions.py
  - tools/content-migration/register_spell_families.py
  - tools/content-migration/test_bind_monster_source_definitions.py
  - content/
  - docs/agents/tasks/OTV2-20261031-monsterai.md
  - docs/agents/evidence/OTV2-20261031-monsterai/
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Outcome

W01a/W01c ordinary admitted monsters use their source targeting, flee threshold,
shared attack clock, all melee indices and existing current combat/condition/Item
owners. Rat 0–8 at 2000 ms, Hunter distance, Fire Devil area, Fire Elemental and
Firestarter field creation and The Monster wall removal reach actual dispatch and PostgreSQL read-back.

Deliver one draft PR. The user explicitly excludes review requests, heavy-tests,
merge and deployment. No new wire capability, command, migration or mechanic ID.

## Architecture and source of truth

- PROVEN primary donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
- PROVEN secondary: Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`.
- Evidence class: OTS_HYPOTHESIS_ONLY; the task explicitly chooses Crystal parity.
- PROVEN complete native projection: 1870 records, including 1863 canonical records
  with exact attack-definition closures and seven preserved legacy identities.
- PROVEN attack census: 1689 melee, 4295 ordinary spell/Item/composed entries,
  and 18 explicit Encounter refusals. Of the ordinary entries, 16 are composed
  and 82 are source Item operations (62 creation, 20 removal).
- Existing Channel position, damage ledger, spell/timer, condition, character
  vitals and durable Item owners remain authoritative; no parallel effect engine.
- The one-field shared position extension is explicitly authorized by PR answer
  6543529998. It reuses existing position revisions and preserves receipt Debug
  bytes. Actual Slot is 184 bytes within the unchanged existing budget.

## High-risk authority/recovery qualification

Applicable: current-owner HP, conditions, timers and durable source Item writes.
Current authority comes from runtime owner/work stamps, exact actor generations,
live player session/control, independently read Map/Item facts and existing
database scope/character fencing. Prepared source proof defines expected identity
and content; it never substitutes for current authority.

Negative cases cover stale owner/actor/session, content/source substitution,
target changes, wrong floor/context, protection, superseded occurrence, cooldown,
clock/refusal and replay. Selected native melee binds exact source index and
per-creature sequence; normal N commits before movement N+1. Timer readers are
specific to each source/pulse/session and validate the shared transaction after
their final awaited read. A lost transaction preserves pending timer owners.

The source Item transaction uses its existing occurrence receipt and writes item,
ground placement and audit rows atomically. Its source reference is the existing
validated native occurrence, within the schema's 1..128-byte bound. No merged
migration, SQL guard or permission is weakened.

## Acceptance criteria

- [x] Source-backed fleeing and targeting consume current player/creature facts.
- [x] All native melee slots, source intervals and extra/1500 ms semantics execute.
- [x] Distance/area attacks and existing owned conditions use current combat owners.
- [x] Full native projection and melee census execute against actual source data.
- [x] Six canonical dispatch/PostgreSQL cases pass, including Rat movement and removal.
- [x] Final summon/Map, eight-library, content/pin and repository checks complete.

## Reference parity

| Behaviour | Reference value/source | Oteryn | Test |
|---|---|---|---|
| Fleeing | Crystal monster.cpp:52–54,1001–1003,1204–1205; admitted 1x, melee-only suppression | monster-ai/summon_plan, profile_schedule | native_fleeing_keeps_ranged_spells_and_returns_to_fighting_after_heal |
| Targeting | Crystal monster.cpp:771–789,853–895,1295–1349; current HP/session damage and source strategy rules | targeting, existing damage_ledger, monster_ai_cycle | native_percentile_strategy_retains_crystal_remainder_and_damage_quirk; live-health/current-damage tests |
| Melee | monster.cpp:1188–1292; shared clock, all indices, last-extra 1500 ms | profile_schedule, ai_monster_melee, creature_bite | native_melee_clock; full 1689-entry census; Rat PostgreSQL |
| Movement retry | creature.cpp:488–495; game.cpp:6996–7000; onAttacking(0) | existing position stamp and movement_attack | issuer/target/out-back/turn/foreign-context cases |
| Source spells | combat.cpp:1666–1743,2337–2375; source geometry, elements and current protections | creature_damage_spell and monster_profile_cycle | 67 focused current-owner/condition/composed/geometry cases |
| Owned conditions | combat.cpp:944–946,1077–1080 | existing atomic player damage/condition and appearance owners | current_source_conditions_share_atomic_damage_and_replay_then_refuse_retirement; appearance execution suite |
| Source ground | monster.cpp:1288–1292; source range zero is uncapped | existing creature_source_items durable writer | Fire Elemental, Firestarter range-zero and The Monster removal PostgreSQL |
| Damage distribution | Crystal combat.cpp:211–219, monster.cpp:2457–2464, tools.cpp:442–450; explicit Q7A difference | Existing uniform_draw project policy retained | Source bounds and replay tests; no truncated-normal parity claim |
| Encounter boundary | exact eighteen source callbacks, answer 6543276153 | explicit unsupported-shape refusal | monster_actual_encounter_attacks_remain_eighteen_explicit_refusals |

Full donor links, per-rule names and the exact eighteen-entry table are in
[implementation evidence](../evidence/OTV2-20261031-monsterai/README.md).
Crystal's first-candidate exclusion in MostDamage differs from the pinned Canary
implementation and is deliberately retained.

## Excluded scope

All seven PR questions have explicit ANSWER A decisions:

- Configurable monster/boss health rates: W14a; keep admitted 1x.
- Arbitrary type/CreatureEvent Think scripts: typed Interaction/Encounter admission,
  retaining the two distinct override/ordering contracts.
- Eighteen Encounter-bound source attack entries: typed Encounter admission and
  composition, individually listed with donor lines and pinned by a refusal test.
- Automatic ordinary Defence effects: separate production integration. Wild
  monster self-healing/haste/invisibility are not newly playable.
- Translation observation: included through the authorized existing owner field.
- SPELL-PRESENT-0: separate cross-client activation. No monster producer into an
  undrained queue, capability, protocol envelope or client FX claim.

- Damage sampling: Q7A (6544219482) retains the existing uniform project policy.
  Crystal truncated-normal parity remains a separate shared combat-randomness
  mechanic (combat.cpp:211–219, monster.cpp:2457–2464, tools.cpp:442–450).
  Source bounds, schedules and replay remain required; no distribution parity claim.

W01b respawn is outside scope. Existing bounded recipient/timer/replay policies
remain. Ambiguous top-occupant callbacks require an existing arrival-order proof;
composed creature secondary effects without an atomic owner receipt refuse before
writes. Cardinal chain traces are a documented project presentation projection.

## Implementation / findings

The complete closure generator binds canonical Ability/Effect/Formula dependencies
after accepted creature overlays and refuses duplicate/missing/oversized closures.
The creature-profile input limit is a separate measured 32 MiB bound; other input
limits remain unchanged. Generated hashes/pins are produced by repository tooling.

Material findings repaired: missing canonical attack bodies, losing later source
slots, native duplicate summon/defence-melee projection, synthetic movement test
contexts, zero-range field refusal, empty source-ground reference, missing Drown
protection mapping and current Map invalidation after timer-bank preparation.
The actual removal case also exposed known static Source/Bundle walls being
misclassified as missing dynamic projectile metadata; the existing creature
reader now retains those qualified flags, matching the player reader. Synthetic
EQMOD and summon fixtures now update their exact copied Creature/Behavior roots
when applying an explicit test overlay; production validation is unchanged.
No new dead-code suppression, model identifier, secret or audit-ledger edit.

## Validation

Actual commands/results are retained in README and the draft PR. Runtime
9c72fa2558915c417831d3d8d374320e70a3d9fa merges main c81c2c9b4; all eight libraries
passed (3305 tests, 20 default opt-in ignores).
The explicit melee, eighteen-Encounter, appearance and full-Content gates passed
1/1/6/1 tests. The 1870-profile projection and House-exit regression passed.
Strict eight-crate Clippy, six generator tests, server Content integration,
World pin-check, server smoke, formatting, governance and diff checks passed.
Standard regeneration/pin-refresh passed in 629.21 s with no data/pin changes.
Six canonical monster PostgreSQL cases (316.50 s), one actual EQMOD case plus its
Content-only regression (98.45 s combined), and four Map/summon cases (183.03 s)
establish eleven actual database scenarios. Each tested revision is in README;
an unset PostgreSQL variable in the general library run is never a database pass.

Node-boot prerequisite inspection found Docker unavailable in the task WSL
integration. The normal local PostgreSQL cluster is not a substitute for the
required container/Platform/TLS topology; no node-boot pass is claimed.

## Self-review

The root reviews the complete change and integrates three disjoint worker packets.
Concrete source, authority, replay and production integration findings above were
repaired and their relevant checks rerun. Worker review is not represented as an
independent protected-integration review.

## PR and closeout

Draft https://github.com/Oteryn/Oteryn-Game/pull/2766 opened after first commit.
No review requested, heavy-tests label, PR merge or deployment. Reserved number blocks
remain unused. The final scan on main c81c2c9b4 returned exit 1 for duplicate
reservations between other PRs #2767/#2777; none overlaps this task's blocks. Final exact head belongs in PR/check evidence after the last commit;
a commit cannot contain its own SHA. The final immutable commit is recorded in the PR after this documentation commit.

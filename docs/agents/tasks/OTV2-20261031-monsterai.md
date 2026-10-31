# OTV2-20261031-monsterai

```yaml
task_id: OTV2-20261031-monsterai
title: Data-driven monster targeting, fleeing and attacks
type: mechanic
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/monsterai
pr: 2766
base_sha: a52549e3b9230c7486a24df66f4e8ec59d1f96cb
head_sha: null
final_head_sha: null
owner: monsterai-task-owner
created_at: 2026-10-31T11:11:44Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/monster-ai/
  - crates/gameplay/combat/
  - crates/gameplay/spell/
  - crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs
  - crates/gameplay/dispatch/src/gameplay_transport/monster_profile_cycle/
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/monster_attacks.rs
  - crates/gameplay/content/src/content/native_gameplay.rs
  - crates/gameplay/dispatch/src/gameplay_transport/actor_conditions.rs
  - crates/gameplay/dispatch/src/gameplay_transport/actor_spell.rs
  - tools/content-migration/bind_monster_source_definitions.py
  - tools/content-migration/register_spell_families.py
  - tools/content-migration/test_bind_monster_source_definitions.py
  - content/creatures/definitions/spell-native-profiles.json
  - docs/agents/tasks/OTV2-20261031-monsterai.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Outcome

Implement W01a and W01c for admitted native creature data through the existing think,
combat, condition and player-state owners. Deliver one draft PR; no review request,
heavy-tests label, merge or deployment.

## Architecture and source of truth

- PROVEN baseline: main `80dca5fd1578d029f953f05690c6246a0236fd16`.
- Primary donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
- Secondary reference: Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`.
- PROVEN provider census: 1,870 creature records, 1,357 melee bindings and only three
  records carrying executable attack definitions in
  `content/creatures/definitions/spell-native-profiles.json`.
- PROVEN rat has a 2,000 ms attack schedule but no native melee binding.
- PROVEN production candidates use zero health/damage and spell composition has
  creature-name and shape allowlists.
- Preserve WorldId/ChannelId separation, active content binding, current actor/session
  generations, owner lock order, existing receipts and durable writers.

## High-risk authority/recovery qualification

Applicable: existing current-owner combat and condition mutation boundaries.
Before final qualification, test stale actor/session/source rejection, cooldown,
duplicate occurrence, protections, committed player vitals and durable reload.

## Acceptance criteria

- [ ] Source-backed fleeing and all target strategies consume live facts.
- [ ] Rat 0–8 melee, donor interval and 1,500 ms threshold reach production.
- [ ] Admitted distance/area attacks and owned conditions use existing combat.
- [ ] Behaviour parity and real dispatch/PostgreSQL regression cases pass.
- [ ] Content is regenerated and pin-checked; no hand-edited derived hashes.

## Reference parity

| Behaviour | Reference value/source | Oteryn owner | Test |
|---|---|---|---|
| Fleeing | Crystal monster.cpp:58,1001,1204,1266; fleeing suppresses melee only | monster-ai think / melee | pending |
| Attack clock | Crystal monster.cpp:1188-1285 | profile_schedule / melee | pending |
| Current target health | Crystal monster.cpp searchTarget | monster_ai_cycle / targeting | pending |
| Distance/area and conditions | Crystal monsters.cpp and combat.cpp; exact rows added with implementation | native source / spell / dispatch | pending |

## Excluded scope

W01b respawn; other mechanic owners; ledger edits; vendor and repository policy.
No new wire capability or command is currently planned. Reserved numbers remain
0632–0635, 226–227, 432–435 and 572–575; any use requires a fresh collision scan.

## Implementation / findings

AUTHORING. Workers own disjoint AI behaviour, attack-owner and production-test paths;
the task owner integrates dispatch/content and is the only publication writer.

Questions that source/repository inspection cannot settle go to this PR as separate
`QUESTION:` comments with options, file:line evidence and recommendation.
Answers are `ANSWER:` comments, read before each new implementation stage and finish.

## Validation

Completed content checks:
- `python3 -m unittest test_bind_monster_source_definitions` from
  `tools/content-migration`: 6 passed.
- `OTERYN_FULL_SPELL_TEST_MANIFEST=content/spells.manifest.json cargo test --locked
  -p oteryn-gameplay-content actual_full_manifest_qualifies_source_world_and_all_owner_profiles
  -- --ignored --nocapture`: 1 passed, no ignored cases in this explicit run.
- `python3 tools/content-migration/register_spell_families.py`: PASS, 38 registered files,
  activation remains false.
- `git diff --check`: passed at content checkpoint.

The serving provider now carries exact attack closures for all 1,863 canonical creatures;
the seven legacy fixture/familiar identities retain their existing qualified data.
The source parser census finds 4,032 direct/condition spells, 53 chains, 42 variants,
6 windups, 64 appearance casts and 116 shapes owned by other consumers or unsupported.
These counts describe admission only, not yet production execution coverage.

PostgreSQL 18.6 is running in an isolated task-local cluster. Production, AI and combat
checks remain pending until the worker APIs are integrated. Full regeneration currently
waits on those APIs because its materializer compiles the game server.
The task owner records exact commands/results and material omissions in the draft PR.

## Self-review

Pending complete implementation diff and source-parity review.

## PR and closeout

Draft only, per owner instruction. Open immediately after the first commit.
No autonomous protected integration is requested.

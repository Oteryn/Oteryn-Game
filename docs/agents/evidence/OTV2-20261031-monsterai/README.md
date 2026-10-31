# W01a / W01c implementation evidence

Task: [OTV2-20261031-monsterai](../../tasks/OTV2-20261031-monsterai.md).
Draft PR: [Oteryn/Oteryn-Game #2766](https://github.com/Oteryn/Oteryn-Game/pull/2766).
Branch: `chatgpt/monsterai`.

The branch implements ordinary admitted monster targeting, fleeing, shared attack
scheduling, every melee index, and source spell/condition/Item execution through
the existing owners. This record distinguishes source contracts, actual executed
validation, and the scope decisions below. The PR records the final immutable head
after the last commit.

The evidence class is **OTS_HYPOTHESIS_ONLY**. Crystal compatibility is the task's
explicit target; this does not establish independently verified official Global
behavior.

## Source and implementation maps

- [AI, targeting, melee and movement](ai-melee.md)
- [Spells, current owners, positions and scope boundaries](source-attacks.md)
- [Real dispatch, PostgreSQL and source Item proofs](production-postgres.md)
- [All eighteen Encounter-bound refusals, with donor file and lines](encounter-refusals.md)

Primary Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`.
Secondary Canary: `47dfd51f45280a59a1d3e50ba7edd573d7234446`.

Exact source closures cover **1,863 canonical creatures** and preserve seven
existing legacy identities; all **1,870 records** project. The melee census
contains **1,689 indices**, including 24 multi-melee profiles and 13 zero-damage
entries. The spell census classifies **4,295 ordinary spell/Item/composed entries**
and **18 explicit Encounter refusals**. The ordinary entries include 16 composed
occurrences and 82 source Item operations: 62 creation and 20 removal. These are
source projection and classification counts, not thousands of end-to-end tests.

Current authority remains with the existing Channel position owner, current player
state and session-scoped damage ledger, native AI schedule, combat/spell/condition
and timer owners, session-fenced character-vitals writer, and durable Item owner.
Prepared source identity never substitutes for current actor, session, content,
Map/Item, runtime-work or database authority. The linked maps identify the concrete
paths and their refusal/replay coverage.

## Owner decisions

All seven questions were posted on the canonical draft PR and answered option A.
No question remains unanswered. Their complete answers remain in the PR history.

| Question | Decision | Answer |
|---|---|---|
| Q1 — Configured health rates | Keep admitted 1×; non-default monster/boss rates belong to W14a | [Q1A](https://github.com/Oteryn/Oteryn-Game/pull/2766#issuecomment-6543120549) |
| Q2 — Arbitrary Think scripts | Separate typed Interaction/Encounter admission; retain the distinct MonsterType and CreatureEvent callback contracts | [Q2A](https://github.com/Oteryn/Oteryn-Game/pull/2766#issuecomment-6543173780) |
| Q3 — Eighteen Encounter attacks | Explicit refusal and named source matrix; preserve the complete callback behavior for separate admission | [Q3A](https://github.com/Oteryn/Oteryn-Game/pull/2766#issuecomment-6543276153) |
| Q4 — Ordinary Defence activation | Separate production integration; wild self-healing, haste and invisibility are not newly playable | [Q4A](https://github.com/Oteryn/Oteryn-Game/pull/2766#issuecomment-6543503050) |
| Q5 — Movement observation | Include one derived, read-only field in the existing position owner, reusing its revision; no new counter, timer or ledger | [Q5A](https://github.com/Oteryn/Oteryn-Game/pull/2766#issuecomment-6543529998) |
| Q6 — Spell presentation wire/UI | Separate SPELL-PRESENT-0; retain source presentation facts without an undrained producer or playable FX claim | [Q6A](https://github.com/Oteryn/Oteryn-Game/pull/2766#issuecomment-6543702206) |
| Q7 — Damage distribution | Retain the existing deterministic uniform project policy; Crystal truncated-normal parity belongs to separate shared combat-randomness work | [Q7A](https://github.com/Oteryn/Oteryn-Game/pull/2766#issuecomment-6544219482) |

Q7A accepts a distribution difference, not merely a different random stream.
Source damage bounds, chances, intervals and replay remain required. Current player
HP publication uses the existing ACTOR_VITALS revision/delta path. W01b respawn is
outside the supplied task. Existing bounded recipient/timer policies and explicit
refusals for unavailable current authority remain in force.

## Actual PostgreSQL checkpoints

All three runs below used the isolated PostgreSQL **18.6** instance and the required
dedicated test-admin role. Credentials are not recorded. An unset database setting
is a non-executing skip and is never counted as PostgreSQL evidence.

| Run | Actual result | Actual PostgreSQL scenarios | Runtime |
|---|---|---|---|
| Six canonical monster cases | **6 passed, 0 failed, 0 ignored; 316.50 s**. Six distinct `MONSTER_AI POSTGRES_CASE_COMPLETED` markers, with no database skip. | **6** | [a629951e40886b1d32143f3799bbbae43e5e7a3a](https://github.com/Oteryn/Oteryn-Game/commit/a629951e40886b1d32143f3799bbbae43e5e7a3a) |
| Existing `equipment_mods::` filter | **2 passed, 0 failed, 0 ignored; 98.45 s**. One PostgreSQL scenario and one Content-only regression; `party_xp_14` database-start/completion markers, PG scenario 53.17 s, and passing test result. | **1** | [a629951e40886b1d32143f3799bbbae43e5e7a3a](https://github.com/Oteryn/Oteryn-Game/commit/a629951e40886b1d32143f3799bbbae43e5e7a3a) |
| Existing `postgres_tests::loop_monster::` filter | **4 passed, 0 failed, 0 ignored; 183.03 s**. Four configured `server_version_num=180006` markers. | **4** | [e4fde5be5f675c3139fd99d3e080d6d43092d6d5](https://github.com/Oteryn/Oteryn-Game/commit/e4fde5be5f675c3139fd99d3e080d6d43092d6d5) |

This is **11 actual PostgreSQL scenarios across 12 selected tests**. The additional
EQMOD Content-only test is excluded from the database count.

The six canonical monster cases cover:

- **Rat:** real 0–8 melee on its 2,000 ms source schedule, actual HP/MP checkpoint,
  and exactly one extra occurrence after held-target translation, without advancing
  the existing deadlines or global AI sequence. A zero-damage extra roll is valid.
- **Hunter:** canonical arrow damage beyond melee reach, source bounds/range,
  repeat-drain invariants and actual HP/MP checkpoint.
- **Fire Devil:** area damage to both admitted players across bounded attempts;
  the checkpointed player's actual HP/MP are persisted. A single cast need not hit
  both players.
- **Fire Elemental:** physical i2118 field creation with exact Creature receipt,
  Item operation, Ground custody, occurrence provenance and audit entry.
- **Firestarter:** the same durable field path with canonical range zero and caster
  and target on different cells.
- **The Monster:** real removal of a source-qualified i2130 wall, with one removal
  receipt/operation, retired Item state, absent Ground custody, retained original
  MINT provenance and unchanged results on repeated draining.

The EQMOD PostgreSQL case exercises fenced equipment reads and actual combat,
including cleave with primary-only leech, reflection with mana shield, perfect shot,
reflection replay, and durable reflected-kill rewards without duplicate awards.

The four LOOP cases cover source-home walkback, SQL cancellation recovery and
leash behavior; shared-budget resumption, one-pass summon registration and the live
cap; outer/inner catalogue identity, replay and stale-owner refusal; and missing-child
rollback without physical or AI leaks. The three summon variants also invalidate
cached tile facts after a real `SELECT 1/0` transaction abort. The main Map case
additionally invalidates a former empty-tile observation after a real Item MINT
whose movement facts are unqualified.

The six canonical cases preserve source min/max, chances and intervals. Only the
test player's HP pool is enlarged where needed. The removal setup uses the existing
guarded typed Item writer; it does not claim player rune-casting or caster-cost
coverage. The LOOP summon variants retain their existing synthetic 1,000 ms / 100%
schedule and maximum of two summons. Repeat-drain checks differ from injecting an
identical durable proposal, and the Map-abort checks do not populate a delayed-spell
reader bank. Detailed assertions and failure histories are retained in
[production PostgreSQL evidence](production-postgres.md).

## Implementation findings

- The generator binds exact Ability/Effect/Formula dependencies after admitted
  Creature/Behavior overlays and refuses missing, duplicate or oversized closures.
  It preserves later melee slots, duplicate authored summon slots and the source
  Defence-melee representation without activating separate ordinary Defence effects.
  Only the measured creature-profile input ceiling becomes 32 MiB; the other input
  ceilings and existing 128-definition per-creature bound remain unchanged.
- Real PostgreSQL exposed an empty Ground provenance reference rejected by the
  unchanged schema with `23514`. The writer now uses the validated native attack
  occurrence within the existing 1–128-byte bound. The separate range-zero correction
  is proved by unchanged canonical Firestarter content.
- The removal regression exposed known static Source/Bundle walls being treated as
  unknown dynamic obstacles. The creature reader now uses the existing player
  reader's distinction: Entry objects with insufficient dynamic collision facts
  still refuse, while qualified source flags reach the unchanged selection algorithm.
  The actual diagnostic recorded 36 rejected casts before the passing correction.
- Two synthetic fixture families needed coherent attached roots: EQMOD's existing
  Cave Rat flee overlay and LOOP's Rat attack/summon overlay, including its deliberately
  incomplete child. Their explicit fixture copies now agree with their Creature and
  Behavior roots; production validation and negative assertions remain strict.
- Current-owner spell/condition integration retains atomic recipient transitions,
  correct Drown protection mapping and transaction revalidation after asynchronous
  timer reads. Movement fixtures now use pinned actors and separately retain
  foreign-context refusal; the production fence was not relaxed.

Shared SQL physical writers, schemas, merged migrations, permission checks and
existing authority owners remain intact. No new migration, capability, wire command,
mechanic ordinal or offer number was allocated; the reserved blocks remain unused.
No vendor, repository policy, workflow or re-audit ledger file is changed by this task.

## Number scan

The final command `python3 .claude/skills/oteryn-dev/scripts/next-free-numbers.py`
ran against `origin/main c81c2c9b43` and returned **exit 1** because PRs
[#2767](https://github.com/Oteryn/Oteryn-Game/pull/2767) and
[#2777](https://github.com/Oteryn/Oteryn-Game/pull/2777) duplicate reservations:
migration 644, capabilities 232/233, commands 444–447 and ordinals 584/587.
This branch allocates no number; its reserved blocks do not occur in the clash
or held-number lists. No external PR or number registry was changed.

## Final integrated validation

The exact completed commands and results for the final merged
runtime tree are listed below: standard regeneration and World pin-refresh, generator tests,
strict Clippy, all eight changed libraries, explicit actual-manifest censuses and
appearance execution, game-server Content integration and smoke, formatting,
governance and diff checks. Default ignored tests are reported as ignored. Explicit
full-manifest runs set `OTERYN_FULL_SPELL_TEST_MANIFEST` to the actual regenerated
repository `content/spells.manifest.json`. PostgreSQL is unset for the general
library command; its database scenarios are proved by the separate runs above.

The complete eight-library and final opt-in/integration commands below ran on
runtime **`9c72fa2558915c417831d3d8d374320e70a3d9fa`**, merging main
`c81c2c9b4333a7c53e362a9bfe32549ef1040ace`. The six monster and EQMOD database
runs used `a629951e40886b1d32143f3799bbbae43e5e7a3a`; the four LOOP database
cases used `e4fde5be5f675c3139fd99d3e080d6d43092d6d5`. The only gameplay/Content
delta after the six-case run is the synthetic summon-fixture repair, tested by
the four LOOP cases. The subsequent main merge changes only three client files.
Standard regeneration returned no data or pin changes. The final evidence commit
changes documentation only; its immutable head is recorded in the PR.

| Command | Actual result |
|---|---|
| `bash tools/merge-driver/regen.sh` | suite: 5 passed, 0 failed, 0 ignored (34.70s). Full standard regeneration and pin-refresh passed; included five repository Content/World tests. |
| `python3 tools/content-migration/test_bind_monster_source_definitions.py` | Ran 6 tests in 3.595s (5 s wall time) |
| `cargo clippy --locked -p oteryn-gameplay-content -p oteryn-gameplay-domain -p oteryn-gameplay-entities -p oteryn-gameplay-combat -p oteryn-gameplay-spell -p oteryn-gameplay-monster-ai -p oteryn-gameplay-dispatch -p oteryn-gameplay-durability --all-targets --all-features -- -D warnings` | Passed, exit 0 (25 s wall time) |
| `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib monster_attacks:: -- --nocapture --test-threads=1` | oteryn_gameplay_dispatch: 6 passed, 0 failed, 0 ignored (316.50s). All six distinct PostgreSQL completion markers; includes Rat translation and The Monster removal. No skip path. |
| `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib equipment_mods:: -- --nocapture --test-threads=1` | oteryn_gameplay_dispatch: 2 passed, 0 failed, 0 ignored (98.45s). Two selected tests: one real PostgreSQL scenario and one Content-only fixture regression. |
| `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib postgres_tests::loop_monster:: -- --nocapture --test-threads=1` | oteryn_gameplay_dispatch: 4 passed, 0 failed, 0 ignored (183.03s). Four real PostgreSQL 18.6 scenarios: source home/leash; one-pass summon and cap; outer/inner source scope; missing-child rollback. Wrong and aborted physical transactions invalidate Map facts. No populated delayed-bank claim. |
| `cargo test --locked -p oteryn-gameplay-content -p oteryn-gameplay-domain -p oteryn-gameplay-entities -p oteryn-gameplay-combat -p oteryn-gameplay-spell -p oteryn-gameplay-monster-ai -p oteryn-gameplay-dispatch -p oteryn-gameplay-durability --all-features --lib -- --test-threads=8` | oteryn_gameplay_combat: 41 passed, 0 failed, 0 ignored (1.52s); oteryn_gameplay_content: 284 passed, 0 failed, 1 ignored (129.63s); oteryn_gameplay_dispatch: 1675 passed, 0 failed, 15 ignored (289.83s); oteryn_gameplay_domain: 143 passed, 0 failed, 0 ignored (0.02s); oteryn_gameplay_durability: 547 passed, 0 failed, 0 ignored (29.89s); oteryn_gameplay_entities: 196 passed, 0 failed, 0 ignored (3.24s); oteryn_gameplay_monster_ai: 135 passed, 0 failed, 4 ignored (8.97s); oteryn_gameplay_spell: 284 passed, 0 failed, 0 ignored (6.43s). Eight complete libraries; default ignored tests are listed explicitly. PostgreSQL is unset for this command and is proved by the separate runs above. |
| `cargo test --locked -p oteryn-gameplay-monster-ai --all-features --lib actual_full_admitted_melee_sources_resolve_every_authored_index -- --ignored --nocapture` | oteryn_gameplay_monster_ai: 1 passed, 0 failed, 0 ignored (28.17s). Explicit actual-manifest census: 1689 entries, 24 multi-melee profiles, 13 zero entries. |
| `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib monster_actual_encounter_attacks_remain_eighteen_explicit_refusals -- --ignored --nocapture` | oteryn_gameplay_dispatch: 1 passed, 0 failed, 0 ignored (29.54s). Explicit actual-manifest refusal set: exactly 18 Encounter entries. |
| `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib creature_damage_spell_appearance_execution_tests -- --ignored --nocapture --test-threads=1` | oteryn_gameplay_dispatch: 6 passed, 0 failed, 0 ignored (181.41s). All six opt-in actual-content appearance execution tests explicitly invoked. |
| `cargo test --locked -p oteryn-gameplay-content actual_full_manifest_qualifies_source_world_and_all_owner_profiles -- --ignored --nocapture` | oteryn_gameplay_content: 1 passed, 0 failed, 0 ignored (63.25s). Actual regenerated manifest and all owner profiles, explicitly invoked. |
| `cargo test --locked -p oteryn-game-server --test integration_rest content_ --quiet` | suite: 123 passed, 0 failed, 0 ignored (1.75s) |
| `cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-check . /home/mole/monsterai-20261031/pin-check` | Passed, exit 0 (27 s wall time) |
| `cargo run --locked -p oteryn-game-server -- --smoke` | Passed, exit 0 (85 s wall time) |
| `cargo fmt --all -- --check` | Passed, exit 0 (8 s wall time) |
| `python3 tools/agents/validate_governance.py` | Passed, exit 0 (0 s wall time) |
| `python3 -m unittest discover -s tools/agents/tests` | Ran 129 tests in 0.294s (0 s wall time) |
| `git diff --check` | Passed, exit 0 (0 s wall time) |


## Node-boot and deployment boundary

The actual command `bash tools/qualification/node_boot/run.sh` returned
`NODE_BOOT_RESULT=BLOCKED reason=exact_platform_checkout_missing` with **exit 2**.
The required Platform pin is `3bc2d0e923db7472ec1f9b534ebe87aaffa1ef05`. Docker is
also unavailable in the task's WSL distribution. The passing isolated PostgreSQL
runs do not qualify the required Platform/container/TLS topology; no node-boot pass
is claimed. Full-workspace, Windows-client and deployment/E2E lanes remain CI work.

The PR stays **DRAFT**. No review request, `heavy-tests` label, PR merge or deployment
was made by this task.

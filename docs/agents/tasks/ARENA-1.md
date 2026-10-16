# ARENA-1 — Tibiadrome and arenas (skeleton task record)

```yaml
task_id: ARENA-1
title: "ARENA-1: Tibiadrome and arenas (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/arena-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 4 row ARENA-1 (row 106); §5 Wave 5"
class: "hard (child graph)"
needs_migration: maybe
needs_registry: maybe
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/arena/ (new) + tests"
  - "content/encounters/arenas/ and content/encounters/tibiadrome/ source inputs"
  - "rulesets/encounters/tibiadrome/ and rulesets/pvp/arena/ source inputs (D1048)"
  - "docs/agents/tasks/ARENA-1.md"
slot_only_paths:
  - "regenerate_content.py outputs, project.json, manifest.json, content.lock.json, pins, Rust pins"
  - "migration if durable"
  - "GAME_EVENT_FOUNDATION_REGISTRY.json"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Tibiadrome and arenas (encounter rooms, rewards).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| BOSS_RAID0 acceptance | ACCEPTED 2026-10-15 (D1050) | acceptance met; child runtime gates remain |
| LOOP-MONSTER-1 | NOT MERGED (Wave 2) | **BLOCKING** |
| PARTY-XP-1, BOSS-ROOM-1 (child prerequisites) | NOT MERGED | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/arena/ (new) + tests
- `content/encounters/arenas/ and content/encounters/tibiadrome/ source inputs`
- `rulesets/encounters/tibiadrome/` and `rulesets/pvp/arena/` source inputs (D1048)

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- regenerate_content.py outputs, project.json, manifest.json, content.lock.json, pins, Rust pins
- migration if durable
- GAME_EVENT_FOUNDATION_REGISTRY.json
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/monster_lab.rs (arena lab)
- encounter runtime (ENC-RT-1)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author
- mint/burn + ledger (rewards)

## Acceptance criteria

- [ ] arena entry gated server-side; reward granted exactly once
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server arena
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- `docs/architecture/reviews/OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md`
- `docs/agents/tasks/archive/OTV2-20260928-arena-summons.md`
- `docs/agents/tasks/archive/OTV2-20261002-monster-lab-arena.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded PvE reference inventory preparation — 2026-10-16

Author: the assigned family lead's arena worker, sole writer of this checkout.
Authoring base: `d3a34a812cb72e982d994edabbc395c45a02394f`, existing draft #2179,
`skeleton/arena-1`; live PR metadata/head refreshed before this preparation.

Current decision evidence supersedes the historical acceptance status in the brief:
[D1046–D1051](https://github.com/Oteryn/Oteryn-Game/pull/2179#issuecomment-6216592796)
correct ownership and split the mechanic, migration and protocol work;
[owner acceptance](https://github.com/Oteryn/Oteryn-Game/pull/2179#issuecomment-6216812425)
accepts A1–A11 plus D1047. D1051 still requires the architecture acceptance PR to
merge before integration; that merge and the child runtime dependencies are not
claimed here. The existing permission to draft owned code without assuming
unmerged schemas permits this source-reference preparation only.

`content/encounters/arenas/pve-catalog.json` contains the 40 existing Creature
identity triples selected by A8, copied at their actual `definition-r1` revisions.
Its only field is the existing Encounter tooling catalog's `definitions` array;
its entries contain only `family`, `key` and `revision`. It introduces no schema,
canonical identity or asset. It is an offline reference inventory, with no Rust
loader/caller, runtime authority or content population/activation claim. The
directory's existing `READY_UNPOPULATED` index remains unchanged. This is partial
preparation, not completed ARENA-CONTENT-1 or ARENA-1.

### Preserved A8 pit mapping

The flattened catalog order preserves A8's four tiers, ten pits each. Each suffix
below has the literal `oteryn:creature.` prefix; list order is pit 1 through 10.
The existing catalog format has no tier/band/scaling fields, so none are added.

| Tier | Catalog positions (1-based) | Ordered Creature key suffixes |
|---|---|---|
| Greenhorn | 1–10 | `honour_guard`, `scar_tribe_warrior`, `dire_penguin`, `bonelord`, `deepling_brawler`, `deathbine`, `wyvern`, `bane_of_light`, `cult_scholar`, `pirat_bombardier` |
| Scrapper | 11–20 | `warlord_ruzad`, `goldhanded_cultist`, `enslaved_dwarf`, `two_headed_turtle`, `draken_abomination`, `rhindeer`, `kerberos`, `rootthing_nutshell`, `juggernaut`, `noxious_ripptor` |
| Warlord | 21–30 | `sulphider`, `branchy_crawler`, `grand_master_oberon`, `wandering_pillar`, `the_monster`, `the_pale_count`, `zoralurk`, `lloyd`, `melting_frozen_horror`, `tentugly_s_head` |
| Paragon | 31–40 | `hazardous_phantom`, `overseer_osverger`, `court_warlock`, `soul_of_dragonking_zyrtarch`, `magma_bubble`, `infernatil`, `essence_of_malice`, `the_corruptor_of_souls`, `prince_drazzak`, `the_pale_worm` |

### Provenance and bounded validation

All inputs are retained on the authoring base above. Each selected canonical
record retains its own donor source binding; this inventory does not upgrade a
donor fact to official Tibia evidence. A8's HP, XP, reward-boss flag and declared
shard match all 40 canonical records; each identity has one retained source
binding naming that same exact reference. No new stats or scaled definitions are
authored. These SHA256 values bind the inputs and the prepared output:

| Input/output | SHA256 |
|---|---|
| `OTERYN_GAME_ARENA1_DECISION_2026-10-15.md` | `cc9636558bc4bd13e7e7b13ad3310fa764c7c8c66fe22c126ad93596b7b339ed` |
| `creatures-00000-00499.json` | `790bf6b275f52977026e243f3405a2ae7cd7d1c9fd6180d74670c73aff5a2234` |
| `creatures-00500-00999.json` | `a450f654319808f39dac810b936c20154672a795549fdddb0e4aab5ac12ea246` |
| `creatures-01000-01499.json` | `d39b9f8a970826f424332d99c1812befa972dd2ad552ade9e328e341eba7555a` |
| `creatures-01500-01862.json` | `6cc26f42b45a939f72198451e884f2062f746cab4e0d930ed8470dbadecd8461` |
| `tools/content-schema/encounter-authoring/validate_encounter.py` | `04881610468e68b39795ad0b578e7d00b0d97168711dc8c3bdcf45cbba52bbe0` |
| `tools/content-schema/encounter-authoring/encounter.schema.json` | `cc77fc37ac446c0e03dcdd1ca2c27dc078195b60a433e48a55e352fb88471b35` |
| `content/encounters/arenas/pve-catalog.json` | `16a6b518be5ca4800a4e8a78a06fe34ae422d8828c8a86549f6c7676888c20ed` |

The existing Python validator's `CreatureRef` participant schema and catalog
resolution at `validate_encounter.py:367` accept Creature triples. Compatibility
was checked before authoring with its retained valid Zamulosh sample: an
in-memory participant containing all 40 actual references and its corresponding
catalog validate successfully. Removing a required reference or changing its
revision is refused as `unresolved definition`. This is a catalog-reader probe,
not an authored Zamulosh/arena encounter or gameplay test. The positive probe
qualifies reference resolution inside an existing valid Encounter; standalone
catalog schema and compiled CONTENT qualification remain unclaimed. Inventory checks require
exactly 40 unique references in A8 pit order, exact canonical revisions, matching
HP/XP/boss facts and source-binding targets; an empty or drifting inventory fails.

Actual local checks on this two-file preparation:
- Actual-file inventory/source check passed, including eight refused inventory
  variants (empty, duplicate, wrong family/revision/key, reordered and extra
  top-level/entry fields), plus the existing consumer probe and its two negative
  cases described above.
- `python tools/content-schema/encounter-authoring/verify_encounter_schema.py`:
  233/233 checks passed; synthetic schema/semantic fixtures only.
- `python tools/agents/validate_governance.py`: passed; 22 documents, 9 lanes.
- `python -m unittest discover -s tools/agents/tests`: 124 tests passed.
- No Rust loader references this filename; no Rust build, generated-content
  rebuild, PostgreSQL validation or gameplay execution is claimed.

Remaining CONTENT-1 work includes actual arena/drome definitions, modifiers,
locations, four tier rules, scaling and the Paragon reward set. INSTANCE/runtime,
durable reward/clock work and protocol work remain separate accepted children.
No new drome references, reward identities, pins, registries, regeneration outputs,
architecture edits, shared wiring or production changes are included.

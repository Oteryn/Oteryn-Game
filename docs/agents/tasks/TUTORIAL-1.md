# TUTORIAL-1 — tutorial island test epic (skeleton task record)

```yaml
task_id: TUTORIAL-1
title: "TUTORIAL-1: tutorial island test epic (skeleton)"
mode: IMPLEMENT
status: draft
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/tutorial-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 4 row TUTORIAL-1 (row 139); §5 Wave 5"
class: "impl"
needs_migration: yes
migration: "apps/game-server/migrations/0130_character_tutorial_progress.sql (number assigned by CP)"
decision: "CP D1426 (#2186 comment 6241833560): Newhaven start island only; inert on StartThais. Owner D1463 (2026-10-19) supersedes the D1410 no-spawn-change reading: new Characters start on the Newhaven islet (32534,32513,7); Anna sails to Thais Peninsula at level 8"
needs_registry: no
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "content/ tutorial source inputs"
  - "apps/game-server/tests/tutorial_*.rs (new)"
  - "apps/game-server/src/domain/tutorial.rs (new, CP D1426)"
  - "apps/game-server/src/durability/character_tutorial.rs (new, CP D1426)"
  - "apps/game-server/migrations/0130_character_tutorial_progress.sql (new, CP D1426)"
  - "apps/game-server/tests/support/tutorial_progress_postgres_cases.rs (new, CP D1426)"
  - "docs/agents/tasks/TUTORIAL-1.md"
registration_lines:
  - "apps/game-server/src/domain/mod.rs: pub mod tutorial;"
  - "apps/game-server/src/durability/mod.rs: pub mod character_tutorial;"
  - "apps/game-server/tests/character_authority_postgres.rs: #[path] include of the tutorial PG cases"
slot_only_paths:
  - "regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Late, non-blocking test epic: Newhaven/Rookgaard tutorial content tested after the Thais start.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| Wave 4 done | NOT DONE | **BLOCKING** |
| brief/packet | MISSING on main; owner answer §6.1 only | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- `content/ tutorial source inputs`
- apps/game-server/tests/tutorial_*.rs (new)

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- quest runtime
- world bundle (content/world)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author

## Acceptance criteria

Replaced by the CP D1426 scenario brief below (its assertions A1-A6). The start stays StartThais
(D1410), so on the current World the tutorial is inert.

- [x] A1: a StartThais Character never enters the tutorial (`tests/tutorial_newhaven.rs`).
- [x] A2-A4: a Newhaven-start Character goes through the reference steps (`tests/tutorial_newhaven.rs`).
- [x] A5: concurrent writers are serialized by a row lock and a compare-and-set version; the loser gets a typed `Conflict` and writes nothing, also when its progress equals the stored one (a retry or a stale equal-state writer; no occurrence identity), see `tutorial_progress_postgres_cases.rs`.
- [x] A6: after a reconnect (new connection generation) and a node restart the tutorial resumes from the stored step; a stale connection, lease or session reads and writes nothing (`tutorial_progress_postgres_cases.rs`).
- [ ] Runtime wiring (login, step-in, death, loot, NPC) stays a slot-only follow-up; nothing calls the writer yet.
  It is blocked on the D1463 follow-ups listed in *Owner D1463 amendment* below.

## Validation commands

```sh
cargo fmt --all -- --check
cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
cargo test --locked -p oteryn-game-server --test tutorial_newhaven
# PostgreSQL 17.6 (OTERYN_TEST_POSTGRES_ADMIN_URL set):
cargo test --locked -p oteryn-game-server --test character_authority_postgres tutorial_progress -- --test-threads=1
cargo test --locked -p oteryn-game-server --test character_authority_postgres every_postgres_target_runs
```

No content changes, so no regeneration or pins.

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §6.1
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.


## CP D1156 allocation clarification — 2026-10-17

This appendix preserves the historical skeleton and makes the reviewed current
allocation explicit in the repository. The source checkpoint read for this
correction is `c580e630f0c8099580727265086e0fb0977ac802`; it contains only this
task record, with no tutorial runtime, content or behavioral test implementation.

[CP D1156](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224947337)
retains TUTORIAL-1 behind Wave 4 and requires a concrete CP scenario brief for the
**Thais start**. The historical tutorial-island/Newhaven/Rookgaard label is a
plan label, not authorization to invent an island, route, reward or completion
criterion. Thais remains the start.

The reviewed allocation has not supplied the required concrete scenario brief,
acceptance assertions or recovery/reconnect expectations. Those missing product
facts prevent a meaningful tutorial implementation or behavioral test from
being asserted complete. No synthetic scenario is substituted for the brief.

| Prerequisite | Current reviewed boundary |
|---|---|
| Qualified Wave 4 interfaces | Required before the dependent tutorial integration |
| Concrete CP Thais-start brief | Required; not supplied in the reviewed allocation |
| Scenario assertions and recovery/reconnect expectations | Must come from that packet |
| Content regeneration and pins | Remain in the shared slot with their owning author |

After the packet is released, the allocated tutorial source inputs and
`apps/game-server/tests/tutorial_*.rs` can be implemented against the qualified
Wave 4 interfaces. The inseparable content/regeneration/pin bundle and all shared
wiring restrictions remain applicable.

This amendment persists the allocation boundary only. It records no runtime
test result, content activation or playable tutorial completion. The task remains
a draft behind the stated prerequisites; CP retains integration/merge authority.


## CP D1426 scenario brief — 2026-10-17

[CP D1426](https://github.com/Oteryn/Oteryn-Game/pull/2186#issuecomment-6241833560) replaces the
Thais-start reading of D1156: the tutorial applies **only** on the Newhaven start island, as in
global. StartThais is a temporary test start where the tutorial never starts. There is no spawn
change (D1410); the tutorial stays inert, gated on start island == Newhaven.

### Sources

- Island: `content/world/areas/islands/islands-00000-00059.json`, record
  `oteryn:area.island.newhaven` (TibiaWiki page 109339 rev 1197143, frame
  `global-target-2026-09-27`): island footprint x 32506-32634, y 32405-32522, floor 7; additional
  component (the tutorial islet with the temple) x 32532-32539, y 32511-32518, floor 7.
- Current start: `content/world/pins/oteryn.json` `entry_start` (32369, 32238, native floor -7),
  i.e. StartThais, outside both areas.
- Rules: `zimbadev/crystalserver` rev `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
  `data-global/scripts/quests/newhaven/` (donor corpus
  `tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz`; git blob ids):
  `on_login_newhaven.lua` b101507c, `movement_init_newhaven.lua` 9d4adb9b,
  `movement_select_vocation.lua` 8366ff4e, `movement_update_tutorial_hunting.lua` 4c3cb41f,
  `#ondroploot_the_corruptor.lua` a6984946, `on_death_newhaven.lua` 680ec85a; tracks and NPC
  transitions in `content/quests/missions/quest-state.json` quest `newhaven_quest`
  (`newhaven_citizen`, `newhaven_tutorial_hunting`, `newhaven_new_loot_the_corruptor`).
  The stages of `oteryn:quest.newhaven_quest` are a chosen approximation and are not used.

### Steps (each from the named script)

| # | Trigger | Condition | Effect | Stored progress |
|---|---|---|---|---|
| 1 | Login | vocationless, no account access, not a citizen (no row or departed) | placed at islet temple (32534, 32513, 7) | step islet |
| 2 | Step on islet hole (32533, 32513, 7) | always | pushed back to the previous cell, message "I should not climb down again but rather venture forth." | unchanged |
| 3 | Step on islet forcefield (32538, 32514, 7) | always | moved to (32544, 32515, 7); tutorial hint 2 if vocationless | step island, hunting and Corruptor tracks armed |
| 4 | Step on (32561, 32485, 7) or (32562, 32485, 7) | hunting track armed | tutorial hint 3 | hunting track cleared |
| 5 | The Corruptor drops loot for the Character | Corruptor track armed | vocation loot added | Corruptor track cleared |
| 6 | Death | a citizen, vocationless, no account access | placed at islet temple | step islet |
| 7 | `a_seagull` / `anna` take the Character away | any row | none | step departed, both tracks cleared |

A login of a citizen (step islet or island) resumes: no placement, progress unchanged.

### Acceptance assertions

- A1: a Character whose start is not on Newhaven (StartThais) gets the inert gate; no event writes
  progress or has an effect, on any cell.
- A2: a Newhaven-start vocationless Character is placed on the islet at login (step 1); a
  gamemaster or a Character with a vocation is not.
- A3: steps 2-5 have exactly the effects above, the hint and the loot once each.
- A4: step 6 returns a vocationless citizen to the islet; step 7 clears every track.
- A5: progress writes are serialized per Character (row lock + compare-and-set version); the
  losing writer gets a typed conflict with the stored progress; a replay writes nothing.
- A6: reconnect/recovery: progress is durable (migration 0130); after a reconnect in the same
  GameSession and after a node restart the Character resumes from the persisted step; the stale
  connection, lease and session are refused.

### Out of scope (follow-ups, each needs its own source-backed task)

Starter items (`lib/others/newhaven.lua`), map marks, the vocation loot items, the Dawnport branch
of the death script, the level-8 door (`on_use_newhaven.lua`), and the runtime wiring of login,
movement, death, loot and NPC events to the writer (slot-only paths). The forcefield teleport
itself already exists (NEWHAVEN-TELEPORT-1). The death rule reads the current step, where the
reference reads the citizen track at the last login.

## Owner D1463 amendment — 2026-10-19

Owner decision D1463 replaces the D1410 "no spawn change" reading: a new Character starts on the
Newhaven islet (32534,32513,7), the islet forcefield (32538,32514,7) leads to (32544,32515,7), and
Anna sails to Thais Peninsula (not Targuna) from level 8. Evidence: #2255 (audit report, head
d008217e).

The gate now matches exactly the two committed Newhaven land components (the islet and the island)
from the B3 ground classes, so another land component inside the Newhaven bounding box stays inert
(`tests/tutorial_newhaven.rs`).

Remaining work, each outside this record's owned paths or needing an owner decision:

- Spawn change to the islet: `content/world/pins/oteryn.json` `entry_start` and its regeneration
  (slot-only). The start and portal-exit tiles must be passable for the native mover first.
- Runtime wiring of login, step-in, death and loot events to the reducer and the writer:
  `gameplay_transport/mod.rs` and `connection.rs` (slot-only), with an end-to-end test through the
  real dispatch in a named CI job.
- Hint delivery (hints 2 and 3) and the islet-hole text: no server frame carries a tutorial hint
  or a status text today; that is a protocol change and needs the owner's sign-off.
- Anna's transport to Thais Peninsula: the landing cell at the Thais port near Hector is
  OTERYN_DESIGN; the Anna and Seagull NPC placements do not exist yet.
- The Corruptor vocation-loot item IDs are unresolved (the reference script is disabled).

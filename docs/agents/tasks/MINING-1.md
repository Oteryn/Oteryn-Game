# MINING-1 — mining and map-resource gathering (task card)

```yaml
task_id: MINING-1
title: "MINING-1: mining and map-resource gathering"
mode: IMPLEMENT
status: in_progress   # basic shiny-stone server USE implemented; other pick actions remain
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 CRAFT-1 adjacent scope; gathering yields require their own decision"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_CRAFT1_DECISION_2026-10-15.md
class: "hard, persistence and economy review"
base_branch: main
branch: claude/happy-davinci-ijz3wy-srv-mining
pr: null
needs_migration: true   # allocated at lane start
needs_registry: false   # reuses existing USE codecs and resource ceilings
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Current owner scope and result

Owner decision 2026-10-23 supersedes generic gathering: **generic mining is
NOT-IN-REFERENCE / out of scope**. Do not invent ore/stone nodes, outputs,
chances or respawn timings. Implement Reference-backed pick USE on the server;
client UI and publication remain outside this worker's allocation.

The basic **carried shiny-stone refining** path is implemented end to end. A
logged-in session uses pick 3456 on shiny stone 10310 through command 2 USE.
One stone becomes one Reference-defined coin or diamond in the main backpack;
the pick remains intact. Both a carried tool handle and a definition hotkey
resolve against the fenced carried tree, including closed nested bags.

This is not completion of every action in Canary's pick handler. Positioned
obstacle removal, map transformations and quest-specific pick actions remain
unimplemented by this change, as listed under Unfinished. No ore/stone mining
or map respawn is claimed.

## Server wiring gap list

Initial local HEAD: `151c4bb8372cea1672e4106afd6b1d0b8253445c`.
Branch: `claude/happy-davinci-ijz3wy-srv-mining`; no frozen SHA or PR.
Allocated migration: **0167**. Reserved command 54–55, capability 48 and
resource-limit entries 0780–0786 were not needed.

| Layer | Found on this HEAD before implementation | Implemented result / remaining gap |
| --- | --- | --- |
| L3 | Reach/capacity helpers; C5 digging content facts; no mining rules | `items/src/mining/` qualifies the normal pick and exact shiny-stone 1/5/45/49 table. No generic mining rule. |
| L4 | Atomic one-unit item-use TRANSFORM, retained receipts, nested custody proofs | `item_use/mining.rs` checks the unchanged pick, native definitions, current Content, capacity and weight under the existing current session/lease/scope/recovery fences. Source, output, proof, receipt and audit commit together. |
| Schema | `game_item_use_records`, nested-source proofs (0131) | Migration 0167 admits cause 3 PickRefine with a required output and an immutable tool/wire/draw proof, deferred atomicity checks and runtime-role grants. |
| Protocol / registry | Existing bounded command 2 USE, carried source, definition hotkey, use-with item/object and result codecs | Existing codec reused unchanged; no command/capability allocation or registry edit. Audit proto documents additive cause 3. |
| Dispatch | Food/potion owner and USE ingress; map use-with rejected | Mechanic-owned `gameplay_transport/mining/` routes USE, retains original bytes and reconciles replay before stale handles can refuse. Unknown commits disconnect without sequencing a terminal refusal. Unsupported map/quest actions still refuse. |
| Offer / runtime | Bundle-world production composition and Item index | Content-qualified offer adds existing capabilities 11 and 15 with their dependency closure. Existing composed production authority forwards to the mechanic; no new game-server runtime owner is needed. |
| Content | Item identities existed, but native policies for pick, shiny stone and diamond were absent | Exactly three native profiles added from admitted identity/physical facts and pinned Reference stack flags. Native provider now has 435 profiles; generated manifest/index/lock digests regenerated. |
| PG E2E | Auto-mounted dispatch lib tree and production `serve_admitted` | `postgres_tests/mining.rs` exercises command/result bytes, nested custody, wrong tool/target, full backpack, missing capability, stale generation, lost acknowledgement, replay/conflict and reload using the installed runtime role. |
| Remaining map owner | C5 content validation, base-map overlay and door owner | No new map transformation, positioned obstacle or quest-state writer in this change. |

## Reference parity table

Pinned Canary revision: `52550d5e3c0ffe565f4bd1014f4e5575abf1059b`.
Evidence is bounded to the named handler and its delegated actions; it is not
an audit of all Canary quests.

| Visible behavior | Reference evidence | Oteryn now |
| --- | --- | --- |
| Normal pick 3456 stays intact | [pick.lua](https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/data/scripts/actions/tools/pick.lua), delegates to onUsePick without consuming the tool | Implemented, checked in the atomic writer, SQL proof and PG scenario. |
| Shiny stone 10310 consumes one unit and produces exactly one item: 1% crystal coin 3043, 5% gold coin 3031, 45% platinum coin 3035, 49% small diamond 3028 | [onUsePick](https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/data-otservbr-global/scripts/lib/register_actions.lua), shiny-stone branch | Implemented. Exhaustive 100-draw rule test; retained occurrence draw; fenced atomic conversion; restart and replay proof. Petrologist achievement progress is not implemented. |
| Pickable ground opens holes; individual ice/stone forms transform and decay | Same handler; [tile_pick.lua](https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/data-otservbr-global/startup/tables/tile_pick.lua); C5 `content/craft/map-facts.json` retains pinned XML return evidence | UNFINISHED: no map-transition dispatch added here. No ore is substituted for these transformations. |
| Positioned stone 1772 at (32648,32134,10) disappears for 20 seconds without reward | Same onUsePick handler | UNFINISHED: no positioned obstacle transition/respawn added here. |
| Secret Library crystals, Dream Courts lock, Grimvale silver, Rookie Guard pillars, Ice Islands, Sea of Light and other quest branches have their own state and gates | Same handler and its named delegated quest actions | UNFINISHED: no quest state is fabricated or bypassed. |
| Generic renewable ore/stone mining | The pinned handler has no generic ore/stone yield table; [TibiaWiki Pick](https://tibia.fandom.com/wiki/Pick) and [Iron Ore](https://tibia.fandom.com/wiki/Iron_Ore) do not establish one | **NOT-IN-REFERENCE / out of scope**, per owner decision. No invented yield or respawn. |
| Atomic result and receipt survive reconnect/restart; full inventory refuses the entire conversion | Owner's server durability requirement; Reference Lua uses separate add/remove calls | Deliberate stronger Oteryn semantics, documented in `docs/contracts/MINING_PICK_USE_V1.md`. |

The three native profiles use existing Crystal identity bindings and admitted
World physical facts. `tools/content-schema/native-gameplay/mining-item-facts.json`
retains only decoded stack/pickup facts from Canary's pinned appearances.dat
(SHA-256 `aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50`).
No original binary or sprite asset was added. Provider generation is reproducible
through `tools/content-migration/register_spell_families.py`.

## Validation evidence

Passed so far (2026-10-23):

- `cargo check --locked -p oteryn-gameplay-dispatch --quiet`.
- `cargo clippy --locked -p oteryn-gameplay-items -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings`.
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`.
- Items `mining::`: 1 exhaustive rule test.
- Durability `item_use_audit::tests`: 8 tests.
- Dispatch `mining::offer::tests`: 1 negotiation/offer test.
- Dispatch `item_use::tests`: 5 existing item-use gate/rule tests.
- Dispatch `world_required_capabilities::tests`: 17 boot/negotiation tests.
- Dispatch `equipment_effects_owner_pinned_production_speed_matches_all_raw_attributes`: 1 consumer test.
- Content `native_gameplay`: 45 pass; 1 pre-existing ignored test, unchanged.
- Spell import generator regression tests: 9 pass.
- Dispatch `mining_pick_refining`: 1 PostgreSQL E2E using the absolute wrapper;
  runtime role asserted before conversion and after reopening the durability root.
  Two concurrent live commands competing for the final unit produce exactly one
  conversion; the source retires at zero and another attempt refuses.
- Content profile generator: 2 tests, including duplicate identity and substituted
  Reference revision refusal; `register_spell_families.py --check` passes.
- Governance: 22 policy documents / 9 lanes pass; governance unit tests: 129 pass.
- Repository policy: 23 files / 68 workflows pass. `git diff --check` passes.

Exact cargo test commands (all use the queued wrapper, PG scope is mechanic-only):

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-items --lib mining:: --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --lib item_use_audit::tests --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib mining::offer::tests --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib item_use::tests --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib world_required_capabilities::tests --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib mining_pick_refining --quiet -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib equipment_effects_owner_pinned_production_speed_matches_all_raw_attributes --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-content --lib native_gameplay --quiet
```

Final dispatch clippy and rustfmt verification pass. Rustfmt explicitly covered
the auto-mounted PG files and child fixtures as well as the workspace roots.
An initial content test build lost its incremental cache files on the shared PC;
its retry passed without deleting or changing shared caches. The initial
equipment module filter matched zero tests; the exact named consumer filter
above then passed. No absent-PostgreSQL early return is claimed as qualification.

D1462 whole-diff checklist review and workspace match/constructor/impl sweep
are complete. All ItemUseKind matches, ItemUseRequest constructors and
FreshAdmissionAuthority implementations in apps/, crates/ and tools/ were
searched; the production implementation forwards USE and other test authorities
retain a closed default. Every new public rule/storage API has a production caller
through USE and is exercised by the named PG dispatch test. Client, credential,
workflow and repository-policy paths were not changed. No Git mutation or
publication was performed; independent candidate review remains with the lead.

## Owned / changed paths

- `crates/gameplay/items/src/mining/`, with one registration in `src/lib.rs`.
- `crates/gameplay/durability/src/durability/item_use/mining.rs`, the existing
  item-use transaction/ancestry modules and audit cause decoder.
- `crates/gameplay/dispatch/src/gameplay_transport/mining/`, auto-mounted
  `postgres_tests/mining.rs` and its child fixtures; minimal USE trait/registration
  and content-qualified offer composition; exact native profile count/unknown
  speed assertions updated in the existing equipment consumer test.
- `apps/game-server/migrations/0167_pick_refining.sql`.
- Native profile registration helper/tests, three-item Reference facts, generated
  native profiles and their manifest/index/lock digests.
- `docs/contracts/MINING_PICK_USE_V1.md`, the audit proto cause comment and this card.

## Unfinished

- Reference map/tile transformations, the positioned 20-second obstacle and
  quest-specific pick actions (including Petrologist progress) are not implemented
  here. The delivered path is carried shiny-stone refining only.
- No PR, commit, external review or publication: the owner prohibited Git writes.
- No pending owner question. The earlier generic-yield blocker was resolved by
  the owner's explicit NOT-IN-REFERENCE decision, not by inventing a mining table.

## Exact changed files

- `apps/game-server/migrations/0167_pick_refining.sql`
- `content/content.lock.json`
- `content/items/definitions/spell-native-profiles.json`
- `content/items/index.json`
- `content/manifest.json`
- `content/spells.manifest.json`
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/item_use.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/mining/mod.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/mining/offer.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/mod.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/mining.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/mining/fixture.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/mining/full_backpack.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/mining/restart.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/mining/wire.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities.rs`
- `crates/gameplay/durability/src/durability/item_use.rs`
- `crates/gameplay/durability/src/durability/item_use/carried_inventory.rs`
- `crates/gameplay/durability/src/durability/item_use/carried_source.rs`
- `crates/gameplay/durability/src/durability/item_use/mining.rs`
- `crates/gameplay/durability/src/durability/item_use_audit.rs`
- `crates/gameplay/items/src/lib.rs`
- `crates/gameplay/items/src/mining/mod.rs`
- `crates/gameplay/items/src/mining/tests.rs`
- `docs/agents/tasks/MINING-1.md`
- `docs/contracts/MINING_PICK_USE_V1.md`
- `docs/contracts/game-events/v1/native_one_item_transaction.proto`
- `tools/content-migration/mining_item_profiles.py`
- `tools/content-migration/register_spell_families.py`
- `tools/content-migration/test_mining_item_profiles.py`
- `tools/content-schema/native-gameplay/mining-item-facts.json`

# ITEM-TIMED-1 — basic server charges, duration and repair

```yaml
task_id: ITEM-TIMED-1
title: "ITEM-TIMED-1: basic server charges, duration and repair"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
pr: null
base_branch: main
base_sha: c938f50b4f727cb0dc1adf60ca62f7d63f4f6541
branch: claude/happy-davinci-ijz3wy-timed
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY
```

## Current result (2026-10-23)

The owner's server-first direction supersedes the historical preparation-only allocation. Basic server implementation is locally validated; awaiting control-plane integration, review and CI. The implementation is preserved in local WIP head `ce68889e5`; this continuation performs no staging, commit, publication, deployment or paid review. The broader family remains partial.

A session negotiating capability 39 sends command 38 to inspect/use charges or confirm Aldo's repair. Existing capability-15 USE also consumes carried charges. Equip starts the owner clock, unequip pauses it, and expiry needs no client packet. Repair preserves the instance and commits payment, restoration and audit together.

Writes prove current session, lease, scope, root and active content. Receipts share the value transaction; ambiguous requests retain command/elapsed identity until reconciliation. Checkpoints follow the accepted 60-second cadence, with one-second expiry scans and forced saves before unequip/actor end/death. Restart resumes the stored budget; downtime consumes none.

## Server wiring gap list

| Layer | Existing on starting HEAD | This change |
|---|---|---|
| L3 | Native timed lanes and Soft Boots repair planner | Reused directly by runtime; no duplicate rules |
| L4 | Fenced checkpoint/expiry and repair planning adapter | Atomic consumption, fee/repair execution, command receipts and equipment expiry custody |
| Schema | 0054 timed rows, 0058 expiry guards | 0179 receipt table and narrow repair/equipment proofs; prior fee/transform guards preserved |
| Protocol | Item move/USE and NPC commands | Bounded request/result codec; append-only capability 39 and command 38 |
| Offer/content | No native executable timed profiles | Four admitted Item profiles (3081/3549/6529/6530); conditional bundle offer; Aldo placement from existing agreeing source candidates |
| Runtime | No timed writer consumers | Dedicated handler, ordinary USE, equip stop/start, owner timer and actor-end hooks |
| Qualification | Rules/storage/planning tests | Real framed admitted dispatch PG scenario: durable rows, results, rollback, replay, reload, authority refusals and concurrent revision CAS |

## Reference parity table

Normalized OTS evidence; no official Tibia parity qualification. Accepted decisions: TIMED_ITEM0 (2026-10-01), TIMED_ITEM0B (2026-10-03), BANK-FEE-0. Prior foundations: #1681, #2155.

| Visible behavior | Reference evidence | Oteryn now |
|---|---|---|
| Five charges, final exhaustion | [Pinned Canary items.xml](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/items/items.xml), Item 3081 | Inspect/use returns remaining charges; fifth use durably burns the item. Automatic protection-on-hit is unfinished. |
| Four-hour Soft Boots; spent form 6530 | Same pinned items.xml, 3549/6529/6530; accepted TIMED_ITEM0B | Equipped budget advances, bag budget pauses, owner expires to 6530. Active/inactive definition swaps on equip/unequip are unfinished. |
| Repair costs 10,000 and restores Soft Boots | [Pinned Aldo Lua](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/npc/aldo.lua); accepted repair decision | Explicit repair action near live Aldo; direct main-bag B3 selection; same-instance 6530→6529, full budget and coin/bank fee in one transaction. Dialogue repair/yes is unfinished. |
| Aldo exists in the World | Committed promotion-candidates-v1.json, agreeing Canary/Crystal cell (32952,32110,6) | Added to placement scope and regenerated; actual compiler admits Aldo (24 written/28 candidates; four pre-existing held NPCs unchanged). |
| Coins first, bank shortfall only under V2/non-junior rules | Accepted BANK-FEE-0 and native fee/planner | Reuses native bank fee path; coin repair proven in PG, bank split/maxima in planning/audit tests. Dedicated repair-bank PG scenario remains. |

## Validation evidence

Cargo uses the queued wrapper, repository toolchain and `--locked`. Initial login-shell PG attempts skipped without routing and are excluded. Routed tests create private databases; only this mechanic's filtered PG tests ran.

```bash
cargo test --locked -p oteryn-protocol-oteryn --quiet
cargo test --locked -p oteryn-gameplay-domain --lib timed --quiet
cargo test --locked -p oteryn-gameplay-durability --lib item_timed --quiet
cargo test --locked -p oteryn-gameplay-durability --lib item_fee_burn --quiet
cargo test --locked -p oteryn-gameplay-durability --lib writer_registration --quiet
cargo test --locked -p oteryn-gameplay-content --lib native_gameplay --quiet
cargo test --locked -p oteryn-gameplay-dispatch --lib capabilities --quiet
cargo test --locked -p oteryn-gameplay-dispatch --lib timed_item_postgres_wire_charges --quiet
cargo test --locked -p oteryn-game-server --test item_timed_postgres item_timed_postgres_cases --quiet
cargo test --locked -p oteryn-game-server --test item_timed_state_postgres timed_ --quiet
cargo clippy --locked -p oteryn-gameplay-dispatch -p oteryn-gameplay-durability -p oteryn-gameplay-content -p oteryn-protocol-oteryn -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo fmt --all -- --check
```

Protocol: 388 passed. Domain: 22 passed. Durability: 9 timed, 20 fee/audit and 2 writer-registration tests passed. Content/native: 45 passed, one pre-existing ignored test. Capability tests: 63 passed. Existing repair planning: 13 passed; filtered timed-state PG: 23 passed (27.43 seconds). New routed PG scenario: one passed (final behavioral run 17.74 seconds). All five touched crates passed all-target Clippy; existing vendor deprecation/documentation warnings are outside these crates.

PG scenario name: `gameplay_transport::monk_save::postgres_tests::timed_item::timed_item_postgres_wire_charges_replay_repair_and_owner_expiry`. Auto-mounted in dispatch `--lib` CI. Covers wire results, capability/stale refusals, exhaustion, restart, repair rollback/replay, USE unknown/retry/replay, retained-burn reconciliation, bag pause/equipped expiry, authority/content fences and concurrent charge CAS.

Also passed: rustfmt for include-mounted files; `python3 tools/content-migration/register_spell_families.py --check`; spell-import tests (9); NPC converter `--check` and converter tests (6); actual world compiler `parity .` (no tile mismatches; Aldo admitted); governance; repository policy; agent tests (129); `git diff --check`. Reviewed the full diff against the owner CODEX-CHECKLIST.md; searched apps/, crates/, tools/ for all new fields/variants/trait methods and their matches/constructors/impls. Public APIs have production callers and real-dispatch coverage; tests remain intact.

## Registry collision repair (2026-10-23)

The unmerged timed-item allocation now uses command **38** and capability **39**.
Registry fragment filenames, IDs, command/capability links, protocol constants,
registered capability IDs, proto and this card agree. Capability tables, dispatch
selection and World offer/boot validation already consume the constants. The two
PG fixture selections now use `CAPABILITY_TIMED_ITEM_V1` instead of numeric 37.
The existing capability test exposed an incomplete registry `offer_gate`; its
text now names the actual `BUNDLE_WORLD_OFFERED_CAPABILITIES` gate. No assertion
was weakened and no previously merged protocol allocation was changed.

`git fetch -q origin main` failed because the worktree Git metadata, including
`FETCH_HEAD`, is mounted read-only outside this directory. Read-only GitHub API
main readback matched local `origin/main` exactly:
`c49d3b6f7fae1750d9f89cae9a915e21a4b4a3ee`. Targeted `git ls-tree` and `git show`
confirmed DEPOT owns command 36/capability 37. Both sources confirmed command 38,
capability 39, resource entries 0700–0706 and migration 0179 are unused on main (renumbered from 0158 after 0176 claimed fee cause 4; repair is cause 5).
No merge was performed.

Repair validation, through `/home/mole/oteryn-cache/codex-bin/cargo` with
`CARGO_NET_OFFLINE=true` and `--locked`:

- `test -p oteryn-protocol-oteryn --quiet`: **388 passed** after the registry repair.
- `test -p oteryn-gameplay-dispatch --lib capabilities --quiet`: **63 passed**.
- `test -p oteryn-gameplay-dispatch --lib timed_item_postgres_wire_charges --quiet -- --nocapture`:
  **1 passed**, real PG, 22.87 seconds; no missing-route message or early return.
- `clippy -p oteryn-protocol-oteryn -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings`:
  **PASS**; existing warnings are confined to vendored dependencies.
- `fmt --all --check`: **PASS**.
- Governance and repository-policy validators: **PASS**; governance tests: **129 passed**.
- `python3 tools/content-migration/register_spell_families.py --check`: **PASS**, 36 files.

The WIP `content/content.lock.json`, manifest and Item/NPC inputs require a World
bundle repin; the old pin would fail CI. The inputs are already committed in the
WIP index, so no staging is necessary for this repair. Ran the repository tool
`cargo run --locked --quiet -p oteryn-world-bundle-compiler -- pin-refresh .`:
it regenerated `content/world/pins/oteryn.json` and
`deploy/synology-game/preprod-topology.toml` to bundle digest
`0b037a2ad2ff44aef4c8bb9fdd590947ff4a3f5114717028de4b6233a9d551dd`.
The identity file did not change.
`cargo run --locked --quiet -p oteryn-world-bundle-compiler -- pin-check . target/item-timed-1-pin-check`
**PASS**: two deterministic builds, 25,089,845 bytes, matching digest/input identity
and accepted entry cell. The repin is complete locally; no additional control-plane
repin is needed for this unchanged content input set.

Whole repair diff reviewed against the owner CODEX-CHECKLIST.md; workspace-wide
reference searches found no stale timed-item allocation. This repair adds no
enum variants, struct fields, trait methods or public functions. No CI routing,
credentials or protection files were changed.

## Unfinished

- Control plane integration, independent review and CI remain required before publication. The repository tool regenerated the bundle pin/topology and `pin-check` passed. No content deployment is claimed.
- Broader parity: equip form swaps, passive effects/combat charges, continuous clocks and ground/house hosts.
- Repair/yes dialogue and a dedicated repair-bank PG case remain; explicit wire confirmation and the native bank fee writer are implemented.
- Client UI excluded. No unrelated PG contention failures. Broader family acceptance remains open.

## Implementation files (WIP head and continuation)

Placement shard 00026→00027 preserves prior records. Migration: 0179; resources: 0700–0706.

```text
apps/game-server/migrations/0179_item_timed_runtime.sql
apps/game-server/tests/support/item_timed_postgres_cases.rs
content/content.lock.json
content/items/definitions/spell-native-profiles.json
content/items/index.json
content/manifest.json
content/spells.manifest.json
content/world/npc-placements/index.json
content/world/npc-placements/placements-00000-00026.json
content/world/npc-placements/placements-00000-00027.json
content/world/pins/oteryn.json
crates/gameplay/content/src/content/native_gameplay.rs
crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs
crates/gameplay/dispatch/src/gameplay_transport/capabilities_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/channel_states.rs
crates/gameplay/dispatch/src/gameplay_transport/connection.rs
crates/gameplay/dispatch/src/gameplay_transport/equipment_move.rs
crates/gameplay/dispatch/src/gameplay_transport/mod.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/timed_item.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/timed_item/authority.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/timed_item/duration.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/timed_item/recovery.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/timed_item/repair.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/timed_item/standard_use.rs
crates/gameplay/dispatch/src/gameplay_transport/source_item_cycle.rs
crates/gameplay/dispatch/src/gameplay_transport/timed_item.rs
crates/gameplay/dispatch/src/gameplay_transport/timed_item_clock.rs
crates/gameplay/dispatch/src/gameplay_transport/timed_item_content.rs
crates/gameplay/dispatch/src/gameplay_transport/timed_item_use.rs
crates/gameplay/dispatch/src/gameplay_transport/timed_item_wire.rs
crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities.rs
crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities_tests.rs
crates/gameplay/durability/src/durability/character_revision_sequencer/writer_registration_tests.rs
crates/gameplay/durability/src/durability/core_modules.rs
crates/gameplay/durability/src/durability/item_fee_burn.rs
crates/gameplay/durability/src/durability/item_fee_burn/purchase_cause.rs
crates/gameplay/durability/src/durability/item_fee_burn_audit.rs
crates/gameplay/durability/src/durability/item_fee_burn_audit/purchase_cause.rs
crates/gameplay/durability/src/durability/item_fee_burn_audit/repair_cause.rs
crates/gameplay/durability/src/durability/item_fee_burn_audit/tests/timed_repair_tests.rs
crates/gameplay/durability/src/durability/item_timed_runtime.rs
crates/gameplay/durability/src/durability/item_timed_runtime/execute.rs
crates/gameplay/durability/src/durability/item_timed_runtime/repair.rs
crates/gameplay/durability/src/durability/item_timed_state.rs
crates/gameplay/durability/src/durability/item_timed_state_audit.rs
crates/protocol-oteryn/src/capability_tables.rs
crates/protocol-oteryn/src/lib.rs
crates/protocol-oteryn/src/timed_item.rs
deploy/synology-game/preprod-topology.toml
docs/agents/tasks/ITEM-TIMED-1.md
docs/contracts/game-events/v1/native_one_item_transaction.proto
docs/contracts/protocol-oteryn/v1/registry/capabilities/0039-timed-item-v1.json
docs/contracts/protocol-oteryn/v1/registry/command_types/0038-timed-item-intent.json
docs/contracts/protocol-oteryn/v1/timed_item_v1.proto
docs/contracts/resource-limits/entries/0700-ITEMTIMED1-INTENT-BYTES.json
docs/contracts/resource-limits/entries/0701-ITEMTIMED1-RESULT-BYTES.json
docs/contracts/resource-limits/entries/0702-DUR03-RL-04-NPC-REPAIR.json
docs/contracts/resource-limits/entries/0703-DUR03-RL-06-NPC-REPAIR-PARTICIPANTS.json
docs/contracts/resource-limits/entries/0704-DUR03-RL-06-NPC-REPAIR-WORK.json
docs/contracts/resource-limits/entries/0705-ITEMTIMED1-REPAIR-COIN-INPUTS.json
docs/contracts/resource-limits/entries/0706-ITEMTIMED1-COMMANDS-PER-SECOND.json
tools/content-migration/register_spell_families.py
tools/content-migration/timed_item_profiles.py
tools/content-schema/npc-authoring/npc_placement_scope.json
```

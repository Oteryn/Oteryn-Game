# MINING-1 — mining and map-resource gathering (task card)

```yaml
task_id: MINING-1
title: "MINING-1: mining and map-resource gathering"
mode: IMPLEMENT
status: waiting   # server allocation received; gathering-yield rule unresolved
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
needs_registry: true   # resource-limit ranges allocated at lane start
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Use a carried pick on an admitted stone or ore map resource, obtain rule-defined drops, and prevent reuse until its cooldown/respawn permits it. Reuse native item-use, map ownership and mint authority; gathering yields are separate from CRAFT-DIG-1 tile opening.

## Server wiring gap list

Read-only audit on 2026-10-23 at `151c4bb8372cea1672e4106afd6b1d0b8253445c`.
The owner's current packet allocates server implementation, migration **0167** if
needed, resource-limit entries **0780–0786**, and command types **54–55** /
capability **48** only if the existing USE wire cannot serve the mechanic.
Client UI and publication are outside this worker's scope.

| Layer | Exists on this HEAD | Missing for MINING-1 |
| --- | --- | --- |
| L3 rules | `items/src/reach.rs` provides same-floor Chebyshev-one reach; content digging validation preserves hole return timers. | No `items/src/mining/`, admitted ore/stone yield table, depletion rule or occurrence-bound draw. |
| L4 writer | `durability/item_use.rs` provides carried-item fencing and command receipts; `map_item_mint.rs` provides scope/reset-bound map MINT and reconciliation. | No mining writer/receipt/node state. Existing map pickup uses separate MINT and TRANSFER transactions, so it cannot supply the required atomic backpack reward plus depletion unchanged. |
| Schema | Existing item lifecycle and map MINT migrations; no mining migration/table found. | Migration 0167, if needed, must admit the mining cause, receipts, node transition and runtime grants together. No migration authored while the yield rule is unresolved. |
| Protocol | `protocol-oteryn/src/world_object.rs`: command 2 USE, carried handle or definition plus `MapUseTarget::Object` / `Item`; bounded codec and result tests already exist. Registry fragments register capabilities 11 and 15. | No new command/codec appears necessary. Mining must use the existing negotiated USE arms; capabilities 11 and 15 are currently absent from the bundle offer. |
| Dispatch | `gameplay_transport/item_use.rs`, `loop_map_pickup.rs`, and `connection.rs` provide working reference paths. | No mining handler; `connection.rs` currently rejects decoded `use_with_map_item`. Resolve live tool/target, reach, scope, item fence and capacity before the atomic writer. |
| Offer / runtime | `capabilities.rs` offers bundle item views and map views; game-server `node/serve.rs` composes existing world owners. | No mining catalogue/node admission or runtime caller. Offering the shared interaction capability also needs its existing contract gates honored. |
| PG E2E | Dispatch `postgres_tests/README.md` describes auto-mounted mechanic cases; BAGS-USE has a real `serve_admitted` byte exchange. | No mining case. Add one filtered auto-mounted case covering reward/result, contention, refusal, replay and durable reload once rules are accepted. |

**Blocking product input:** CRAFT1 C5 and WORLD-INTERACTION-0 §6 explicitly
exclude gathering yields. The Reference audit below does not establish a generic
ore/stone mining table. The owner was asked to supply another Reference source or
accept explicit Oteryn node/output, quantity/chance and respawn rules. No yield,
cooldown or mint authority is inferred from the implementation allocation.

## Dependencies

CRAFT1 C5 explicitly excludes digging yields; accept the gathering-yield contract before implementation. Confirm ITEM-USE-1c, WORLDINT-USE-1, MAP-OVERLAY ownership/reset, native MINT and deterministic RNG seams at lane start.

This is a waiting plan, not a runtime allocation or a new accepted contract. Live
Issue/PR state and owning contracts must be checked when the lane starts.

## owned_paths (proposed; confirm at lane start)

- `crates/gameplay/items/src/mining/` — pure rules and separate child test files.
- `crates/gameplay/durability/src/durability/mining/` — own storage submodule, codecs, receipts and separate tests.
- `crates/gameplay/dispatch/src/gameplay_transport/mining/` — separate later wiring slice and dispatch tests.
- `apps/game-server/src/lifted_tests/auto/mining_postgres_cases.rs` — auto-mounted PG cases; no shared test mount edits.
- `docs/agents/tasks/MINING-1.md` — this task record.

Migration numbers and resource-limit ranges: **allocated at lane start**; none
reserved here. Exact migration, content input and protocol paths require an explicit
lane allocation; shared `mod.rs`/`lib.rs` changes are registration only. No workflow,
repository-policy or Soul War paths belong to this plan.

## Scope

Validated tool/resource matching, reach and ownership checks; admitted drop tables; cooldown/depletion and respawn state; atomic reward mint plus receipt and resource transition. Bind resource identity to WorldId and ChannelId separately, with the map owner resolving the stable resource key. Decide persistence and reset semantics before storage authoring; never silently change CRAFT1 volatile digging.

## Excluded scope

Ordinary hole opening already owned by CRAFT-DIG-1; terrain editing, new mining skills, speculative crafting economy, client redesign, Soul War files and reference asset redistribution.

## Target layout and slices (ADR-0022)

1. Pure rules in the existing L3 `oteryn-gameplay-items` rules module
   `items/src/mining/`. Keep policy, plans and arithmetic free of SQL and transport.
2. Durable state in `crates/gameplay/durability/src/durability/mining/`, as its
   own submodule. Split transactions, decoding and tests into small child files.
   Reuse recovery admission, session-generation fencing, revision sequencing and
   native value receipts. Schema/grants and their writer are inseparable.
3. Dispatch wiring is a separate later slice in `gameplay_transport/mining/`.
   Every added public function needs a production caller and a named end-to-end
   test through real `protocol-oteryn` dispatch before mechanic completion.

No giant mechanic files, speculative framework or new dependency fork. Keep
WorldId and ChannelId distinct and retain Character session-generation fences.

## Reference parity

Canary (`opentibiabr/canary`) source paths to consult as evidence:

- `data/scripts/actions/tools/pick.lua`
- `data-otservbr-global/startup/tables/tile_pick.lua`

TibiaWiki page names to consult: **Pick, Mining, Iron Ore**.

### Reference parity table — 2026-10-23 audit

Canary source revision: `52550d5e3c0ffe565f4bd1014f4e5575abf1059b`.
Evidence is limited to the named source paths and the delegated implementation;
this is not a claim that every quest script in Canary was audited.

| Visible behavior | Reference evidence | Oteryn now |
| --- | --- | --- |
| A normal pick invokes tool behavior without an ordinary tool burn. | [pick.lua](https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/data/scripts/actions/tools/pick.lua) registers item 3456 and delegates to `onUsePick`; the delegated handler does not consume the pick. | Carried USE codec exists; mining tool qualification is absent. |
| Pickable ground opens a hole rather than yielding ore. | [tile_pick.lua](https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/data-otservbr-global/startup/tables/tile_pick.lua) assigns action 101; [register_actions.lua, `onUsePick`](https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/data-otservbr-global/scripts/lib/register_actions.lua) transforms eligible ground and starts decay. | CRAFT1 C5 owns volatile hole opening and explicitly excludes gathering yields. Its timer cannot be reused as a mining respawn rule. |
| One positioned stone disappears for 20 seconds, with no reward. | The same `onUsePick` removes stone 1772 at `(32648,32134,10)` and recreates it after 20,000 ms; no item is awarded by that branch. | No mining node. The obstacle timer does not establish an ore yield. |
| Quest crystals and liquid silver have specific rewards and timers. | `onUsePick` has Secret Library crystal selection with a per-player 120-second timer; Grimvale silver has 40% success, a per-player 120-second attempt timer and a 600-second vein return after success. | These are distinct quest-specific rules, not an admitted generic iron/stone table. |
| A shiny stone is consumed to produce coins or a diamond. | `onUsePick`, target 10310: 1% crystal coin, 5% gold coin, 45% platinum coin, 49% small diamond; the target is removed. | This conversion does not satisfy renewable ore/stone mining. |
| Ordinary ore acquisition is not established as pick mining. | [TibiaWiki Pick](https://tibia.fandom.com/wiki/Pick) describes hole/stone/ice use; [Iron Ore](https://tibia.fandom.com/wiki/Iron_Ore) lists creature drops and quest acquisition. No generic table was established; no authoritative Mining page was found in this bounded lookup. | Generic yield quantities/chances and respawn duration remain **UNKNOWN**. Custom-server mining pages are not substituted as Tibia parity. |
| Depletion and receipts survive restart. | The inspected timed map changes use runtime decay/events, not proof of durable shared mining receipts. | Durable restart behavior is required by the current owner packet, a deliberate difference to record in the accepted gathering rule. |

Still required: accept eligible node/output definitions, quantities/probabilities
and respawn timing, and record the deliberate durable reset/restart semantics.
If upstream has no matching rule, obtain an accepted Oteryn rule instead of calling
invented drops parity.

Evidence is not runtime authority. Record pinned source revisions and any deliberate
difference in the implementation packet before content admission.

## Acceptance criteria

- [ ] A second character racing the same node cannot obtain another drop for the same depletion occurrence. Wrong tool, out-of-reach, stale scope/session generation and unavailable inventory refuse without consuming or minting. Exact retries and uncertain commits reconcile the retained original receipt; respawn cannot replay an old reward.
- [ ] Durable state and retained receipts survive restart, rollback and reconnect.
- [ ] Bounds and lock order are enforced in the writer; concurrent losers are typed.
- [ ] Properties cover drop conservation, checked quantity/time arithmetic, deterministic occurrence-bound draws and no double harvest per occurrence.
- [ ] Rules, storage and dispatch stay separate; tests live in child files.
- [ ] Production callers and real-dispatch end-to-end tests cover all new public APIs.

## Validation

### Audit checkpoint evidence — 2026-10-23

- `python3 tools/agents/validate_governance.py` — PASS (22 policy documents,
  9 project lanes).
- `python3 tools/repository/validate_repository_policy.py` — PASS (23 files,
  68 workflows, exact-candidate routing regressions).
- `python3 -m unittest discover -s tools/agents/tests` — PASS, 129 tests.
- `git diff --check` — PASS.
- Whole task-card diff reviewed against the owner's
  `~/oteryn-cache/CODEX-CHECKLIST.md`. Only the task card changed; no new enum
  variants, struct fields, trait methods or public functions require site updates.
- Cargo filters covering this change: **none** (task-card-only audit). No Rust
  compilation, clippy, rustfmt or PostgreSQL qualification is claimed. The
  commands below remain planned implementation checks, not executed evidence.

Planned commands after implementation; these filters must name the actual new tests
and produce nonzero test counts. Run only this mechanic's PostgreSQL cases. The
wrapper supplies `OTERYN_TEST_POSTGRES_ADMIN_URL`; absent routing returns `Ok(())`
without claiming PG qualification. GitHub CI runs the complete registered suites.

```sh
cargo fmt --all --check
cargo clippy --locked -p oteryn-gameplay-items -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo test --locked -p oteryn-gameplay-items --lib mining::
cargo test --locked -p oteryn-gameplay-items --lib mining::property_
cargo test --locked -p oteryn-gameplay-durability --lib mining::
cargo test --locked -p oteryn-gameplay-dispatch --lib mining::
cargo test --locked -p oteryn-game-server --lib lifted_tests_auto::mining_postgres_cases::
python3 tools/agents/validate_governance.py
git diff --check
```

PG cases must exercise the installed migrator/runtime role, commit/reload,
rollback, replay/conflict, stale fences, scope isolation and concurrent writers.
Add mechanic-specific property tests and an end-to-end dispatch case; include
restart/unknown-commit reconciliation rather than storage-only success claims.

## Unfinished

- Required owner input: identify the Reference mining table or accept an explicit
  Oteryn-specific node/output, quantity/chance and respawn rule. The allocation
  supplies durability requirements but not those product values.
- L3 mining rules, L4 atomic writer/schema, USE dispatch, content/runtime offer
  and real-dispatch PG E2E remain unimplemented pending that input. No backend-only
  slice was added and a player gains no mining behavior from this audit.
- No commit, publication, client edit, migration or registry allocation was made.

## Risks

Shared resource versus Character lock ordering, cross-channel duplication, reset races and conflating volatile digging with durable gathering rewards.

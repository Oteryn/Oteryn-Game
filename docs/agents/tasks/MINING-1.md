# MINING-1 — mining and map-resource gathering (task card)

```yaml
task_id: MINING-1
title: "MINING-1: mining and map-resource gathering"
mode: IMPLEMENT
status: waiting   # contract verification and lane allocation pending
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 CRAFT-1 adjacent scope; gathering yields require their own decision"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_CRAFT1_DECISION_2026-10-15.md
class: "hard, persistence and economy review"
base_branch: main
branch: null
pr: null
needs_migration: true   # allocated at lane start
needs_registry: true   # resource-limit ranges allocated at lane start
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Use a carried pick on an admitted stone or ore map resource, obtain rule-defined drops, and prevent reuse until its cooldown/respawn permits it. Reuse native item-use, map ownership and mint authority; gathering yields are separate from CRAFT-DIG-1 tile opening.

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

TODO: which tiles actually support gathering rather than opening; tool consumption; eligible ores, quantities and probabilities; cooldown/respawn duration; reset/restart behaviour. If upstream has no mining-yield rule, record that gap and obtain an accepted rule instead of calling invented drops parity.

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

## Risks

Shared resource versus Character lock ordering, cross-channel duplication, reset races and conflating volatile digging with durable gathering rewards.

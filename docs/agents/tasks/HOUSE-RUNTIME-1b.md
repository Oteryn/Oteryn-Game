# HOUSE-RUNTIME-1b — house scope runtime and entry (consolidated task card)

```yaml
task_id: HOUSE-RUNTIME-1b
title: "HOUSE-RUNTIME-1b: house scope runtime and entry"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
class: "hard (session-generation fencing, scope handoff, runtime authority)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row HOUSE-RUNTIME-1b; §7 'HOUSE-RUNTIME-1a/1b/1c'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md   # §0.2, §1.5, §2.2
  - docs/architecture/reviews/OTERYN_GAME_ACCEPT_BATCH_2026-10-15_ACCEPTANCE_DECISION.md  # ADMIT0/BED0 accepted; ADMIT0 §3.2 gate stays
base_branch: main
branch: null
pr: null
needs_migration: false
needs_registry: true   # RL-04 resource limit once measured
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2165, branch skeleton/house-runtime-1b

## Goal

Houses run as their own scope: load the HOUSE-RUNTIME-1a interior, enter and leave through the
native scope handoff, and refuse `HOUSE_CLOSED` until the exit gate (SCOPE-HANDOFF transfer
contract, ADMIT0 §3.2) lifts. Measure HOUSERT0-RL-02 and RL-04. Second of
HOUSE-RUNTIME-1a → 1b → 1c.

## Dependencies (re-verified on origin/main, 2026-10-22)

| Dependency | State |
|---|---|
| HOUSE-RUNTIME-1a (interior shapes, `load_house_interior_in_transaction`, migration 0095) | MERGED (#2163) |
| SCOPE-HANDOFF-1 (migration 0074, prepare/commit/abort/reconcile, tile reservations) | on main (`durability/house_scope_handoff.rs`) |
| MAP-LOAD-1 | MERGED (archive) |
| ADMIT-CAP-1 | MERGED (#2187) |
| NPC-ECON-1 | MERGED (#2197) |
| Exit gate: architect D573 / SCOPE-HANDOFF-2 transfer contract + ADMIT0 §3.2 (capability selection, scope ack, lost-ack recovery, no snapshot before ack) | OPEN — D1063 forbids merge before the transfer contract |
| SESSION-INGRESS-1 | OPEN — precedes the House consumer in the shared order |
| MAP-OVERLAY-1c (spatial encoder, qualified room source, House catalogue) | BLOCKING — re-implementation pending (draft #2167 closing) |
| HOUSE-RUNTIME-1c (last-position house columns, exit placement) | follows this task — re-implementation pending (draft #2166 closing) |
| HOUSE-1a / HOUSE-ACL-1 / HOUSE-1b (composed `game_house_access` body / grant / fence parts) | BLOCKING for real access — re-implementation pending (drafts #2137, #2140, #2141 closing) |

Shared order recorded by CP: `NPC-ECON-1 (#2197) → #2198 → ADMIT-CAP-1 (#2187) → SESSION-INGRESS-1 → House consumer`.
Check the exact post-merge preimages and the next free lifecycle tag at the slot.

## Owned paths (ADR-0022 layout)

- `crates/gameplay/world/src/house_runtime.rs` (new, L4 runtime: private HouseRuntimeOwner lifecycle, bounded node-local HouseId→owner directory, closed-gate refusal)
- `crates/gameplay/world/src/house_runtime_tests.rs` (new)
- `crates/gameplay/world/src/house_runtime_loading.rs` (new, activation adapter over pinned WorldBase/BundleFacts and the 1a interior read)
- `crates/gameplay/world/src/house_runtime_loading_tests.rs` (new)
- `crates/gameplay/world/src/house_scope/actor_placement.rs` (new, D1373 actor producer: reservation, materialization, consumption on the one physical carrier)
- `crates/gameplay/world/src/house_scope/actor_placement_tests.rs` (new)
- `crates/gameplay/world/src/house_scope/mod.rs` (existing; D1373 consumer — wiring-only additions)
- `apps/game-server/tests/house_runtime_postgres.rs` (new, focused thin wrapper, `test = false`)
- `apps/game-server/tests/support/house_runtime_postgres_cases.rs` (new, shared case leaf)
- `docs/agents/tasks/HOUSE-RUNTIME-1b.md`

Slot-only (final commit at the queue turn): module registration in
`crates/gameplay/world/src/lib.rs`; the D1421 mount in `apps/game-server/tests/durability_postgres.rs`
(one support mount, routing constant, existing HouseScope module mount; preserve the full union);
the `house_runtime_postgres` entry in `apps/game-server/Cargo.toml`;
`docs/contracts/RESOURCE_LIMITS_REGISTRY.json` (RL-04 once measured); dispatch wiring in
`crates/gameplay/dispatch/src/gameplay_transport/{connection.rs,mod.rs}`. No migration.

## Wiring points (verified on origin/main)

- `crates/gameplay/durability/src/durability/house_scope_handoff.rs` (`assign_house_scope`, `prepare_house_entry`, `commit_house_entry`, `release_house_entry_tile`, `*_in_transaction` variants)
- `crates/gameplay/durability/src/durability/house_interior.rs` (`load_house_interior_in_transaction`)
- `crates/gameplay/housing/src/house_scope/handoff.rs` (`select_entry_tile`), `crates/gameplay/housing/src/house_scope_handoff.rs`
- `crates/gameplay/world/src/house_scope/mod.rs` (`exit_into_channel` → `ExitNotAdmitted`)
- `crates/gameplay/world/src/world_runtime.rs` (scope registration; `ScopeContentGenerationFence`)
- `crates/gameplay/world/src/map/boot.rs`, `map/facts.rs`, `crates/gameplay/content/src/content/native_cell_lookup.rs`
- `crates/gameplay/content/src/content/project/native_house_tiles.rs` (`QualifiedHouseTiles` / `QualifiedHousePlacement`)
- `crates/gameplay/durability/src/durability/house_spell_acl.rs` (not a substitute for `game_house_access`)
- `crates/gameplay/durability/src/durability/character_last_position.rs`, `crates/gameplay/dispatch/src/gameplay_transport/last_position.rs` (1c hooks)
- Prototype only: `apps/game-server/examples/runtime_actor_carrier_prototype.rs`

## Accepted decisions carried from the draft

- **D1062–D1065 (closed boundary):** the runtime boundary takes the native `HouseEntryRequest` by
  reference and returns `HousePrepareOutcome::Refused(HouseClosed)` before load, tile reservation,
  source freeze or durable PREPARE; saved-house login activation also refuses `HouseClosed`. Exit
  delegates to the real `exit_into_channel`, so leave, kick, revalidation and quiesce keep
  `ExitNotAdmitted`. No open flag, no fabricated Ready state, actor or session store.
- **D1062 capacity wording:** native prepare counts live/reconnectable House sessions **plus**
  PREPARED handoffs against 200; do not add a weaker local count (D1064 wording is a CP item).
- **D1362 lifecycle:** additive House lifecycle shape, sealed claim predicate, exact complete
  predecessor history, same-transaction preparation/persistence and a durable successor; binds the
  initial receipt, the exact native session-use membership and one ledger advance; historical tag-3
  result requires Released. Scope-generation comparisons order only within the same scope identity.
  The Platform observation keeps its strict five-second age without refresh.
- **D1373 actor producer/consumer:** one physical carrier, a private Channel/House domain and real
  HousePlayer slots. Reservation, materialization, consumption and uncertain commit keep the same
  slot and tile. Source state comes from the real Channel Slot (position revision, lifecycle,
  combat); no default player state or synthetic Channel scope. The stage consumes the PREPARED
  object in its own SQL transaction, checks source runtime/session/lease/pin, locked tile, single
  qualified House identity/digest and current 0047 ownership, has **no COMMIT or bind API**, and
  cannot activate a player with partial state.
- **D1421 test registration:** focused target has a literal support include and `test = false`;
  the canonical aggregate `durability_postgres` mounts the support leaf once. Routing: focused
  target with absent URL keeps the strict refusal; the aggregate with an entirely absent URL emits
  PRE-ROUTING/NONCANONICAL and returns before runtime/DB construction; a present empty, non-Unicode
  or unsafe URL fails the validator in both. (PG-COVERAGE-1 failed when the target had inline tests
  and no `support/*_cases.rs`.)
- **Fixture cleanup:** the PG fixture keeps the admin connection outside the caught case, always
  attempts its own-database DROP and close, returns the original error/panic after cleanup, and
  emits QUALIFIED only when case and cleanup both succeed.

## Native source findings (constraints for the implementation)

1. `HouseId` is World-global and revision-free (WorldId + qualified house key); `scope_key` tag 02,
   `instance_id()` is the deterministic runtime identity. A generic `RuntimeScopeRefV1::Instance`,
   generation integer or caller UUID is not a House or assignment proof. Origin Channel is routing
   evidence, never a selectable home Channel.
2. Do not duplicate SCOPE-HANDOFF-1's journal, idempotency, session writer or state machine.
   Commit re-reads `game_house_access`; the destination gets a fresh GameSessionId and lease.
3. Native prepare requires one node to hold both source Channel and destination House (another-node
   House → Busy). Cross-node placement needs a qualified custody seam; keep the same-node restriction.
4. `assign_house_scope` is a control-role command; runtime may request assignment but never mint
   ControlActor/grant or write assignment tables.
5. `ChannelRuntimeV1` and `ComposedFreshAdmission` are Channel-bound; do not instantiate a fake
   Channel for a House, copy the carrier or add a second actor/session store or pathfinder.
   `ScopeContentGenerationFence` cannot be synthesized from a HouseId.
6. BundleFacts records only generic house-tile positions, not HouseId→tiles/entrance. `entry_cell`
   is the outside entrance, not an interior arrival tile. Missing metadata is never an empty house.
7. `ChannelSpellStates` stores PlayerSpellState outside the carrier Slot with no take/rollback/
   House-adoption port, and its initializer would prune or default state. CP must allocate a native
   transfer owner (vitals, cooldowns, conditions, regeneration, pending custody) before commit.
8. `house_spell_acl::house_access_for_character` is not `game_house_access`; HOUSE-1a owns the
   composed body, HOUSE-ACL-1 the grant part, HOUSE-1b the fence part. Lock order property → fence → grant.
9. Shared hunks requested from CP (seven): single native HouseId move/re-exports, one validated NPC
   field access, private PREPARED views and sealed bridge, two finite qualified-House read accessors.

## Ordered flow once the gate lifts (for the final owner)

1. Check the exit gate first; closed → `HOUSE_CLOSED`, source actor unchanged. Freeze the source only
   as one serialized native actor command; never hold source and destination mutexes together.
2. Coalesce first activation per HouseId/generation; same-World fallback only; revalidate property/
   content before Ready; load failure never publishes readiness.
3. Reserve the door-adjacent tile or same-house radius-3 fallback via `select_entry_tile` + the real
   handoff reservation; revalidate tile, restrictions, fence, ACL and guild revisions at commit.
4. One ordered transaction: PREPARED read, tile reservation, materialization, consumption, claims,
   bind, generation advance. Unknown outcome keeps the handoff ID and reconciles before releasing
   anything; release the tile only with the persisted placement proof.
5. Door exit, leave, kick, revoked grant, disposition and shutdown use the reverse handoff once
   qualified, to the recorded origin Channel first; blocked entrance uses last-position radius3/step4
   fallback (1c writes the destination tile and clears house columns in the same commit).
6. ACL/guild revision changes revalidate every resident; kick policy OWNER any / SUBOWNER guests / GUEST none.
7. Content fence/disposition → Closing: block new prepares, drain residents and pending handoffs,
   then report quiescent to HOUSE-1b. VACANT alone never authorizes entry.
8. Idle unload after five minutes with zero residents and no pending installs; durable items survive.

## Measurements and failure matrix required before activation

- RL-01 five-minute idle; RL-03 200 residents incl. PREPARED; item bound 2000 (HOUSEOWN0-RL-14).
- RL-02: end-to-end door transition **p99 ≤ 500 ms**, cold vs warm reported separately, including
  activation, interior I/O, reservation, prepare/commit, binding, scope ack and usable snapshot.
- RL-04: unset until measured; derive max active houses per node from recorded hardware, build,
  content, PostgreSQL config and repeated runs (empty/typical/200 residents, 199/200/201, cold
  activation, churn, assignment replacement, closing). Register it and enforce typed overload.
- Failure matrix: closed gate/missing grant → typed refusal with no side effect; concurrent
  activation → one owner; ACL/fence changes between prepare and commit → current locks decide;
  capacity race → 201st NoRoom; lost response → same operation ID, no duplicate actor; restart with
  PREPARED/COMMITTED → reconcile before either actor resumes; retired owner callbacks cannot mutate.

## Draft result

Closed-boundary component: 4 pure + 1 PG 17.6 case PASS on an earlier source; the D1362/D1373 actor
stage was source-reviewed but not natively qualified (first compile hit three fixture-only E0277
conversions, later fixed). No runtime, actor transfer, restart or RL-02/RL-04 qualification exists.

## Acceptance criteria

- [ ] Entering a house loads the 1a interior; leaving returns via scope handoff.
- [ ] Entries (door and saved-house login) refuse `HOUSE_CLOSED` until the exit gate lifts.
- [ ] HOUSERT0-RL-02 and RL-04 measured and recorded in evidence.
- [ ] Restart leaves no Character stranded in a house scope.
- [ ] PG case registration follows D1421 (focused strict target + one aggregate mount).
- [ ] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] One mechanic per module file in its ADR-0022 layer.
- [ ] New tests in their own child files.
- [ ] Edits to mod.rs/lib.rs/connection.rs/capabilities.rs are wiring-only.
- [ ] No file over ~1000 lines grows by more than ~150 lines.

## Validation commands

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-world house_runtime
cargo test -p oteryn-gameplay-world house_scope
cargo test -p oteryn-gameplay-durability house_scope_handoff
cargo test -p oteryn-game-server --test durability_postgres house_runtime   # needs DATABASE_URL
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
```

## Reference evidence

- docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
- docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md §1.5, §2.2
- docs/agents/tasks/HOUSE-RUNTIME-1a.md
- Draft #2165 (CP decisions D1011, D1062–D1065, D1122, D1329, D1362, D1373, D1421 on PR #2131; evidence file `docs/agents/evidence/HOUSE-RUNTIME-1b-D1362-D1373-SOURCE-CHECKPOINT-2026-10-17.md` exists only on the draft branch (not on main))

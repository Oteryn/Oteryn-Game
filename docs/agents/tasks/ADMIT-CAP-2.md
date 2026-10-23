# ADMIT-CAP-2 — player field effects behind world-required capabilities (consolidated task card)

```yaml
task_id: ADMIT-CAP-2
title: "ADMIT-CAP-2: player field effects behind world-required capabilities"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row ADMIT-CAP-2; §7 'ADMIT-CAP-1, ADMIT-CAP-2' (ADMIT0 §6)"
brief: "docs/architecture/reviews/OTERYN_GAME_ADMIT0_WORLD_REQUIRED_CAPABILITIES_DECISION_2026-10-01.md (§3–§7)"
class: "hard, authority and protocol review"
base_branch: main
branch: null
pr: null
needs_migration: false
needs_registry: true   # field refusal outcome / error-code row, owned by the protocol/error CP owner
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. No field-creation guard from this card is retained on main; ADMIT-CAP-1 is a prerequisite, not this implementation. Evidence: `crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities.rs`.

Remaining: Implement the player field/wall guard at the current guarded write boundaries. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2150, branch skeleton/admit-cap-2

## Goal

A positively player-origin field or blocking wall (FIELD-2) is created or applied in a
scope only when that scope's effective world-required capability set (fixed per Channel
ownership generation, ADMIT0 §3–§4) satisfies ADMIT0 §6. The check runs at the existing
guarded new-write boundary, after historical-receipt replay and under the existing
authority locks. A refusal is typed and leaves no durable trace. Monster/system fields
are unaffected.

Single row (ADMIT0 §6 guard on top of ADMIT-CAP-1). No further children.

## Accepted decisions

- **ADMIT0** ACCEPTED (#2209, ACCEPT-BATCH-2026-10-15). §6 requires
  `WORLD_SPATIAL_FIELDS` and `WORLD_INTERACTION_V1` for FIELD-2 player effects and walls.
  Instance scopes have no declared set and no player field/wall activation until their own
  contract. WorldId is not ChannelId. ADMIT0 §7 rejects a durable capability table or a
  stored per-session selection; restart rebuilds the binding from boot facts.
- **D1132/D1140 (CP, #2131 comment 6224930839):** the leaf takes a borrowed
  `&BoundRequiredCapabilities` and must call `matches_current(scope, generation, policy)`
  before reading `effective()`. At that checkpoint `WORLD_SPATIAL_FIELDS` had no ID, so only
  `WORLD_INTERACTION_V1` was required. **Now superseded on main:** CP D1442 registered
  `WORLD_SPATIAL_FIELDS` as capability 30 (requires 6 `WORLD_SPATIAL_ENTITIES`, not offered
  before FIELD-WIRE-1; `crates/protocol-oteryn/src/capability_tables.rs`). The
  re-implementation requires both members; confirm with CP at allocation.
- **D1193/D1194 → D1207/D1208 (CP, #2131 comments 6225592781, 6225633260, 6225826999,
  6225834935):**
  - The check lives in the real guarded spell-item writer, after exact-receipt
    classification. The writer takes a mandatory `SpellFieldOrigin` =
    `Player(&dyn PlayerFieldCapabilityCheck) | Monster | System`; the legacy item wrapper
    and the outer commit wrapper forward it explicitly. No `Option`/default origin; unknown
    player sources are never relabelled Monster/System.
  - `SpellFieldClass { Field, Wall, NotField }` is derived from typed item qualification
    (a positive fact on `QualifiedItemDefinition`), never from lifetime/decay/collision.
    Ordinary MagicField qualification gives Field even for a permanent nonblocking field;
    the typed native Barrier preview matched to the exact qualified item gives Wall;
    ordinary items stay NotField even with decay. Unknown source refuses.
  - Durability must not import a gameplay type: the check goes through a closed
    durability-side port. Main now has
    `crates/gameplay/durability/src/durability/fresh_admission.rs::CurrentCapabilityCheck`
    and `AuthenticatedSessionCapabilities::current_check()`; reuse that seam rather than
    adding a parallel one, if it fits.
  - Scope, generation and `world_policy_revision` come from the current root's validated,
    locked Runtime guard, never from the boot binding or a retained request.
  - The check runs before the first new receipt, operation, cost or audit write.
  - Familiar's empty-operation cost path forwards an explicit origin; tests use a refusing
    implementation, never a production no-op.
- **D1160:** a single registration line per new module.
- **D1122:** draft checkpoints are allowed; they are not qualification.

## Dependencies (re-verified against main)

| Dependency | State |
|---|---|
| ADMIT0 acceptance | MERGED (#2209) |
| ADMIT-CAP-1 (`BoundRequiredCapabilities`, `for_channel_boot`, `matches_current`, fresh admission + V2 recovery gate) | MERGED (#2187) |
| D1442 capability 30 `WORLD_SPATIAL_FIELDS` registered | on main |
| FIELD-1 / FIELD-2 (LOOP-WORLD-1 children) | BLOCKING — `blocked` in `docs/agents/tasks/active/LOOP-WORLD-1.md` |
| FIELD-WIRE-1 (field payloads; offer of capability 30) | BLOCKING for activation (not started) |
| Protocol/error owner: field refusal outcome and registry row | BLOCKING for activation (CP owner) |

The guard and its writer/origin plumbing can be built now; FIELD-2 activation and any offer
change wait for their owners.

## Reference behaviour from the draft (re-implement, do not copy)

### Guard leaf

- Match current scope/generation/policy, then require the members from the boot owner's
  sorted effective set. Internal typed refusals distinguish stale binding from missing
  capability.
- Tests (draft: 4): real boot constructor with an explicit registered-edge offer graph; the
  actual bundle offer missing interaction; independently mismatched World, Channel,
  old/new generation and policy; instance scope; a successor whose effective set differs
  without mutating the predecessor. The test graph never enables the production offer.

### Real field dataflow (call sites the guard must reach)

| Existing boundary | Insertion |
|---|---|
| `ordinary_field_items.rs::append_creations` (joins `create_item` ops to the caster-cost transaction; reads current World field policy) | Once nonempty field creations are classified from qualified content, require the bound set for this exact Channel generation before any new write. Capability does not grant PvP or relax tile policy. |
| `native_world_item_cast.rs::prepare_barrier` (Magic Wall / Wild Growth) | Same check before a new player barrier acquisition, including walls without a damage recipe. Do not classify a wall by `field_condition != None`. |
| `native_combat_cast.rs` preparation dispatch → `apply_spell_items_in_transaction_guarded` | Add capability validity to the existing new-write verdict beside `field_policy_current`; prepare-only validation is insufficient; only positively identified field/wall operations. |
| `world_item_cast.rs::cast_world_items_inner` | Preserve its current refusal of unsupported `tile_item_operation` (Rune V1 lacks an exact source handle); no blanket guard on every conjure. |
| `field_step_ingress.rs::step_with_field_ingress` → `StepIngressRead::validate_current` | For a player-origin field/wall, check the same set before effect/collision publication. Missing capability never lets a player walk through an existing wall. Creature-origin contact keeps its own branch. |
| `spell_periodic.rs::tick_with_standing_read` / `tick_current_creature_field_damage` | The future FIELD-2 player-origin periodic consumer uses the same set; no blanket return around all ticks (would suppress monster damage or consume unrelated timers). |

- Positive Field/Wall applicability covers both MintGround and MintInventory, including the
  optional Ground-overflow route under both overflow settings (lead review finding, fixed).
- Player vs monster/system is established by real provenance: `DurableFieldOrigin`
  (`spell_items_abi.rs`) is player creation history; `DurableCreatureFieldOrigin` is a
  separate typed source validated in `field_step_ingress.rs`. Unknown, malformed or stale
  player origin never falls through as system permission. Real monster work keeps its
  distinct source-scope writer.

### Replay and no-trace refusal

- `apply_spell_items_in_transaction_guarded` returns `AlreadyCommitted` for an exact
  historical receipt **before** `new_write_verdict`. The guard must not turn a committed
  unknown outcome into "nothing committed", and historical success is not permission to
  reinstall in a new generation.
- A refused new attempt leaves no receipt, item/ground/field line, cost/charge, training,
  audit/event, HP/condition/position/cooldown/presentation change, and releases
  proven-uncommitted reservations through the callers' existing definite-rollback path. No
  failure receipt.
- No new lock and no inverse lock order: spell lane → Channel/state → SQL authority/ground.
  The set is immutable per generation; existing current-scope locks stop a replaced
  generation from committing.
- Replacement/reconnect: a new list needs a new world-policy revision and ownership
  generation; transfers into a Channel stay refused until ADMIT0 §3.2's lifting conditions.
  The old `required_capability()` check in `connection.rs` before `authority.admit` is not
  the accepted predicate.

## Cross-domain findings (REPORT_ONLY)

- `spell_field_policy.rs::read_world_field_policy_in_transaction` gives World mode,
  protection level and in-fight duration, not capability closure; `ready = true` proves
  nothing about capabilities. `AdmissionAuthorityGuardStateV1::Runtime` carries generation
  and revision, not a capability set; a connection's `SelectedCapabilities` is a session
  selection, not the generation's required set.
- The draft found real PG roots (`character_authority_postgres.rs`, `bank_postgres.rs`,
  `account_achievement_postgres.rs`) that path-included durability without the gameplay
  module; ADR-0022 crates remove that composition problem, but the durability-side port is
  still required by the layer rule.
- ADMIT0 §5's three codes belong to admission/reconnect/recovery; there is no field-effect
  wire/refusal number. The protocol/error owner allocates one before activation.

## Target layout (ADR-0022)

| Concern | Path |
|---|---|
| Guard leaf | `crates/gameplay/dispatch/src/gameplay_transport/world_required_field_guard.rs` (new); tests `crates/gameplay/dispatch/src/gameplay_transport/world_required_field_guard_tests.rs` (new) |
| Bound set (reuse, do not duplicate) | `crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities.rs` (exists, 502 lines) |
| Durability port / origin type | `crates/gameplay/durability/src/durability/field_capability_port.rs` (new), or extend `crates/gameplay/durability/src/durability/fresh_admission.rs::CurrentCapabilityCheck` (3,296 lines: wiring only) |
| Field classification | `crates/gameplay/durability/src/durability/spell_items_abi.rs` (`QualifiedItemDefinition`, exists, 452 lines); producers in `crates/gameplay/durability/src/durability/native_world_items.rs` / `crates/gameplay/items/src/native_world_items.rs` (verify the producer at allocation) |
| Guarded writer | `crates/gameplay/durability/src/durability/spell_item_transaction.rs` (3,513 lines: origin parameter + one call site) |
| Callers | `crates/gameplay/dispatch/src/gameplay_transport/{ordinary_field_items,native_world_item_cast,native_combat_cast,world_item_cast,field_step_ingress,spell_periodic}.rs` (call sites only; `native_combat_cast.rs` 3,584, `native_world_item_cast.rs` 1,383, `world_item_cast.rs` 1,101 lines) |
| Gate inventory | `crates/gameplay/dispatch/src/gameplay_transport/bundle_gates_tests.rs` (keep world-item casts and field ingress closed) |
| Capability tables | `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs`, `crates/protocol-oteryn/src/capability_tables.rs` (no offer change in this task) |
| PG cases | new child case file under `apps/game-server/tests/support/` (new), mounted from `character_authority_postgres.rs` |

## Qualification matrix

| Boundary | Cases |
|---|---|
| Set and binding | both members; each missing; both absent; missing binding; wrong World/Channel/generation; same policy token under another owner; stale content. |
| Boot/admission agreement | declared-list edit mid-generation does not mutate the set; next generation recomputes; invalid/cyclic/over-bound closure never publishes ready; fresh/reconnect/recovery force the same set and consume no nonce on refusal. |
| Creation families | ordinary field, barrier without damage, supported rune once its handle exists, unrelated conjure control; multi-tile refusal rolls back the whole cast. |
| Prepare/commit race | owner replaced after prepare; session/connection/lease/content change; tile/creator invalid; direct and retained retry. |
| Uncertain COMMIT/replay | lost response then exact replay once; conflicting binding; reconcile under successor; no duplicate field/payment/training. |
| STEP | player-origin damage and walls with each member absent; stale creator; malformed origin; ordinary collision unchanged. |
| Monster/system | creature-origin contact and periodic DOT (incl. creator death), static/system field, unrelated tick: no refusal, no tick consumed. |
| Lifecycle | same-generation reconnect keeps the set; restart rebuilds from boot facts; old-generation fields do not grandfather authority. |

No RNG draw or field/condition cursor is consumed on refusal.

## Acceptance criteria

- [ ] A player field/wall in a scope whose effective set lacks either member is refused with a typed outcome and leaves no durable trace.
- [ ] The effective set is read from the current generation's binding; a mid-generation change takes effect only at the next generation.
- [ ] Monster/system fields are unaffected; unknown player origin is refused, not exempted.
- [ ] Exact historical receipts replay unchanged ahead of the guard.
- [ ] Every concurrent transition serialized by the existing lock order; the losing writer is typed.
- [ ] Restart/session-generation change resumes from boot facts and existing fences, no stored `allowed` flag.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests live in their own child files, not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`spell_item_transaction.rs`, `native_combat_cast.rs`, `fresh_admission.rs` take call sites only).

## Open questions

- Confirm with CP that both `WORLD_SPATIAL_FIELDS` (now 30) and `WORLD_INTERACTION_V1` are required (D1442 supersedes the D1132 interaction-only checkpoint).
- Reuse `CurrentCapabilityCheck` or add `field_capability_port.rs`.
- Field refusal outcome code and registry row (protocol/error owner).

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-dispatch world_required
cargo test -p oteryn-gameplay-durability spell_item
cargo test -p oteryn-protocol-oteryn
OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test -p oteryn-game-server --test character_authority_postgres field
python3 tools/agents/validate_governance.py
```

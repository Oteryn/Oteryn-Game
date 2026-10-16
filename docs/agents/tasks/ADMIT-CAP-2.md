# ADMIT-CAP-2 — player field capability predicate (draft checkpoint)

```yaml
task_id: ADMIT-CAP-2
title: "ADMIT-CAP-2: player field effects behind world-required capabilities"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row ADMIT-CAP-2; §7 'ADMIT-CAP-1, ADMIT-CAP-2' (ADMIT0 §6)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_ADMIT0_WORLD_REQUIRED_CAPABILITIES_DECISION_2026-10-01.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/admit-cap-2
needs_migration: false
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## D1132/D1140 authoring checkpoint

[CP D1132/D1140](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224930839)
now assigns the owned leaf a borrowed `&BoundRequiredCapabilities` entry. It must
call `matches_current(scope, generation, policy)` before reading `effective()`.
WORLD_SPATIAL_FIELDS has no ID until FIELD-WIRE-1 and **is not required by #2150**
at this checkpoint. This overrides the historical two-member requirement below;
WORLD_INTERACTION_V1 remains required. No substitute ID or FIELD offer is added.

The actual #2187 source was read at live head
`1d16eda0b7de19e240e8fd5a20c2d476ee103967` (OPEN, unmerged). Its
`gameplay_transport/world_required_capabilities.rs` exposes both production read
methods and validates its immutable set through `for_channel_boot`. The new leaf
uses that exact type and those methods; no cfg(test) `as_slice`, duplicated
closure, session-selection authority or new owner is introduced. The dependency
is not copied into this branch and must be integrated by its owner.

Authored owned files:
- `world_required_field_guard.rs`: match current scope/generation/policy, then
  require registered WORLD_INTERACTION_V1 from the boot owner's sorted set.
  Internal typed refusals distinguish stale binding from missing interaction.
- `world_required_field_guard_tests.rs`: four focused source tests using the real
  I boot constructor. They cover an explicit registered-edge test offer graph,
  the actual bundle offer missing interaction, independently mismatched World,
  Channel, old/new generation and policy, instance scope, and a successor whose
  effective set differs without mutating the predecessor. The test graph does
  not enable the production interaction offer or assert current authority.

The API deliberately accepts only the owner-held binding and independently
resolved current facts for a **positively player-origin field/wall** operation.
It does not classify absent/unknown provenance as monster/system. Real callers
must preserve those sources' existing paths and all caster/session/content,
physical/tile/PvP and durable authority checks. A successful capability predicate
alone cannot authorize a mutation. Recheck at the existing guarded new-write
boundary, under its authority locks; immutable prepared facts are insufficient.
Historical receipt reconciliation remains with the transaction owner.

D1140 assigns the real `connection.rs` handoff to #2187 as the first A2 item after
#2197 and #2163. No caller, shared module registration, registry/error code,
FIELD activation, schema, content or other owner's source is changed here.
Missing-binding refusal, monster/system isolation, no-trace cost rollback,
prepare/commit replacement and replay/restart require actual caller integration.
An external `tests/admit_cap_field_guard.rs` target is not fabricated around an
unexported, unwired leaf; its meaningful native integration proof remains open.

**NOT_FULLY_QUALIFIED.** Explicit Rustfmt of both owned Rust files and
`git diff --check` PASS. Current branch lacks the unmerged I dependency/module
registration; compile, strict lint, all four new tests and full package/runtime/
PostgreSQL validation are **NOT_RUN**. Production acceptance is **RED** pending
the actual caller and the retained integration matrix. Source tests provide no
rollback, provenance, liveness or runtime proof. D1122 allows this draft authoring
checkpoint; no push, freeze, ready transition, external review trigger or CI
restart is performed here.

## Historical goal and integration packet

Player-created spatial field effects (FIELD-2) apply in a scope only when that scope's effective capability set contains WORLD_SPATIAL_FIELDS and WORLD_INTERACTION_V1. The effective set is fixed per scope generation, so admission and field application never disagree within one generation.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

Single row (ADMIT0 §6 guard on top of ADMIT-CAP-1 world-required capabilities). No further children.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ADMIT-CAP-1 (family entry gate) | BLOCKING (Wave 1, not merged) |
| FIELD-2 (LOOP-WORLD-1 child) | BLOCKING (`blocked` in docs/agents/tasks/active/LOOP-WORLD-1.md) |
| acceptance of ADMIT0 | ACCEPTED (#2209); no longer a blocker |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/gameplay_transport/world_required_field_guard.rs`
- `apps/game-server/src/gameplay_transport/world_required_field_guard_tests.rs`
- `apps/game-server/tests/admit_cap_field_guard.rs`
- `docs/agents/tasks/ADMIT-CAP-2.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- capabilities.rs: `GATED` table l.165 and offered sets l.76/104/132 (WORLD_SPATIAL_FIELDS, WORLD_INTERACTION_V1 rows)
- field application call site introduced by FIELD-2 (guard hook; path fixed by CP after FIELD-2 merges)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json and OTERYN_GAME_ERROR_CODE_REGISTRY.json (refusal code)

## Inseparable bundles (never split across PRs)

- guard + effective-set snapshot per generation + refusal code + tests

## Acceptance criteria

- [ ] A player field effect in a scope whose effective set lacks WORLD_SPATIAL_FIELDS or WORLD_INTERACTION_V1 is refused with a typed code and leaves no durable trace.
- [ ] The effective set is read once per scope generation; a mid-generation capability change takes effect only at the next generation.
- [ ] Monster/system fields are unaffected by the guard.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server admit_cap
cargo test -p oteryn-game-server field
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_ADMIT0_WORLD_REQUIRED_CAPABILITIES_DECISION_2026-10-01.md §6
- docs/agents/tasks/active/LOOP-WORLD-1.md (FIELD-2 state)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded native integration packet — 2026-10-15

This is `REPORT_ONLY_NATIVE_INTEGRATION_PACKET` for existing draft #2150 at
`8e4c5f9e3c1c063b3f8476f4ed8b7f0b3abdd86a`. The active allocation owns this card
only. The original plan, open criteria, historical dependency table and builder/CP
rules above are preserved. No guard code, shared wiring, content, schema, registry,
offer gate or selected capability changes are made here. The packet identifies the
actual caller changes for an eventual inseparable implementation; it is not runtime
qualification, an architecture approval or permission to open FIELD-2.

### Accepted rule and the exact missing owner join

Protected-main `bda15eba7a84f65fcb963a86e8b2bf18d69c800a` contains ACCEPTED
ADMIT0 under `ACCEPT-BATCH-2026-10-15`. Its §§3–4 fix the effective requirement set
for one Channel ownership generation and require the same set at authenticated fresh
admission, reconnect and recovery, including atomic revalidation. Its §6 requires
both `WORLD_SPATIAL_FIELDS` and `WORLD_INTERACTION_V1` for FIELD-2 player effects
and blocking walls. Instance scopes have no such declared set or player field/wall
activation until their own contract supplies one. WorldId is not ChannelId.

The original CANDIDATE acceptance hold is historical. ADMIT-CAP-1's **implementation**
remains a prepared dependency: archived `deliverables/admit-cap-1/RESULT.json` says
`REVIEWED_PARTIAL`, retained base `19ec21c299d0f1763b82aa1030032d0780703de5`,
#2187. Its real `RequiredCapabilities::for_boot` validates bounded closure/floor and
reuses `SelectedCapabilities::select`; `as_slice` exposes that immutable result and
`select_for_session` applies the same effective set plus trusted transport-profile
requirements. The type explicitly contains no World/Channel, scope generation,
current owner, authenticated session or nonce. It is not a merged generation-bound
owner or a fresh authority source. Do not build a second closure/negotiator in CAP-2.

At this candidate head, the protocol registry has `WORLD_INTERACTION_V1` with its
offer closed and dependencies `ITEM_USE_V1` and `WORLD_MAP_VIEW_V1`. It has no row
named `WORLD_SPATIAL_FIELDS` or `MAP_STATE_V1`. Existing spatial-entities/map-view
names do not authorize an alias for those decision names. The production, item-view
and bundle offered sets in `gameplay_transport/capabilities.rs` do not offer the
required field pair. ADMIT-CAP-1's prepared card records this same unresolved
registry/build-floor adapter. Resolving names/numbers/dependency closure and opening
wire/client offers belongs to their existing CP owners, never to a hardcoded CAP-2
fallback or a new numeric allocation in this packet.

**Concrete cross-domain finding:** no inspected native field boundary receives a
current generation-bound ADMIT0 effective set. The current World-field policy reader
is not that missing set. `durability/spell_field_policy.rs::read_world_field_policy_in_transaction`
locks the actual admission runtime guard, requires ready/current ownership generation,
then reads the control-published `world_policy_revision` row with `FOR SHARE`; it
produces World mode, protection level and in-fight duration. Neither its result nor
`ready = true` proves capability closure. `AdmissionAuthorityGuardStateV1::Runtime`
currently carries generation and revision dimensions, not a field capability set.
Likewise, a connection's `SelectedCapabilities` is a negotiated session selection,
not the Channel generation's effective requirement set.

### Real field dataflow, not an absent-caller assumption

| Existing function/source | Actual ownership and mutation boundary | Required CAP-2 insertion, after ADMIT-CAP-1 integration |
|---|---|---|
| `ordinary_field_items.rs::append_creations` (`:71`) | Joins ordinary `create_item` operations to the original caster-cost transaction; validates actual SQL authority, current physical caster, exact command/session/lease/connection/content, complete actor footprint and Channel generation, then obtains current World field policy and appends Ground removals/mints. Present safe-field selection restricts unsupported player combat. | Once nonempty field creations are classified from qualified current content, require the owner-held effective set for this exact Channel generation before field requests/reservations can become a new write. Reuse its existing authority/world-policy checks; capability sufficiency must not grant PvP or relax tile policy. |
| `native_world_item_cast.rs::prepare_barrier` (`:463`) | Prepares actual Magic Wall/Wild Growth-style source Item effects using current caster/item/tile and World policy, source duration and existing physical/cost plan. | Check the same generation-bound field set before producing a new player barrier acquisition. Cover blocking effects even if no damage recipe exists. Do not classify a wall only by `field_condition != None`. |
| `native_combat_cast.rs` preparation dispatch (`:3135`, `:3169`) | Calls the existing native world-item preparer and ordinary field append, builds one retained original request, stages physical installation and rechecks original tile/party/World-policy/caster facts. | Add field capability validity to the existing new-write verdict alongside `field_policy_current` at `:3194`, before `apply_spell_items_in_transaction_guarded` (`:3221`). A retained field attempt must prove the same current scope generation at the mutation boundary; prepare-only validation is insufficient. Apply the guard only to positively identified field/wall operations, not unrelated spells. |
| `world_item_cast.rs::cast_world_items_inner` (`:805`) | Real dispatcher exists; `:841` refuses unsupported `tile_item_operation` because Rune V1 lacks an exact source Item handle, while random-item/conjure use the existing owner transaction. `:1005` obtains current Item authority and `:1060` uses the guarded writer. | Do not assume this already activates barrier/rune fields or blanket-guard every conjure. Preserve its current refusals. If a later admitted route adds field operations here, carry their real family classification and the same current-generation verdict into this existing transaction/replay path. |
| `field_step_ingress.rs::step_with_field_ingress` (`:295`) → `StepIngressRead::validate_current` (`:47`) | Real STEP reads ordered current destination Items under the owner lane, runtime/state locks and SQL ground locks; verifies origin/destination/current session/control/content/generation before physical movement. Player provenance and actual creator session/lease are independently resolved (`:390`); the current World policy is read in the same transaction (`:413`). | For a positively player-origin field/wall effect, verify the same scope-owner effective set before effect/collision publication. Classify provenance without bypassing ordinary Item collision or first-field ordering. Missing capability cannot mean walking through an existing blocking wall. Failure publishes neither STEP nor field HP/DOT/state mutation. Keep creature-origin contact on its own existing branch. |
| `spell_periodic.rs::tick_with_standing_read` (`:195`) and `tick_current_creature_field_damage` (`:115`) | Current standing tile/content/fence checks feed the existing condition owner. Creature-origin HP/DOT has its genuine Combat/death path; periodic standing reads do not install/refresh entry conditions. | The eventual FIELD-2 player-origin periodic consumer must use the same bound set before applying its effect. Do not add a blanket capability return around all ticks: that would suppress monster damage, consume unrelated timers or change existing conditions. A standing fact or saved source string is not an admission grant. |

Actual live callers are already present: `gameplay_transport/mod.rs:3202` calls
STEP ingress on the non-bundle route; the bundle route at `:3189` uses existing
collision without inventing field metadata. Native cast dispatch already reaches the
ordinary/barrier preparation above. The gate inventory in `bundle_gates_tests.rs`
keeps world-item casts and field ingress closed in their respective bundle families.
This report does not open either gate or claim all player-field effects currently
run. In particular ingress explicitly refuses unsupported current player-PvP hits;
adding capabilities would not supply missing PvP/skull/equipment/Combat facts.

### Generation owner and authority flow proposed for the CP slot

1. **Boot/configuration owner:** integrate ADMIT-CAP-1's trusted registered/offered
   graph and declared list beside the existing world-policy revision. Compute closure
   and mandatory floor once before readiness, then retain the resulting set with the
   actual WorldId, ChannelId and ownership generation in the existing generation owner.
   No per-command config read, mutable global flag, caller-provided `fields_enabled`
   or new detached guard authority. Its lifetime ends with that generation.
2. **Readiness/admission owner:** bind the retained set to the actual runtime assignment
   and publication lifecycle. Existing `native_admission_source/runtime_status.rs`
   publication and `AdmissionAuthorityGuardStateV1::Runtime` revision/owner data are
   trace points, not permission to invent another publication dimension. ADMIT-CAP-1
   must finish the accepted authenticated/current-scope checks before FIELD-2 opens.
   The old single `required_capability()` check in `connection.rs:1181`, before
   `authority.admit`, is not that accepted predicate and must not be reused as one.
3. **Field caller:** resolve the owner-held set from the same runtime generation whose
   current scope/session/Item authority is already checked in the SQL transaction.
   Both symbolic capability memberships must hold. Compare actual runtime scope and
   generation to the locked current authority; stale/missing owner state refuses.
   Content pin and current creator/session/lease remain independent checks. A prepared
   request may retain its expected binding; it cannot mint the current set by copying it.
4. **Commit boundary:** preserve current spell lane → Channel/state → existing SQL
   authority/ground ordering; do not add an inverse-order lock or hold a new detached
   capability lock across COMMIT. Because the effective set is immutable for the
   generation, validation can use the same owner-held set while the existing current
   scope/authority transaction locks prevent a replaced generation from committing.
   Carry capability refusal through the existing new-write verdict and rollback path;
   release physical/presentation reservations only after definite non-commit.
5. **Replacement/reconnect:** a new list needs a new world-policy revision and ownership
   generation, even if support appears unchanged. Stop old ownership, invalidate its
   capability binding, rebuild and validate closure before new readiness, and attach
   sessions only through accepted fresh/reconnect/recovery checks. Selection changes
   require the accepted replacement snapshot before deltas. Transfers into a Channel
   remain refused until ADMIT0 §3.2's acknowledgement/recovery lifting conditions are
   implemented by their owning contract; CAP-2 does not create a transfer protocol.

This does not require a new durable capability table or durable selected set per
session: ADMIT0 §7 rejects that design. Restart resumes via current boot-generation
binding and the existing assignment/readiness/session fences, not a stored `allowed`
boolean. Recovery from a historical spell receipt is a separate classification.

### Replay, no-trace refusal and provenance exception

`durability/spell_item_transaction.rs::apply_spell_items_in_transaction_guarded`
(`:1801`) first validates transaction authority and serializes the exact command.
It checks an immutable receipt's full binding and returns `AlreadyCommitted` before
consulting `new_write_verdict` (`:1866`). This is intentional recovery behavior.
A new CAP-2 guard must not turn a previously committed unknown outcome into a claim
that nothing committed. Preserve classification/reconciliation of the original plan;
historical success is not permission to reinstall into a new scope generation, mint
again, charge again or replace its old capability binding with request-derived facts.

For a genuinely new refused field/wall attempt, rollback leaves no spell Item receipt,
Item/ground/field line, cost/charge, training write or durable audit/event attributable
to this attempt. It also leaves HP, conditions, position, cooldown and presentation
unchanged and releases proven-uncommitted reservations. An existing historical receipt
is not a new durable trace from refusal and must remain intact. Do not write a special
failure receipt merely to implement this guard. The task requires typed refusal, but
ADMIT0 §5's three codes belong to admission/reconnect/recovery; it supplies no separate
field-effect wire/refusal number. CP's protocol/error owner must integrate the field
refusal into accepted native outcomes and any required registry row before opening
activation. This packet assigns no string/number as a public code.

Player versus monster/system is established by real issuer/provenance, not by an
optional creator pointer being absent. `DurableFieldOrigin` (`spell_items_abi.rs:221`)
is immutable player creation history; actual creator freshness is independently
resolved by ingress. `DurableCreatureFieldOrigin` is a separate typed source, and
`field_step_ingress.rs:151` validates it before staging creature contact. Preserve
that source's existing behavior and Combat/death ownership. Unknown, malformed,
conflicting or stale player origin must never fall through as environmental/system
permission. Static map/system fields likewise retain their own admitted source rules;
the CAP-2 exception is not permission to bypass their existing gates or create a new
system issuer. Guarding every MagicField or every periodic tick would violate the
original monster/system exception.

### Native refusal/replay/generation qualification matrix for implementation

| Boundary | Required cases and unchanged-state evidence |
|---|---|
| Set and binding | Both members present; each missing alone; both absent; missing owner binding; wrong World, Channel or ownership generation; same textual policy token under another owner; stale content independently. Hold other facts valid in each negative case. |
| Boot/admission agreement | Declared-list edit within generation does not mutate retained set; next generation recomputes it. Invalid/unoffered/cyclic/over-bound closure or missing mandatory floor never publishes ready. Fresh/reconnect/recovery checks run after authentication and at final revalidation, force the same set and consume no nonce on refusal. |
| Creation families | Real ordinary field, blocking barrier without damage, supported rune/item route once its handle contract exists, plus unrelated conjure/ordinary Item control cases. Missing capability leaves costs, charges, receipts, Items, timers, training and presentation unchanged; multi-tile refusal rolls back the whole original cast. |
| Prepare/commit race | Replace scope owner after preparation, change current session/connection/lease or content, or invalidate tile/creator. Exercise original direct path and retained retry with no earlier receipt; every stale new write refuses under existing locks and definite rollback releases reservations. |
| Uncertain COMMIT/replay | Commit then lose response, replay exact binding once, conflict one binding, reconcile under supported successor and replaced scope owner. Exact historical outcome remains readable without a new write; no historical receipt reacquires physical authority or creates duplicate field/payment/training. |
| STEP/player effects | Positively player-origin damage and blocking walls with each capability absent; stale creator session/lease/placement, wrong source generation/content, unknown policy and malformed origin independently. No movement/HP/DOT/cursor/cooldown publication on the refused composite operation. Ordinary non-field collision is unchanged. |
| Monster/system controls | Real creature-origin initial contact and periodic DOT, including creator death, plus qualified static/system source and an unrelated tick. The CAP-2 player guard adds no refusal or tick consumption. Missing/forged player origin does not gain this exception. |
| Lifecycle and visibility | Same-generation reconnect keeps the immutable set; new generation uses the new set after authentication; restart reconstructs owner binding from real boot facts; old-generation field data/retained intents do not grandfather new authority. Selection change emits replacement baseline before domain deltas; blocked transfers preserve current authority. |

Use the actual native owners and real PostgreSQL rollback/concurrency/reload fixtures
for durable cases once the inseparable integration exists. Source-only boolean tests
would not prove admission/field agreement, locks, no-trace rollback or recovery safety.
Determinism review must verify the guard consumes no RNG draws or field/condition
cursor on refusal and preserves accepted owner ordering. Gate inventory, registry/
error mapping, boot/admission/recovery wiring and client compatibility require their
CP-owned slot changes; they cannot be completed by registering a detached helper.

### Evidence and local qualification

`/workspace/oteryn-prep/admit-cap-2-evidence/` retains the reproducible read-only
source extractor, 28 exact Git-source hashes (25 candidate sources and three
protected-main decisions), literal capability-registry rows, and five verified ADMIT1
export bindings. The prepared ADMIT1 source hash is
`96883292575ce3ad81415700982c3f2e3f26f1e8892745363f0fdf666bb0b7cc`;
its historical ten-test result is not rerun or promoted to native integration proof.
Governance validation, 124 governance tests, whitespace and exact card-only scope are
recorded in raw logs/final binding. No Rust/Cargo, PostgreSQL, runtime, client or
performance tests were run here. Whole-diff worker self-review preserves all original
card bytes; CP and independent protocol/security/determinism review remain required.

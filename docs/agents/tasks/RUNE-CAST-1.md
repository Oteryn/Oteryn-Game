# RUNE-CAST-1 — gated legacy USE bridge qualification

```yaml
task_id: RUNE-CAST-1
status: implementing
mode: REPAIR
repository: Oteryn/Oteryn-Game
branch: lane-rune-1b
base_sha: a9b388fae
head_sha: 9d6be0df9
pr: null
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/rune_use_bridge.rs
  - crates/gameplay/dispatch/src/gameplay_transport/rune_use_bridge/
  - crates/gameplay/dispatch/src/gameplay_transport/rune_use_bridge_postgres_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/rune_item_cast.rs
  - crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast.rs
  - crates/gameplay/spell-rules/src/dispatch_plans/
```

## Module layout planned before continuation edits

The existing adapter stays in rune_use_bridge.rs. Child replay.rs composes the
existing native retained-batch replay; revision.rs resolves the current Item
revision under the existing spell lane. Refusal diagnosis belongs in the Rune
reservation's own child use_refusal.rs, calling shared legality rules. Pure
spatial checks extracted from the existing spell operational validator belong
in L3 spell-rules/dispatch_plans/cast_legality.rs; carrier eligibility is in
rune_use.rs. L4 spell and L5 diagnosis
reuse them. The existing real-dispatch PG test module remains registered once
under monk_save_postgres_tests.rs and reuses its private fixture. The pre-existing drive_until helper is extracted to monk_save_wait.rs,
adding the root-readiness boundary required by the observed Monk-save flake.
Shared hubs receive wiring only. No new executor, SQL, migration or capability offer.

## Continuation authority and scope

PROVEN: owner supplied the exclusive RUNE-CAST-1 and Rune connection route lease;
local uncommitted repair only, no Git mutation/publication or external review.
AUTHORING continuation from WIP HEAD 9d6be0df9; no frozen candidate or PR supplied.
The task card was absent from the reapplied WIP tree; this reconstructs the
owner's packet and records continuation evidence. Bound META 3.1.0 at
1bfb5ff98c8aa156e73669a14e083a1d464c29fb was read through a read-only connector.
ADR-0022 and RUNE-USE-0 §§4–6 govern the unchanged adapter/payment seam.
Capability 22 stays production-gated; only injected test sessions offer it.
Full Rune-family acceptance and activation remain outside this slice.

## Integrator findings

1. PROVEN: complete replay_legacy_rune moved to rune_use_bridge/replay.rs.
   Revision resolution moved to revision.rs; native combat only acquires/threads
   the lane and invokes that seam. Native module stays private, exposing only
   replay_with_rune and current_rune_revision through actor_spell re-exports.
   Legacy batch binding also moved out of ordinary_combat.rs into replay.rs.
   Native combat net growth against the reapplied main parent is 83 lines.
2. PROVEN: rune_current runs only when attempt.legacy and attempt.rune are both
   Some. Existing ITEM-SPELL-V2 attempts have legacy=None and retain their prior
   retry behavior. No governing-document expansion or existing-path behavioral
   change is claimed.
3. PROVEN: rg identified spell::check_operational_rules, attack::sees,
   native sight_steps/read_tile and ordinary_combat target selection/legality.
   Authored protection, floor, reach, sight and Rune blocking checks were moved
   verbatim in meaning from the L4 operational validator to the L3 cast_legality
   module; the old inline implementations were deleted. The validator and
   diagnose-only USE reporting now call these same functions. Native validation
   preserves floor -> sight -> range -> Rune blocking order. Refusal diagnosis
   preserves the previous floor/range-before-tile-IO ordering through reach_refusal.
   use_refusal lives in its own Rune child and cannot authorize a cast or payment.
   Existing native target selection and final legality remain authoritative.
4. PROVEN: conflict resolved retaining MAP-PICKUP production_generation_with_pin
   and Rune production_generation_with_source with one shared implementation.
   PG broad validation reproduced MONK-SAVE-FLAKE-1: cancelling the release at the
   save-attempt counter can leave its sole holder busy during the next read.
   Extracted existing drive_until into monk_save_wait.rs; it takes the root and
   observes both the existing counter predicate and root.is_ready(). The counter,
   durable assertions and deadlines remain unchanged. Every caller was updated.
5. Final exact counts and line-count evidence are recorded below after validation.

## Reuse inventory

- Existing cast_rune_item and native cast executor/payment/owner installation;
  no second executor or payment implementation.
- Existing retained native batch and replay_with_rune, original command binding,
  pending marker and CurrentSpellAccessOwner refresh; no new replay cache.
- Existing Rune reservation, equipment_successor, inventory consumption preparer,
  qualified Rune/source context and same-transaction Item authority assertions.
- Existing pure authored operational legality, extracted into L3 cast_legality;
  existing attack::sees, sight_steps, locked read_tile and ordinary target ordering.
- Existing single character_revision_sequencer fixture, Harness, run, join_two,
  prepare_current_session and fresh_fixture; fixture was never loaded again.
- Existing Dependencies construction parameterized with with_content, retaining
  its single runtime/TLS initialization body in the owner file.
- Existing production_generation_with_pin delegates to the one parameterized
  production_generation_with_source body; main's MAP-PICKUP callers retain its API.
- Existing LOOK read_frame, large-stack thread body and backpack seed body:
  test-only visibility/parameterization in item_look_postgres_tests.rs; no copies.
- Existing capability test command builder and RUNE_USE_OFFERED exposed with a
  test-only re-export in capabilities.rs; original tests updated rather than
  registered again. Existing condition gate_player test seam exposed minimally.
- Existing spell-rules ordinary_combat::tests::source catalogue loader exposed
  with one test-only re-export in its owner file for cast_legality_tests.rs.
- Existing drive_until moved, not copied; the fixture root's is_ready method
  supplies the additional no-checked-out-holder cancellation condition.

## Authority, negative cases and recovery

PROVEN by source: bridge resolves direct backpack custody, instance revision,
actor/session fence and target generation from current owners. Client handles do
not grant authority. Pending retries bind the original USE; the existing native
receipt remains the one value-moving idempotency key. Deadlines remain the
existing connection/semantic-pass deadlines; no production SQL was added.
No schema, durable cause, protocol code or capability registry changed.

PG matrix: creature/generation and handle substitution, position selection,
wrong floor, condition gates before target checks, protection-zone and TOO_FAR
wire dispositions, absent 22, unqualified source, stale session, moved content,
proven pre-commit rollback, post-commit acknowledgement loss and stale retained
revision. The fixture asserts debit/revision/receipt/health after reload and on
retry. Three original spell-rules regression cases exercise floor/sight/range
precedence, the known-solid creature exception versus unknown solidity, and
aggressive-only protection-zone admission using the existing authored loader.

## Validation notes

The login shell initially resolved cargo to rustup ahead of the supplied wrapper.
That initial focused test returned early without a PG URL and is explicitly NOT
PG evidence. All final evidence uses /home/mole/oteryn-cache/codex-bin/cargo.
The first real PG run detected the out-of-range diagnostic IO ordering regression;
reach_refusal restores the previous ordering. The first broad run reproduced the
Monk-save holder-cancellation flake, repaired as described above. No test was
removed, ignored or relaxed. Final repaired-tree results follow below.

## Final self-review

Re-read the complete WIP-plus-repair delta against the owner-supplied
/home/mole/oteryn-cache/CODEX-CHECKLIST.md and the repository checklist.
Workspace rg covered every NativeCastDispatch::RuneRefused match, PendingNativeCast
constructor/legacy field, NativeMarkerIntent constructor/destructure, LegacyRuneUse
constructor/field, test Point/Case variant, PositionRefusal variant, use_rune trait
implementation/forwarder, and moved drive_until caller across apps/, crates/, tools/.
Test authorities inherit the rejecting use_rune default; the composed owner forwards to the bridge.
No duplicate fixture load or test registration was added. Production public rule
functions are called from the bridge/native dispatch route exercised by the named
rune_use_bridge PG test. Test-only wrappers/re-exports are listed above.
CI lib-rest's `not test(content::)` filter includes the new test path; local PG
qualification is explicitly supplied-wrapper evidence, separate from CI status.

## File line counts

Main parent is a9b388fae; WIP is 9d6be0df9. All counts include uncommitted repairs.

| File | Main before | WIP before repair | Final |
| --- | ---: | ---: | ---: |
| `crates/gameplay/dispatch/src/gameplay_transport/actor_spell.rs` | 4150 | 4150 | 4152 |
| `crates/gameplay/dispatch/src/gameplay_transport/bundle_gates_tests.rs` | 666 | 667 | 670 |
| `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs` | 308 | 311 | 311 |
| `crates/gameplay/dispatch/src/gameplay_transport/capabilities_tests.rs` | 2745 | 2753 | 2753 |
| `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` | 8261 | 8285 | 8285 |
| `crates/gameplay/dispatch/src/gameplay_transport/item_look_postgres_tests.rs` | 349 | 369 | 369 |
| `crates/gameplay/dispatch/src/gameplay_transport/mod.rs` | 8167 | 8189 | 8189 |
| `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs` | 1348 | 1386 | 1366 |
| `crates/gameplay/dispatch/src/gameplay_transport/monk_save_wait.rs` | 0 | 0 | 29 |
| `crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast.rs` | 3584 | 3789 | 3667 |
| `crates/gameplay/dispatch/src/gameplay_transport/ordinary_combat.rs` | 2881 | 2903 | 2891 |
| `crates/gameplay/dispatch/src/gameplay_transport/rune_item_cast.rs` | 221 | 370 | 266 |
| `crates/gameplay/dispatch/src/gameplay_transport/rune_item_cast/use_refusal.rs` | 0 | 0 | 135 |
| `crates/gameplay/dispatch/src/gameplay_transport/rune_use_bridge.rs` | 0 | 365 | 372 |
| `crates/gameplay/dispatch/src/gameplay_transport/rune_use_bridge/replay.rs` | 0 | 0 | 121 |
| `crates/gameplay/dispatch/src/gameplay_transport/rune_use_bridge/revision.rs` | 0 | 0 | 77 |
| `crates/gameplay/dispatch/src/gameplay_transport/rune_use_bridge_postgres_tests.rs` | 0 | 691 | 708 |
| `crates/gameplay/spell-rules/src/dispatch_plans/cast_legality.rs` | 0 | 0 | 78 |
| `crates/gameplay/spell-rules/src/dispatch_plans/cast_legality_tests.rs` | 0 | 0 | 103 |
| `crates/gameplay/spell-rules/src/dispatch_plans/mod.rs` | 5 | 5 | 7 |
| `crates/gameplay/spell-rules/src/dispatch_plans/ordinary_combat.rs` | 329 | 339 | 332 |
| `crates/gameplay/spell-rules/src/dispatch_plans/rune_use.rs` | 0 | 0 | 15 |
| `crates/gameplay/spell/src/spell/mod.rs` | 795 | 795 | 778 |
| `docs/agents/tasks/RUNE-CAST-1.md` | 0 | 0 | 197 |

Only rune_use_bridge.rs (+372) and rune_use_bridge_postgres_tests.rs (+708)
grow by more than 200 lines against main. The bridge is L5 current-owner resolution
and composition; all pure legality is L3 and payment stays with the existing L4
owner. The PG file owns one real-dispatch mechanic suite with ten adversarial
scenarios and reuses its parent/LOOK fixtures; it introduces no runtime logic.

## Final validation results

All Cargo runs used the queued wrapper and +1.95.0; check/clippy/tests used --locked --quiet.
- fmt --all --check; check and clippy --all-targets -- -D warnings: PASS for
  oteryn-gameplay-dispatch, oteryn-gameplay-spell-rules and oteryn-gameplay-spell.
- test -p oteryn-gameplay-spell-rules: 57 passed; spell: 221 passed.
- dispatch --lib boundary_unknown_monk_save: 1 passed (16.91s).
- dispatch --lib rune_use_bridge: 1 test / 10 PG scenarios passed (489.68s).
- dispatch --lib rune_wire: 2; rune_item_cast: 2; rune_instance_and_revision: 1 passed.
- protocol-oteryn --lib world_object: 15 passed.
- dispatch --lib runs 1–3: each 903 passed / 0 failed / 14 existing ignored; 397.50s, 365.18s, 319.34s.
- Governance validator PASS; governance unit tests: 124 passed.
- Pre-existing vendor Tokio missing_docs warning appears without failing clippy.

# MAP-OVERLAY-1c reset owner integration gaps

Assessment of allocated #2167, branch `skeleton/map-overlay-1c`, source
`59172979bc3c333fb2eda43b6dae52065bbbf9ef`. This is a bounded source assessment,
not an accepted API contract, a reset implementation, or runtime qualification.
CP gap-review source: issue comment `6216366289` (2026-10-16).

The accepted behavior remains ADR-0021 §4.7, HOUSE-CUSTODY-0 §3.6 and
SOCIAL_MAP §§1.5–1.8. Those requirements allocate reset ownership to this child;
they do not make the existing component seams into a composed reset owner.

## Finding and disposition

**DERIVED / MATERIAL_BLOCKER:** no coherent implementation of the remaining
whole-reset path fits the current owned paths. Preserve the existing one-item
retirement component. Do not substitute callbacks, test-only publishers, a caller
epoch, an empty-house assumption, or a second content initializer for the missing
production integrations. CP must allocate the concrete owner/consumer edits below,
with the inseparable schema and wiring slot. No new authority model is proposed.

| Required integration | Existing source evidence | Missing owner/consumer behavior and allocation |
| --- | --- | --- |
| Preflight and durable reset admission | `durability/world_reset.rs::candidate_schema_sql` defines an uninstalled job/cohort; control has `INSERT`, runtime only reads and updates revision. `game_world_reset_job_change` refuses every binding or phase update. Tests `support/world_reset_postgres_cases.rs::{admit,replace_with}` insert under control and explicitly end a fixture session; they are not production callers. | Bind the actual current map/epoch, complete World Channel set and predecessor assignments to an admitted target. Qualify target House membership and current property/catalogue eligibility before any record or closure. Atomically retain the admitted RETIRING state with the closure behavior required by the owner; recover that decision after restart. The publisher and consumer are absent, rather than reusable accepted port signatures. |
| Close admission, logout, and drain every actor/pending writer | `gameplay_transport/mod.rs` owns `ComposedFreshAdmission::input_gate`; `gameplay_transport/input_gate.rs::InputGate::quiesce` closes and drains permits, has no reopen operation, and is documented for shutdown save. The existing connection close and session retirement paths belong to transport. | Compose the real World-wide close/logout/drain through transport and node ownership; prove every old writer ends before retirement and that source/timer work cannot repopulate Ground. `gameplay_transport/{mod,connection}.rs` are slot-only; `input_gate.rs` and node lifecycle consumers are outside owned paths. A new helper inside `world_reset.rs` would have no production consumer. |
| Fresh ordinary Channel assignments and resumable cohort | `durability/runtime_scope_assignment.rs::RuntimeScopeAssignmentWriter::{submit,reconcile}` and `publish_runtime_readiness` are real current-incarnation/fenced per-Channel components. The reset item writer verifies a control-inserted predecessor cohort and closed readiness; it does not create or resume it. | Drive the existing assignment owner for all Channels onto the resetting node with retained operation identities and independently current node evidence; reconcile unknown outcomes and assignment turnover on restart. Keep House scopes outside the cohort. Reuse the assignment writer; do not add a World scope or weaken holder checks. The node/transport orchestration consumer must be allocated. |
| Complete target House membership and property eligibility | `content/project/native_house_tiles.rs::QualifiedHouseTiles` qualifies a specific native map region and preserves Unknown when the document is absent. The durable custody table exists. There is no `durability/house_interior.rs` runtime writer on this branch. | Supply actual active/target World content, frame, catalogue and stable House membership, with the owning property view and serialization. Region absence or a tile marker cannot stand in for a complete target catalogue. Before HOUSE-1b, refuse a differing catalogue whenever actual property rows exist; do not assume the future table name is already installed. HOUSE-RUNTIME-1a (#2163) remains required for the real positive custody test. |
| Final all-Channel census and atomic activation | `freeze_next_world_reset_item` is a single-Channel one-item selector. `durability/content_activation.rs::record_content_activation` begins and commits its own semantic transaction. `content/activation.rs::AuthorizedContentGeneration` is sealed; its production implementation uses `NodeBootQuiescence`, minted before runtime/listeners exist. The candidate reset schema forbids ACTIVATED. | In one guarded owner transaction: lock the reset row, qualify the entire closed current Channel cohort and empty Ground census, take `LOCK TABLE game_item_house_interior_locations IN SHARE MODE`, recheck target House/property eligibility, and commit actual content activation plus target digest, checked epoch N+1 and ACTIVATED. Then publish the generation/readiness using real quiescence. Changes to durable content activation, its SQL owner, sealed live activation authority, and node publication consumers require allocation; two independently committed API calls cannot satisfy atomicity. |
| Boot/resume and same-digest/first reset consumers | `gameplay_transport/source_item_cycle.rs::ensure_source_map_initialized` gates ordinary source initialization; `durability/native_map_items.rs` implements current-source initialization/adoption. Native activation is consumed in `node/serve.rs` and `gameplay_transport/qualification.rs`. These paths do not consult the reset job. | Before any old-map activation, admission or initialization, resolve RETIRING and resume fresh assignments/retirement; after durable ACTIVATED, initialize the target epoch without adopting retired identities. Preserve exact retry history and ordinary crash behavior. Cover legacy Ground at first bundle cutover and an intentional same-digest reset. The existing initializer/boot consumers are outside owned paths; duplicating them in reset code is insufficient. |

The item writer currently also takes a genuine
`ReconciledCharacterAuthority` to obtain the root recovery fence
(`world_reset.rs::world_reset_phase` → `record_for` → `assert_recovery_fence`).
This is the root recovery admission, not a live player's authority: reuse the
genuine reconciled root proof after logout and retain its current fence. Do not
manufacture a Character/session witness or add a weaker reset recovery authority.

## Exact shared CI fix

**PROVEN / MATERIAL_BLOCKER:** the expected production Type-2 insert site list in
`durability/item_mint_audit.rs::tests::every_type2_insert_takes_its_tuple_from_a_type2_transaction`
omits `src/durability/world_reset.rs`. This is outside owned paths and overlaps
HOUSE-RUNTIME-1a's `house_interior.rs` addition. CP should lease the exact expected
list hunk at the coordinated slot and preserve both real insert sites after
integration. Do not remove a site, weaken the assertion, or edit the shared file
without that lease. This assessment does not claim a newly executed CI failure.

## Qualification after the integrations are allocated

The integration must demonstrate a real multi-Channel reset, exact session/actor
drain, failure of old-generation writes, inventory/House preservation through the
native writers, target House preflight refusals without closure, and a concurrent
House insertion blocked through the final activation commit. A failed final
recheck must leave RETIRING. Resume must work after process restart at retained
RETIRING, assignment replacement, item commit and activation/unknown-outcome
boundaries; it must never admit the old map over partially retired Ground.

Measure actual retirement duration on a declared dataset and Channel count, with
Ground roots, corpse entries, assignment and persistence conditions recorded.
Keep retire-only timing distinct from whole-reset closure time; a one-item or
test-process wall time does not qualify `MAP01-RESET-RETIRE-MS`. No accepted numeric
bound is inferred, and this assessment records no performance pass.

No Rust, schema, registries, test gate, or shared CI list is changed by this packet.
No cargo/PG run, new duration measurement, whole-reset resume pass, freeze,
publication, or production readiness is claimed.

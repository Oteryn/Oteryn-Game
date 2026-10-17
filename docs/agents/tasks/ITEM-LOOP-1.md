# ITEM-LOOP-1

```yaml
task_id: ITEM-LOOP-1
title: "ITEM-LOOP-1: equip, drop/pickup, containers, stacks, cap, starter pack on the bundle (skeleton)"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/item-loop-1
issue: 1622
pr: 2190
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 2 ITEM-LOOP-1 (rows 62-66, 68); App. 7 ITEM-LOOP-1; wave 1 (migration chain #1)"
class: "hard (durable item)"
migration: yes
registry: maybe
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/item_move.rs
  - apps/game-server/src/gameplay_transport/equipment_move.rs
  - apps/game-server/src/gameplay_transport/container_view.rs
  - apps/game-server/src/durability/item_transfer.rs
  - apps/game-server/src/durability/ (new item-loop writers)
  - apps/game-server/tests/item_loop_*.rs (new)
slot_only_paths:
  - migration number (chain #1: ITEM-LOOP-1 -> DAWNPORT-1 -> LOOP-PROGRESS-1 -> DEATH-ML-LOSS-1)
  - connection.rs
  - mod.rs
  - capabilities.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - RESOURCE_LIMITS_REGISTRY.json
  - content/starter pins if STARTER-1 changes content
public_contracts: []
depends_on:
  - "EQUIP-0: MERGED (ACCEPT-EQUIP-0 #2090)"
  - "#2039 ITEM-USE-1a/1b: MERGED"
  - "ITEM-USE-1c: MERGED #2082"
  - "ITEM-MOVE-2a: PARTIAL: EQUIP-WIRE-1 slice #2009; residue here"
  - "BAGS0, ITEM_MOVE_WIRE1, STACK0, STARTER_BACKPACK0, B3 ACCEPTED: effective acceptance #2209 on main e2da9805"
  - "MAP-PICKUP-1 (rows 62-63 base-map pickup): BLOCKING: not merged"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

The item loop works end to end on the bundle World: equip/unequip, drop/pickup (base-map pickup gated on MAP-PICKUP-1), nested containers, stacks, capacity and weight refusal, starter pack.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| EQUIP-0 | MERGED (ACCEPT-EQUIP-0 #2090) |
| #2039 ITEM-USE-1a/1b | MERGED |
| ITEM-USE-1c | MERGED #2082 |
| ITEM-MOVE-2a | PARTIAL: EQUIP-WIRE-1 slice #2009; residue here |
| BAGS0, ITEM_MOVE_WIRE1, STACK0, STARTER_BACKPACK0, B3 ACCEPTED | Effective acceptance #2209 on main e2da9805 |
| MAP-PICKUP-1 (rows 62-63 base-map pickup) | BLOCKING: not merged |

**BLOCKING for integration:** MAP-PICKUP-1 (rows 62-63 base-map pickup), applicable child/source and slot gates. Acceptance #2209 resolves the historical CANDIDATE gate above; B3-3 still requires accepted item weights and the character capacity source. This batch prepares only a detached conditional weight planner and does not enable weight checks in existing writers.

## Owned paths (builder edits freely)

- `apps/game-server/src/gameplay_transport/item_move.rs`
- `apps/game-server/src/gameplay_transport/equipment_move.rs`
- `apps/game-server/src/gameplay_transport/container_view.rs`
- `apps/game-server/src/durability/item_transfer.rs`
- `apps/game-server/src/durability/character_equipment.rs` — CP D1141 native owner and swap receipt
- `apps/game-server/tests/support/equipment_move_postgres_cases.rs` — existing registered owner cases
- apps/game-server/src/durability/ (new item-loop writers)
- apps/game-server/tests/item_loop_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number (chain #1: ITEM-LOOP-1 -> DAWNPORT-1 -> LOOP-PROGRESS-1 -> DEATH-ML-LOSS-1)
- connection.rs
- mod.rs
- capabilities.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- RESOURCE_LIMITS_REGISTRY.json
- content/starter pins if STARTER-1 changes content
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `gameplay_transport/item_move.rs` (command type 9, `committed_item_move` connection.rs l.594)
- `equipment_move.rs` (`move_equipment` l.571)
- `durability/item_transfer.rs` (DUR-03 TRANSFER)
- `apps/game-server/migrations/` (last 0087_character_commands.sql); migration + writer + reconcile + guards land together

## Inseparable bundles

migration+writer+durability; mint/burn+ledger for starter pack (STARTER-1 MINT). Children split into separate PRs in App. 7 order: ITEM-MOVE-2a residue, BAGS-1, ITEM-MOVE-2b, STACK-WIRE-1, STACK-1, STARTER-1, B3-3.

## Acceptance criteria

- [ ] Each child: RED then GREEN, PostgreSQL 17.6 durability tests, restart/replay.
- [ ] Overweight refusal (B3-3) only after B3 acceptance.
- [x] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

plan App. 7 ITEM-LOOP-1; Reference item move rules. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 2 ITEM-LOOP-1 (rows 62-66, 68); App. 7 ITEM-LOOP-1.

## Builder instructions

- Work on branch `skeleton/item-loop-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: bounded native-type planner prepared; 12 focused tests and Clippy pass
status: authoring
branch: skeleton/item-loop-1
blocker: accepted item-weight scale and character capacity source; transaction/caller integration and child gates
next_action: CP/architect qualify weight/capacity sources, independent review and transaction integration slot
```

## Bounded B3-3 local preparation (2026-10-15)

Existing draft #2190; source `6648cf29f3f26b6ed25c529ec87bee8bf163d26a`.
This allocation owns exactly `durability/item_capacity.rs`,
`tests/item_loop_capacity.rs` and this record, overriding the skeleton's broad
family allocation for this batch. No other item writer, transport, registry,
migration, shared wiring or content/pin path is changed. No worker commit,
push, ready flag, queue action or external review is performed.

### Accepted rules and verified residue

The lead established effective acceptance #2209 on main
`e2da9805d296a27b06b32335bf67394509d4910b`. Read-only inspection confirmed B3
§4.2/§5 assigns overweight refusal to B3-3 but still requires evidence for item
weights and the character capacity source. Acceptance does not supply a scale,
base capacity or universal vocation/level formula. GAME-ITEM-01 §7.2 requires
deterministic server evaluation over resolved definitions, quantities and
containment, with exact arithmetic/inclusion parity gated.

Accepted BAGS0 §7 supplies the narrower inclusion rule: an incoming container's
own weight plus contents, carried equipment slots and main-backpack tree, and
checks only for a move into the character from outside. Moves within that
character do not change its carried weight. BAGS0 §3 caps an incoming tree at
500 items including its root. The helper reuses that bound without changing a
registry or admitting a graph.

The existing `item_transfer.rs` header still explicitly says weight is not
checked; `ItemDefinitionFacts` lacks weight and no capacity source is exposed
by this writer. Current entry/merge placement checks already exist and are not
duplicated. The native `InventoryItem`, `TypedDefinitionRef`, stack safety ceiling
and Content `ReferenceItemField<u32>` are reused. Their relevant sources are
unchanged between this allocation, accepted main and the retained real-library
build's source `aaeffba3e6982ba721afa147b82fd8242731ddff`.

### Prepared behavior and boundary

The detached helper computes a conditional projection for a whole incoming
source/tree. It binds each per-unit weight to the entire native item definition
(family, key and revision), multiplies the actual quantity with checked u64
arithmetic, counts each supplied instance once, adds the current carried total,
and returns totals/remaining capacity or a local typed refusal. Exact capacity
fits; overweight is refused without truncating the incoming count. No partial
pickup/count selection or receiver choice is implemented. Existing stack-class,
definition-specific stack-limit and placement admission remain with their owners.

Unknown carried weight/capacity, non-Known weight facts, duplicate instances,
definition mismatches, invalid quantities, oversize inputs and arithmetic overflow
fail closed. Known zero weight remains distinct from Unknown. Internal moves
return `Unchanged` without another weight charge or a capacity-source dependency.
These local errors are not registered wire codes.

The incoming list is an owner-qualified canonical flattening, not a graph proof.
Arbitrary omitted descendants, forged direction, stale totals or arbitrary Known
facts cannot become trustworthy through this helper. All numeric inputs must
already share one admitted integer unit; no ounces/hundredths interpretation,
metadata promotion, source authority or capacity formula is invented here.

### Exact future transaction integration (REPORT_ONLY)

1. Qualify the native per-unit weight representation/unit and authoritative current
   character capacity source under their owning accepted evidence. Capacity may
   be migration-sensitive; it must not be inferred universally from level/vocation.
2. The existing owner, not the client, must prove external versus internal movement
   and obtain the complete canonical source/tree and carried slots/backpack tree
   under the applicable current item/containment/character locks. It must validate
   root ownership, source liveness, every descendant, graph limits and each stored
   quantity/definition. This helper cannot detect omitted descendants or prove that
   an allegedly external source is not already carried.
3. Resolve exact compatible active Content facts and the same admitted unit for
   all rows and capacity. Preserve existing command ingress, source/World binding,
   recovery/session/lease/scope fences, placement/stack rules and cause replay;
   World and Channel remain separate. `Known` and matching immutable definition
   refs are not current activation/authority evidence.
4. Add the check to owning freeze/preflight and commit materialization paths as
   required by the existing DUR-03 lifecycle. At commit, reread/recompute carried
   total, capacity and incoming tree under the same serialized current transaction
   boundary immediately before item/location/event/receipt writes. A prior plan
   reserves nothing and must not survive a competing transfer or capacity change
   as current evidence. Refusal commits no partial movement.
5. Reconcile cause/intent/event evidence and local-refusal mapping in that owning
   slot, with PostgreSQL current-state, concurrent-writer, unknown-outcome, restart
   and replay tests. Do not add a second receipt, a new writer or guessed registry
   value for this helper. Final module/test placement, independent exact-head
   review and CP/architect gates remain required.

This batch does not close B3-3, MAP-PICKUP-1, the other ITEM-LOOP children or the
family's final migration slot. Production weight refusal remains disabled until
the owning sources and actual transaction integration qualify it.

### Focused validation and self-review

- Real-library direct `rustc --test`: **12 passed, 0 failed, 0 ignored**. Tests
  path-load only this unregistered helper and import the actual native public
  library types. No dependency excerpt, schema surrogate or extra wrapper is used.
- Cases cover exact capacity/one-unit overweight, incoming container plus all
  supplied descendants exactly once, order independence, internal moves, complete
  incoming stack quantity, unknown/ambiguous facts, all three definition-binding
  components, invalid quantities, duplicate identities, exact u64/u32 boundaries,
  overflow, empty input, 500/501 items, and a second request replanned against an
  updated total. Fixtures are synthetic and do not attest a live graph or source.
- Strict `clippy-driver --test --emit=metadata` with repository deny lints,
  owned-file `rustfmt --check`, `cargo fmt --all --check`, governance validation
  (22 documents, 9 lanes), **124 governance tests**, and `git diff --check` passed.
  Every Rust command used the shared flock/target, two jobs and
  `CARGO_INCREMENTAL=0`. No Cargo build or implicit server-bin link was run.
- An earlier metadata check recorded the new planner file's absence. It is only
  missing-implementation/setup evidence, **not a semantic regression RED**. The
  production writer's unchecked weight is a documented gap outside this batch's
  write allocation, not a runtime behavior corrected or tested by this helper.
- Source hashes and actual logs are retained under
  `/workspace/oteryn-prep/item-loop-1-evidence/`; `provenance.json` records native
  library/source bindings, commands, artifact hashes and scope. Full package,
  PostgreSQL, durable replay/concurrency, native-content and production-authority
  qualification were not performed and are not claimed.
- Whole-diff review confirms only the three allocated paths, no staged content or
  commit, checked arithmetic without saturation/truncation, no duplicate placement
  planner, and no source/authority/persistence/registry/schema/wiring invention.
  The concurrency case is pure changed-input arithmetic, not an executed lock or
  durable fencing test. Actual authority-consuming boundaries remain future work.
- Future module wiring must reconcile the test's source inclusion with repository
  duplicate-source policy; this batch adds no registration.

Final Rust SHA-256:

- `item_capacity.rs`: `861b1645a804bc31d2b1eba2a66104d9b38e2a9d3cc528b8fe4fae2673535c70`.
- `item_loop_capacity.rs`: `c8e8d9deb9e983279d3de0c9861c55fd60edbffb4289dd798056afcfcca95619`.


## CP D1104 bounded Reference-capacity follow-up (2026-10-16)

Allocation: A owns Capacity + ItemDefinitionFacts; E retains progression. This
follow-up changes only `durability/item_capacity.rs`, `tests/item_loop_capacity.rs`
and this taskcard. It introduces no shared wiring, writer, persisted field, profile
selection or transaction integration. Parent executor owns all Cargo/PostgreSQL
validation and publication; this worker performs neither.

The earlier preparation's statements that no capacity formula/unit conversion is
supplied describe that historical generation. They are superseded only for the
new detached `project_reference_capacity_centi_oz(level, vocation)` helper:

- **PROVEN (source inspection):** the helper loads the same accepted SPELL-D5
  packet as `spell_character_facts::maxima`, including Formulae revision 1205374:
  `tools/content-schema/spell-authoring/samples/vocation-vitals-candidate-2026-09-28.json`,
  SHA-256 `78c7a01e329757251a63f0ffa1cb65fdb3682a163e88b2e4dc615a3e74798864`.
  Rookie boundary and capacity coefficients come from that packet, without
  duplicating guessed coefficients. Checked whole-ounce formula evaluation is
  followed by exactly one checked multiplication by 100 into u64 centi-ounces,
  the item-weight unit accepted under #1336/D207. The existing planner's u32/u64
  arithmetic performs no additional conversion.
- **CONFLICT / still open:** Reference DATA ONLY is neither current Character
  authority nor an admitted Character baseline. Newhaven gives 600 oz at level 1
  while Formulae gives 400 oz; the existing Character conditional 200 oz remains
  an unresolved CP profile/provenance choice recorded in
  `docs/architecture/GAME-CHAR-01_STAGE_B_REFERENCE_EVIDENCE_RECONCILIATION.md` §9.
  No universal 200 oz adjustment, new state or profile
  decision is introduced. Production capacity admission and transaction integration
  remain gated.
- **DERIVED (test cases authored, execution delegated):** all eleven base/promoted
  and no-vocation keys at levels 1, 7 and 8; differentiated post-boundary levels 9
  and 20; unknown vocation, level zero, exact centi-ounce fit/one-centi-ounce
  overweight, and u32::MAX levels without u32 truncation or clamping. These are
  pure reference-data cases, not live Character or transaction-authority evidence.

No Cargo, PostgreSQL, rustc or Clippy was run by this worker. `rustfmt` was absent
from the worker PATH; targeted formatting and executable checks are delegated to
the parent executor. Earlier validation/hashes above belong to the earlier
planner generation and do not qualify this follow-up.


### Parent executor qualification of the Reference follow-up

Actual Rust/Cargo 1.94.0 native execution, original parent
`c088872471ebb68f77674857878718455dbf99c5`, with unchanged reviewed Rust bytes:

- `cargo test --locked -p oteryn-game-server --test item_loop_capacity --quiet`:
  **16/16 PASS**, including the four new Reference groups.
- Focused strict Clippy and required
  `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`:
  **PASS**. `cargo fmt --all --check`: **PASS**.
- Required `cargo test --locked -p oteryn-game-server --quiet`: **PASS**,
  58 result groups, **9801 passed / 0 failed / 37 ignored**, including doctests.
  `OTERYN_TEST_POSTGRES_ADMIN_URL` was explicitly unset: this is normal package
  qualification, not actual PostgreSQL or current-authority qualification.
- Governance validator: **PASS** (22 documents /9 lanes). This branch's distinct
  `tools/agents` tree `8fd789afc6d9e9cbcf7f0e112b1d08f97ca97310`
  was tested directly: **124/124 governance unit tests PASS**.
- Independent bounded review of the exact two Rust source hashes: **PASS**,
  no actionable findings. `git diff --check`: **PASS**.

The first package build with two build jobs hit the actual 8 GiB memory ceiling
(no OOM). The executor stopped only its second compiler, retained the completed
first compiler, preserved the interrupted build log, then completed the package
with `CARGO_BUILD_JOBS=1`. This was a resource interruption, not a test assertion
failure. Incremental compilation remained disabled; dev/test debug remained zero.
No source, deadline, assertion, PostgreSQL guard or lint was weakened.

Final reviewed Rust SHA-256:
- capacity: `49bb8059764637343b9ca9f3d3c3610285c9ac8c9ebc9201874e4b19765b9090`;
- tests: `1d67fe590d6312ab3dba844bf81bc955978f90c00aef86202867ab9e2bcafb8e`.

Evidence, native ELF, commands and qualification limits are preserved under
`/workspace/oteryn-family/validation/loop-reference-evidence/` and
`/workspace/oteryn-family/LOOP_REFERENCE_QUALIFICATION.json`.
These results qualify the isolated planner/Reference-data change; they do not
select the outstanding Character baseline or qualify production transaction
wiring, durable concurrency, shared equipment paths or the complete ITEM-LOOP-1.
The PR remains a draft; shared-slot and final integration gates remain binding.

### D1199 derived Global capacity — NOT_FULLY_QUALIFIED

CP D1199 (comment6225695524) supersedes D1163/D1184 imported-state authoring.
Oteryn has no imported Characters or stored capacity. Migration0106 and the
private durable participant proposal are withdrawn; no E progression call or
death hook is required. Historical checkpoints remain evidence of their own bytes.

The pure Global projection uses current level and vocation with the accepted
Formulae1205374 profile (including its rookie boundary), raising the common base
from400 to600oz. Its result is u64 centi-ounces. All11 native vocation keys,
rookie/vocation boundaries, invalid profiles, u32::MAX level, exact capacity and
level-up/loss consumer cases are covered. Weight inputs are already-normalized
Option<u32> centi-ounces, bound to the exact definition; no Content dependency,
second conversion, stored total or imported-state enum remains in this helper.

Actual transfer enforcement still requires complete current equipment/backpack
weight facts from compatible admitted Content under freeze/commit locks. Current
ItemDefinitionFacts and generation_item_facts omit weight and the request lacks
the complete carried-definition closure. CP request6225875673 asks for the exact
producer/caller allocation and binding compatibility decision. No invented
provider, client total, unconditional transfer refusal or admission bypass is
installed. A private locked native-reader proposal is preserved separately and
is not production integration or a PostgreSQL PASS.

Actual native `cargo test --locked --offline -p oteryn-game-server --test
item_loop_capacity --quiet -- --test-threads=1` passed21/21,0failed/ignored/filtered,
with the actual registered library and native types (PG URL unset). Independent
source review, changed-file fmt, diff and governance PASS. Full package/all-target
Clippy/PG/runtime and new exact-head CI are NOT_RUN. Previous strict target Clippy
RED from unused production helpers remains open; the current native build again
reports unused-helper warnings. No lint/visibility weakening is applied.
D1122 permits the disclosed checkpoint on the existing draft; runtime acceptance,
full package/all-target qualification and final review remain open.

### D1211 native weight admission — NOT_FULLY_QUALIFIED

CP comment6225880755 assigns the weight producer, complete carried-definition
closure and API callers to A under D1164. ItemDefinitionFacts now carries
Option<u32> centi-ounces, surfaced only from nested Known physical/weight facts.
NativeGameplayState implements the definition-keyed ItemWeightLookup port; its
source digest binds the request and the lookup at both freeze and commit. No
client total, extra unit conversion or missing-weight zero is accepted.

The existing transfer TX reads actual progression/build and recursively weighs
all equipment roots and backpack descendants while holding the native root and
instance locks. Invalid custody/world/duplicate/cycle state refuses. Incoming full
quantity is added once through the reviewed pure planner. WeightUnknown,
WeightGenerationMismatch, WeightDefinitionMismatch and Overweight refuse before
new item mutation; the transport maps them to existing Rejected, with no new IDs.
Current level and custody are re-read at commit. No stored capacity or migration.

TRANSFER-only intentv2 binds normalized weights and the source generation. Shared
push_facts remains byte-identical for NPC/fee/reward consumers. Matching terminal
v1 receipts replay their original result before current weights are consulted;
outstanding v1 reservations conflict rather than silently adopting v2 semantics.
Existing canonical Reference pickup receives a Content-source-bound compatibility
adapter; that older seam still has no production caller. Other API callers receive
minimal field/signature compatibility, without unrelated shared-writer changes.

Six native PG cases are authored for exact fit/replay, one-centi over/unknown
incoming, generation change, changed carried custody, unknown carried weight, and
a real XP50→51/death51→1/respawn receipt sequence. The level case uses an explicitly
finite test XP oracle, not a production XP curve. Approved test-only weight facts
remain isolated in tests/support. PG execution is NOT_RUN: this cloud executor
does not have the canonical PG route. Final native compile/unit check receipts,
source hashes and independent review are retained outside the checkout under
/workspace/oteryn-family/loop-global-d1163/native-weight-d1211/. Full package,
strict all-target qualification, runtime acceptance and exact-head CI remain open.
D1122 permits a disclosed checkpoint; no ready/merge or runner changes.

Actual D1211 native qualification receipts: affected production and three real
PG-wrapper targets compiled successfully (item_transfer_postgres, item_ref_production,
character_authority_postgres). Native producer/admission tests passed8/8 with0failed/ignored
(PG URLunset). Initial full all-target checks were RED (E0277 then six E0505 cleanup
borrows); failed source bytes and logs remain retained. All reported compile defects
were repaired; a later successful targeted check is not a full all-target PASS.
Generation-only PG fixture correction preserves valid weights and terminal replay;
its fresh targeted check and transport mapping test receipts are captured separately.

CP D1230l6226522623 resolves the baseline: no-vocation cap(L)=600+20*(L-2)oz.
Level2=600oz and level3=620oz are PARITY_GUI; level1=580oz is PARITY_DERIVED,
including actual death delevel. The checked u64 centioz projection implements
56000+2000*L without changing the Reference helper or profession rookie profiles.
The six authored PG scenarios now use level50=156000 and level51=158000;
exact/over/level51 fixture weights are155999/156000/157998centioz respectively.
Independent source review and fmt/diff/governance PASS. New22pure qualification
is recorded separately; actualPG remains NOT_RUN, and full package/all-target
strict Clippy/runtime/exact-head gates remain open. The former baseline conflict
is resolved; this does not replace those missing executable gates with a PASS.
No new item-weight conversion or E progression participant is requested.

Additional actual native evidence: item_move transport tests32/32 PASS,0failed/ignored;
generation-only negative fixture target compiled PASS after its independent repair.
The original affected-three-wrapper compile PASS and producer8/8 PASS retain their
exact immutable source scope; D1230l projection qualification uses fresh22 tests.

Final D1230l native projection target passed22/22,0failed/ignored. Current
affected three real wrapper targets compiled PASS after that same projection
change (exit0, logSHA85329ba3c28c03718c4ccdcdc0d78e96a2be7275b22e858ba54f79d9a1936554).
The immutable producer8/8 and transport32/32 receipts remain valid for their
unchanged source; they are not a claim of current full-package execution.


### D1141 native whole-item equipment swap — local NOT_FULLY_QUALIFIED

The existing equipment owner now prepares whole-item exchanges into occupied
slots under its current Character/equipment/item/backpack locks. The incoming
item replaces the target occupant; the occupant receives a fresh immutable main
backpack entry. Count and quantities stay unchanged, both item revisions advance
once, and one equipment revision/receipt/audit commits atomically without changing
CharacterRevision or charging carried capacity again. Other hand/group conflicts,
current fences, Content pins, slot requirements and plain/non-timed checks remain.

Different definitions exchange as whole items. Same-definition exchange requires
both actual quantities to be one and both classes Known NonStackable. The
existing owned NativeGameplayState producer reuses generation_item_facts for
exact inventory-admitted definition/class facts in the same active generation.
Unknown-class same-definition exchanges and stack merge/top-up shapes remain
refused; no synthetic class,
new conversion, alternate Content authority or guessed wire ID is introduced.

Only swaps use the v2 event with both exact item observations and four actual
location lines. Its immutable resolved facts are SHA-bound before writes; terminal
replay does not reconstruct the departed occupant as current. Existing v1 bytes,
replay SELECT and non-swap SQL column lists remain unchanged.

The real existing item_ref_production wrapper contains a new authored grouped PG
case for full-backpack exchange, independent other-hand conflict, stale connection
generation, quantities/revisions, immutable replacement ordinal, complete audit,
replay/substitution, unknown same-definition refusal and known non-stackable
same-definition exchange. The fixture never installs external SQL. Actual PG,
fullpackage, strictalltargetClippy and runtime qualification are NOT_RUN for this
increment; compilation/cheap receipts are recorded separately outside the checkout.
Earlier native projection/producer/transport PASS retains its unchanged-source
scope; it is not runtime proof for the new swap.

CP D1282 (#2131/6227784667) assigns additive migration 0113 and the narrow
shared guard/resource rows. The accepted proposal is now installed as
0113_equipment_swap.sql; the fixture uses the canonical sqlx migration set,
without external SQL injection. Full latest 0083 item guards, 0045 slot guard,
0035 receipt guard and unchanged 0076 seven-location exclusivity are preserved.
Stack classes are receipt-attested current Rust Content observations bound to
content_digest; SQL checks receipt consistency, PARITY_PENDING for independent
Content parity. No SQL Content oracle is claimed. EQUIP-SWAP touched=2,
locations=4, participants=2/work=6 are registered; DUR03-RL-07 remains unchanged
and worst-case measurement remains NOT_RUN. Stack merge/top-up is unfinished.

The remote bb468ef9 candidate stays frozen for the already assigned D1267/D1271
B qualification. Publication of this local swap plus 0113/fixture/registry waits
for that run to finish, then one coherent push. The new head's targeted PG/native
swap qualifier is queued on B after #2191, one start/no retry. This local increment
remains NOT_FULLY_QUALIFIED; previous compilation receipts cover unchanged Rust
bodies, not execution of migration 0113 or the new PG case.

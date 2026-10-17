# HOUSE-RUNTIME-1a — house interior shapes and item placement (skeleton task record)

```yaml
task_id: HOUSE-RUNTIME-1a
title: "HOUSE-RUNTIME-1a: house interior shapes and item placement"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row HOUSE-RUNTIME-1a; §7 'HOUSE-RUNTIME-1a/1b/1c' (HOUSE_RUNTIME0 ACCEPTED)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/house-runtime-1a
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## D1168 loader authoring — NOT_FULLY_QUALIFIED

An isolated authoring successor of `e061be95895b25a61dc7cc1771d2b9711e1fc092`
adds `load_house_interior_in_transaction` in the owned durability module and an
EXECUTE-only native read function in allocated migration0095. The caller supplies
its existing physical transaction, actual native `HouseId`, expected generation
and `NodeIncarnationProof`. SQL independently proves secret possession and holds
the native current registration and exact World/House assignment SHARE locks,
using the existing incarnation-before-assignment order until that transaction ends.
No pool, recovery authority or supplied current-access boolean is introduced.

Locations, exact Item identity/state and matching provenance come from one SQL
statement snapshot. Missing/mismatched item/provenance and nonempty containers
refuse the load; a 2001st row refuses the entire result rather than truncating it.
The runtime receives EXECUTE on this bounded function, never SELECT on the private
tables. The function has a fixed owner-schema search path and PUBLIC/control
execution is revoked. Secret bytes remain one separate SQL parameter.

This is point-in-time snapshot data, not reusable admission, ACL, current placement
or future-write authority. Native placement/rebind and runtime installation are
still pending their D1168 owner. Three additional PG cases are authored for exact
currentness/identity refusals and empty state, a real native committed placement
snapshot, and registration/assignment locks held through caller rollback.
Official Rustfmt1.94 focused formatting and `git diff --check` pass. Compiler,
Clippy and PostgreSQL execution remain NOT_RUN; independent review and actual
bounded executor qualification are required before freeze/publication.

## Goal

Durable house interior shapes and item placement inside a house through the one-item transaction, with custody and audit (packet §2.1).

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

First of HOUSE-RUNTIME-1a -> 1b -> 1c. Releases TIMED-HOUSE-1, MAP-OVERLAY-1c and WRITE-1 house items.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| SCOPE-HANDOFF-1 | MERGED (#1782) |
| HOUSE-CUSTODY-1 | MERGED (archive OTV2-20260930-house-custody-1) |
| HOUSE_RUNTIME0 | ACCEPTED (plan §7 status list) |
| one-item operation tag and migration number | leased by CP at the slot |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/house_interior.rs`
- `apps/game-server/src/durability/house_interior_audit.rs`
- `apps/game-server/tests/house_interior_postgres.rs`
- `apps/game-server/tests/support/house_interior_postgres_cases.rs`
- `apps/game-server/tests/support/house_test_fixture.rs`
- `docs/agents/tasks/HOUSE-RUNTIME-1a.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/NNNN_house_interior_shapes.sql
- durability/item_mint_audit.rs arm
- native_one_item_transaction.proto (one-item operation tag, leased by CP)
- character_authority_postgres.rs path-mod line
- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0091)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- durability/mod.rs registration
- durability/item_mint_audit.rs operation arm
- native_one_item_transaction.proto oneof (tag leased by CP)
- GAME_EVENT_FOUNDATION_REGISTRY.json and RESOURCE_LIMITS_REGISTRY.json rows

## Inseparable bundles (never split across PRs)

- migration + house_interior writer + audit arm + proto tag + registries (packet §2.1)

## Acceptance criteria

- [ ] Placing/removing an item in a house is one one-item transaction with audit; no duplication under concurrency.
- [ ] House custody (HOUSE-CUSTODY-1) respected: OWNER, SUBOWNER and GUEST may place items from their own inventory/equipment with a current storage/access grant; placement without that current grant is refused with a typed code.
- [ ] Interior shape limits enforced via RESOURCE_LIMITS.
- [ ] State survives restart.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server house_interior
cargo test -p oteryn-game-server house_custody
cargo test -p oteryn-game-server item_transfer
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
- docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md §2.1
- PR #1782; archive OTV2-20260930-house-custody-1
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.


## Local preparation — 2026-10-15 (unpublished)

- Source: `a9e0277c8c79071bcfe7b8aa7ec7b78f2592232d`, existing draft #2163,
  `skeleton/house-runtime-1a`. Local diffs only; no commit, push, freeze, ready flag,
  integration, migration, tag lease, registry or content change is authorized here.
- HOUSE_RUNTIME0 is accepted through merged #1771 despite its historical CANDIDATE
  heading. SCOPE-HANDOFF-1 and HOUSE-CUSTODY-1 are merged. Packet §2.1 has
  `candidate_bases: []`; 1b/1c, HOUSE-1a, MAP-LOAD and ADMIT do not block this early
  structural preparation. Production access remains migration 0074's no-row stub.
- Prepared `house_interior.rs`: closed house-side placement/take/tile-move facts,
  location bounds, item/house/placer/placement binding, same-house scope-generation
  equality, reclaim preservation and checked +1 provenance revision. The §5.2 role
  table is a pure policy rule, never a current-authority token or a commit gate.
- Prepared `house_interior_audit.rs`: immutable house-side audit relationships only.
  It allocates no protobuf tag and provides no envelope encoder/decoder. The character
  source/destination, complete event and worst-case encoded bounds remain slot work.
- Prepared the standalone test wrapper, focused semantic cases and §10 test-only
  access fixture. The fixture is guarded to a dedicated `house_interior_*` database,
  test administrator and PostgreSQL 17.6; it reads independently supplied fixture
  properties/grants under FOR SHARE. It supplies no session/scope authority and no
  shape function. Production access, table privileges and all migrations are untouched.
- Acceptance boxes above remain open. These sources do not implement or qualify a
  production shape writer, event round trip, restart/replay or live-authority fence.

### cross_domain_finding — required indivisible slot bundle (REPORT_ONLY)

The accepted packet §1.3/§2.1 requires three SECURITY DEFINER shape functions, runtime
EXECUTE-only grants and transaction-local insertion/removal proofs. Those changes
belong to the slot-only migration, not to the owned Rust files. Existing `0011`
reservation `destination_kind` and receipt `shape` constraints, its placement proofs,
`0014`'s `game_item_transfer_consistency_guard` and source evidence must gain the house
clauses and nullable ChannelId without weakening their existing bodies. Migration
`0025` currently forbids runtime table writes and requires same-transaction provenance;
`0074` supplies only the fail-closed access stub. Implementing a Rust table writer or
inventing future SQL functions here would bypass that accepted closure.

CP/architect qualification must approve the complete shape signatures/transaction
owner, tile-source signature, leased operation tag and additive event grammar,
reservation/receipt changes, retained provenance proof, registry bounds and module/test
registration. When the cases join the protected aggregate, `apps/game-server/Cargo.toml`
must also mark the standalone wrapper `test = false` under the repository duplicate-test
rule; that manifest is outside these owned paths and needs CP routing. Until the slot,
none of these integration edits are implemented or assumed by this preparation.
The future writer must preserve recovery/admission → cause/replay → live session,
assignment/incarnation and guards → character_root → locked house access → item-id
order, with house scope generation and all authorization revisions re-read in the same
commit. The current-session generation must be resolved independently of prepared
command/audit facts. Missing/stale facts, session/scope replacement, ACL/guild revision
changes, provenance substitution, transaction reuse, concurrent item moves and
restart/replay all remain required writer qualification cases; structural tests do
not discharge them.

### Local validation evidence

Final source-accurate isolated validation passed: 7 structural cases and 1 real
PostgreSQL 17.6 case (all existing migrations, no-row production stub, runtime table
privilege closure, fixture roles and ACL revocation lock conflict). Isolated Clippy
passed with the native root lint table; that isolated run did not compile the server
library. The later native-wrapper qualification below uses the real server library.
Workspace formatting, diff whitespace, governance validation and 124 governance unit
tests passed. Initial offline patch resolution and unconditional test mounting failures
were outside-tree harness setup issues; their logs are retained separately, not counted
as product RED evidence.

The outside-tree harness mounts unchanged prepared sources, literal native dependency
excerpts and the real protocol crate; it records exact source hashes and excerpt ranges
in `/workspace/oteryn-prep/house-runtime-1a-evidence/sources.json`. Separate semantic,
PostgreSQL and Clippy logs plus the actual test binary are retained there. This is not
whole-server Clippy, protected CI, independent review or final writer qualification.
CP and architect qualification remain required before any slot.


### Documentation repair — 2026-10-15

The coordination review verified a pre-existing P2 in the acceptance checklist:
“non-owner placement refused” contradicted accepted HOUSE_RUNTIME0 §5.2. The
checklist now explicitly allows OWNER, SUBOWNER and GUEST to place from their own
inventory/equipment with a current storage/access grant, and requires a typed
refusal without that current grant. The writer must still independently re-read
current authority and authorization revisions under the accepted transaction locks.

This repair changes only the checklist and this note. The correct Rust policy,
structural validator, audit facts, tests and fixture are unchanged; all acceptance
boxes remain pending. All five Rust file hashes match round-1 RESULT.json and the
source-bound retained scope evidence; the four directly mounted task sources also
match sources.json. No Rust, PostgreSQL or Clippy rerun is required or claimed for
this documentation-only correction. Prior runtime evidence retains its original
limited scope; CP/architect and slot qualification remain pending.


### Native standalone-wrapper qualification — 2026-10-15

The initial precautionary wrapper removal was reversed after reading the actual
`check_duplicate_test_sources.py` implementation: it checks test-bearing sources
compiled into multiple test targets. An auto-discovered standalone wrapper without
a `test = false` entry is not, by itself, proof of duplication. The unchanged
wrapper is restored for qualification through the real server library; none of the
four implementation/support Rust files, Cargo manifests or integration owners change.
The byte-identical outside-tree wrapper copy remains reconstruction evidence.

The first native invocation stopped before compiling the wrapper or running tests:
the restored sparse clone lacked 58 unique baseline include inputs, producing 60
missing-file compiler diagnostics. Every missing input is tracked at the exact base;
this is a clone setup failure, not a wrapper/import defect or product RED. The exact
restoration list is retained in `house-runtime-1a-evidence/native-wrapper-missing-inputs.json`.
Root restored every missing input from the exact base, then the corrected command
`cargo test --locked -p oteryn-game-server --test house_interior_postgres --quiet`
passed: **8 passed, 0 failed, 0 ignored, 0 filtered out** (7 structural cases and
1 real PostgreSQL 17.6 fixture/closure case), in 1.21 seconds of test execution.
It used the actual server library and native public types, without type substitution,
under the shared Rust lock, shared target and two build jobs. The first native log
is preserved as `native-wrapper-setup-failure.log`; the passing result is retained
as `native-wrapper-test.log`. This is additional standalone-wrapper qualification,
not a reinterpretation of the earlier isolated harness results.

The byte-identical standalone wrapper remains in the publication patch. No actual
duplicate-source violation was established; the guard's 13 Python tests passed,
while the complete compiled CI source-graph guard remains CI work. When these cases
join a protected aggregate in the CP slot, wrapper/Cargo/aggregate registration
must prevent actual double compilation of test-bearing sources. No Cargo or shared
module wiring was changed. This does not qualify the future shape writer, registered
event, session/scope mutation fences or full package; all acceptance boxes remain pending.

## CP implementation slot — 2026-10-15

This section supersedes the earlier preparation-only scope statements for this
slot; the historical evidence above is retained unchanged. CP #162 comment
6196051193 allocated #2163 (initially 0089; 0088 belongs to #2208). CP #2131
[6224954807](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224954807)
and [6225069600](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6225069600)
supersede that number with migration **0095**.
The exclusive integration base is `c04ab3fbfad8b7cdbb6310342c06b3fadf6a4f0d`,
combining the published preparation with main `b62a5595b6be426dd4c9cc016c9b3883e9193c9d`.
The exact 15-path lease also includes the shared audit/proto/registries, module and
aggregate registration, Cargo wrapper registration, and the six-line crate-private
Node proof secret accessor. No content, pins, connection, capability, client, other
migration or unrelated authority owner is changed. The worker makes no commit,
push, ready, queue or merge action; the owner requires the existing PR to stay draft.
All acceptance boxes remain pending exact-head CI and independent architect review.

### Integrated writer and database closure

- `freeze_house_item`, `commit_house_item` and `reconcile_house_item` reuse the native
  Root/recovery/semantic transaction and type-2 activation owners. One immutable
  session/command reservation owns the TransactionId, EventId, DB Unix-millisecond
  timestamp, exact intent and durable three-unit retry/reconciliation budget.
- Historical committed receipts are read before current access/session checks or
  new work. The permanent receipt retains IDs/time/hash; its audit payload is optional
  after outbox retention. EventId-only LEFT JOIN distinguishes absence from a present
  mismatched Tx/hash/envelope, which fails closed. Missing payload never becomes a
  missing receipt, another transaction or renewed authority.
- A closed `prepare` phase runs the same independently resolved authoritative SQL
  checks as `apply`, then returns before durable DML. A refused preflight rolls back
  its transient reservation. A successful preflight is retained intent, never a grant;
  apply rechecks every current fact after its durable work charge.
- The three runtime-EXECUTE SECURITY DEFINER shapes read actual named House session,
  Character/account/World/live root, connection generation, Character lease, House
  assignment/generation and the native node possession predicate. The accessor only
  borrows opaque secret bytes for a bound SQL parameter; no new predicate, token,
  constructor, serialization or secret logging is added. Historical handoff lineage
  and old Channel admission guards are not substituted for current House authority.
- Locks retain native recovery/admission and cause serialization, current session/
  assignment, Character root, House access/count, then sorted ItemInstance rows.
  Access cardinality must be exactly one; role, content fence, ACL/guild revisions,
  actual House tile membership, source/provenance and capacity are checked in the
  physical commit. OWNER/SUBOWNER/GUEST own placement and guest own-only take remain
  the accepted role rules. No CharacterRevision or quantity/definition is changed.
- Private owner-produced same-xid shape evidence closes actual custody, retained
  reclaim provenance, one receipt and one audit event. All seven latest native guard
  bodies are retained verbatim after additive House clauses: instance guard **0075**,
  transfer consistency **0076**, item/container/placement union **0083**, equipment
  slot/state **0045**. The source-binding manifest records every original suffix.
- B3 slot removal replaces only that table's old immutable trigger with an exact
  kind-2/shape-5/slot-9/item/actor/World/old-placement/current-xid proof check and
  deferred complete receipt/event/destination proof. UPDATE and unproved DELETE
  retain `23514`; generic immutable consumers and runtime DELETE privileges are
  unchanged. Regular equipment removal advances the real equipment revision once,
  using the same House event, without a fabricated equipment receipt.

### Audit, limits and real registration

The additive `house_transfer` oneof uses **candidate tag 11**, the next-free value
prepared for CP/architect review, not a claim of an independently reserved number.
Existing operation fields/tags and schema/retention tuples are unchanged. The closed
codec binds destination placement Tx to the envelope Tx while allowing a historical
take source Tx, preserves reclaim identity and WorldId, and excludes ChannelId.
Canonical decoding rejects duplicate/unknown fields and malformed endpoints.
The maximal admitted House tile-move fixture measures **5,543-byte payload and
5,913-byte envelope**, including a 128-byte build ID, below unchanged 7,936/9,216 caps.
RESOURCE_LIMITS records these consumers and accepted **HOUSEOWN0-RL-14 = 2,000**.
The native modules and authority aggregate are registered; the standalone wrapper's
Cargo `test = false` prevents default duplicate execution of its shared cases.
No inactive `cfg(any())` substitute or production type replica is used.

### Source-bound qualification and limits

Outside-tree native validation compiles unchanged real Root/native authority sources
and dependencies, with literal test-module mounts and a recorded dispatch bridge;
selected test bodies are unchanged. Original untransformed native-unit Clippy with
`-D warnings` passed. A normal native `cfg(test)` rlib and tiny test driver avoid the
known global unit-harness ELF memory failure; this is not a whole-package runtime
or full global-unit run. The borrowed item-transfer fixture mount reproduces its
actual outer target's existing `expect_used` allowance; support bytes and global
lint denies remain unchanged.

`attempt-5/pg-tests.log`: **21 passed, 0 failed, 0 ignored, 0 filtered**, 15.17 seconds:
10 owned pure cases, 9 actual PostgreSQL **17.6 / 170006** cases and 2 unchanged native
legacy golden/retention-tuple controls. Actual cases include concurrent two-Root
commit/replay; restart and durable retry cap; provenance substitution; current fence,
ACL/closing/tile changes; privileged SQL wrong/stale node possession; duplicate access;
B3/equipment sources; production-stub no-trace preflight; role rules; and capacity.
Audit-retention probes follow a genuine commit, restore all guards before operational
replay and independently corrupt/remove only test audit DATA. They are not a real
90-day pruning-clock or end-to-end collector qualification.

Capacity uses independently seeded **inactive occupancy DATA** with real FK/provenance
and placement ledgers, restored guards, and a genuine active actor/native mint/transfer/
House commit for **1,999 → 2,000**, then StorageFull with unchanged source/event/reservation.
The foreign reclaim subject is inert fixture DATA, never current actor/session/node/
access authority. These tests do not claim 100-Character load, runtime dispatch or
production admission. Positive property/access/tile facts are explicitly §10 fixture
DATA; production `game_house_access` remains the **0074 no-row stub**, and the new
`game_house_access_tile` returns **false** until its accepted owning property source.
HOUSE-VIEW owns the future content-revision trigger; 1b/1c, entry/exit admission lift,
House/client wire and real property/guild authority are not implemented here.

Earlier failures are retained accurately: missing tracked include/handoff inputs and
validation mounts are setup failures; owned compile errors were repaired. Attempt 3
actually ran **11/8** (seven BYTEA fixture binding errors and a 513-byte maximum fixture,
corrected without weakening validation). Attempt 4 actually ran **20/1**: the genuine
native B3 immutable-trigger defect, now repaired and qualified by attempt 5. The peer
source findings for the latest guard union, destination Tx binding, grant cardinality,
preflight no-trace and retention identity have explicit repaired dispositions. No
superseded run is presented as qualification of changed bytes.

Durable final evidence is under
`/workspace/oteryn-prep/house-runtime-1a-slot-1-evidence/`: native commands/results,
source and bridge manifests, retained RED outputs and lossless artifact archives,
registry delta, exact native guard bindings, final scope/patch and source hashes.
Production/registered-target metadata, formatting, architecture and governance checks
are recorded separately there. Exact-head protected CI, full compiled source-graph
check and independent architect/CP qualification remain required before readiness.

## Exact-head CI source-guard repair — 2026-10-15

Published candidate `6b4b23f848675361553813b9bb7429a5305aaa55` reached the full native
unit suite in CI run **38779927939**, job **116577926281**: **3,172 passed,
1 failed, 29 ignored**. The sole failure was
`every_type2_insert_takes_its_tuple_from_a_type2_transaction`: its actual insert-site
scan found the new `src/durability/house_interior.rs`, missing from the exhaustive
expected list. The tuple-selection and compile-time-constant assertions had passed.

The repair adds that one path in sorted order. All prior sites, production walker,
statement extraction, tuple/encoder checks and other source remain unchanged.
Source-bound std-only Rust validation executes the exact original whole test and
its three unchanged helpers over all **555 actual native source files**: original C
is RED on the missing expected site; the repaired list is GREEN. Two isolated source
mutants still fail the original assertions: missing selected schema tuple and a
compile-time retention tuple even with both selected-tuple calls present.

Validation sources/ranges/hashes, complete source manifests and raw results are in
`/workspace/oteryn-prep/house-runtime-1a-slot-2-evidence/`. This is a targeted source-
guard repair, not another full native or PostgreSQL run. Production, SQL, proto,
registry and the earlier component evidence are unchanged. The new candidate still
needs exact-head CI/required review; the PR stays draft and only root publishes it.

## Family authoring and qualification repair — 2026-10-16

The user directly confirmed exclusive authorship of this family in this chat:
the previous team will not write these branches, and no additional CP lease number
is required for current owned paths. The lead is the sole author of this checkout.
This pass explicitly returns to AUTHORING from published reference
`967d972d29feba8307cff84f3f3b7929dea240cb`; it has not selected a new qualified head.
No shared file, migration, registry, wiring or content is changed.

The current-main read is `24fc332f256638e3b784bff1b3c85b18e2665a76`.
The older base/migration-number statements above are historical slot provenance,
not assertions about current main. The published exact head has green game-gate
and 18 successful checks plus one skipped check, as freshly read from GitHub.
Those checks do not qualify the source changes in this authoring pass.

Two independent nonauthor source reviews covered the published complete bounded
scope, including its actual callers, SQL guards, audit codec, resource registries
and test registration. They found no demonstrated production authority bypass but
identified missing uncommitted-apply authority and distinct-command/concurrent
revocation qualification. The repair adds three literal native test bodies:

- A 27-case matrix: Place, Take and TileMove each run in fresh databases with one
  independently changed current connection generation, Character lease, terminal
  session, native assignment replacement, native assignment revocation, node
  revocation, guild revision, ACL revision or missing access row. A valid frozen,
  uncommitted candidate first attempts the native writer, then the matching SQL
  apply under the runtime role using original expectations and actual fixture-node
  possession. Full value/provenance/receipt/event rows remain unchanged; retained
  reservation identities remain fixed and exactly one retry unit is consumed.
- Three distinct-command races against genuine source custody, with both candidates
  frozen before commit. Exactly one commits and the other is SourceChanged. Checks
  bind winner receipt/proof/event, item revision, custody, placement transaction,
  reclaim subject/revision and ordinals; losing reconciliation adds no value effect.
- Both orders of actual ACL revocation versus native apply. PostgreSQL blocking
  observations establish the interleaving. A test-only scheduling trigger preserves
  every existing guard; its connection is closed before joins wait, including on
  observation errors. No synthetic current-authority record or guard disabling is
  used. The common SQL access-lock boundary precedes all three shape branches;
  interleaving uses Place while the authority matrix exercises each actual shape.

Session deletion is not an applicable operator because committed handoff FKs retain
the session; terminal state is exercised instead. Physical assignment deletion is
forbidden by its monotonic/receipt guard; native Revoke supplies missing authority.
Expected facts are never reconstructed from the negatively mutated current row.
All owned PostgreSQL entry paths now error on missing/non-Unicode canonical test
configuration instead of returning success without execution. Exact PostgreSQL
17.6 checks and all original test bodies/semantic assertions remain retained.

Source-review repairs were batched: an initial comparison wrongly assumed
HouseWriterError implemented PartialEq; explicit fieldless variant matching fixes
that test without modifying production error types. Race identity assertions and
actual revocation scheduling were completed before this candidate is frozen.
These are source dispositions, not claims that RED/GREEN execution occurred.

Validation so far: official SHA-verified Rust1.94 rustfmt parse/format with child
traversal disabled and diff whitespace pass. Native compilation, Clippy, all package
checks and PostgreSQL execution remain pending the approved PC executor/global slot
under D1011. No successful execution count is claimed for the added cases.
An external source-bound executor/checkpoint packet is retained at
`/workspace/house-family-handoff/`; it is not an automatic queue or background job.

The registry conflict has a separately reviewed additive-union proposal preserving
all current-main entries and existing maxima; applying it/merging main still needs
the concrete shared CP slot. Candidate tag11 still requires CP/architect disposition.
Production access remains the accepted section10 no-row/false staging boundary.
No additional mechanic is invented for this limited task; HOUSE-ACL, runtime1b/1c,
entry/exit contract and content owners retain their integration responsibilities.
All original acceptance boxes and final CP/CI/merge qualification remain open.

### CP 0095 correction — authoring checkpoint

Only migration numbering, its HOUSEOWN0-RL-14 consumer path and this record change.
No merge/rebase was performed under the current owner prohibition; CP main
reconciliation and any additional D1160/D1162 registration remain pending Root/CP
resolution. Existing House/PG registration stays intact; no new checks or readiness
are claimed.

D1122 checkpoint state: numbering/JSON/whitespace checks PASS; the renamed0095
full-package/PG qualification is NOT_RUN. Historical native component evidence
remains bound to its original source generation; no fresh executable PASS,
ready or merge/rebase is claimed.

## Loader stored-state negative coverage successor — LOCAL_CHECKPOINT / NOT_FULLY_QUALIFIED

The `b8303ef9` production loader,0095 migration and caller-TX API remain byte-identical.
Added three real-PG tests (five isolated database scenarios), all authored NOT_RUN:

- A real native placement provides the source item/location/provenance. Explicit privileged
  stored-state fixture copies grow the physical snapshot to exactly2000 rows, which must load;
  the2001st row must yield StorageFull, never a truncated/partial snapshot.
- After a real one-item control, deleting its provenance or changing only its placement-TX
  binding must yield InvalidStoredState rather than silently omit the physical location.
- After a real one-item control, an actual row in ordinary container entries or corpse
  container entries under the placed item must yield InvalidStoredState.

All scenarios keep the independently registered current node and actual House assignment from
`run_native`; no boolean/mock authority or recovered matching authority is substituted.
`SET LOCAL session_replication_role=replica` is used only inside explicit admin fixture
transactions in each isolated test DB to create otherwise-prohibited stored corruption. It
changes no production DDL, grants, loader checks or authority records; it restores at TX end.
The cloned overflow rows use distinct UUIDv7 item, minted-transaction and placement IDs;
they are negative stored-data fixtures, not claimed committed mint history.

Official Rustfmt1.94 and git diffcheck are source checks only. Compile/native/PG execution of
this successor remains NOT_RUN; no evidence is transferred from earlier source generations.
Publication remains held by the existing CP main-merge requirement versus direct user noMerge;
the local candidate is retained with a verified recovery bundle; no push, merge or rebase is performed.

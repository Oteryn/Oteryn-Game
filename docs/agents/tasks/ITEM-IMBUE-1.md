# ITEM-IMBUE-1 — imbuing (skeleton task record)

```yaml
task_id: ITEM-IMBUE-1
title: "ITEM-IMBUE-1: imbuing"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
issue: 1622
pr: 2144
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row ITEM-IMBUE-1 (row 72); §7 'ITEM-TIMED-1' IMBUE-* rows"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/item-imbue-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Imbuements: content (24 types x 3 tiers, slots, shrines), item-state table with apply/clear/ImbueCause/checkpoints/expiry, ticking runtime and effects, scrolls and etcher, `IMBUE_V1` wire.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

IMBUE-CONTENT-1 (content), IMBUE-1 (hard), IMBUE-RT-1 (hard, combat), IMBUE-SCROLL-1 (hard), IMBUE-WIRE-1 (Reg).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING |
| ITEM-MOVE-2a | BLOCKING |
| NPC-ECON-1 (per child) | BLOCKING |
| GOLD-FEE-2 | MERGED |
| ATTACK-1b | MERGED (#1798) |
| SPEED-1 | MERGED |
| PREMIUM-1 | MERGED (#2110) |
| NPC-TRADE-1 | MERGED |
| ITEM-SEM-USE / USE-WIRE-V1 / ITEM-MOVE-WIRE-1 | MERGED (archive records) |
| acceptance of IMBUE_FORGE0 | ACCEPTED via protected ACCEPT-BATCH; entry and child integration gates remain |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (exclusive bounded root allocation)

- `apps/game-server/src/domain/imbuement.rs`
- `apps/game-server/src/durability/imbuement.rs`
- `apps/game-server/src/durability/imbuement_tests.rs`
- `docs/agents/tasks/ITEM-IMBUE-1.md` (packet/notes)

This exact allocation supersedes the family's broad suggestions. No shared module
registration or source authoring/regeneration path is allocated here.

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

- apps/game-server/src/domain/timed_item.rs and timed_item_host.rs (checkpoint/expiry pattern)
- ability/combat pipeline hook for IMBUE-RT-1
- connection.rs dispatch; capabilities.rs `GATED` (`IMBUE_V1`)

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author
- migration + imbue writer + durability
- gold fee burn + imbuement row + ledger (mint/burn + ledger)
- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] Apply charges fee and writes state atomically; clear is idempotent.
- [ ] Ticking checkpoints survive restart; expiry writes exactly once.
- [ ] Effects pass combat parity tests.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server item
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md §3-§7, §14
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded authoring packet, before code (2026-10-15)

Existing draft #2144, exclusive `skeleton/item-imbue-1` at unchanged base
`470891e2d579d60ebcf8755085ba8a32e825c065`. Root supplied META 3.1.0 and exact
four-path lease; no worker commit, push, freeze, lifecycle or remote mutation.
Accepted-source baseline is primary `3ef900c160c9713092a86bfcd1a8377c7587c898`.
IMBUE-FORGE-0 §§3, 4, 6, 7, 15 supplies the tier/access, row binding and bounds:
3 slots (indices 0..2), remaining 0..72,000,000 ms, checkpoint transaction 10 items,
10 participants and 40 work (one participant plus at most 3 row writes per item).
The 60 s ticking interval belongs to future runtime, not a new clock here.

### Actual residue and source limits

No Rust imbuement state/writer/effect/protocol or SQL table exists on this base.
Item slot metadata exists, but its tier codec Two/Three/Ten cannot express Basic-
only maximum 1. Authoring has 24/72 draft entries, DRAFT_NOT_RUNTIME_READY; canonical
Imbuement and Quest indices are READY_UNPOPULATED. Unknown per-family limits and
conflicting source/effect profiles remain. Draft fees differ from the brief's
reference examples. No candidate fee, category map, source profile or schema is
silently made admitted content. Content changes need their own exact lease.

### Approved useful implementation

Prepare private structurally validated slot/binding records and a deterministic
checkpoint-delta/batch planner, no SQL or installed schema. A row preserves item,
slot, definition key/content revision, category/tier, applying occurrence and
TransactionId; a checkpoint changes only remaining time downward. Content revision
is not a monotone state counter. No duplicate timer is written: native timed lanes
already exist and cannot be blindly reused as imbuement content-revision truth.

Use existing PremiumGate for the Premium-only subcheck. With no native current
quest reader, expose only outstanding access requirements, not an apply grant:
shrine/scroll-create still require qualified quest and item bindings; completed-
scroll apply still requires qualified item rules. No quest bool/proof is invented.
Unknown slot/tier rules fail closed. Structural slot/category checks do not prove
that any source is admitted, an account is current, or a transaction may execute.
Tests exercise missing rules, native closed/open Premium classification, slot/
category collisions, preserved bindings, monotone time and exact batch ceilings.

### cross_domain_findings / pending owner hooks

- Admitted catalogue/Quest reader and Basic-capable item rules are absent. Future
  current Premium must come from the existing fence owner at the real boundary;
  this code never opens a gate. Raw/persisted records cannot reconstruct current
  session, lease, scope, ownership, content admission or quest authority.
- CP must implement the accepted table/guards, apply/clear/expiry audit/receipt,
  new admitted FeeBurnCause arms and item-only fee/ledger composition in one
  transaction, exact reserved revision-set replay/REVISION_CHANGED records, and
  current fences/root/item locks. No guessed fee, event/command/capability ID or
  dead writer is supplied. Native fee and timed models remain their owners.
- The batch planner is not a checkpoint commit or CAS proof. Future owner must
  independently read current rows, compare the complete before binding, make all
  batches commit before declaring an interval complete, freeze ticking/effects on
  failure, retry unchanged live values, and audit-delete expiry separately. Zero
  remaining rows still occupy slots/categories until their audited deletion.
- Combat/effects, capacity's committed-state agreement, ticking, 60 s accounting,
  ambiguous writes, transfer/stop/restart/concurrency and wire/client integration
  remain unqualified. Full family acceptance boxes stay open. No schema is
  installed and no runtime is enabled by this unregistered component.

Durable evidence is preserved directly in
`/workspace/oteryn-prep/item-imbue-1-evidence/` (no private target/ELF copies).

### Local qualification boundary (2026-10-15)

The exact final three Rust sources pass 12 focused tests (12 passed, 0 failed,
0 ignored). A dependency-free isolated harness imports the actual owned modules
and test source by absolute path; it includes four verified literal native source
excerpts for ItemInstanceId/UUID validation and PremiumGate. It does not substitute
validators or grant authority. Open Premium fixtures exercise the existing native
constructor's classification only, not a current fence read or mutation permission.
The initial 12/0 run precedes fixture strengthening (unique per-row applying IDs
and isolated duplicate-slot/category cases); its separate source manifest/log is
retained. The final tests bind tested-source-manifest.json, not that initial run.

A binding's transaction_id is the original applying transaction, not a later
checkpoint transaction. Category is raw projected content data, not a proposed
installed column or a catalogue proof. No timer/state revision counter is added.
The complete before binding is preserved even when applications from distinct
content revisions coexist. SQL/PG, current owner/session fences, durable CAS,
clock/failure suspension, audit/fees, admission, runtime and protocol integration
are not qualified by these local tests. No family acceptance box is closed.

The exact source-bound harness also passes all-target Clippy with warnings denied.
No full-server build is claimed for these unregistered modules. Governance
validation passes for 22 policy documents / 9 lanes; its unit suite passes 124
checks. The durable folder includes tested/final source manifests, four literal
native excerpt hashes, immutable registration-root copies, local.patch and exact
restoration instructions. Whole owned diff self-review found no known issue in
this bounded data-only scope; independent qualification and all owner hooks above
remain required.
Direct rustfmt --check on all three owned Rust files and workspace cargo fmt
--all --check pass. Final source verification confirms tested bytes and unchanged
native/registration roots. No commit or publication was performed.


### Private native checkpoint-storage successor (2026-10-15, authoring)

The root allocated a new, unused four-path successor at existing draft head
`f30fe2a59f0179104948778129a2d5d2dd758f30` to `gm_audit_1`. The prior published
planner packet and its qualification remain history above. This successor prepares
actual private storage under accepted IMBUE-FORGE-0 §§4.1, 4.3 and 15. It adds no
public Root commit/admission/timer entrypoint. A supplied remaining value, even a
monotone one with a genuine equipment fence, does not prove elapsed ticking time.
The actual elapsed owner and its admitted provenance remain a concrete CP gate.

The proposed `game_item_imbuements` table is private and UNINSTALLED. It retains
all original item/slot/key/content-revision/category/tier/applying-occurrence/Tx
facts, one category per item and remaining 0..72,000,000. Its preparation guard
allows only downward remaining updates with every other column unchanged; all
DELETE/TRUNCATE is refused. No apply/clear/expiry API, fee, item/value mutation,
event, new checkpoint identity/revision or CharacterRevision write is introduced.
The future audited applying/clear/expiry owner must compose its genuine closed
proofs before the final schema/grants/migration are installed. Initial private
slot rows in tests are explicitly privileged synthetic DATA, not an application,
Imbuement catalogue admission or elapsed-time source.

`load_checkpoint_in_transaction` consumes the actual opaque native
`EquipmentAuthority`, whose original producer checks recovery, current session,
lease, Channel assignment/Node possession, live Character and independently current
Content in that physical transaction. The unchanged equipment reader rechecks
physical transaction identity and reads/locks native equipment state and real
Item rows. The helper verifies all chosen item names are currently equipped and
locks all retained slots for each chosen item in deterministic item/slot order.
Its returned complete before snapshot remains DATA, never a reusable authority.

`store_checkpoint_in_transaction` replans the supplied batch, so caller-assembled
usage/count/order/parent fields cannot bypass the planner. It re-reads the complete
native equipment snapshot and every selected item's retained slot rows, compares
all before facts before its first update, and applies full-binding/remaining SQL
CAS. A nested savepoint rolls the whole primitive back on any refusal/error even
if a compositor later commits its enclosing transaction. Zero rows remain present.
A stale replay is refused; an eventual elapsed/reconciliation owner must inspect
actual committed after data rather than infer an outcome from a retry error.

Current native equipment supports nine non-backpack equipment slots; its ordinary
main backpack is a distinct owner slot. The real-PG maximum here is nine equipped
items, 27 private rows and 36 work. The accepted planner ceiling remains ten items,
30 rows and 40 work, with its existing pure max/max+1 checks. No synthetic tenth
native equipment slot is created to claim unsupported runtime coverage.

Qualification is still pending. Eight new original PostgreSQL cases are authored
for zero/binding reload, each immutable-before substitution, savepoint rollback
with outer commit, direct monotonicity/deletion guards, independent current native
session/lease/scope/Node/content/physical-TX failures, concurrent stale-before CAS,
actual unequip/re-equip and forged usage, and the real nine-slot maximum. They use
unchanged native item-transfer fixtures, genuine Root recovery, mint/transfer and
equipment writer APIs, and the existing equipment owner's explicit test-only
classification profile pattern. Native guards stay enabled. Test-only Content
activation, ordinary untimed items and raw Imbuement rows do not claim admission
E2E, timed-item support or effects. The fixture grants its isolated runtime role
only SELECT/UPDATE(remaining_ms) on the private table, never a production grant.

Heavy Rust/PG validation remains on the root's resource HOLD. Direct rustfmt and
`git diff --check` pass; no native metadata/build/test result is claimed yet.
Outside evidence will be retained in
`/workspace/oteryn-prep/item-imbue-1-checkpoint-native-2-evidence/`.
Production registration/grants/migration, actual elapsed/fight/PZ/runtime clocks,
60-second accounting and all-boundary flush, failure suspension/effects,
audited apply/clear/expiry, native tenth-slot compatibility and full family
integration/CI/CP remain open. No worker commit/push/ready/merge occurred.
### Native private checkpoint qualification, 2026-10-15

The subsequent actual native run qualified the private storage source above. All **20
original cases passed**: twelve retained pure cases and eight genuine PostgreSQL cases,
zero ignored or filtered, in13.47s. Every PG setup required the configured native admin
route and asserted server_version_num170006. This supersedes only the earlier statement
that these authored cases had not executed; the production/elapsed-owner gates remain open.

The exact owned domain/storage/test source was compiled with the actual native crate,
private Root/equipment APIs and unchanged item-transfer fixture. Outside-tree registration
was cfg(test)-only; existing bestiary fixture registration was reused rather than duplicated.
The sole added fixture expectation context preserves that fixture's original native
clippy::expect_used context. The selected-body transform removes only the twenty test
attributes, changes parent visibility and adds forwarding glue. Whole-file reverse
reconstruction binds the unchanged test bodies and assertions to the driver and raw logs.
The actual storage production implementation has no cfg(test) alternative; its only
conditional section is the existing test-module declaration.

Fresh Cargo capture intentionally stopped at sentinel86/Cargo101 before the root test
harness. Native selected-body metadata/Clippy, normal cfg(test) rlib, driver linking and
execution then all exited0 on that same captured dependency graph. Configured Clippy denials
remain active and baseline Rust warnings remain visible; this is not a blanket-Dwarnings
or original global --test metadata/full-server CI pass. No redundant compile of unchanged,
unregistered production code is counted as qualification of this component. Candidate source
and extern hashes before/after execution matched; shared oom/oom_kill counters remained12/7.

Actual PG coverage includes full before-binding substitutions, wrong requested session,
connection/lease/scope/World/Character/Content and independently registered foreign Node
against otherwise valid native facts, plus genuine opaque authority moved across physical
transactions. These requested-fence tests are not represented as independent retirement of
each authoritative database owner. A genuine native unequip/re-equip changes the equipment
revision and invalidates the old complete snapshot. Downward updates reach zero while
retaining every row and immutable application identity; native equipment and CharacterRevision
remain unchanged. Real direct SQL guard negatives reject rebinding, increases, DELETE and
TRUNCATE. An injected second-row error proves rollback of the first update even when the
caller commits its enclosing transaction. Two independently opened SQL transactions using
genuine native equipment authority serialize one successful CAS and one Stale outcome;
native admission relation locking also serializes them before finer row locks.

The real installed equipment owner supports nine ordinary wearable slots, not ten. Actual
PG qualification therefore uses nine genuine equipped items,27 retained rows and36 work;
the accepted pure planner separately proves10items/30rows/40work and maximum-plus-one
refusal. A genuine native MINT in Ground custody is refused as non-equipped. No synthetic
tenth equipped slot, timed equipment admission or elapsed-time correctness is claimed.

Independent Item review found one pre-execution fixture race: concurrent recovery opens
against the native one-pass Root could fail before CAS. Recovery is now genuinely reconciled
sequentially before joining; each physical transaction still obtains its own unchanged
native opaque equipment authority. The correction was source-reviewed before this passing
run. No behavioral RED was fabricated for the earlier authored fixture.

Durable evidence is /workspace/oteryn-prep/item-imbue-1-checkpoint-native-2-evidence:
attempt-1 contains actual commands/status/logs, exact input and literal-macro provenance,
cfg/body transforms and source/extern/resource manifests. Completed custom metadata, rlib
and driver may be losslessly archived with raw/archive/decompressed hashes and exact restore
arguments; no shared dependency, source or authority artifact is removed. Governance checked
22 required documents/9 lanes and124 original tests; final card/source binding, whitespace,
formatting and exact four-path scope accompany the frozen manifest.

The admitted elapsed/actor-tick producer is still absent. Supplied remaining values remain
DATA, and genuine current equipment authority does not justify their elapsed amount. This
private primitive has no public Root commit/admission method, production installation or
grants, timer/controller/effect, apply/clear/expiry DELETE, value or audit write. The eventual
elapsed owner, numbered schema/security composition, shared registration/runtime hooks and
exact-head integrated CI remain CP work. This packet is a qualified private checkpoint
storage component, not completed ITEM-IMBUE-1, full ticking or ready/merge authority.

### Sparse checkpoint correctness repair (2026-10-16, authoring)

On parent `bc8a8b6f0c5646f6672ec68b86dc80eec91a4508`, the pure planner deliberately
omits unchanged item groups, but the native store required its write-set item IDs
to equal every item in the loaded before snapshot. A mixed changed/unchanged
snapshot, and an entirely unchanged nonempty snapshot, therefore failed with
`InvalidBatch` before reaching the native current-state check.

The store now accepts a canonical changed-item subset of the loaded items. It
still replans the complete supplied batch, refuses any changed item outside that
original selection, and re-reads and compares the **entire** original equipment,
item selection and slot snapshot before its first row update. Omitted rows do not
escape stale binding/time detection. Savepoint rollback, full row CAS, bounds,
immutable application identities and zero-row retention are unchanged. A no-op
batch performs no row updates and still requires the current full before state.

Two new original native PostgreSQL regression bodies exercise mixed/all-no-op
snapshots, unchanged rows and equipment, an omitted item's substituted content
revision, a changed item outside the selection, and a genuinely committed
checkpoint of an omitted item that must make the original snapshot stale without
changing the other item. The source now contains 22 tests (12 pure, 10 PG).
Actual Rust 1.94 native library execution used a disposable validation copy with
test-only module/fixture registration, unchanged real Root/equipment/fixture
implementations and the exact owned source bytes. This is native library-test
evidence in that documented registration overlay, not production installation or
PR-head CI coverage. The earlier 20/20 result still binds its historical source.

The RED run selected all ten PG cases against parent storage: **6 passed, 4
failed**, zero ignored, 3206 filtered, 82.01 s. The two new cases failed at the
predicted sparse-batch refusals: mixed store returned `InvalidBatch`, and the
omitted-item case received `InvalidBatch` where its assertion required `Stale`.
The other two failures were unchanged native Root `RootPassDeadlineExceeded`
outcomes in the nine-equipped-item and complete-before-binding cases. These
timeouts are separate from the two reproduced regression failures. RED panic
cleanup evidence is retained with the exact failed fixture identifiers.

The GREEN run selected all **22 original cases (12 pure + 10 PG): 21 passed, 1
failed**, zero ignored, 3194 filtered, 58.18 s. Both new regressions and the
nine-equipped-item case passed. The complete-before-binding case again failed
with `Unavailable(RootPassDeadlineExceeded)` under the unchanged two-second Root
pass bound. Every PG setup asserted `server_version_num=170006`. This is a
partial qualification result, not a full 22/22 qualification PASS.

The cloud PostgreSQL allocation was changed from one to two CPUs during the
controlled attempt; CPU throttling was measured, but that observation does not
establish the cause of either Root timeout. No source predicate, Root deadline,
schema, guard or lint was relaxed. The canonical owner-PC route passed its V2
preflight with all slots free; a guarded qualification recipe is being prepared
for independent review. No owner-PC test result is claimed yet. Full affected
native qualification, strict Clippy and remote exact-head CI remain pending.

The RED/GREEN ELF, exact storage source and raw logs are preserved under
`/workspace/oteryn-family/validation/imbue-red-evidence/` and
`/workspace/oteryn-family/validation/imbue-green-evidence/`, each with `SHA256.json`.
GREEN storage SHA256 is
`96830f09c222f058d1b78d77be2b8a76208286e16ba56a55063b38e376424cdc`;
unchanged original/new test source SHA256 is
`8b9fc82a7315f173c3374270ef995855441969ea902b859eed48111150130653`.
Direct Rust 1.94 rustfmt and `git diff --check` pass. No full mechanic acceptance
criterion is closed by this checkpoint repair or its partial native run.

This repair introduces no elapsed-time producer or grant. Accepted IMBUE_FORGE0
§6 I1 already admits the Imbue/ImbueClear/ImbueScroll fee sources; the current
shared `item_fee_burn::{FeeBurnCause,burn_fee_in_transaction}` and
`item_fee_burn_audit::FeeBurnCauseV1` implement CharmUnassign only and cannot be
reused unchanged for item-only imbuement composition. The shared
`item_mint_audit::OneItemOperationV1::TimedExpiry` is specifically timed-item
expiry, not an admitted Imbue expiry event. Their actual cause/audit integration,
the Basic-capable content codec and current admitted catalogue/access readers,
continuous eligible elapsed hooks, registration and schema installation remain
the specific CP-owned integration work. Unmerged family prerequisites gate the
slot, not this isolated authoring repair. No commit or publication was performed.


### Terminal private qualification checkpoint (2026-10-16)

The frozen private test input is `PC_SOURCE_PACKET.json` SHA-256
`ce0c7dc10623a9b54971a7b5ddff1f9cf2101aed34b4123efc9f2c8675ea792d`,
full source manifest `75fe38c2dd11e827a26d637c0747722175141c6ff8ea86e8ec88ff2065883e4b`.
Its historical task-card bytes precede this documentation candidate.

Historical combined run `evidence-run-1792167456282407854` in
`PC_FINAL_EVIDENCE.json` passed fmt and strict package all-target Clippy but
failed package tests (exit 101): library 3150 passed / 29 ignored; later
`character_authority_postgres` 1490 passed / 1 failed at
`postgres_target_aggregation::every_postgres_target_runs_its_cases_in_a_ci_run_target`.
No focused PG stage executed in that historical run. Launch 041 subsequently
held before tests on the occupied canonical PG lock (observation 043), neither
a focused test PASS nor a test FAIL.

Actual focused run `evidence-run-1792176419124648929` is retained in
`PC_FOCUSED_FINAL_EVIDENCE.json`, including exact native commands, logs and
hash-verified ELF archive receipts. Runner V3 SHA-256
`6f86ba9abef7c0dbb611a9bb59b93f3fcc39531253f58016ce67907700fac14a`;
recipe V3 `8aedcff0eda55fb0d6b61ffa023aaa118e7bad1bc95e42064d7298bc30dace56`.
Forge passed 9/9, including three actual PostgreSQL 17.6 cases. Imbue passed
22/22 (12 pure + 10 actual PostgreSQL 17.6 cases, each asserting 170006),
including complete-before-binding, with the same unchanged Root deadlines.
Storage passed 7/7 pure native depot tests in the test-only registration overlay;
its four actual PostgreSQL cloud cases remain separate historical evidence.
The earlier cloud Imbue 21/22 `RootPassDeadlineExceeded` failure remains retained;
this later PASS establishes no causal diagnosis for that earlier failure.

Cleanup is clean, source mode plain, with no remaining owned process group.
`PC_CALL_047_RESULT.json` attests `RELEASED_NO_ACTIVE_CLAIM`, supervisor 163594,
start ticks 2603394, boot `1de2caf4-8501-4962-98e7-73177714c673`.
These are private fixture results, not production installation or exact-head CI.

The historical combined-package registration failure belongs to the Storage/Forge
shared workflow guard; Imbue does not own that guard problem. Imbue's own required
isolated normal-package qualification was pending at that checkpoint. The later
candidate qualification below records its actual result. Full-head remote CI is
a separate publication check.

Other existing product integration slots and full mechanic acceptance remain gated.
No Rust, workflow or shared source changed here; no commit, publication, new PR,
ready, merge or complete package qualification is claimed.

### Final sparse-selection candidate qualification (2026-10-16)

The sparse checkpoint repair and two new regression bodies retain storage SHA256
`96830f09c222f058d1b78d77be2b8a76208286e16ba56a55063b38e376424cdc`
and tests `8b9fc82a7315f173c3374270ef995855441969ea902b859eed48111150130653`.
The native focused22/22 above includes10 actual PostgreSQL17.6 cases with the
unchanged Root deadlines. Sparse/no-op/mixed updates remain valid; an outside
write-set item or stale omitted selection refuses before mutation. Savepoint,
immutable binding, equipment authority and CAS predicates remain intact.

Separate Imbue-only input: parent `bc8a8b6f0c5646f6672ec68b86dc80eec91a4508`,
packet `de52f41c57f9ff5a8d31fc973469e1588190e60df22943002f9af9f0a45d157b`,
manifest `bdec748e17629a151a8b12cae4c3bda83d52b5f37b3024153c7df9f9f549a351`.
Native Rust1.94 fmt and strict package all-target Clippy passed. The first full
package invocation genuinely failed after8652PASS/1FAIL/36ignored: the raw source
copy lacked Git metadata needed by `world_bundle_artifact_boot` (`git ls-files`).
This failure is retained in `IMBUE_NATIVE_PACKAGE_V4_EVIDENCE.json`; no source,
assertion, deadline, workflow or pin was changed to address it.

Reviewed V5 reconstructed genuine private Git metadata outside the frozen source
from the same cached parent:18811 tracked paths, exact parent index,65 executable
entries, and only these three owned dirty files; no intent-to-add or synthetic
commit. The failed artifact target then passed1/1. A complete normal invocation
`cargo test --locked -p oteryn-game-server --quiet -- --test-threads=1` passed
**9785/0/37ignored in57 result groups**, including doctests, with the PG URL
explicitly unset. This normal check is separate from the actual focused PG cases.
Run `evidence-run-1792181181866845373` under
`/root/oteryn-codex-20261016/imbue-package-20261016-native-1` retains actual
commands/results/logs, source, native target artifacts and real-Git proof.
`IMBUE_NATIVE_PACKAGE_V5_EVIDENCE.json` SHA256
`b3c47d18e1b40e54564a33d3683c8cfc194e85134e00c6f8995813dd1dbbbc08`
binds the terminal result; normal log SHA256
`a2fc094671989ad041654259fe8b7bacfec4a55a8304eb0f4c26dabb70d6e996`.
Cleanup is plain/empty/no owned group; observation105 confirms slotB released
and `NO_ACTIVE_CLAIM`. Final card edits are documentation only; executable
source remains identical to the qualified input. Production installation,
elapsed authority/runtime integration, ready and merge retain their CP gates.

### D1163 slot-tier authoring — NOT_FULLY_QUALIFIED

CP comment6225136278 allows Basic, Intricate and Powerful on every slot-bearing
item. The unsupported per-item maximum/tier gate and obsolete refusal are removed
from check_target_slot; only the owned native callers change. Unknown slot count,
zero/out-of-range slots, category/occupancy and complete snapshot checks remain
fail closed. outstanding_access and its Premium/quest/content semantics are
unchanged. A new native-type test covers all three tiers with a real PremiumGate
subcheck and retains higher-tier refusal for Free accounts. These helpers confer
no current session, admitted Content or mutation authority.

Independent source review, changed-file Rustfmt, diff and governance PASS. The23
authored native cases, strict Clippy, full package and PostgreSQL/runtime for this
increment are NOT_RUN. Earlier focused22/22 and full9785/0/37ignored qualification
bind f48912af source; those PASS results do not qualify these changed bytes. The
current canonical module root does not register the Imbue test fixture; no plain
package run excluding it is presented as focused validation. Existing native
overlay stage is the qualification route; the old recipe's22-count binding needs
an explicitly updated23-case input, with no runner changed in this authoring.
D1122 permits the disclosed checkpoint on the existing draft. Shared fee/audit
and native elapsed source remain gated by2197→2155. No ready/merge.

### Retained-slot snapshot repair (2026-10-17, authoring)

Fresh-read input is draft #2144 at
`4d0da90807c4a886008f738db221bc2aedfc7933`. The structural target-slot helper
checked each retained row against the global three-slot ceiling when constructed,
but did not compare it with the item's supplied declared slot count. Consequently,
a one-slot item with a retained slot-1 or slot-2 row could pass a request for free
slot 0 and a different category. A zero-time retained row had the same gap.

The helper now refuses the complete snapshot with `InvalidSnapshot` whenever any
retained row's index is outside the supplied declared slot count. This only
rejects a structurally inconsistent apply input; it does not delete, rebind or
reinterpret retained rows. D1163's all-tier eligibility, unresolved Content/quest
requirements, Premium subchecks and checkpoint storage are unchanged.

One new original native regression covers every out-of-range retained index for
declared counts 0, 1 and 2, at zero, one and maximum remaining milliseconds. Valid
highest retained indices for two- and three-slot items continue to allow another
free slot. There are now 24 authored cases (14 pure and 10 PostgreSQL).
The RED input combines the unchanged input production files with the new test;
the GREEN input adds only the one-predicate domain repair. Native RED/GREEN,
Rustfmt, strict Clippy, full package and PostgreSQL 17.6 execution are pending.
Historical results above do not qualify this increment. Whitespace checking of
both local patches produced no diagnostics. Independent nonauthor source review
found no blocking issue in the executable delta: domain SHA256
`cdbfde1af5d048d850deb9ad2e87c6227800c203d199a6160e0a308c5bc63a97`,
test SHA256 `fbde3926f956d9c20b1c0af619e49dc5d0f9fe03384e3132c2719d33510cf40b`.
This source review is not native execution or final published-head review.

The dependency observation is refreshed without treating a merged PR as runtime
coverage: #2155 merged as `6db7da23607849d0cb72d82897600fc05e1c99f9`, but its
timed-repair planner and native coverage do not provide Imbue elapsed provenance.
#2197 was still an open draft at `6be32b062b741df8070d49d4e2c79dc197271ffe` on
this inspection. The current shared fee API remains `CharmUnassign`-only and
requires CharacterRevision composition; it cannot serve the accepted item-only
Imbue fee/audit transaction unchanged. Those concrete owner integrations, schema,
registration, effects and wire work remain open, as do all family acceptance
criteria. No production authority, source admission or runtime activation is added.

### Permanent native test registration (2026-10-17, authoring)

This continuation keeps the retained-slot repair above and makes its tests part
of the normal repository targets. The current user instruction to complete this
draft and the lead's explicit single-writer allocation authorize the bounded
test-root changes. D1160 permits own registrations; the lead also allocated the
one existing PostgreSQL domain reexport replacement needed to preserve actual
native types while adding the uninstalled Imbuement modules. This is a test
composition change, with no production module or schema installation.

The unchanged pure `CheckpointBatch` planner is extracted from the SQL source
into `domain/imbuement_checkpoint.rs`. `domain/mod.rs` registers that module and
the raw Imbuement model only under `cfg(test)`. `lib.rs` mounts the 14 pure cases
from `durability/imbuement_tests.rs`; it does not mount the PostgreSQL fixtures
or SQL owner. Every original assertion remains. The existing structural case
additionally checks both occurrence fields for nil and the accepted/rejected
UUID version and variant boundaries, retaining the 14-case count.

`SlotRecord::new` now calls the actual public native
`ItemInstanceId::from_bytes` constructor for UUID structural validity. The
native constructor delegates directly to the same unchanged
`validate_uuid_v7` function and then wraps the bytes; checking its error result
therefore preserves the former validity predicate. This does not grant item
custody or derive any current authority. The retained-slot fix remains the
separate `binding.slot >= slots` snapshot rejection.

The existing `character_authority_postgres` target registers the exact private
storage source and its 10 original PostgreSQL cases. Its domain namespace
reexports all original `production_server::domain` public items, retaining the
same native type identities, and adds only the two owned Imbuement modules.
The target retains its existing real Item TRANSFER and Bestiary harnesses,
foundation, equipment and recovery owners. No stand-in methods or authority
types are introduced. All SQL, locking, revalidation and savepoint bodies stay
unchanged. Five raw-row fixture functions are shared without changing their
bodies or values; they remain DATA, never elapsed or current-fence evidence.

The canonical target is also selected by the ordinary integration shard without
a database. Its Imbuement wrapper therefore follows the existing native
`configured_admin` contract: an absent route emits PRE-ROUTING/NONCANONICAL plus
`IMBUE-STORAGE-PG NOT_RUN`, and does not execute database work. This is not a
PostgreSQL PASS. The dedicated canonical job supplies the route; each actual
case still uses the real harness and asserts server version `170006`. An
invalid configured route or PostgreSQL version continues to fail. No test is
marked ignored and no feature or workflow gate is added.

The shared raw model has a narrow test-only `dead_code` expectation in the
PostgreSQL namespace because its structural/access helpers execute in the
library. Existing fixture `expect_used` context is retained. No repository
allowlist, duplicate-source guard or production lint changes. The pure and
PostgreSQL test-bearing files each have one intended harness; shared model,
planner and fixture files contain no tests.

This assembled source supersedes the earlier unregistered candidate for new
qualification. Native slot RED/GREEN, all 14 pure cases, all 10 PostgreSQL 17.6
cases, normal package, fmt, strict all-target Clippy, Cargo dep-info duplicate
guard and exact-head CI are pending. Source/body/hash checks do not establish
runtime PASS. Elapsed provenance, audited fee/apply/clear/expiry, Content,
runtime activation and full family acceptance remain outside this checkpoint.

### D1329 native registration and slot repair qualification (2026-10-17)

This receipt supersedes the pending native status in the authoring checkpoint
above. The exact 10 non-card source files were qualified on Molehill-PC with
Rust 1.95.0. Workspace formatting, governance and all 124 governance cases,
strict all-target Clippy, and the actual Cargo dep-info duplicate-source guard
passed. The guard observed 55 test-harness units and the existing 11 exceptions;
no exception was added. A manual Debug implementation reads the two preserved
error payloads in the newly compiled private PG owner; enum variants, conversion,
SQL, locks, fences and savepoint behavior are unchanged.

The retained-slot regression was executed with only its slot predicate removed:
one intended failure, exit 101. Restoring the exact GREEN model then passed all
14 pure cases and all 10 existing native PostgreSQL cases, with ten observed
server_version_num=170006 markers. No case was ignored, skipped or counted as PG
coverage from the absent-URL route. The runner verified all 11 frozen files and
restored the exact complete candidate after RED.

| Receipt | Result | SHA-256 of complete log |
| --- | --- | --- |
| Slot RED | Intended regression reproduced | `3e08530c4eb17bba86246ba0c909d837e29f78d14b08e09a606a6b4a8896105d` |
| Pure GREEN | 14 passed, 0 failed, 0 ignored | `0e61ce653259918f4518dec68bed838a27c418c87138be662920ce9688c39ae2` |
| Native PG GREEN | 10 passed, 0 failed, 0 ignored | `4606757611d368297271a202ce24828ce0ccf599ddd71de19303307ae654a411` |
| Actual duplicate-source guard | PASS, 55 harness units | `466c2037f51ba756d6b9e0f50a3b3d475bb97def0476b5ad19a99d8ef670b58e` |

D1329 execution used locked dependency fetch before the unchanged owner
in-pg-netns.sh helper, then its isolated namespace forwarding to the existing
shared PostgreSQL 17.6 on host 127.0.0.1:55432. This checkpoint started no database
server or container. Full normal-package execution is still running at this
publication checkpoint; the new commit's CI is not yet qualified.

The separate native Expire implementation remains a reviewed continuation with
its own unexecuted source packet. Its results are not included above. Apply,
Clear, genuine elapsed provenance, Content and runtime admission and the final
composed migration binding remain distinct completion dependencies.

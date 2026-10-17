# HOUSE-RUNTIME-1b — house scope runtime and entry

```yaml
task_id: HOUSE-RUNTIME-1b
title: "HOUSE-RUNTIME-1b: house scope runtime and entry"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row HOUSE-RUNTIME-1b; §7 'HOUSE-RUNTIME-1a/1b/1c'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md
base_branch: main
base_sha: f010472bcb6794a2cbc56dfaf755bd122c10225e
branch: skeleton/house-runtime-1b
needs_migration: false
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## 2026-10-17 authored-source checkpoint — NOT_FULLY_QUALIFIED

The verified draft predecessor is `91faa999d4ad507e3d19538fafd36a42c1555dc4` after CP's
ordinary merge of main `f010472bcb6794a2cbc56dfaf755bd122c10225e`. Root subsequently
observed actual main `ce7bccab973625d5d3d0d0c566426903229d2373`; the relevant native
carrier and House module preimages were unchanged. This checkpoint preserves the real
authored implementation and exact reviews in the owned evidence file:

[D1362/D1373 complete source, tests, manifests and recovery instructions](../evidence/HOUSE-RUNTIME-1b-D1362-D1373-SOURCE-CHECKPOINT-2026-10-17.md).

That evidence contains the complete recoverable semantic patches, the owned actor
source, actual native stage, exact dependency preimages, compiler-input reproduction
and independent reviews. The source remains inert in this checkpoint because shared
activation order and the remaining actual owner seams are not complete. The existing
four Rust files keep entry/login `HouseClosed` and exit `ExitNotAdmitted`. No unused
loader wrapper is being published as runtime completion.

### Authored implementation and allocated ownership

[D1362](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234053855) assigned
the additive House lifecycle shape, sealed claim predicate, exact complete predecessor
history, same-transaction preparation/persistence and durable successor to this PR.
The authored component now binds the actual initial receipt, exact native session-use
membership and one ledger advance; its historical tag-3 result requires Released.
Both scope-generation comparisons preserve ordering only within the same scope identity.
The original Platform observation keeps its strict five-second age without refresh.

[D1373](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234768517) additionally
assigns the actual actor producer and consumer in the two owned House paths below.
The implementation uses the existing one physical carrier, a private Channel/House
domain and real HousePlayer slots. Reservation, materialization, consumption and an
uncertain commit retain the same physical slot and tile. Source state comes from the
real Channel Slot, with its complete position revision, lifecycle and combat state;
there is no default player state or synthetic Channel scope. Reviewed denial paths
preserve free-list exhaustion behavior, exact source namespace, retired-namespace
rollback and full retained payload before source removal.

The actual owned native stage consumes the private native PREPARED object in its original
SQL transaction. It checks the actual source runtime/session/lease/pin, canonical locked
tile, single qualified House identity/digest and current locked 0047 ownership before
retaining the real #2163 interior snapshot and reserving/materializing/consuming the
physical actor. Both physical owners and the SQL transaction remain borrowed by the
stage. It has no COMMIT or bind API, writes no successor session/receipt/lease/generation,
and cannot activate a player with only part of its state transferred.

The native loader is already merged in #2163, at
`cecbfda32b8d4cbb830566cf1b7d1173bef2a6cd`. Its bounded native snapshot is used as
point-in-time materialization input; it is not current Content, placement, admission
or future-write authority.

### Exact source/review and native disposition

| Component | Exact disposition |
| --- | --- |
| D1362 claims, complete history/ledger and two generation-domain fixes | Independent SOURCE REVIEW PASS. Eleven pure regression bodies are in the actual six-file native source assembly. |
| First native six-file compile | Library compiled; three fixture-only E0277 error conversions stopped compilation before LIST or case execution. Fixed using the existing debug-string conversion, without changing production errors or adding a broad allow. |
| Corrected native six-file attempt 2 | Actual PC slot C run started `2026-10-17 14:49:19 UTC`, exit 0; native LIST exactly 11 and **11 pure cases PASS, 0 failed, 0 ignored**, in 0.19 seconds after 47.22 seconds compilation/link. All six before/after source hashes match; final fixture SHA-256 `cf4d5372f46ddbd1836e9253976b83a25ff30bfb41d25a6affbf5427fe79e310`. Seventeen dead-code warnings for unconnected callers are retained. No PG or strict Clippy result is inferred. Complete native result/LIST/test logs and source-before/source-after are embedded in the evidence; result SHA-256 `dfe5c4edab35da18d8b7da0e637b48cf9dce2fd5da4e3bbe3d968a5691e68f8d`. |
| One-carrier core3 | Independent SOURCE REVIEW PASS at actor SHA-256 `de655311758bdc85a3bb67a529467d250add3039df8b328d9475b35cda8020b4`; fifteen pure arena cases. Native format/compile/LIST/execution NOT_RUN in this checkpoint. |
| Actual native stage | Independent SOURCE REVIEW PASS at stage patch SHA-256 `800caa75490a65aa6b5de75455a380f7b34a1bee5ca60f1408e8036cd5ed0926`; final composed core3/stage eight-file manifest `c19dde52b40bc9a5bcb481f91c665394a831080c4494502a95a9dd6bf4dbbb3a`. Compile/native/PG execution NOT_RUN. |
| Full runtime, current Content/FND activation, complete actor transfer, PG rollback/restart and RL-02/RL-04 | Incomplete and unqualified. The pure cases and original selected component receipt do not qualify these boundaries. |

The evidence records the original compact semantic source separately from the actual
native formatter output. Broad formatting is preserved only to reproduce native
inputs; it is not permission to publish unrelated shared formatting. The known three
fixture conversions are also supplied as a compact correction for later source
composition. Every embedded artifact has a hash and exact recovery instructions.

[The original component receipt](https://github.com/Oteryn/Oteryn-Game/pull/2165#issuecomment-6226079840)
qualifies historical source `5b164bcf5f0b6d86854498050a91f8817c1d5ab6`: selected native
Clippy/rlib/link, LIST exactly five, four pure tests and one PG 17.6 test passed, with
server version 170006 and one post-cleanup marker. Missing-URL refusal separately
exited 101. [D1214b](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6226222851)
confirmed that selected receipt. The later `4d53a1fc6799989b538709e02fa26d1ad301e9eb`
was a documentation-only successor. This is preserved historical evidence, not a
current full-head qualification. Earlier D1122 slot holds below are historical only.

### Concrete remaining integration

The shared order is `#2197 → #2198 → #2187 → SESSION-INGRESS-1 → House consumer`.
Exact post-merge preimages and the next free lifecycle tag must be checked at that
slot. [The concrete seven-hunk CP request](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6237005166)
is also preserved in the evidence: the single native
HouseId move/re-exports, one already-validated NPC field access, private PREPARED views
and sealed bridge, and two finite qualified-House read accessors. The actual #2167
spatial encoder/mount/qualified-room source must compose with those hunks; a current
full-bundle House catalogue remains Unknown.

The actual `ChannelSpellStates` owner stores PlayerSpellState outside the carrier Slot.
It has no existing take/rollback/House-adoption port, and its current initializer would
prune a removed source or create default state. CP must allocate its exact native
transfer owner, preserving existing vitals, cooldowns, conditions, regeneration and
pending owner custody. The owned stage deliberately cannot commit until that complete
transfer and current House-aware adoption exist. A second actor map, default spell
state or historical receipt cannot fill this gap.

The final actual owner must perform PREPARED read, entry tile reservation, materialization,
consumption, claims, bind and generation advance in one ordered transaction. It must
replace the old terminal-destination / generation-greater-than-one placement surrogate,
resolve uncertain COMMIT before releasing retained owners, and use the original initial
receipt for retry/restart. Native invalid-interior/placement rollback, same-TX and
history rejection, crash recovery, complete usable-door RL-02 and measured RL-04 remain
mandatory. Entry/login and exits remain closed until these implementations are integrated.

D1329 requires shared PG 17.6 only, `cargo fetch --locked` before `in-pg-netns.sh`, and
cleanup of owned databases. D1342/D1379 and [D1397/D1397a](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6236550340)
require the real canonical `/home/mole/oteryn-cache/heavy.lock`, with the granted outer
Ubuntu/mole bridge for an existing Ubuntu-24.04 runner. [D1399b](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6236882742)
sets MAIL D1396a then HUNT D1392 before the next House heavy run; new worktrees go in
Ubuntu/mole, and each run removes only its own target/temp files. Root alone publishes
the coherent draft successor with exact ref/file readback. No ready, merge, activation
or external review trigger is part of this checkpoint.


## Historical 2026-10-16 D1122 checkpoint — superseded disposition

This checkpoint retains the bounded closed-boundary implementation at source head
`5b164bcf5f0b6d86854498050a91f8817c1d5ab6`. The evidence below updates the
historical execution dispositions later in this record; it does not complete the
original runtime acceptance scope or open entry/exit gates.

| Check | Observed disposition |
|---|---|
| Independent bounded source review | PASS; four owned Rust files, native closed refusals, no production caller or shared registration. |
| PREPARE and actual Cargo CAPTURE | PASS; retained PREPARE proof and actual175 CAPTURE exit0, source match, quiescence and all112 resource samples PASS. CAPTURE records compiler context; it does not execute test cases. |
| Selected native Clippy, cfg(test) rlib and driver link | PASS; actual184 reports all three exits0, source match, quiescence and all30 resource samples PASS. Body reconstruction verifies all five original test bodies remain literal. This is selected native compiler evidence, not a global test-ELF or package-wide qualification. |
| Native LIST and four closed-boundary case executions | NOT_RUN; actual189 stopped on slot admission EAGAIN before child creation or LIST evidence. No case PASS or test RED is inferred. |
| Native PostgreSQL17.6 case execution | NOT_RUN; no runtime case or database qualification is claimed. |
| Full runtime, native loader/caller integration, restart/recovery and RL02/RL04 | NOT_RUN / incomplete; the dependencies and original acceptance items below remain open. |

Exact retained observations are
`/workspace/house-pc-execution-20261016/175-2165-capture-proof-observed.json`
(SHA-256 `301607d482317edaab717c53c3e3bf8e74bdfe92cbc143144bdb97b50704d678`)
and `/workspace/house-pc-execution-20261016/184-2165-native-proof-observed.json`
(SHA-256 `75e8145caf0a3cf961b58408ab1707416e59e50c3c8ad3abc4c4ab5116e37a6f`).
The latter pins the actual rlib, driver, metadata, compiler results and literal-body
proof; none of those artifacts is a runtime test result.

The operational blocker is a prechild slot HOLD. Actual191 observed A and PG held
by RANK_2168 (supervisor283316, group283705); the A record lacks custody fields
required by the current guard. The B kernel lock was free at that observation,
which grants no later reservation. No lock record, guard or competing job was
changed. The concrete coordination request is
[CP2131 comment6225557139](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6225557139).
True native interior loader/read authority, actor/map and reservation/placement
callers, transfer contract, shared registration and integrated measurements still
require the owning native teams and CP decisions listed below.

Retained formatter evidence is `/workspace/house-family-handoff/runtime.json`
(SHA-256 `aad1b4500dfbb96ce4ac5557325c7a592fa68a4418897db0699eacb8f7ddadf1`),
`tasks[0].authoring_packet.validation.rustfmt`: official verified Rust1.94.0,
`--edition2024 --config skip_children=true --check`, PASS on all four owned Rust
files. Their current SHA-256 values match that packet exactly; no formatter rerun
is needed for this task-record-only checkpoint.

[CP D1122](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224930201)
permits one coherent checkpoint push on the existing draft with this explicit
NOT_FULLY_QUALIFIED / RED / NOT_RUN accounting and passing fmt/whitespace checks.
The PR remains draft; no ready action, `@codex review` request or manual CI restart
is authorized by this checkpoint. Full checks remain required before a frozen
review candidate. No observed owned test RED exists; unexecuted cases stay NOT_RUN.

## Goal

Houses run as their own scope: loading the interior from 1a, entering and leaving through scope handoff, refusing HOUSE_CLOSED until the SCOPE-HANDOFF-1 exit gate lifts; measure HOUSERT0-RL-02 and RL-04.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

Second of HOUSE-RUNTIME-1a -> 1b -> 1c.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| HOUSE-RUNTIME-1a | BLOCKING (not merged) |
| MAP-LOAD-1 | MERGED (archive MAP-LOAD-1) |
| SCOPE-HANDOFF-1 exit gate | exit-dependent items stay open until it lifts (packet §1.5); entries refuse HOUSE_CLOSED |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/gameplay_transport/house_runtime.rs`
- `apps/game-server/src/gameplay_transport/house_runtime_loading.rs`
- `apps/game-server/src/gameplay_transport/house_runtime_tests.rs`
- `apps/game-server/tests/house_runtime_postgres.rs`
- `apps/game-server/src/house_scope/actor_placement.rs` (D1373 actual producer)
- `apps/game-server/src/house_scope/mod.rs` (D1373 actual consumer)
- `docs/agents/evidence/HOUSE-RUNTIME-1b-*.md`
- `docs/agents/tasks/HOUSE-RUNTIME-1b.md` (this record: status and notes only)

CP D1062 fixes the four original implementation paths. D1373 additionally assigns
the two House scope paths above to the actual producer and consumer. Other shared
registrations, lifecycle/carrier dependency hunks and the aggregate test target remain
subject to their exact allocated slot; the embedded source backup does not apply them.

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

- world_runtime.rs scope registration
- durability/house_scope_handoff.rs (existing) entry/exit
- map/boot.rs house scope load

## Inseparable bundles (never split across PRs)

- scope runtime + entry refusal HOUSE_CLOSED + measurements RL-02/RL-04

## Acceptance criteria

- [ ] Entering a house loads the 1a interior; leaving returns via scope handoff.
- [ ] Entries refuse HOUSE_CLOSED until the exit gate lifts (packet §1.5).
- [ ] HOUSERT0-RL-02 and RL-04 measured and recorded in evidence.
- [ ] Restart leaves no Character stranded in a house scope.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server house_runtime
cargo test -p oteryn-game-server house_scope_handoff
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
- docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md §1.5, §2.2
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.


## Allocated native owner/integration packet (2026-10-15, card only)

Exact draft base `169675ea76351e5745db6ac2fc3af5b7704a6826`, branch
`skeleton/house-runtime-1b`; reference protected main
`bda15eba7a84f65fcb963a86e8b2bf18d69c800a`, read only. This allocation owns **only this
card**. Original plan, builder/CP rules and six unchecked acceptance items above remain
unchanged. Historical shared registration permission does not expand this lease. All new
paths and interfaces below are **proposals for CP/architect review, not allocated files**;
no Rust/runtime implementation, source wiring, schema, grants or registry changes occur.

HOUSE-RUNTIME0 is accepted (SOCIAL-MAP #1771; reference main reconciles its heading), the
HOUSE-RT-INBOX packet remains the split/access/exit-gate authority, and effective
ACCEPT-BATCH #2209 accepts ADMIT0/BED0. Acceptance of ADMIT0 does not lift its §3.2 transfer
protocol gate. Runtime1a is a prepared local/draft subset, not a merged complete writer.
HOUSE-RT-INBOX §0.2 and §1.5 (reference main, exact source path in this card's briefs)
retain the historical integration dependency: HOUSE-1a waits for1b/1c acceptance with all
exit-dependent items met; HOUSE-1b, HOUSE-ACL-1, HOUSE-WIRE-1 and GUILDHALL-1 depend on
HOUSE-1a. That dependency is distinct from the current CP queue classification:
`/workspace/oteryn-prep/QUEUE.json` records HOUSE-1b as `BLOCKED_DEPENDENCY`, whereas
HOUSE-ACL-1, HOUSE-WIRE-1 and GUILDHALL-1 are explicit `HELD` rows (the owning explicit hold
source is `OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md` §0.1). This packet proposes no
lift, reclassification or status override and creates no allocation, GUI/kick wire or
guildhall contract.

### Native source findings and concrete deltas

1. Native `HouseId` (`durability/house_scope_handoff.rs`) is already World-global and
   revision-free: WorldId plus the qualified house content key; `scope_key` is tagged02,
   `instance_id()` is its deterministic native runtime identity. Use those exact APIs.
   A generic `RuntimeScopeRefV1::Instance`, same generation integer or caller-supplied
   instance UUID is not a House or current assignment proof. WorldId and ChannelId stay
   distinct; origin Channel is routing evidence, never a selectable home Channel.
2. SCOPE-HANDOFF-1 already owns migration0074, assignment commands/history, handoff rows,
   prepare/commit/abort/reconcile, tile reservations and stale-generation cleanup. Native
   `prepare_house_entry` / `commit_house_entry` keep recovery/admission locks, exact source
   session/connection/lease/Channel generation, node incarnation, destination House generation,
   and re-read `game_house_access` in the commit. The committed destination gets a fresh
   GameSessionId and advanced lease, awaiting its transport/actor. Do not duplicate this
   journal, idempotency store, session writer or handoff state machine in the runtime.
3. **Exit remains closed on reference main:** `house_scope::exit_into_channel()` returns
   `ExitNotAdmitted`; `validate_request` rejects Exit/non-Channel source. ADMIT0 §3.2 still
   requires destination capability support/selection before commit, scope-identifying
   postcommit acknowledgement, lost-ack recovery, and no snapshot/gated message before
   acknowledgement/recovery, under the transfer owner's protocol/security review.
   Therefore1b's top-level runtime guard must refuse **every entry HOUSE_CLOSED**, including
   login activation, while the exit gate is closed. A successful lower-level entry fixture,
   accepted ADMIT0, or prepared ADMIT-CAP arithmetic never opens this gate. Test the shipped
   exit refusal; no feature flag, boolean injection or alternate fallback may bypass it.
4. Native prepare currently requires the proving node to hold BOTH source Channel and
   destination House; another-node House returns Busy. HOUSE-RUNTIME0 §3 allows placement
   on another node of the same World when the origin node is full. That alternative requires
   an independently qualified native cross-node handoff/custody seam, not forwarding an
   in-memory actor or forging a source proof. Until then, retain same-node restriction and
   typed refusal; do not claim the full hosting/fallback acceptance item is delivered.
5. `assign_house_scope` is an actual control-role command with exact-house grants,
   OperationKey replay, shared assignment-writer high water, source revision, predecessor
   checks and current node registration. Runtime activation may request existing assignment
   ownership, but cannot mint ControlActor/role/grant or update assignment tables itself.
   Assignment rows/generations survive unload; replacement increments generation and excludes
   incompatible bare instance sessions. An old assignment receipt is history, not current
   node/house liveness. Reuse native NodeIncarnationProof and current assignment validation.
6. `map::boot` checks pinned WorldBase, map revision, item definitions, collision/doors;
   BundleFacts already records house tiles and excludes house doors from Channel DoorBook.
   `NativeMovementCollisionIndex` is a sealed owner port; `ChannelRuntimeV1` is concretely
   ChannelRuntimeAssignmentBinding + single fixed-slot actor carrier, not a generic House
   runtime constructor. `ComposedFreshAdmission` is also Channel-bound. The shared actor/
   collision/content adapters must be owned explicitly in CP: do not instantiate a fake
   Channel for a House, copy the carrier, add a second actor/session store or implement
   another pathfinder. `world_runtime::ScopeContentGenerationFence` cannot be synthesized
   from a HouseId/generation or a test constructor to grant active content.
   Existing `content/project/native_house_tiles.rs` also supplies QualifiedHouseTiles /
   QualifiedHousePlacement with typed definition identity, exact scope-bound cells, doors
   and an outside `entry_cell`; omission is Unknown, the retained strict source region
   proves no houses, and `house_at` checks its native scope. Reuse these qualification
   rules where applicable, but this candidate map port is not an installed bundle House
   catalogue: current BundleFacts records only generic house-tile positions, not the full
   HouseId→tiles/entrance binding. Mapping that source to the active bundle and exposing
   exact-key/current entrance metadata requires the owning content/map adapter. In
   particular `entry_cell` is not an interior arrival tile. Removed-house fallback needs
   accepted retained entrance metadata or the real admission owner's fallback; never
   guess a coordinate or treat missing metadata as an empty/valid house.
7. Prepared1a (`a9e0277...` source; verified exported RESULT) contains structural/provenance/
   role/audit validation and real fixture/native-wrapper checks. Its `HouseInteriorLocation`
   and generation fields are audit inputs, not current authority. It implements no complete
   transactional SECURITY DEFINER shapes or generation-qualified interior loader. Native0025
   already owns durable HouseInterior locations/provenance and closes runtime direct table
   writes;0074 access function is still a fail-closed no-row stub.1b must request the actual
   owning read/shape seam, not treat prepared1a as an installed service or bypass grants.
8. Existing `house_spell_acl::house_access_for_character` reads native ownership and simple
   guest/subowner lists, but returns no complete property/content-fence/ACL+guild revision
   contract required by the packet. It is not a substitute for `game_house_access`.
   HOUSE-1a owns the final composed body, HOUSE-ACL-1 the grant part and HOUSE-1b the fence
   part. Preserve property→fence→grant locks and exact revisions, including granting guild
   and membership rows. Missing final property lifecycle or guild facts means refusal, not
   invented current flags. Tests may replace the stub only in the isolated operator test
   World; no fixture authority can reach production.

### Proposed exact paths and native interfaces (CP decision required)

| Proposed new file | Narrow role and interface obligation |
|---|---|
| `apps/game-server/src/gameplay_transport/house_runtime.rs` | Private HouseRuntimeOwner lifecycle and serialized actor-command owner, plus a node-local bounded HouseId→owner directory. Owns load/ready/closing/idle state and placement reservations; holds native actor handles/carrier adapters, not a second actor/session truth. Publicly reachable construction only after actual current assignment/incarnation/content qualification; local state never grants DB authority. |
| `apps/game-server/src/gameplay_transport/house_runtime_loading.rs` | Private activation adapter over native pinned WorldBase/BundleFacts/collision/door facts and the1a-owned generation-qualified HouseInterior read. Validates exact house tile membership, native coordinates and current item obstruction; no new map storage, pathfinder, content ID or property reader. |
| `apps/game-server/src/gameplay_transport/house_runtime_tests.rs` | Focused native owner-command/generation/closed-gate/tile/lifecycle tests; source included once by the runtime owner. |
| `apps/game-server/tests/house_runtime_postgres.rs` | Actual native handoff/assignment/access/current-session/restart integration tests and bounded load evidence driver; explicit Cargo/aggregate registration decided in the CP slot to avoid duplicated test-bearing sources. |

Interface sketches describe **obligations**, not new authority tokens or allocated APIs:

- Activation accepts native HouseId, an independently current native assignment/incarnation
  result, and the actual active content generation/World bundle. Requests assignment through
  the existing owner using its exact grants; waits for the single winner, then revalidates
  before Ready. A capacity proposal does not authorize placement or assignment.
- Loading calls a native1a read owned by `durability/house_interior.rs` (future hook, not an
  existing method) in a fenced, consistent transaction; validates each actual0025 item
  location/provenance and its World/House/generation binding. It obtains only this house's
  tile/door subset from active native facts, preserving map-authored handles vs durable
  ItemInstanceId entities. Do not infer tile membership from arbitrary spatial-position
  bytes or a map item's bed/head coordinate; use actual qualified native encoding.
- Owner command inputs reuse native GameSessionId/ExactActorRef/HouseId/handoff ID and
  independently current connection, lease, scope generation and content binding. Commands
  are serialized with native actor movement/combat and item placement; they return existing
  typed handoff/refusal outcomes or retained operation identity. A prepared request/receipt
  is never sufficient current proof for an actor mutation.
- Select/reserve uses existing `house_scope::handoff::select_entry_tile` over candidates
  qualified by native walkability/house membership and native ordered radius3 fallback,
  then atomically reserves through the real handoff owner. This helper does not qualify
  candidates or enforce current authority. Revalidate before placement/commit; no-room is
  typed. All simultaneous actors/item moves/other placements see that reservation until
  durable placement release; no new pathfinding engine or separate SQL reservation ledger.
- Move/use/leave dispatch reuses native movement/collision/WorldUse and local-object owners
  with a CP-qualified House binding. PZ/no attacks/no regeneration/no creatures and entry
  summon dismissal come from real actor policy owners, not duplicated online tick/combat
  engines. Map-authored doors/depot/beds remain nonpickupable; BED owner supplies its own
  accepted reservation/wake hooks, not a1b bed implementation. Client item movement stays
  blocked behind HOUSE-ITEM-WIRE-1; this packet adds no capability or wire.

### Ordered activation, entry, exit and closure flow

1. Before any load/session mutation, check the shipped exit-admission gate. Closed means
   HOUSE_CLOSED and source actor remains unchanged. When independently lifted later, freeze
   the source only as one native actor owner command serialized with combat/locks/trade/
   pending operations, rechecking them at prepare. Preserve existing runtime-first native
   lock order; never hold destination/source actor mutexes across each other or arbitrary
   async database waits. A combat restriction landing before commit aborts/reconciles
   source; a raw `HouseEntryPrecheck` boolean is only a precheck, never current DB proof.
2. Coalesce first activation by exact HouseId/generation and bound node load. Origin Channel
   node is preferred if it has measured capacity; qualified fallback must remain same World.
   Assignment acquired/observed through native control owner, then content+interior loaded
   and current property/content closure rechecked before Ready. Duplicate activators wait;
   load failure never publishes readiness or creates another owner. An empty house still
   needs valid current content/property/assignment, not invented empty defaults.
3. Reserve the qualified door-adjacent tile or native same-house radius3 fallback. Native
   prepare counts live/reconnectable House sessions **plus PREPARED handoffs** toward200;
   committed awaiting-transport sessions already count as sessions, not a second resident.
   Local occupancy and durable reservations must agree; local actor count alone cannot
   admit201. Before the commit revalidate chosen tile, source actor restrictions, property
   state/fence, exact ACL and guild revisions under owning locks. Use the actual native
   handoff commit; source becomes terminal only at its durable commit point, destination
   session is fresh and preserves origin Channel. No separate transfer writer.
4. Unknown prepare/commit/abort outcome retains the same handoff ID and native owner state;
   query/reconcile its durable outcome before lifting source freeze, creating destination,
   dropping reservation or sending snapshot. Committed means only destination can install;
   uncommitted/aborted means only source resumes. Bind destination through actual admission/
   transport/guard owner and publish only after required acknowledgement. Release the tile
   through `release_house_entry_tile` using its actual persisted placement proof; an actor
   counter or caller `placed=true` cannot discharge it.
5. Door exit, leave, authorized kick, revoked grant, disposition and shutdown all call the
   native reverse handoff seam **after it is qualified**, targeting recorded origin Channel
   first. Unavailable/draining origin uses normal same-World Channel admission selection,
   never player choice. Entrance blocked/occupied uses existing native last-position
   radius3/step4 fallback, never refuses solely for that obstruction.1c must clear house
   columns and write actual destination tile under new session's order key in the same
   commit;1b calls that owner hook and cannot claim1c work. Closed exit gate means no
   fabricated exit, placement or unsafe deletion of the source actor.
6. Current ACL/guild revision change schedules revalidation of every resident through actual
   current grants; revoked residents exit through step5. Item mutations independently
   revalidate at their1a commit, including session and House generations; a refreshed runtime
   cache is not a write grant. Kick policy keeps OWNER any / SUBOWNER guests / GUEST none
   and exact current target presence; no new kick GUI/wire is allocated here.
7. Content fence/disposition enters Closing, blocks new prepares/activation and drains all
   committed residents plus pending handoffs before reporting quiescent to HOUSE-1b. Freeze
   item admissions and resolve unknown writes through their real owner; no success while
   a retained mutation/reservation can still commit. A fenced house cannot become Ready
   from an older cached property row. VACANT alone does not authorize entry: without a
   current owner/access grant the composed function still returns no access.
8. Idle unload uses native monotonic owner time: five minutes with zero residents, no
   unbound committed destinations, unresolved/pending handoffs, reservations or owner work
   that could install a character. New entry resets/cancels idle and serializes against
   unload; revalidate before releasing the native assignment via its qualified owner.
   Durable items/provenance survive. Do not delete assignment/history or consume new
   generation locally. At node shutdown/reset/replacement, close admission, reconcile or
   drain through native handoff/recovery, retire old owner and invalidate callbacks before
   activating new content/generation; no live hot content swap under the old fence.

### Concrete shared call sites / owner gates

| Existing owner/path | Required CP delta, not authored here |
|---|---|
| `foundation/runtime_actor_carrier.rs`, `gameplay_transport/mod.rs` (`ComposedFreshAdmission`) | Small native House binding/carrier/admission adapter and single actor ownership/retirement; preserve runtime→spell/attack lock discipline and source freeze. Existing Channel constructor is not a House constructor. |
| `map/boot.rs`, `map/facts.rs`, `content/native_cell_lookup.rs`, `world_runtime.rs` | Qualified house tile/door/collision subset over the same bundle and active content generation; keep Channel house blocking, native coordinates, handles and existing collision/fallback. Expose only owner-approved House binding rather than copy map boot/pathfinding. |
| `durability/house_interior.rs` (prepared1a), native0025 | Actual generation-qualified consistent load and the complete accepted SECURITY DEFINER transfer shapes; no runtime table grants. Revision projection belongs HOUSE-VIEW-1, not1a/1b speculative storage. Dynamic interior changes must reach movement/item obstruction through the real single writer/read seam. |
| `durability/house_scope_handoff.rs`, `house_scope/mod.rs` | Qualify reverse exit + ADMIT selection/ack/recovery conditions and cross-node seam;1b adds genuine runtime callers only. Preserve existing replay/fences/reservations/recovery journal and control assignment grants. |
| `durability/character_last_position.rs`, `gameplay_transport/last_position.rs` (1c) | House saved-position/current access and fallback, atomic house-column clear on exit/recovery and actual destination write; current Channel-only projection is not sufficient. |
| `game_house_access` / HOUSE-1a, HOUSE-ACL-1, HOUSE-1b | One composed owner body plus separate dependency-gated grant/fence children, current property/guild/member locks and revisions; no replacement by simple spell ACL reads or fixture function. |
| gameplay connection/mod/capabilities, lib/module registration and RL registry | Wire only actual allocated call sites at CP slot; packet adds no capability/message. Exact runtime paths and RL04 measured allocation need explicit CP acceptance first. |

### Failure / unknown / restart / retired-owner matrix

| Boundary/operator | Required observation before acceptance |
|---|---|
| Closed gate, missing grant/property/interior/readiness or unsupported destination selection | Typed refusal; no source termination, new session, load-ready authority, reservation leak or snapshot. Exit gate refusal remains the real shipped behavior until reviewed lifting. |
| Concurrent first activation / node full / assignment replacement | One owner and one assigned generation; waiter observes winner, overload typed; different World/House/node/incarnation/generation never substitutes. Remote-host path remains refused without native cross-node proof. |
| ACL/guild/member/content-fence changes between precheck/prepare/commit | Exactly current commit locks decide; loser source remains admitted or reconciles its retained handoff; never stale access accepted. Test each independently with unrelated fences valid. |
| Tile/item obstruction or capacity200 race | One reservation wins; item write/placement respects it,201st refuses NoRoom. Freed/released tile reused only after actual native proof; no committed destination stranded by early release. |
| Timeout/lost response at prepare/commit/abort/tile release | Reuse operation ID, recover immutable history and independently current authority; no abort of committed handoff, duplicate actor/session or new reservation on retry. |
| Crash/restart with PREPARED or COMMITTED handoff, destination unbound | Existing reconcile/recover_stale methods run before either actor resumes. Before commit source wins; after commit destination wins. Retain committed reservation until placement/terminal/stale-owner proof. |
| Old node/lease/connection/scope generation callback after replacement or shutdown | Cannot mutate actors/items/sessions, publish readiness/snapshot or reacquire control from retained receipt. Reconcile durable finality under independently current incarnation; never replay old owner authority. |
| Unrecoverable house / rejected saved tile / unavailable origin / blocked entrance | Actual1c+exit owner records fallback Channel and chosen tile/cleared house columns atomically; fresh retry cannot return to old interior. Until exit gate lifts this remains acceptance-pending, not simulated success. |
| Disposition or idle/reset racing prepare/resident/unknown item mutation | Closing rejects new work; zero proven residents and unresolved installs before quiescence/release. New generation uses actual refreshed content; old owner never revives. Durable item state/history retained through reload. |

### Boundedness and measurements required before activation

RL01 remains five-minute idle; RL03 is200 total residents/reserved admissions as native prepare
counts it. Include unbound and reconnectable sessions; active/inflight counters use checked
arithmetic and release only on confirmed outcome. Bound per-house candidate selection by the
native tile set/radius fallback and existing item bound (HOUSEOWN0-RL-14, prepared1a2000);
validate that installed registry/owner bound rather than trusting a raw number. Share immutable
WorldBase per node instead of reloading/copying a World per house. No unbounded activation
waiter queue, global house scan per movement or second O(WorldTiles) pathfinder per request;
precompute the qualified per-house tile subset at load under the current bundle. Existing
limits remain controlling; new queue/memory bounds require measured CP registry selection.

RL02 target is end-to-end door transition **p99<=500ms**, measured before activation: source
owner command/prechecks, activation+interior I/O when cold, tile reservation, durable prepare/
commit, destination binding, required scope acknowledgement and usable replacement snapshot.
Report cold vs warm separately; do not hide cold load or acknowledgement/network delay in a
server-only timer. Retain stage timings, error/refusal rates and commit/reconcile counts;
refusals/timeouts are reported separately, not discarded to improve success latency.

RL04 remains **unset until measured**; propose a maximum active houses per node only from
recorded hardware/container limits, release build/artifact/content hashes, World/map and
PostgreSQL versions/config, warmup, sample count, concurrency/rate/house-size/resident/item
mix, and repeat runs. Exercise empty/typical/max200 residents and max admitted interior
items, concurrent cold activation,199/200/201, node saturation, idle/reload churn, assignment
replacement and closing/recovery. Measure peak/steady RSS, shared-vs-per-house allocations,
CPU/owner-step p95/p99, runtime tick lag, DB pool/lock waits and end-to-end transition p99.
Increase active houses until the first latency/resource/owner-bound breach, then propose
headroom and repeat at the candidate cap with simultaneous Channel load. Register the chosen
RL04 and enforce typed overload before assignment/readiness; do not hard-code an arbitrary
cap or call synthetic helper timing the required measurement. Final native PG/current-fence,
real protocol ack/lost-ack, actor conservation and retired-owner tests accompany that result.
No measurements or runtime tests were executed in this card-only round.

Local qualification is governance plus its tests, whitespace/scope checks, exact base/main
native source hashes and prepared1a exported-file hash verification. Evidence:
`/workspace/oteryn-prep/house-runtime-1b-evidence`. Full1a/1b/1c delivery, path allocation,
exit lifting, CP/architect exact-head independent qualification and every original open
acceptance item remain pending.

## 2026-10-16 bounded closed-boundary authoring

CP/architect [D1062–D1065](https://github.com/Oteryn/Oteryn-Game/pull/2165#issuecomment-6216594046)
supersedes the historical card-only/path-unfixed allocation above. The owner also
confirmed exclusive authorship of this existing draft in the current session; no
additional CP lease number is required. This pass starts from exact branch head
`b0abc4ceaad2ce905f5b11cc91d1bdd800c90bd0`. It writes only the four fixed files and
this record, with no shared registration, migration, content, registry or wiring.

The new runtime boundary accepts the native `HouseEntryRequest` by reference and
returns `HousePrepareOutcome::Refused(HouseClosed)` before load, tile reservation,
source freeze or durable PREPARE. Saved-house login activation also refuses
`HouseClosed`. The loading sibling is explicitly a refusal, not a successful
loader, empty interior or readiness/assignment proof. It has no open flag. The
exit boundary delegates to the actual `house_scope::exit_into_channel`; leave,
kick, revalidation and quiesce therefore retain `ExitNotAdmitted`. There is no
HouseRuntimeOwner/Ready state or actor/session store fabricated to fill the
missing native ports. These unregistered boundaries have no production caller yet.

Four native-type cases cover a permissive actor precheck, independently
changed invalid/stale request inputs with the exact request retained, saved-house
activation, and the actual native exit refusal. The PostgreSQL case uses the
real migrated PostgreSQL17.6 schema with a schema-valid Channel-session fixture and
compares complete source-session, handoff and assignment rows before/after each
boundary. It proves only closed refusal; it supplies no positive current authority,
house placement/access/load or actor proof. The standalone target is active and
mounts all five cases once, using the public production types rather than copying
test-bearing Foundation/durability modules. The production source module has no
second test mount. Missing or non-Unicode PG routing returns an error, so the
database case cannot report success without execution. Canonical execution
requires PostgreSQL17.6 and the case's actual QUALIFIED marker. These cases are
authored, not executed. CP
must retain one test-bearing mount when consolidating its aggregate at the slot.

Authority-invariant applicability for this bounded component:

| Boundary | Identity / current / temporal mutation operators |
|---|---|
| Door and saved-house activation | Refuse before all authority-consuming work. A permissive precheck, absent assignment, stale lease or missing tile cannot reach loading/PREPARE. No successful authority grant exists in this component. |
| Exit | Delegates the actual native refusal, with no local fallback/session/position write. |
| Recovery / restart | NOT_IMPLEMENTED: no persisted outcome is interpreted and no node-wide cleanup is invoked from a house-local call. The existing native reconciliation/release owners remain required. |

D1064's wording about PREPARED handoffs does not narrow the native capacity rule:
`durability/house_scope_handoff.rs` counts live/reconnectable House sessions **plus**
PREPARED handoffs against200. This pass does not change that writer or add a weaker
local occupancy count. Clarification of the wording remains a CP/architect item.

Concrete remaining integration owners and acceptance gates:

- 1a/durability owner: actual current-generation-qualified interior load, consistent
  item locations/provenance and proper runtime read grant; no direct runtime SELECT
  or admin pool substitute. This source base has no such loader.
- Native actor/map owners: real House carrier, active Bundle exact House→tiles and
  arrival/retained outside entrance binding, current content/assignment/incarnation
  checks, shared actor policy and collision/door adapters. Generic house flags and
  a forged Channel cannot fill these ports.
- Native handoff/admission owners: actual reservation/current placement callers,
  serialized lifecycle/idle unload/quiesce, restart and uncertain-outcome matrix,
  real House guard rebind, capacity200 conservation and destination installation.
- Architect D573/SCOPE-HANDOFF-2: transfer contract and independently qualified
  ADMIT0 §3.2 selection/scope ack/lost-ack/no-snapshot lift. Until then every entry
  remains `HOUSE_CLOSED`; D1063 prohibits merge before the transfer contract.
- CP: #2163 merged prerequisite, shared registration/aggregate test slot, and real
  integrated RL02 cold/warm p99≤500ms plus RL04 measurements/registry evidence.
  Timing this refusal helper cannot satisfy either measurement.

Validation is intentionally limited by D1011: no Cargo, rustc, heavy build or PG
execution was attempted in this cloud environment. The initial formatter command
was unavailable. The lead subsequently provided an official verified Rust1.94.0
formatter outside the repository; all four owned Rust files were formatted and
passed its parse/format check with child traversal disabled. This is not type
checking or test execution. Governance validation and its124 tests passed; exact
five-file scope and tracked/untracked whitespace checks passed.
The external source-bound handoff is
`/workspace/house-family-handoff/runtime.json`; it records the exact input/output
hashes and actual static check dispositions. Every original acceptance box remains
open. No commit, push, ready, review trigger, merge or runtime activation occurred.

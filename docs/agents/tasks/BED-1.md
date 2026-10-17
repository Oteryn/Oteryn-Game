# BED-1 — house beds: sleeper, bed USE, owner wake, settlement

```yaml
task_id: BED-1
title: "BED-1: house beds: sleeper, bed USE, owner wake, settlement"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/bed-1
base_sha: f010472bcb6794a2cbc56dfaf755bd122c10225e
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row BED-1 (row 121); App. 7 LOOP-VITALS-1, LOOP-TRAIN-1, house beds; §5 Wave 5"
class: "hard (persistence, protocol review)"
needs_migration: yes
needs_registry: maybe
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/house_bed/ (new)"
  - "apps/game-server/src/durability/bed_sleeper.rs (new)"
  - "apps/game-server/src/house_bed/*_tests.rs"
  - "docs/agents/tasks/BED-1.md"
slot_only_paths:
  - "apps/game-server/migrations/00NN_bed_sleepers.sql"
  - "apps/game-server/src/gameplay_transport/item_use.rs / connection.rs (bed USE arm)"
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (if Reg confirmed)"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## 2026-10-17 raw BedKey occupancy — source checkpoint, native held

This checkpoint extends #2147
`87674e53cd4be954b19dfb049a5d41013182fc03` with three owned Rust leaves
and this card, which retains the complete
[reproducible source evidence](#reproducible-occupancy-source-checkpoint).
The source was frozen before this documentation update and has not changed.
The draft remains **NOT_FULLY_QUALIFIED**. The older sections below are historical
records of their named source generations, not qualification of this new packet.

### Implemented raw read

`StoredBedOccupancy` and `read_bed_occupancy` use the actual `StoredBedKey`:
World, canonical HouseId and every head-cell coordinate. One PostgreSQL statement
reads the open occupant and House-wide occupancy revision in one MVCC snapshot.
A freed row remains available for its character's settlement and no longer
occupies that bed when another character uses the same key.

The query returns at most two matching open rows and refuses ambiguity. A
missing House revision while any open sleeper remains in that House also
refuses, including a lookup of an empty neighbouring bed. Stored identity,
lease, appearance, timestamps and revision still use the existing strict decoder.
The private schema and lifecycle/revision triggers remain unchanged.

This DTO is raw storage data. An absent revision is `None`, not revision zero,
proof of a new House, active head/foot placement, ACL permission or current
authority. The read neither reserves the bed nor starts a transaction, writes
a sleeper, installs production DDL or grants regeneration. Actual USE/wake
writers still require their independent current producers and native locks.

### Exact source and regression inventory

| Owned source | Frozen SHA-256 |
|---|---|
| `durability/bed_sleeper.rs` | `9b111c1aad110e6abf5013ca499815e91544d3b1a445d191553d770664e77405` |
| `house_bed/sleeper.rs` | `50ed072d5aca2deb7382985e18406bcc95cf7098a9d0783797fc71c11feeb336` |
| `house_bed/sleeper_tests.rs` | `b375914e741a007647d88009db0b18199b56fbc77eedb463f2c79713935bb26b` |

All ten previously published test bodies are preserved. Three actual-PG cases
add exact-key/free/reuse behavior, coherent revision/occupant snapshots across
a concurrent rollback and commit, and missing-counter/ambiguous-row refusal.
The selected inventory is **13 cases: one pure and twelve PostgreSQL cases**.
This includes the existing own-database error/panic cleanup regression.

| Check for this thirteen-case source | Result |
|---|---|
| Independent source review | **PASS** for the exact production pair and pre-format test `05dbf0ed...`; full report SHA `d1ea8f53c53f74c1b3f851b2dca443a930adc532329352ffe0c9791af6eacdd8` is embedded in the evidence. |
| Focused native formatting | **PASS**, applied by root on Molehill. Production pair unchanged; tests become `b375914e...`. Exact formatter delta SHA `b528eef524f7e28c63f2e5f0451473cf516b818d47536188afcd2acb0a48d1f3` is retained. |
| Source checkpoint reconstruction | **PASS**: exact patch applied to pinned published preimages reproduces all three frozen files byte for byte. This is not compilation. |
| Native compilation and execution of all thirteen cases | **NOT_RUN**: no House heavy window after the D1399b MAIL/HUNT queue yet. No PG result is inferred from source review or formatting. |
| Mounted-head, package Clippy, production sleep/wake/logout/settlement and restart | **NOT_RUN / incomplete**; this checkpoint adds no module registration or production writer. |

The earlier **10/10 PASS** belongs only to the cleanup source assembly recorded
below: main `f010472bcb6794a2cbc56dfaf755bd122c10225e`, its exact prior three
BED leaves and two own module lines, Molehill slot B starting 13:04:41 UTC.
That result does not execute these three new occupancy cases. Neither the prior
published head nor this source checkpoint installs the BED modules in production.

### Native window and retained execution constraints

[D1399b](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6236882742)
orders the current MAIL D1396a run before HUNT D1392. The coordinator will obtain
a House window after that queue; checkpointing this owned source does not claim
that such a window was granted or that a native run started.
[D1397](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6236432747)
requires the single actual `/home/mole/oteryn-cache/heavy.lock` in Ubuntu/mole.
An existing Ubuntu-24.04 worktree uses the granted
[D1397a outer bridge](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6236550340)
for the whole run, together with the D1342 slot lock and verified environment.
A duplicate lock path in another distribution cannot serialize these jobs.

The actual native recipe remains main `f010472b...` plus the exact three leaves
and two explicitly labelled own module lines, as **SOURCE ASSEMBLY**. Run
`cargo fetch --locked` first, then use only D1329's existing `in-pg-netns.sh`
and shared PostgreSQL17.6. Each case creates and drops only its own bounded
`oteryn_bed_2147_*` database; cleanup failure remains failure. The operator checks
Windows C: space before the run and cleans only its own target/temp files after
it, following D1399b. No server/container lifecycle action is introduced.

### Actual producer gates remain open

[D1370](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234571417)
assigns the real OFFLINE receipt/build extension, chain guards, PG fixture and
FND clock/predecessor source to #2174. Its earlier migration0118 reservation
and the later migration0120 description in
[D1381](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6235225446)
still need an explicit CP reconciliation. This checkpoint makes no migration
number choice. D1381 moves the native marker writer to
`durability/character_build`; the eventual BED hook must bind that actual writer
after the real #2174 and #2196 merges.

The real #2196 food/vitals/soul before-values and mutation participant must share
the same physical transaction. Full active-bundle House placement/currentness
from #2167, installed canonical property/BED schema and actual logout/sleeper
provenance remain separate dependencies. The recorded helper is an inert
proposal, and its participant-required branch must not become a blocking
runtime call. No detached calculator or copied producer substitutes for them.

The actual Logout microsecond anchor binds BED's millisecond row as
`slept_at == committed_at_us / 1000`; multiplying the stored milliseconds would
reject a legitimate submillisecond remainder. The future consumer preserves
the real food millisecond remainder and caps. The native boundary, same-TX
rollback and replay cases still require the actual owners and are **NOT_RUN**.

The PR remains draft; all original production acceptance items stay open.

## 2026-10-17 shared PostgreSQL fixture cleanup — AUTHORING

Predecessor after CP's ordinary main merge:
`b1375be27cf817df99c2e8d7781b6df88523f79b`. Owned authoring is released by
[D1345](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233992418).

This packet changes only the private native test fixture. Each case allocates a
bounded `oteryn_bed_2147_<tag>_<pid>_<nonce>` database through the existing shared
PostgreSQL17.6 helper route. All connection, migration, private-schema and test
work after CREATE is covered by cleanup. Returned errors and assertion panics
drop only that exact owned database before returning or resuming the panic; a
failed DROP remains a failed case. The administrative connection is closed after
cleanup. No PostgreSQL server/container lifecycle operation is introduced.

All nine existing test bodies are preserved exactly: one pure case and eight real
PostgreSQL cases. One additional real PostgreSQL regression injects a returned
error and an assertion panic, checks that each original failure is preserved,
then independently verifies that the owned database is absent from `pg_database`.
The production `durability/bed_sleeper.rs` and `house_bed/sleeper.rs` remain
unchanged. Their private schema, lifecycle triggers and row reads are still not
a production sleep, wake, logout or settlement writer.

Validation of this packet: focused native formatting and compilation PASS;
the coordinator's Molehill run starting 2026-10-17 13:04:41 UTC on slot B passed
all ten cases (`10 passed; 0 failed; 0 ignored; 3269 filtered out`, 19.60 seconds
after a 1m59s build). This includes the original pure case, eight original real
PostgreSQL cases and the new error/panic cleanup regression. The injected panic
is caught and verified by the regression; it is not a skipped or failed case.

The evidence is explicitly a SOURCE ASSEMBLY: main
`f010472bcb6794a2cbc56dfaf755bd122c10225e`, the three actual BED files and only
their lib/durability module registration lines. The tested fixture SHA-256 is
`5c4cae6da93e572fc6880afb32404fc00ead2750f776fa6abcb8884328e997d6`;
the production files match the predecessor exactly. The run used D1342's native
environment and slot target, `cargo fetch --locked` PASS, and D1329's existing
`in-pg-netns.sh` against shared PostgreSQL17.6. No lifecycle action was run.

The published branch has no BED module registration. This ten-case source
assembly therefore does not qualify the published head or install production
BED behavior. Canonical mounted-head tests, package Clippy and production
sleep/wake/logout/settlement/restart qualification remain NOT_RUN or incomplete;
the ordered integration criteria below remain open.

[D1351](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233992418) and
[D1355](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234008098)
resolve the next producer boundaries:

| Required implementation | Owner and remaining gate |
|---|---|
| Full active-bundle House placement and currentness | #2167 at `061c16a6f4ac5f337c56f1189421fa3ecb0cd127`, following #2156. D1355(a) explicitly amends that scope; scoped overlays alone are insufficient. BED resolver conversion waits for it. |
| OFFLINE marker, logout and activation | #2174 at `b5576758e5309dffdd8993bd9956e3c798e985ad`; BED does not replace its transaction owner. |
| Durable food, vitals and soul participant | #2196 at `6fbd70254f323d29a9bad15db74ea0adb9d55a92`; D1355(b) requires actual durable soul before-values in the same physical transaction. |
| Caller-TX BED step | D1355(c) grants #2147 the hook in #2174's `apply_marker`, after its plan and fence locks and before `write_offline_marker`, only after #2174 and #2196 merge. |
| House composition | D1349/D1235 order remains #2141 → #2137 → #2167 → #2147. Specific-cause freeing precedes ownership UPDATE in its owning transaction. |

The granted caller hook is not implemented by this fixture packet. No new durable
schema, copied producer, fake bed resolver or detached regen calculator is added.
The PR remains draft and all production acceptance items remain open.

## 2026-10-16 D1122 checkpoint — historical qualification

This section and its receipts describe the earlier source generation. The
current fixture packet and pending native execution are stated above.

This checkpoint retains source head
`1a9faa28b046a674c5da5a56c9ace6cd6c54267a` on existing draft #2147.
The implemented component is private sleeper storage/read primitives: native
House/Character identity and uniqueness constraints, immutable occupied-to-freed
state, first wake time/cause preservation, ownership-change fallback, transactional
occupancy revisions and caller-transaction row locking. The authored reconnect and
rollback cases check storage persistence and atomicity; they do not supply a
production restart, logout marker or offline-settlement integration.

| Check | Current-source disposition |
|---|---|
| Authored native cases | Nine: one pure and eight PostgreSQL17.6 cases, including competing free/reconnect and insert commit/rollback coverage. Authorship is not execution. |
| Native compilation and selected native Clippy | NOT_RUN; no current-generation successful compiler evidence is claimed. |
| Native test discovery and all nine case executions | NOT_RUN; the source-only native recipe and mount proposal are not execution evidence. |
| PostgreSQL17.6 qualification | NOT_RUN for this source generation; historical old-unit results and portable/facsimile checks cannot qualify these cases. Missing/non-Unicode URL errors in source rather than reporting skipped success. |
| Production bed USE, wake, logout/settlement and restart | NOT_RUN / incomplete; no production writer or runtime authority is delivered by these raw storage primitives. |

The exact source recipe is
`/workspace/house-family-handoff/2147-validation/manifest.json`
(SHA-256 `acd28e3a8b7c534bd34f5269dc304105a35ac158157f1a44f5759096c0d610e4`).
Its three owned Rust hashes match the current files; it explicitly records
`no_jobs_executed` and does not establish native PASS. No current-generation native
test RED or PASS is inferred from earlier results.

CP D1171 assigns OFFLINE/logout-marker/activation to #2174, durable food/soul to
#2196, and the caller-TX sleeper step and BED-CONTENT-1 head/foot leaf to #2147.
The native call in training/offline.rs follows #2174, and any map/facts.rs accessor
follows #2156. Those unimplemented integrations remain incomplete; the initial
interface request is at
[CP2131 comment6224909931](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224909931).
[CP D1171](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6225337534)
resolves the allocation, not the missing implementation or evidence.
The specific-cause free must still precede ownership UPDATE in the same owning
transaction. No caller boolean, raw admin SQL, stored coordinate or separate bed
training backend replaces those native seams. D1162's registration order remains
#2163 → #2141 → #2137 → #2147; no registration or shared wiring is added here.

Fresh focused formatter check on all three current owned Rust files passed:
`rustfmt --edition 2024 --config skip_children=true --check`, official Rust1.94.0.
The exact hash-bound receipt is
`/workspace/house-family-handoff/checkpoint-publication-policy/2147-exact-fmt.json`
(SHA-256 `e0817b37ce1c34d15303eaffad2a8ae6598cc1d3152a1c484882eecd5a655016`). Package fmt remains NOT_RUN.

[CP D1122](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224930201)
allows one coherent checkpoint push on the existing draft with explicit
NOT_FULLY_QUALIFIED / RED / NOT_RUN accounting and passing fmt/whitespace checks;
the lead verified the current three source hashes and formatter result.
The PR remains draft, with no ready action, `@codex review` request or manual CI
restart. Full checks remain required before a frozen review candidate. All
original acceptance items remain open.

## Goal

Sleeper table, bed USE and sleep, owner wake and offline settlement per BED0 §4-§8.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| BED0 acceptance | ACCEPTED: ACCEPT_BATCH §1, owner item 5 (#2209) | acceptance gate released; child dependencies remain |
| OFFLINE-1 | NOT MERGED | **BLOCKING** |
| STATUE-1 (shared activation path) | NOT MERGED | **BLOCKING** |
| HOUSE-RUNTIME-1a, 1b, 1c | NOT MERGED (Wave 3) | **BLOCKING** |
| HOUSE-VIEW-1 | NOT MERGED | **BLOCKING** |
| WORLDINT-WIRE-1 | MERGED (#2094) | ok |
| BED-CONTENT-1 | MERGED (#1861) | ok |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/house_bed/ (new)
- apps/game-server/src/durability/bed_sleeper.rs (new)
- `apps/game-server/src/house_bed/*_tests.rs`

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/00NN_bed_sleepers.sql
- apps/game-server/src/gameplay_transport/item_use.rs / connection.rs (bed USE arm)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (if Reg confirmed)
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/content/item_bed_promotion.rs
- apps/game-server/src/gameplay_transport/item_use.rs (USE map_item arm, WORLDINT-WIRE-1)
- offline activation path (STATUE-1)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability

## Acceptance criteria

- [ ] sleep ends the session through the fenced logout path
- [ ] owner wake and settlement idempotent across restart
- [ ] only the house ACL may use a bed
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server house_bed
cargo clippy -p oteryn-game-server -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md §4-§8
- `docs/architecture/reviews/OTERYN_GAME_ITEM_SEM_BED_GROUP_PACKET_2026-10-05.md`
- `docs/agents/evidence/OTV2-20261005-bed-facts-v1.json`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded local preparation — 2026-10-15, draft #2147

Exact floor `ac3a40da2b2b783b7015b90f3d6d9eb56bcab3f3`, branch
`skeleton/bed-1`. The adjacent four-path allocation governs this preparation;
no historical broad directory grant or registration exception expands it.
Accepted BED0 was read at protected main
`ac9f172836f00226b68088aa04db95ccf1ab994e`; no merge/rebase is performed.
QUEUE_READINESS admits a private sleeper component, not an atomic native writer.

### Implemented private schema/read subset

- `house_bed/sleeper.rs` carries raw persisted rows, the four exact freed causes,
  native appearance::Sex variant, nonzero lease/revision and `StoredBedKey`. Its identity uses the
  existing native `HouseId` (WorldId + revision-free house content key) and native
  `LogicalCell`, never a generated bed UUID or a Channel-local house identity.
  A stored coordinate is explicitly not proof of a valid active-map head pair.
- `durability/bed_sleeper.rs` provides private schema SQL: native property/Character
  FKs, Character-to-World binding, one Character row including freed rows, one
  open sleeper per native BedKey, nonzero u64 lease, closed variant/free causes,
  paired/timestamp-ordered free fields, immutable sleep identity and a one-way
  occupied→freed transition. Freed rows cannot be overwritten/reopened.
- Private `AFTER UPDATE OF owner_character_id` fallback on the actual native
  ownership table frees only open rows with `owner_change` at the database clock,
  after taking the house revision row lock. Same-owner revisions do not wake;
  already-freed rows retain their first time/cause. The native ownership guard
  and writer source/permissions are unchanged. This DDL is not installed.
- Insert/free/delete trigger the house occupancy revision in the same transaction;
  rejected writes and rollback retain the prior revision/state. Revision overflow
  fails the whole mutation. This is storage bookkeeping, not marker provenance.
- Actual single-statement raw read includes the house revision. The caller-owned
  transaction variant takes `FOR UPDATE OF s`, pins an existing row and neither
  starts/commits an independent transaction nor frees/deletes/awards anything.
  Missing revision, bad lease/identity/variant/time/free pairing fail closed.
- `sleeper_tests.rs` exercises the exact owned source and actual native public
  types/SQLx/Tokio, native migrations and isolated PostgreSQL 17.6 fixtures. Native
  root/property fixture insertion is migration-owner setup, not an admission,
  acquisition or current-grant witness. Admin SQL tests private row lifecycle;
  no executable production insert/wake/settlement writer is offered.

`slept_at` and `freed_at` use **Unix milliseconds**, matching the actual native
`character_build.rs` `committed_at = floor(epoch * 1000)` representation. The
owner-change fallback uses database `clock_timestamp()` in those same units.
The OFFLINE owner must own typed elapsed-second conversion/settlement arithmetic;
this component introduces none. A real PG regression with native millisecond
`slept_at` exposed the old seconds fallback (6 PASS / 1 FAIL), now corrected.
Earlier six-test GREEN and native checks are explicitly the superseded old-unit
variant, not evidence for this final source.

No schema is installed in production; no grants, numbered migration, shared
registration, runtime effects, food consumption or vitals/skill truth are added.
All family acceptance checkboxes remain open.

### cross_domain_findings / precise integration packet

1. Native `HouseId` is already public and World-global. Native qualified house
   placement knows House content identities and logical cells, but no active-map
   bed pair/head normalizer is exposed here. BED-CONTENT-1 supplies authoring
   `ReferenceItemBed` facts; these alone do not prove an active pair. USE, choice,
   reservation, insert, view and bed-removed checks must share the real current
   bundle's reciprocal same-house head/foot resolver. Raw stored coordinates
   cannot substitute for that qualification.
2. `game_house_ownership` / `house_access_for_character` supply native ownership
   and ACL reads, but the current property row lacks BED0's OWNED /
   MOVE_OUT_PENDING / content-fence model, and this lease has no accepted guild-
   derived revision grant carrier. The writer must take the actual property
   `FOR SHARE` plus granting guild/membership locks, revalidate current revisions
   and current session/lease/scope/Premium/block/reach conditions. A caller's
   boolean or this raw row is not current authority. Owner-wake/bed-removed remain
   house-runtime-only; access revocation is not a wake cause.
3. `character_build::BuildCause` and migration 0030 admit training,
   vocation_choice and promotion, not logout/offline_settlement markers or
   offline activation storage. Reuse the existing build state/formula/receipt
   owner; do not create a separate bed training or vitals backend. The current
   `FreshAdmissionStore::release` owns terminal release, final position and its
   lifecycle receipt, with no sleeper/build-marker transaction hook. OFFLINE-1 /
   STATUE shared activation and real house logout/admission hooks are required.
4. Logout bundle: exact first-receipt replay before mutations; actual marker
   timestamp/lease and activation, and the native Character sex at insert; cleanup of the Character's older sleeper row;
   qualified new head insert. Bed unique conflict must commit the marker without
   sleeper or activation, not leave training enabled. Canonical multi-house
   revision locking must include the old row's house when cleanup moves between
   houses. Triggers preserve revision atomicity but cannot manufacture the
   required marker/property/grant provenance or lock order.
5. Owner/disposition bundle: real house runtime or ownership transaction acquires
   its property/house-revision locks, then frees only open rows at the database
   clock with the exact accepted cause. A second wake is a no-op preserving its
   first time/cause. The private fallback supplies only actual owner-column changes;
   ACL revocation does not wake. **Mandatory CP ordering:** a disposition or other
   specific-cause free must run BEFORE the native ownership UPDATE in the same
   transaction. Otherwise the generic AFTER fallback would record `owner_change`,
   and the one-way guard correctly refuses rewriting that first cause. There is
   no native operation-cause seam here and no invented context flag. Production
   installation remains blocked until the actual property/disposition writer
   guarantees this order (or owns an accepted explicit native cause seam).
6. Admission bundle: actual offline-settlement transaction reads/locks the row;
   only matching previous lease plus that lease's latest logout build receipt
   count. It deletes any row, including stale/freed, with revision advancement
   and first-receipt replay. `sleep_s=min(d,freed_duration_s when freed, otherwise d)`, cap
   21 days; training still runs over OFFLINE0's d after owner wake. No plain
   caller-supplied latest-marker flag or standalone delete is accepted as this
   integration. Apply that accepted seconds-duration rule through the OFFLINE
   owner's conversion from native Unix millisecond timestamps; this raw component
   does not guess settlement conversion/rounding. BED-REGEN remains separate until
   DUR-02 durable vitals: no food
   consumed and no regeneration coefficients promoted from PARITY_PENDING.
7. CP slot must install schema + permissions/marker provenance guards + actual
   native writer/hooks inseparably, then revalidate then-current main and test
   fence/revision changes, marker conflicts, ambiguous commit/replay, disposition,
   stale previous lease, head/foot equivalence and settlement/value conservation.
   No registry, wire, shared module or content/pin edits are prepared in this lease.

### Evidence

Commands, original logs, initial/final source hashes, native snapshot registration
manifest and scope/self-review are in
`/workspace/oteryn-prep/bed-1-evidence/index.md`. This is private lifecycle/read
qualification only, not live sleep, wake, offline training or settlement delivery.

## Owner-assigned authoring repair — 2026-10-16

The owner directly confirmed exclusive assignment of this existing draft to the
current family lead and its sole BED-1 author; the previous team will not write.
No additional CP lease ID is required. This instruction supersedes the earlier
missing-allocation-artifact uncertainty; it does not broaden this round's four
paths: `house_bed/sleeper.rs`, `house_bed/sleeper_tests.rs`,
`durability/bed_sleeper.rs` and this card. The authoring floor remains exact PR
head `2d5be266946baae97f988f7cc46f6415464b1ba7`, worktree
`/workspace/bed-1`. No shared registration, migration, publication or review
trigger is authorized in this round.

CP finding P2 (silent PostgreSQL success) is **fixed in source, execution pending**:
`run_case` now returns an error when `OTERYN_TEST_POSTGRES_ADMIN_URL` is missing
or not Unicode. There is no success/skip fallback. The existing canonical local
admin-route restriction and exact PostgreSQL `server_version_num=170006` check
remain mandatory. No credential is printed by the new error.

Two additional PostgreSQL cases exercise the actual existing private schema:

- Competing owner-wake/bed-removed storage transitions: an uncommitted first free
  blocks the loser (`55P03`), the committed first time/cause remains unchanged,
  and the losing retry affects zero rows with no extra occupancy revision. A new
  connection reloads the same first result and repeats the losing no-op.
- Competing inserts into one bed: a pending winner blocks its competitor; commit
  preserves uniqueness (`23505`) without an extra revision. A second house case
  rolls back the initial insert, lets the competitor succeed, and verifies the
  rolled-back Character row/revision did not survive or affect the other house.

These are migration-owner storage tests, not runtime authority or a real process
restart qualification. The new connection reload is not a claim that restart,
OFFLINE receipt replay, current access or a production wake writer is delivered.

On this exact authoring floor `BuildCause` still contains only `Training`,
`VocationChoice` and `Promotion`; no native logout/offline marker transaction
exists. `FreshAdmissionStore::release` has no sleeper-marker hook. No registered
house runtime supplies current house/node/assignment, active head-pair or
owner-wake/bed-removed proof on this branch. An ACL read alone cannot provide
those missing facts. Therefore production sleep/free/delete APIs are not added:
the affected integration remains with OFFLINE/STATUE and HOUSE owners. A missing
accepted owner seam is not replaced by caller booleans, raw admin SQL or a new
authority trait. The disposition-before-ownership-UPDATE requirement above
remains unchanged.

Validation in this round: `git diff --check` and the governance validator pass;
all 124 governance unit tests pass. The family lead formatted the owned
`sleeper_tests.rs` source with verified official Rustfmt 1.94
(`/workspace/house-tools/rustfmt`, edition 2024, `skip_children`); focused source
formatting passes. Package `cargo fmt`, Clippy, all nine sleeper tests and
PostgreSQL remain unrun: D1011 requires the owner-PC executor and no global PC
slot is available. The executor must register/qualify these owned modules under the
actual native test manifest and run all nine sleeper tests (one pure, eight
PostgreSQL 17.6), plus required package formatting/Clippy. Missing URL must fail
the PG cases; canonical 17.6 must execute them, not report skipped success.
All family acceptance checkboxes remain open; this repair does not close BED-1.

## 2026-10-17 source proposal recovery archive — NOT_APPLIED

At the owner's request, the previously external source proposal is preserved
below inside this task record, rather than only in a local handoff or PR comment.
The `patch_lines` array joins to the original UTF-8 patch bytes and SHA256.
The original base and historical source-only review are retained for recovery;
they do not qualify the current branch or authorize applying shared hunks.
Recheck the actual successor source and obtain the required CP shared slot,
migration allocation and independent review before implementation.
**Shared proposal NOT_APPLIED; native execution NOT_RUN; runtime acceptance
and all existing integration gates remain unchanged.** This appendix changes
only this owned task record and introduces no executable implementation.

Previous reviewable handoff: https://github.com/Oteryn/Oteryn-Game/pull/2147#issuecomment-6230753007.

```json
{
  "archive_version": 1,
  "owning_pr": 2147,
  "status": "SOURCE_PROPOSAL_NOT_APPLIED_NOT_RUNTIME_QUALIFIED",
  "original_source_sha": "720636b7170ce2b9b618a0769c9ff0797a717a4a",
  "patch_sha256": "ab0988670268f110f8ac7e0098e1d34e3828ab3de8d18ab959b015a5c4352d8c",
  "patch_lines": [
    "--- a/apps/game-server/src/map/boot.rs\n",
    "+++ b/apps/game-server/src/map/boot.rs\n",
    "@@ -52,6 +52,14 @@\n",
    "     StartNotEnterable,\n",
    "     /// A palette Item key the §1.6 item index lacks or places at another `id`.\n",
    "     ItemReference,\n",
    "+    /// A real placed bed lacks this bundle's qualified House metadata.\n",
    "+    BedPlacementUnavailable {\n",
    "+        world: WorldId,\n",
    "+        item: crate::durability::item_mint::TypedDefinitionRef,\n",
    "+    },\n",
    "+    BedDefinitionUnavailable {\n",
    "+        world: WorldId,\n",
    "+    },\n",
    "     /// The bundle places no door a player can use (MAP-DOOR-1), which a preprod node refuses.\n",
    "     NoDoors,\n",
    "     /// MAP-FLOOR-1: a floor-change tile whose successive destinations never end.\n",
    "@@ -69,6 +77,12 @@\n",
    "             Self::StartNotEnterable => f.write_str(\"the world bundle start tile is not enterable\"),\n",
    "             Self::ItemReference => {\n",
    "                 f.write_str(\"a world bundle palette item does not match the item index\")\n",
    "+            }\n",
    "+            Self::BedPlacementUnavailable { .. } => {\n",
    "+                f.write_str(\"placed bed lacks active bundle House qualification\")\n",
    "+            }\n",
    "+            Self::BedDefinitionUnavailable { .. } => {\n",
    "+                f.write_str(\"placed item bed definition is Unknown or Conflict\")\n",
    "             }\n",
    "             Self::NoDoors => f.write_str(\"the world bundle places no usable door\"),\n",
    "             Self::NoFloorChanges => {\n",
    "@@ -96,6 +110,12 @@\n",
    "         match refusal {\n",
    "             FactsRefusal::Load(error) => Self::Load(error),\n",
    "             FactsRefusal::ItemReference => Self::ItemReference,\n",
    "+            FactsRefusal::BedDefinitionUnavailable { world } => {\n",
    "+                Self::BedDefinitionUnavailable { world }\n",
    "+            }\n",
    "+            FactsRefusal::BedPlacementUnavailable { world, item } => {\n",
    "+                Self::BedPlacementUnavailable { world, item }\n",
    "+            }\n",
    "         }\n",
    "     }\n",
    " }\n",
    "@@ -188,7 +208,7 @@\n",
    "         item: impl Fn(&str) -> Option<ItemDefinition>,\n",
    "     ) -> Result<BundleWorld, BootRefusal> {\n",
    "         let mut spell_tiles = SpellTilesBuilder::new(Arc::clone(&self.base));\n",
    "-        let (facts, blocked, floors) = scan(&self.data, &item, &mut spell_tiles)?;\n",
    "+        let (facts, blocked, floors) = scan(world_id, &self.data, &item, &mut spell_tiles)?;\n",
    "         let spell_tiles = Arc::new(spell_tiles.finish(facts.doors())?);\n",
    "         let blocked = Arc::new(blocked);\n",
    "         if !enterable(&self.base, &blocked, self.start) || facts.doors().blocks(self.start) {\n",
    "@@ -237,6 +257,7 @@\n",
    " /// catalogue `floor_change` fact (MAP-FLOOR-1) or a `teleport` attribute (NEWHAVEN-TELEPORT-1).\n",
    " /// `spell_tiles` collects the spell tile facts of the same sectors (BUNDLE-TILES-1).\n",
    " fn scan(\n",
    "+    world_id: WorldId,\n",
    "     data: &[u8],\n",
    "     item: &impl Fn(&str) -> Option<ItemDefinition>,\n",
    "     spell_tiles: &mut SpellTilesBuilder,\n",
    "@@ -300,7 +321,12 @@\n",
    "                 );\n",
    "             }\n",
    "         }\n",
    "-        facts.push(manifest, &sector, &mut blocked);\n",
    "+        if let Err(refusal) = facts.push(world_id, manifest, &sector, &mut blocked) {\n",
    "+            refused = Some(refusal);\n",
    "+            return Err(oteryn_world_bundle::Error::Format(\n",
    "+                \"placed bed lacks active bundle House qualification\".into(),\n",
    "+            ));\n",
    "+        }\n",
    "         spell_tiles.push(manifest, &sector, &blocked, item, &masks);\n",
    "         Ok(())\n",
    "     });\n",
    "@@ -1048,6 +1074,7 @@\n",
    "             blocks_projectile: solid == Some(true),\n",
    "             pickupable: false,\n",
    "             door: false,\n",
    "+            bed: crate::content::ReferenceItemField::NotApplicable,\n",
    "         })\n",
    "     }\n",
    " \n",
    "@@ -1077,6 +1104,7 @@\n",
    "             blocks_projectile: solid == Some(true),\n",
    "             pickupable,\n",
    "             door: false,\n",
    "+            bed: crate::content::ReferenceItemField::NotApplicable,\n",
    "         })\n",
    "     }\n",
    " \n",
    "@@ -1322,6 +1350,7 @@\n",
    "                     blocks_projectile: false,\n",
    "                     pickupable: false,\n",
    "                     door: false,\n",
    "+                    bed: crate::content::ReferenceItemField::NotApplicable,\n",
    "                 }),\n",
    "                 \"item:coin\" => items(key).map(|definition| ItemDefinition {\n",
    "                     solid: None,\n",
    "@@ -1433,6 +1462,7 @@\n",
    "             blocks_projectile: solid == Some(true),\n",
    "             pickupable,\n",
    "             door: key == \"oteryn:item.tibia.i1629\",\n",
    "+            bed: crate::content::ReferenceItemField::NotApplicable,\n",
    "         })\n",
    "     }\n",
    " \n",
    "@@ -1632,6 +1662,7 @@\n",
    "         let world = door_world_with(|key| {\n",
    "             door_items(key).map(|item| ItemDefinition {\n",
    "                 door: false,\n",
    "+                bed: crate::content::ReferenceItemField::NotApplicable,\n",
    "                 ..item\n",
    "             })\n",
    "         })?;\n",
    "@@ -1656,6 +1687,7 @@\n",
    "                         blocks_projectile: true,\n",
    "                         pickupable: false,\n",
    "                         door: true,\n",
    "+                        bed: crate::content::ReferenceItemField::NotApplicable,\n",
    "                     })\n",
    "                 } else {\n",
    "                     door_items(key)\n",
    "--- a/apps/game-server/src/map/facts.rs\n",
    "+++ b/apps/game-server/src/map/facts.rs\n",
    "@@ -23,6 +23,9 @@\n",
    " use super::overlay::{AddedItem, TilePos};\n",
    " use super::view::{self, EntryFacts, MapFacts, placement_key};\n",
    " use super::{LoadError, palette_appearance};\n",
    "+use crate::content::{ReferenceItemBed, ReferenceItemField};\n",
    "+use crate::durability::item_mint::TypedDefinitionRef;\n",
    "+use crate::foundation::WorldId;\n",
    " use oteryn_protocol_oteryn::world_map::{MAX_BASE_ORDINAL, MapDefinition};\n",
    " use oteryn_world_bundle::bundle::{Family, Manifest, PaletteEntry, Sector, TerrainKind};\n",
    " use oteryn_world_bundle::sector::Attrs;\n",
    "@@ -30,7 +33,7 @@\n",
    " use std::num::NonZeroU32;\n",
    " \n",
    " /// The served definition of the Item a palette key names.\n",
    "-#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n",
    "+#[derive(Debug, Clone, PartialEq, Eq)]\n",
    " pub struct ItemDefinition {\n",
    "     /// The §1.6 index entry of the key: 1 + its Item compact id in the active generation.\n",
    "     pub reference: NonZeroU32,\n",
    "@@ -41,6 +44,8 @@\n",
    "     /// The content profile's `item_type` is `Door` (MAP-DOOR-1): the only thing that makes an\n",
    "     /// Item a door, whatever its appearance id.\n",
    "     pub door: bool,\n",
    "+    /// BED-CONTENT-1 truth state from the served key/revision; absence stays Unknown.\n",
    "+    pub bed: ReferenceItemField<(TypedDefinitionRef, ReferenceItemBed)>,\n",
    " }\n",
    " \n",
    " /// Why the bundle's palette cannot be served.\n",
    "@@ -49,6 +54,15 @@\n",
    "     Load(LoadError),\n",
    "     /// A palette Item key that the §1.6 index lacks or places at another `id`.\n",
    "     ItemReference,\n",
    "+    /// A placed bed needs this bundle's own qualified House placement; ENTRY-room\n",
    "+    /// metadata and numeric tile.house cannot supply that missing qualification.\n",
    "+    BedPlacementUnavailable {\n",
    "+        world: WorldId,\n",
    "+        item: TypedDefinitionRef,\n",
    "+    },\n",
    "+    BedDefinitionUnavailable {\n",
    "+        world: WorldId,\n",
    "+    },\n",
    " }\n",
    " \n",
    " impl From<LoadError> for FactsRefusal {\n",
    "@@ -58,13 +72,14 @@\n",
    " }\n",
    " \n",
    " /// The facts of one palette entry, and whether an Item entry is solid.\n",
    "-#[derive(Debug, Clone, Copy)]\n",
    "+#[derive(Debug, Clone)]\n",
    " struct PaletteFacts {\n",
    "     family: Family,\n",
    "     id: u32,\n",
    "     facts: EntryFacts,\n",
    "     solid: bool,\n",
    "     door: PaletteDoor,\n",
    "+    bed: ReferenceItemField<(TypedDefinitionRef, ReferenceItemBed)>,\n",
    " }\n",
    " \n",
    " /// The per-placement facts that differ from the palette entry's.\n",
    "@@ -138,10 +153,11 @@\n",
    "     /// door entry does not block here: [`DoorBook`] answers it by its current state.\n",
    "     pub(crate) fn push(\n",
    "         &mut self,\n",
    "+        world_id: WorldId,\n",
    "         manifest: &Manifest,\n",
    "         sector: &Sector,\n",
    "         blocked: &mut BTreeSet<TilePos>,\n",
    "-    ) {\n",
    "+    ) -> Result<(), FactsRefusal> {\n",
    "         for tile in &sector.tiles {\n",
    "             let pos = TilePos {\n",
    "                 x: tile.x,\n",
    "@@ -154,6 +170,24 @@\n",
    "             let top = tile.items.iter().filter(|item| item.depth == 0);\n",
    "             let mut blocks = false;\n",
    "             for (ordinal, item) in top.enumerate() {\n",
    "+                match self\n",
    "+                    .palette\n",
    "+                    .get(item.palette as usize)\n",
    "+                    .map(|entry| &entry.bed)\n",
    "+                {\n",
    "+                    Some(ReferenceItemField::NotApplicable) => {}\n",
    "+                    Some(ReferenceItemField::Known((identity, _bed))) => {\n",
    "+                        // Only the actual bundle's qualified House receipt can bind\n",
    "+                        // this placed bed. ENTRY metadata and tile.house cannot.\n",
    "+                        return Err(FactsRefusal::BedPlacementUnavailable {\n",
    "+                            world: world_id,\n",
    "+                            item: identity.clone(),\n",
    "+                        });\n",
    "+                    }\n",
    "+                    Some(ReferenceItemField::Unknown | ReferenceItemField::Conflict) | None => {\n",
    "+                        return Err(FactsRefusal::BedDefinitionUnavailable { world: world_id });\n",
    "+                    }\n",
    "+                }\n",
    "                 let door = self\n",
    "                     .palette\n",
    "                     .get(item.palette as usize)\n",
    "@@ -218,6 +252,7 @@\n",
    "                 blocked.insert(pos);\n",
    "             }\n",
    "         }\n",
    "+        Ok(())\n",
    "     }\n",
    " \n",
    "     /// Orders the recorded placements and house tiles for lookup.\n",
    "@@ -375,6 +410,7 @@\n",
    "     };\n",
    "     let mut solid = false;\n",
    "     let mut door = PaletteDoor::No;\n",
    "+    let mut bed = ReferenceItemField::NotApplicable;\n",
    "     if entry.family == Family::Item {\n",
    "         let definition = item(&entry.key)\n",
    "             .filter(|definition| definition.reference == reference)\n",
    "@@ -386,6 +422,7 @@\n",
    "             .solid\n",
    "             .unwrap_or_else(|| !walkable_floor_change(&entry.key) && !terrain_floor(entry));\n",
    "         door = palette_door(&entry.key, item);\n",
    "+        bed = definition.bed;\n",
    "     }\n",
    "     Ok(PaletteFacts {\n",
    "         family: entry.family,\n",
    "@@ -393,6 +430,7 @@\n",
    "         facts,\n",
    "         solid,\n",
    "         door,\n",
    "+        bed,\n",
    "     })\n",
    " }\n",
    " \n",
    "@@ -494,6 +532,60 @@\n",
    "     use crate::foundation::{ChannelId, WorldId};\n",
    "     use crate::map::boot::{boot, tests as fixture};\n",
    " \n",
    "+    #[test]\n",
    "+    fn placed_known_bed_refuses_missing_bundle_qualification_with_actual_world() {\n",
    "+        use crate::content::{ReferenceBedDirection, ReferenceBedPart, ReferenceItemTarget};\n",
    "+        let world =\n",
    "+            WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).expect(\"id\");\n",
    "+        let channel =\n",
    "+            ChannelId::decode(&[2, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).expect(\"id\");\n",
    "+        let (bytes, pins) = fixture::floor_bundle();\n",
    "+        let item = |key: &str| {\n",
    "+            let mut definition = fixture::floor_items(key)?;\n",
    "+            let occupied =\n",
    "+                ReferenceItemTarget::new(\"oteryn:item.tibia.i3702\", \"bed-test-r1\").expect(\"target\");\n",
    "+            definition.bed = ReferenceItemField::Known((\n",
    "+                TypedDefinitionRef {\n",
    "+                    family: \"Item\".into(),\n",
    "+                    production_key: key.into(),\n",
    "+                    revision_ref: \"bed-test-r1\".into(),\n",
    "+                },\n",
    "+                ReferenceItemBed {\n",
    "+                    part: ReferenceBedPart::Head,\n",
    "+                    partner_direction: ReferenceBedDirection::South,\n",
    "+                    occupied_male: occupied.clone(),\n",
    "+                    occupied_female: occupied,\n",
    "+                },\n",
    "+            ));\n",
    "+            Some(definition)\n",
    "+        };\n",
    "+        assert!(\n",
    "+            matches!(boot(&bytes, &fixture::floor_pins(pins), world, channel, item),\n",
    "+            Err(crate::map::boot::BootRefusal::BedPlacementUnavailable { world: observed, item })\n",
    "+                if observed == world && item.family == \"Item\" && item.revision_ref == \"bed-test-r1\")\n",
    "+        );\n",
    "+    }\n",
    "+\n",
    "+    #[test]\n",
    "+    fn placed_unknown_or_conflicting_bed_truth_is_not_proven_nonbed() {\n",
    "+        let world =\n",
    "+            WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).expect(\"id\");\n",
    "+        let channel =\n",
    "+            ChannelId::decode(&[2, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).expect(\"id\");\n",
    "+        for truth in [ReferenceItemField::Unknown, ReferenceItemField::Conflict] {\n",
    "+            let (bytes, pins) = fixture::floor_bundle();\n",
    "+            let item = |key: &str| {\n",
    "+                let mut definition = fixture::floor_items(key)?;\n",
    "+                definition.bed = truth.clone();\n",
    "+                Some(definition)\n",
    "+            };\n",
    "+            assert!(\n",
    "+                matches!(boot(&bytes, &fixture::floor_pins(pins), world, channel, item),\n",
    "+                Err(crate::map::boot::BootRefusal::BedDefinitionUnavailable { world: observed }) if observed == world)\n",
    "+            );\n",
    "+        }\n",
    "+    }\n",
    "+\n",
    "     fn item_solid(key: &str) -> bool {\n",
    "         let entry = PaletteEntry {\n",
    "             key: key.into(),\n",
    "@@ -508,6 +600,7 @@\n",
    "                 blocks_projectile: false,\n",
    "                 pickupable: false,\n",
    "                 door: false,\n",
    "+                bed: ReferenceItemField::NotApplicable,\n",
    "             })\n",
    "         };\n",
    "         palette_facts(&entry, &definition).expect(\"facts\").solid\n",
    "@@ -549,6 +642,7 @@\n",
    "                         blocks_projectile: false,\n",
    "                         pickupable: false,\n",
    "                         door: false,\n",
    "+                        bed: ReferenceItemField::NotApplicable,\n",
    "                     })\n",
    "                 };\n",
    "                 palette_facts(&entry, &definition).expect(\"facts\").solid\n",
    "--- a/apps/game-server/src/map/spell_tiles.rs\n",
    "+++ b/apps/game-server/src/map/spell_tiles.rs\n",
    "@@ -461,6 +461,7 @@\n",
    "                 blocks_projectile: false,\n",
    "                 pickupable: false,\n",
    "                 door: false,\n",
    "+                bed: crate::content::ReferenceItemField::NotApplicable,\n",
    "             });\n",
    "         }\n",
    "         let index = KEYS.iter().position(|known| *known == key)?;\n",
    "@@ -477,6 +478,7 @@\n",
    "             blocks_projectile,\n",
    "             pickupable,\n",
    "             door: index == 8,\n",
    "+            bed: crate::content::ReferenceItemField::NotApplicable,\n",
    "         })\n",
    "     }\n",
    " \n",
    "@@ -730,6 +732,19 @@\n",
    "                             == ReferenceItemField::Known(ReferenceItemType::Door)\n",
    "                 )\n",
    "             });\n",
    "+            let bed = match profile.map(|profile| &profile.semantics.bed) {\n",
    "+                Some(ReferenceItemField::Known(bed)) => ReferenceItemField::Known((\n",
    "+                    crate::durability::item_mint::TypedDefinitionRef {\n",
    "+                        family: \"Item\".into(),\n",
    "+                        production_key: key.into(),\n",
    "+                        revision_ref: revision.into(),\n",
    "+                    },\n",
    "+                    bed.clone(),\n",
    "+                )),\n",
    "+                Some(ReferenceItemField::NotApplicable) => ReferenceItemField::NotApplicable,\n",
    "+                Some(ReferenceItemField::Conflict) => ReferenceItemField::Conflict,\n",
    "+                Some(ReferenceItemField::Unknown) | None => ReferenceItemField::Unknown,\n",
    "+            };\n",
    "             let attributes = profile.map(|profile| &profile.attributes);\n",
    "             Some(ItemDefinition {\n",
    "                 reference,\n",
    "@@ -739,6 +754,7 @@\n",
    "                     .unwrap_or(true),\n",
    "                 pickupable,\n",
    "                 door,\n",
    "+                bed,\n",
    "             })\n",
    "         }\n",
    "     }\n",
    "--- a/apps/game-server/src/node/serve.rs\n",
    "+++ b/apps/game-server/src/node/serve.rs\n",
    "@@ -1959,6 +1959,19 @@\n",
    "                     if classification.item_type == ReferenceItemField::Known(ReferenceItemType::Door)\n",
    "             )\n",
    "         });\n",
    "+        let bed = match profile.map(|profile| &profile.semantics.bed) {\n",
    "+            Some(ReferenceItemField::Known(bed)) => ReferenceItemField::Known((\n",
    "+                crate::durability::item_mint::TypedDefinitionRef {\n",
    "+                    family: \"Item\".into(),\n",
    "+                    production_key: key.into(),\n",
    "+                    revision_ref: revision.into(),\n",
    "+                },\n",
    "+                bed.clone(),\n",
    "+            )),\n",
    "+            Some(ReferenceItemField::NotApplicable) => ReferenceItemField::NotApplicable,\n",
    "+            Some(ReferenceItemField::Conflict) => ReferenceItemField::Conflict,\n",
    "+            Some(ReferenceItemField::Unknown) | None => ReferenceItemField::Unknown,\n",
    "+        };\n",
    "         let attributes = profile.map(|profile| &profile.attributes);\n",
    "         Some(crate::map::facts::ItemDefinition {\n",
    "             reference,\n",
    "@@ -1968,6 +1981,7 @@\n",
    "                 .unwrap_or(true),\n",
    "             pickupable,\n",
    "             door,\n",
    "+            bed,\n",
    "         })\n",
    "     };\n",
    "     checked\n"
  ],
  "historical_source_manifest": {
    "state": "PARTIAL_COHERENT_SOURCE_PROPOSAL_NOT_COMPLETE_D1231_ROOT_REVIEW_PENDING",
    "source_sha": "720636b7170ce2b9b618a0769c9ff0797a717a4a",
    "pr": 2147,
    "dependency_pr": 2156,
    "cp": "D1200/D1231 6226177496",
    "files": {
      "apps/game-server/src/map/boot.rs": {
        "before_sha256": "6516fa76093e21b4aecd805adb798064b30667bc8a46a938bb51f66c1f8d9a1b",
        "after_sha256": "81fcd1ee5d48182fdce0284689c10542609531e7cd6255b6a544a04f6786bdb6"
      },
      "apps/game-server/src/map/facts.rs": {
        "before_sha256": "2767286d0a5d33aae5c7d313dc53f9638e45b9bc2fc40a96eaa1696d96bb920a",
        "after_sha256": "ed58f23462708d85def16fcb27d6b96ef9895cd0e1150b04ad7fa27d4bee568f"
      },
      "apps/game-server/src/map/spell_tiles.rs": {
        "before_sha256": "35862440a341e58e2c813b73943c3711c6de15ff02b1457711e4d608a3ac317d",
        "after_sha256": "eb0dd617b00bc7c3fd6e24561fa2ffa68ce52ccd517d14e6cf727b5b51ceab75"
      },
      "apps/game-server/src/node/serve.rs": {
        "before_sha256": "395ea324ddfbbfcb3436e75ddd992bc022ded4827b8762d96466740c00212125",
        "after_sha256": "d3b9f95dea0d5c35008c56e8c5dee5f00bfa00781a8a44d1cb755b0a2317bd3f"
      }
    },
    "patch_sha256": "ab0988670268f110f8ac7e0098e1d34e3828ab3de8d18ab959b015a5c4352d8c",
    "implemented": [
      "Actual served item_index key/revision + Known ReferenceItemBed producer in node/serve.rs, same realproducer in pinnedbundle test helper",
      "Real native TypedDefinitionRef tuple with bed facts carried through ItemDefinition into PaletteFacts; no itemnumber/appearance identity guesses",
      "Actual WorldId passed CheckedBundle::boot->scan->BundleFacts::push",
      "Only actually placed top-level Known bed refuses with BedPlacementUnavailable; unused palette metadata does not refuse; nonbed behavior retained",
      "Updated real callers/testfixtures for nonCopy ownedtypedbed metadata; one actualsource unit case authored, NOT_RUN",
      "Preserve Known tuple/NotApplicable/Unknown/Conflict in realproducer and PaletteFacts; missingprofile isUnknown, nevernonbed",
      "Only actuallyplaced Unknown/Conflict (or missingpaletteentry) refuses BedDefinitionUnavailable; explicitfixtureNotApplicable only",
      "Added actualsource placedUnknown/Conflict regression, authoredNOT_RUN"
    ],
    "remaining_exact_gap": [
      "Active bundle QualifiedHouseTiles producer and same-generation receipt/access are absent: NativeEntryMovementCells::from_bundle at exact2156 clones ENTRY-room house_tiles under old scope and says bundlelookups must refuse",
      "No actualbundle placement.contains(cell) exactone -> HouseId::new(world,&def.key) binding or corresponding physicalrevision/content/placement receipt can be supplied by currentbootAPI",
      "Do not reuse unrelatedentry fixture, stringify tile.house, addnumber->key map, claim staticfacts as DBcurrentness, or add uncalledmockresolver",
      "When realbundle producer exists, exactoneplacement+tile.housezero/nonzero consistency and callerTX game_world_house_instances FORSHARE revision+bothdigests remain required"
    ],
    "qualification": "No HouseId/bedplacement or runtimeauthority granted; knownplacedbedbootfailclosed until genuinebundlebinding is supplied",
    "native_tests": "NOT_RUN",
    "pc_calls": false,
    "shared_apply": false,
    "push_merge": false,
    "source_only_fmt_apply_checks": "PASS: official1.94edition2024skip_children four proposedRustfiles and gitapply--check exactsnapshot; no native execution",
    "root_review_finding": "V1 _=>None finding accepted and fixed; firstreview preserved inroot-review-v1-finding.json andv1patch/manifest"
  },
  "historical_source_review": {
    "state": "PASS_SOURCE_ONLY_PARTIAL_ROUTE_NOT_COMPLETE_D1231",
    "reviewer": "Root independent of author checkpoint_policy_medium",
    "source_sha": "720636b7170ce2b9b618a0769c9ff0797a717a4a",
    "patch_sha256": "ab0988670268f110f8ac7e0098e1d34e3828ab3de8d18ab959b015a5c4352d8c",
    "fixed_finding": "V1 collapsed Unknown/Conflict/missingprofile into None; final preserves exact truthstates endtoend, onlyNotApplicableprovesnonbed",
    "verified": [
      "Actual served key/revision TypedDefinitionRef plus native Known ReferenceItemBed, no guessed identity",
      "Real WorldId boot->scan->push and typed refusal from actual sectorload",
      "Only actually placed top-level bedunknown/knownmissingqualification refuses; unusedpalette and explicitNotApplicableremain valid",
      "Actual producers/changedfixtures in4sourcefiles; no shared apply",
      "D1235 confirms activebundle producer2167, ENTRYmetadata cannot supplybinding"
    ],
    "new_findings": [],
    "limits": [
      "No complete BedPlacementFact/beds vector/bed_at until actual2167QualifiedHouseTiles receipt",
      "Exactonecontains, tile.houseconsistency and sameTX0047revision/content/placement checks NOT_IMPLEMENTED",
      "Shared order2156then2167then2147; directusernoMerge/rebase preserved",
      "Compile/native tests NOT_RUN; fmt/applychecks source-onlyPASS"
    ],
    "CP_dependency": {
      "decision": "D1235",
      "comment": 6226289022,
      "owningPR": 2167
    }
  }
}
```

## Reproducible occupancy source checkpoint

The following UTF-8 artifact is preserved in the reversible text encoding described below within this owned card. Its embedded patch and review may be recovered without adding a repository evidence path. Artifact bytes: 32417; SHA-256: `26078095a20640dfce02e6126d3ae5f8537e1d84b4cc0c8b645529b56f398e7b`.

### Reversible archive encoding

The archived Markdown below uses one reversible escape to keep unified-diff context lines compatible with the repository whitespace gate. A standalone line `\x20` represents exactly one ASCII space followed by a newline. Decode those lines before checking the original archive or patch hashes, or recovering the held source. Every other archive byte, including the final newline, is unchanged. This encoding grants no permission to apply the archived runtime changes.

For a UTF-8 string `card` containing this task card, the original artifact can be recovered with:

```python
archive = card.split("`````markdown\n", 1)[1].split("`````", 1)[0]
archive = "".join(
    " \n" if line == "\\x20\n" else line
    for line in archive.splitlines(keepends=True)
)
```

<!-- card-artifact:bed2147-occupancy-source-checkpoint.md -->
`````markdown
# BED-1 — raw occupancy source checkpoint

State: **frozen formatted source; native compilation and 13-case execution NOT_RUN under the D1399b window order**. Published predecessor: `87674e53cd4be954b19dfb049a5d41013182fc03`. This document preserves the exact three source leaves handed to root for native qualification. The coordinator will publish these exact three Rust leaves and the owned task card, which retains this evidence in its appendix; this documentation preparation has changed none of the frozen Rust bytes.

## Prepared behavior

`StoredBedOccupancy` is a raw stored-data view of an exact World/House/head-cell key, an optional nonzero House occupancy revision and the open sleeper at that key. One actual PostgreSQL statement reads the revision and occupant in one MVCC snapshot. It preserves every key component, refuses ambiguous open rows, and refuses an open House sleeper whose required occupancy counter is absent. A missing counter is recorded as `None` when no open row requires it; this does not mean revision zero and is not proof that a House or bed is current or newly created.

The read neither grants authority nor writes state. It introduces no schema, trigger, pool, module mount or runtime caller. The existing private BED schema remains a separately installed dependency; isolated fixture qualification is not production installation.

## Regression inventory and review

The ten published case bodies remain present. Three actual-PG cases add exact-key/free/reuse coverage, a concurrent snapshot test across rollback and commit, and missing-counter/ambiguous-row failure coverage. Inventory: **13 cases: one pure and twelve PostgreSQL cases**. The existing cleanup regression exercises both returned errors and a caught assertion panic; each fixture may drop only its own uniquely named database.

Independent source review passed for production hashes `9b111c1a...` and `50ed072d...` and the pre-format test hash `05dbf0ed...`. Root then applied native rustfmt to the test file, producing `b375914e741a007647d88009db0b18199b56fbc77eedb463f2c79713935bb26b`; the two production hashes stayed unchanged. The exact formatting patch and original review are embedded for an auditable relationship. Root confirmed that the 13-case run has not started: D1399b orders the current MAIL window and then HUNT ahead of the requested House window. The old ten-case PASS must not be relabeled as a result for these new cases.

## Native execution boundary

The recorded recipe uses actual main `f010472bcb6794a2cbc56dfaf755bd122c10225e` plus these three leaves and the two explicit BED module lines as **SOURCE ASSEMBLY**. Root alone controls the sequential Molehill run, full-run slot lock, verified build environment and target directory. First run `cargo fetch --locked`, then run PG through the existing `in-pg-netns.sh` under D1329 using only the shared PG 17.6. No PostgreSQL server/container lifecycle action or another database is authorized by this evidence.

The previous cleanup packet had a separate source-assembly result: **10/10 PASS** on Molehill slot B starting 2026-10-17 13:04:41 UTC, at main `f010472bcb6794a2cbc56dfaf755bd122c10225e` plus its exact three leaves and two own mounts. The published #2147 branch was unmounted, so that was not exact-head qualification. This occupancy packet has no native execution result. Root authorizes one bounded source checkpoint with an accurate task card and embedded evidence before the House window; it remains NOT_FULLY_QUALIFIED and unmounted. Only a later actual terminal receipt can qualify the thirteen-case run.


### Updated shared PC execution order

[D1399b](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6236882742) keeps heavy runs sequential: current MAIL D1396a, then HUNT D1392. A House window is still required. Use the actual Ubuntu/mole `/home/mole/oteryn-cache/heavy.lock` for the full run under [D1397](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6236432747). Existing Ubuntu-24.04 inputs use the granted [D1397a bridge](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6236550340), not a duplicate lock or a new mount. Check Windows C: free space before execution, retain the D1342 slot/environment and D1329 shared PG route, and remove only the run's own target/temp afterward. New worktrees belong in Ubuntu/mole. This checkpoint starts no native job.

## Recovery and integrity

This evidence is retained in the owned task card alongside the exact three owned Rust leaves in one bounded source checkpoint. Its embedded patch, formatting delta and review remain data: they register no module, install no dependency schema and activate no caller. No new repository evidence path or scope expansion is used. Runtime permission, dependency order and qualification gates still apply.

Recover each artifact from the matching section below. Preserve its exact UTF-8 bytes and verify the listed byte length and SHA-256. Reconstruct an isolated review tree from each file's pinned base ref and Git blob in `source-inventory.json`, then check and apply the patch there. Never apply a historical patch to an unspecified current checkout. The checkpoint preparation independently applied the recorded patch to those exact preimages and compared every resulting file byte for byte with the frozen source. That integrity check is not Rust compilation or PostgreSQL qualification.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| `source-inventory.json` | 2660 | `53385190859e85511604068f310d588ba4194a3c90f13c6baf4b43344e81052f` |
| `bed2147-occupancy-formatted.patch` | 13243 | `22493b008617fc6c78e7096b1513b21233924d17d47a6e1886f223c253049078` |
| `native-format.patch` | 8199 | `b528eef524f7e28c63f2e5f0451473cf516b818d47536188afcd2acb0a48d1f3` |
| `independent-source-review.md` | 1980 | `d1ea8f53c53f74c1b3f851b2dca443a930adc532329352ffe0c9791af6eacdd8` |

## Embedded artifacts

### source-inventory.json

<!-- checkpoint-artifact:source-inventory.json -->
````json
{
  "pr": 2147,
  "expected_predecessor": "87674e53cd4be954b19dfb049a5d41013182fc03",
  "production_source_predecessor": "87674e53cd4be954b19dfb049a5d41013182fc03",
  "evidence_class": "FROZEN formatted source checkpoint; native compilation and 13-case execution NOT_RUN under D1399b; no PASS claim",
  "mount_recipe": [
    {
      "path": "apps/game-server/src/durability/mod.rs",
      "add_if_missing": "pub mod bed_sleeper;",
      "scope": "one own module line"
    },
    {
      "path": "apps/game-server/src/lib.rs",
      "add_if_missing": "pub mod house_bed { pub mod sleeper; }",
      "scope": "one own namespace registration; rustfmt normalizes only this added hunk"
    }
  ],
  "cargo_fetch": "cargo fetch --locked",
  "test_command": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "oteryn-game-server",
    "--lib",
    "house_bed::sleeper::tests::",
    "--",
    "--test-threads=1",
    "--nocapture"
  ],
  "expected_tests": 13,
  "expected_original_tests": 10,
  "expected_pg_tests": 12,
  "pg_route": "D1329 existing in-pg-netns.sh; shared PG17.6 only, no lifecycle actions. Each case creates/drops only its own oteryn_bed_2147_* DB.",
  "change": "Read-only raw occupancy by exact StoredBedKey, one SQL snapshot with House-wide revision, no caller/authority/mutations/new schema. Three real-PG cases added; published baseline ten test bodies preserved.",
  "files": [
    {
      "path": "apps/game-server/src/durability/bed_sleeper.rs",
      "base_ref": "87674e53cd4be954b19dfb049a5d41013182fc03",
      "base_git_blob": "d15c64b976f1ce9250eb30d8601546b266bef71d",
      "base_sha256": "7a4d88ce68e80e7ce7638bd329050f006b8dd16d2c77c43cda2dabf9d5f4d39f",
      "result_sha256": "9b111c1aad110e6abf5013ca499815e91544d3b1a445d191553d770664e77405",
      "result_bytes": 13084
    },
    {
      "path": "apps/game-server/src/house_bed/sleeper.rs",
      "base_ref": "87674e53cd4be954b19dfb049a5d41013182fc03",
      "base_git_blob": "c23d1fcd3e6ef32f11db9d7b17459bf0d3ef3456",
      "base_sha256": "06ec9e07e3328998dfedc33ba1c6db072a5dfdc3165a1f0d3c6395520a5fc3ca",
      "result_sha256": "50ed072d5aca2deb7382985e18406bcc95cf7098a9d0783797fc71c11feeb336",
      "result_bytes": 2539
    },
    {
      "path": "apps/game-server/src/house_bed/sleeper_tests.rs",
      "base_ref": "87674e53cd4be954b19dfb049a5d41013182fc03",
      "base_git_blob": "92f4c51775fa13827b3fb2c128c978a57d080f5c",
      "base_sha256": "5c4cae6da93e572fc6880afb32404fc00ead2750f776fa6abcb8884328e997d6",
      "result_sha256": "b375914e741a007647d88009db0b18199b56fbc77eedb463f2c79713935bb26b",
      "result_bytes": 36044
    }
  ]
}
````

### bed2147-occupancy-formatted.patch

<!-- checkpoint-artifact:bed2147-occupancy-formatted.patch -->
````diff
--- a/apps/game-server/src/durability/bed_sleeper.rs
+++ b/apps/game-server/src/durability/bed_sleeper.rs
@@ -12,7 +12,7 @@
 use crate::domain::CharacterId;
 use crate::foundation::WorldId;
 use crate::house_bed::sleeper::{
-    FreedCause, FreedSleeper, OccupiedVariant, StoredBedKey, StoredSleeper,
+    FreedCause, FreedSleeper, OccupiedVariant, StoredBedKey, StoredBedOccupancy, StoredSleeper,
 };
 use crate::house_scope::HouseId;
 use sqlx::postgres::{PgConnection, PgRow};
@@ -135,6 +135,82 @@
         .transpose()
 }
\x20
+const READ_BED_OCCUPANCY: &str = "WITH wanted AS (SELECT encode($1,'hex')::uuid AS world_id, \
+    $2::text AS house_key, $3::integer AS bed_x, $4::integer AS bed_y, $5::integer AS bed_z) \
+    SELECT uuid_send(s.world_id) AS world_identity,s.house_key,s.bed_x,s.bed_y,s.bed_z, \
+    uuid_send(s.character_id) AS character_identity,s.lease_generation::text AS lease, \
+    s.occupied_variant,s.slept_at,s.freed_at,s.freed_cause,r.revision::text AS revision, \
+    EXISTS(SELECT 1 FROM game_house_bed_sleepers live \
+        WHERE live.world_id=w.world_id AND live.house_key=w.house_key \
+        AND live.freed_at IS NULL) AS has_open_sleeper \
+    FROM wanted w LEFT JOIN game_house_bed_revisions r \
+        ON r.world_id=w.world_id AND r.house_key=w.house_key \
+    LEFT JOIN game_house_bed_sleepers s \
+        ON s.world_id=w.world_id AND s.house_key=w.house_key \
+        AND s.bed_x=w.bed_x AND s.bed_y=w.bed_y AND s.bed_z=w.bed_z \
+        AND s.freed_at IS NULL LIMIT 2";
+
+/// Reads the exact key and its House-wide revision in one statement snapshot.
+/// At most two rows are returned so a broken uniqueness constraint refuses
+/// instead of silently choosing an occupant. The open-row check uses the
+/// existing partial bed index and catches a missing House revision even when
+/// another bed in that House is occupied.
+///
+/// This starts no transaction and takes no gameplay or mutation authority.
+/// A None occupant is raw storage data, not permission to reserve or insert;
+/// callers must still resolve the active bed and hold their actual native
+/// runtime/property/access fences. No marker or settlement is inferred.
+pub async fn read_bed_occupancy(
+    connection: &mut PgConnection,
+    bed: &StoredBedKey,
+) -> Result<StoredBedOccupancy, SleeperReadError> {
+    let rows = sqlx::query(READ_BED_OCCUPANCY)
+        .bind(bed.house.world_id().as_bytes().as_slice())
+        .bind(bed.house.house_key())
+        .bind(bed.head.x)
+        .bind(bed.head.y)
+        .bind(bed.head.z)
+        .fetch_all(connection)
+        .await?;
+    let [row] = rows.as_slice() else {
+        return Err(SleeperReadError::InvalidStoredState);
+    };
+    let invalid = || SleeperReadError::InvalidStoredState;
+    let occupancy_revision = row
+        .try_get::<Option<String>, _>("revision")?
+        .map(|value| {
+            value
+                .parse::<u64>()
+                .ok()
+                .and_then(NonZeroU64::new)
+                .ok_or_else(invalid)
+        })
+        .transpose()?;
+    if occupancy_revision.is_none() && row.try_get::<bool, _>("has_open_sleeper")? {
+        return Err(invalid());
+    }
+    let sleeper = row
+        .try_get::<Option<Vec<u8>>, _>("character_identity")?
+        .map(|bytes| {
+            let character = CharacterId::from_bytes(bytes.try_into().map_err(|_| invalid())?)
+                .map_err(|_| invalid())?;
+            let sleeper = decode(row, character)?;
+            if &sleeper.bed != bed
+                || sleeper.freed.is_some()
+                || Some(sleeper.occupancy_revision) != occupancy_revision
+            {
+                return Err(invalid());
+            }
+            Ok(sleeper)
+        })
+        .transpose()?;
+    Ok(StoredBedOccupancy {
+        bed: bed.clone(),
+        occupancy_revision,
+        sleeper,
+    })
+}
+
 /// Pins an existing row until the caller's transaction ends. This does not
 /// settle, delete, wake, qualify current authority or resolve the offline marker.
 /// Future mutation owners must acquire their actual property/revision locks in
--- a/apps/game-server/src/house_bed/sleeper.rs
+++ b/apps/game-server/src/house_bed/sleeper.rs
@@ -57,6 +57,20 @@
     pub occupancy_revision: NonZeroU64,
 }
\x20
+/// One raw storage snapshot for an exact bed key, including its House-wide
+/// occupancy revision. This is not proof that the key names an active bed,
+/// that the House exists, or that a character may use or wake it.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct StoredBedOccupancy {
+    pub bed: StoredBedKey,
+    /// None means no revision row was observed. It does not establish that the
+    /// House or bed is new; callers still need their actual native owners.
+    pub occupancy_revision: Option<NonZeroU64>,
+    /// Only an open sleeper can occupy this key. Freed rows remain available
+    /// to their character's settlement and are not returned as occupants.
+    pub sleeper: Option<StoredSleeper>,
+}
+
 #[cfg(test)]
 #[path = "sleeper_tests.rs"]
 mod tests;
--- a/apps/game-server/src/house_bed/sleeper_tests.rs
+++ b/apps/game-server/src/house_bed/sleeper_tests.rs
@@ -1,7 +1,8 @@
 //! Real private-schema tests. No marker, access grant or gameplay sleep is simulated.
 use super::*;
 use crate::durability::bed_sleeper::{
-    PRIVATE_SLEEPER_SCHEMA_SQL, SleeperReadError, read_sleeper, read_sleeper_for_update,
+    PRIVATE_SLEEPER_SCHEMA_SQL, SleeperReadError, read_bed_occupancy, read_sleeper,
+    read_sleeper_for_update,
 };
 use sqlx::{Connection, PgConnection};
\x20
@@ -679,3 +680,193 @@
         Ok(())
     })
 }
+
+fn stored_bed(
+    world: u8,
+    house: &str,
+    head: LogicalCell,
+) -> Result<StoredBedKey, Box<dyn std::error::Error>> {
+    let world = crate::foundation::WorldId::decode(&id(world)).map_err(debug)?;
+    Ok(StoredBedKey {
+        house: HouseId::new(world, house).ok_or("invalid fixture HouseId")?,
+        head,
+    })
+}
+
+#[test]
+fn bed_occupancy_lookup_is_exact_and_preserves_freed_rows_during_reuse() -> TestResult {
+    run_case("bed_lookup", async |c| {
+        let head = LogicalCell { x: 10, y: 20, z: 7 };
+        let bed = stored_bed(8, "oteryn:content.house.a", head)?;
+        let empty = read_bed_occupancy(c, &bed).await.map_err(debug)?;
+        assert_eq!(empty.bed, bed);
+        assert_eq!(empty.occupancy_revision, None);
+        assert_eq!(empty.sleeper, None);
+
+        insert(c, 1, "oteryn:content.house.a", 8).await?;
+        insert(c, 2, "oteryn:content.house.b", 8).await?;
+        insert(c, 4, "oteryn:content.house.c", 9).await?;
+        let occupied = read_bed_occupancy(c, &bed).await.map_err(debug)?;
+        assert_eq!(occupied.bed, bed);
+        assert_eq!(occupied.occupancy_revision, NonZeroU64::new(1));
+        assert_eq!(
+            occupied.sleeper,
+            read_sleeper(c, character(1)?).await.map_err(debug)?
+        );
+
+        for head in [
+            LogicalCell { x: 11, y: 20, z: 7 },
+            LogicalCell { x: 10, y: 21, z: 7 },
+            LogicalCell { x: 10, y: 20, z: 8 },
+        ] {
+            let target = stored_bed(8, "oteryn:content.house.a", head)?;
+            let empty = read_bed_occupancy(c, &target).await.map_err(debug)?;
+            assert_eq!(empty.bed, target);
+            assert_eq!(empty.occupancy_revision, NonZeroU64::new(1));
+            assert_eq!(empty.sleeper, None);
+        }
+        for (world, house, who) in [
+            (8, "oteryn:content.house.b", 2),
+            (9, "oteryn:content.house.c", 4),
+        ] {
+            let target = stored_bed(world, house, head)?;
+            let other = read_bed_occupancy(c, &target).await.map_err(debug)?;
+            assert_eq!(other.bed, target);
+            assert_eq!(other.occupancy_revision, NonZeroU64::new(1));
+            assert_eq!(
+                other.sleeper,
+                read_sleeper(c, character(who)?).await.map_err(debug)?
+            );
+        }
+        let different_world = stored_bed(9, "oteryn:content.house.a", head)?;
+        let unknown = read_bed_occupancy(c, &different_world)
+            .await
+            .map_err(debug)?;
+        assert_eq!(unknown.bed, different_world);
+        assert_eq!(unknown.occupancy_revision, None);
+        assert_eq!(unknown.sleeper, None);
+
+        sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=110,freed_cause='owner_wake' WHERE character_id=encode($1,'hex')::uuid")
+            .bind(id(1).as_slice()).execute(&mut *c).await?;
+        let free = read_bed_occupancy(c, &bed).await.map_err(debug)?;
+        assert_eq!(free.occupancy_revision, NonZeroU64::new(2));
+        assert_eq!(free.sleeper, None);
+        let retained = read_sleeper(c, character(1)?)
+            .await
+            .map_err(debug)?
+            .ok_or("missing freed row")?;
+        assert_eq!(
+            retained.freed,
+            Some(FreedSleeper {
+                at: 110,
+                cause: FreedCause::OwnerWake
+            })
+        );
+        insert(c, 3, "oteryn:content.house.a", 8).await?;
+        let reused = read_bed_occupancy(c, &bed).await.map_err(debug)?;
+        assert_eq!(reused.occupancy_revision, NonZeroU64::new(3));
+        assert_eq!(
+            reused.sleeper,
+            read_sleeper(c, character(3)?).await.map_err(debug)?
+        );
+        assert_eq!(
+            read_sleeper(c, character(1)?)
+                .await
+                .map_err(debug)?
+                .ok_or("lost freed row")?
+                .freed,
+            retained.freed
+        );
+        assert_eq!(revision(c, "oteryn:content.house.a").await?, "3");
+        Ok(())
+    })
+}
+
+#[test]
+fn bed_occupancy_revision_and_occupant_share_the_same_commit_snapshot() -> TestResult {
+    run_case("bed_snapshot", async |c| {
+        insert(c, 1, "oteryn:content.house.a", 8).await?;
+        let bed = stored_bed(
+            8,
+            "oteryn:content.house.a",
+            LogicalCell { x: 10, y: 20, z: 7 },
+        )?;
+        let before = read_bed_occupancy(c, &bed).await.map_err(debug)?;
+        let mut other = another_connection(c).await?;
+        for commit in [false, true] {
+            let mut tx = c.begin().await?;
+            sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=110,freed_cause='owner_wake' WHERE character_id=encode($1,'hex')::uuid")
+                .bind(id(1).as_slice()).execute(&mut *tx).await?;
+            insert(&mut tx, 2, "oteryn:content.house.a", 8).await?;
+            let inside = read_bed_occupancy(&mut tx, &bed).await.map_err(debug)?;
+            assert_eq!(inside.occupancy_revision, NonZeroU64::new(3));
+            assert_eq!(
+                inside.sleeper.as_ref().map(|s| s.character),
+                Some(character(2)?)
+            );
+            assert_eq!(
+                read_bed_occupancy(&mut other, &bed).await.map_err(debug)?,
+                before
+            );
+            if commit {
+                tx.commit().await?;
+                assert_eq!(
+                    read_bed_occupancy(&mut other, &bed).await.map_err(debug)?,
+                    inside
+                );
+            } else {
+                tx.rollback().await?;
+                assert_eq!(read_bed_occupancy(c, &bed).await.map_err(debug)?, before);
+                assert_eq!(read_sleeper(c, character(2)?).await.map_err(debug)?, None);
+            }
+        }
+        other.close().await?;
+        Ok(())
+    })
+}
+
+#[test]
+fn bed_occupancy_refuses_missing_revision_and_ambiguous_open_rows() -> TestResult {
+    run_case("bed_invalid", async |c| {
+        insert(c, 1, "oteryn:content.house.a", 8).await?;
+        let bed = stored_bed(
+            8,
+            "oteryn:content.house.a",
+            LogicalCell { x: 10, y: 20, z: 7 },
+        )?;
+        let empty = stored_bed(
+            8,
+            "oteryn:content.house.a",
+            LogicalCell { x: 11, y: 20, z: 7 },
+        )?;
+        // Corruption exists only in this test's own migration-owner database.
+        sqlx::query("DELETE FROM game_house_bed_revisions")
+            .execute(&mut *c)
+            .await?;
+        for target in [&bed, &empty] {
+            assert!(matches!(
+                read_bed_occupancy(c, target).await,
+                Err(SleeperReadError::InvalidStoredState)
+            ));
+        }
+        sqlx::query("INSERT INTO game_house_bed_revisions VALUES(encode($1,'hex')::uuid,'oteryn:content.house.a',1)")
+            .bind(id(8).as_slice()).execute(&mut *c).await?;
+        assert!(
+            read_bed_occupancy(c, &bed)
+                .await
+                .map_err(debug)?
+                .sleeper
+                .is_some()
+        );
+        sqlx::query("DROP INDEX game_house_bed_live_key")
+            .execute(&mut *c)
+            .await?;
+        insert(c, 2, "oteryn:content.house.a", 8).await?;
+        assert_eq!(revision(c, "oteryn:content.house.a").await?, "2");
+        assert!(matches!(
+            read_bed_occupancy(c, &bed).await,
+            Err(SleeperReadError::InvalidStoredState)
+        ));
+        Ok(())
+    })
+}
````

### native-format.patch

<!-- checkpoint-artifact:native-format.patch -->
````diff
--- a/apps/game-server/src/house_bed/sleeper_tests.rs
+++ b/apps/game-server/src/house_bed/sleeper_tests.rs
@@ -681,7 +681,11 @@
     })
 }
\x20
-fn stored_bed(world: u8, house: &str, head: LogicalCell) -> Result<StoredBedKey, Box<dyn std::error::Error>> {
+fn stored_bed(
+    world: u8,
+    house: &str,
+    head: LogicalCell,
+) -> Result<StoredBedKey, Box<dyn std::error::Error>> {
     let world = crate::foundation::WorldId::decode(&id(world)).map_err(debug)?;
     Ok(StoredBedKey {
         house: HouseId::new(world, house).ok_or("invalid fixture HouseId")?,
@@ -705,7 +709,10 @@
         let occupied = read_bed_occupancy(c, &bed).await.map_err(debug)?;
         assert_eq!(occupied.bed, bed);
         assert_eq!(occupied.occupancy_revision, NonZeroU64::new(1));
-        assert_eq!(occupied.sleeper, read_sleeper(c, character(1)?).await.map_err(debug)?);
+        assert_eq!(
+            occupied.sleeper,
+            read_sleeper(c, character(1)?).await.map_err(debug)?
+        );
\x20
         for head in [
             LogicalCell { x: 11, y: 20, z: 7 },
@@ -718,15 +725,23 @@
             assert_eq!(empty.occupancy_revision, NonZeroU64::new(1));
             assert_eq!(empty.sleeper, None);
         }
-        for (world, house, who) in [(8, "oteryn:content.house.b", 2), (9, "oteryn:content.house.c", 4)] {
+        for (world, house, who) in [
+            (8, "oteryn:content.house.b", 2),
+            (9, "oteryn:content.house.c", 4),
+        ] {
             let target = stored_bed(world, house, head)?;
             let other = read_bed_occupancy(c, &target).await.map_err(debug)?;
             assert_eq!(other.bed, target);
             assert_eq!(other.occupancy_revision, NonZeroU64::new(1));
-            assert_eq!(other.sleeper, read_sleeper(c, character(who)?).await.map_err(debug)?);
+            assert_eq!(
+                other.sleeper,
+                read_sleeper(c, character(who)?).await.map_err(debug)?
+            );
         }
         let different_world = stored_bed(9, "oteryn:content.house.a", head)?;
-        let unknown = read_bed_occupancy(c, &different_world).await.map_err(debug)?;
+        let unknown = read_bed_occupancy(c, &different_world)
+            .await
+            .map_err(debug)?;
         assert_eq!(unknown.bed, different_world);
         assert_eq!(unknown.occupancy_revision, None);
         assert_eq!(unknown.sleeper, None);
@@ -736,13 +751,32 @@
         let free = read_bed_occupancy(c, &bed).await.map_err(debug)?;
         assert_eq!(free.occupancy_revision, NonZeroU64::new(2));
         assert_eq!(free.sleeper, None);
-        let retained = read_sleeper(c, character(1)?).await.map_err(debug)?.ok_or("missing freed row")?;
-        assert_eq!(retained.freed, Some(FreedSleeper { at: 110, cause: FreedCause::OwnerWake }));
+        let retained = read_sleeper(c, character(1)?)
+            .await
+            .map_err(debug)?
+            .ok_or("missing freed row")?;
+        assert_eq!(
+            retained.freed,
+            Some(FreedSleeper {
+                at: 110,
+                cause: FreedCause::OwnerWake
+            })
+        );
         insert(c, 3, "oteryn:content.house.a", 8).await?;
         let reused = read_bed_occupancy(c, &bed).await.map_err(debug)?;
         assert_eq!(reused.occupancy_revision, NonZeroU64::new(3));
-        assert_eq!(reused.sleeper, read_sleeper(c, character(3)?).await.map_err(debug)?);
-        assert_eq!(read_sleeper(c, character(1)?).await.map_err(debug)?.ok_or("lost freed row")?.freed, retained.freed);
+        assert_eq!(
+            reused.sleeper,
+            read_sleeper(c, character(3)?).await.map_err(debug)?
+        );
+        assert_eq!(
+            read_sleeper(c, character(1)?)
+                .await
+                .map_err(debug)?
+                .ok_or("lost freed row")?
+                .freed,
+            retained.freed
+        );
         assert_eq!(revision(c, "oteryn:content.house.a").await?, "3");
         Ok(())
     })
@@ -752,7 +786,11 @@
 fn bed_occupancy_revision_and_occupant_share_the_same_commit_snapshot() -> TestResult {
     run_case("bed_snapshot", async |c| {
         insert(c, 1, "oteryn:content.house.a", 8).await?;
-        let bed = stored_bed(8, "oteryn:content.house.a", LogicalCell { x: 10, y: 20, z: 7 })?;
+        let bed = stored_bed(
+            8,
+            "oteryn:content.house.a",
+            LogicalCell { x: 10, y: 20, z: 7 },
+        )?;
         let before = read_bed_occupancy(c, &bed).await.map_err(debug)?;
         let mut other = another_connection(c).await?;
         for commit in [false, true] {
@@ -762,11 +800,20 @@
             insert(&mut tx, 2, "oteryn:content.house.a", 8).await?;
             let inside = read_bed_occupancy(&mut tx, &bed).await.map_err(debug)?;
             assert_eq!(inside.occupancy_revision, NonZeroU64::new(3));
-            assert_eq!(inside.sleeper.as_ref().map(|s| s.character), Some(character(2)?));
-            assert_eq!(read_bed_occupancy(&mut other, &bed).await.map_err(debug)?, before);
+            assert_eq!(
+                inside.sleeper.as_ref().map(|s| s.character),
+                Some(character(2)?)
+            );
+            assert_eq!(
+                read_bed_occupancy(&mut other, &bed).await.map_err(debug)?,
+                before
+            );
             if commit {
                 tx.commit().await?;
-                assert_eq!(read_bed_occupancy(&mut other, &bed).await.map_err(debug)?, inside);
+                assert_eq!(
+                    read_bed_occupancy(&mut other, &bed).await.map_err(debug)?,
+                    inside
+                );
             } else {
                 tx.rollback().await?;
                 assert_eq!(read_bed_occupancy(c, &bed).await.map_err(debug)?, before);
@@ -782,20 +829,44 @@
 fn bed_occupancy_refuses_missing_revision_and_ambiguous_open_rows() -> TestResult {
     run_case("bed_invalid", async |c| {
         insert(c, 1, "oteryn:content.house.a", 8).await?;
-        let bed = stored_bed(8, "oteryn:content.house.a", LogicalCell { x: 10, y: 20, z: 7 })?;
-        let empty = stored_bed(8, "oteryn:content.house.a", LogicalCell { x: 11, y: 20, z: 7 })?;
+        let bed = stored_bed(
+            8,
+            "oteryn:content.house.a",
+            LogicalCell { x: 10, y: 20, z: 7 },
+        )?;
+        let empty = stored_bed(
+            8,
+            "oteryn:content.house.a",
+            LogicalCell { x: 11, y: 20, z: 7 },
+        )?;
         // Corruption exists only in this test's own migration-owner database.
-        sqlx::query("DELETE FROM game_house_bed_revisions").execute(&mut *c).await?;
+        sqlx::query("DELETE FROM game_house_bed_revisions")
+            .execute(&mut *c)
+            .await?;
         for target in [&bed, &empty] {
-            assert!(matches!(read_bed_occupancy(c, target).await, Err(SleeperReadError::InvalidStoredState)));
+            assert!(matches!(
+                read_bed_occupancy(c, target).await,
+                Err(SleeperReadError::InvalidStoredState)
+            ));
         }
         sqlx::query("INSERT INTO game_house_bed_revisions VALUES(encode($1,'hex')::uuid,'oteryn:content.house.a',1)")
             .bind(id(8).as_slice()).execute(&mut *c).await?;
-        assert!(read_bed_occupancy(c, &bed).await.map_err(debug)?.sleeper.is_some());
-        sqlx::query("DROP INDEX game_house_bed_live_key").execute(&mut *c).await?;
+        assert!(
+            read_bed_occupancy(c, &bed)
+                .await
+                .map_err(debug)?
+                .sleeper
+                .is_some()
+        );
+        sqlx::query("DROP INDEX game_house_bed_live_key")
+            .execute(&mut *c)
+            .await?;
         insert(c, 2, "oteryn:content.house.a", 8).await?;
         assert_eq!(revision(c, "oteryn:content.house.a").await?, "2");
-        assert!(matches!(read_bed_occupancy(c, &bed).await, Err(SleeperReadError::InvalidStoredState)));
-        Ok(())
-    })
-}
+        assert!(matches!(
+            read_bed_occupancy(c, &bed).await,
+            Err(SleeperReadError::InvalidStoredState)
+        ));
+        Ok(())
+    })
+}
````

### independent-source-review.md

<!-- checkpoint-artifact:independent-source-review.md -->
````markdown
# Independent BED occupancy source review

Result: **PASS — no material source findings** in the bounded raw read component.

Reviewed candidate based on published #2147 `87674e53cd4be954b19dfb049a5d41013182fc03`.
Exact source hashes:

| File | SHA-256 |
|---|---|
| apps/game-server/src/durability/bed_sleeper.rs | 9b111c1aad110e6abf5013ca499815e91544d3b1a445d191553d770664e77405 |
| apps/game-server/src/house_bed/sleeper.rs | 50ed072d5aca2deb7382985e18406bcc95cf7098a9d0783797fc71c11feeb336 |
| apps/game-server/src/house_bed/sleeper_tests.rs | 05dbf0eda36c0590d98417b1736f42f3d89105a79117586a4df3b1193eb7ac04 |

The query binds the complete native World/House/x/y/z identity. It returns the
occupant and House-wide revision within one statement snapshot; the two-row limit
detects ambiguous live occupancy instead of choosing a first row. The correlated
open-row check rejects missing revision state even when the requested bed is empty
and a different bed in the same House is occupied. Existing decode validation is
reused, and exact key/revision equality is checked before returning a sleeper.

Freed rows are excluded from occupancy and retained for character settlement.
An absent row/counter remains explicitly raw storage state: the type and function
documentation confer no active-content, ownership, access, reservation, logout
marker or runtime authority. No transaction, schema, mutation, runtime mount or
activation is added by this component.

The added tests use the actual private schema and real connections. They cover
full key isolation, free/reuse while retaining the old freed row, statement
visibility before/after commit and rollback, missing revision state and two live
occupants after corruption limited to the test's own database.

This is a source review only. I did not run native compilation or PostgreSQL, and
do not treat the new tests as passing. Published BED source remains the dependency
of the separate House/Guild qualification assembly.
````

`````

# BED-1 — house beds: sleeper, bed USE, owner wake, settlement

```yaml
task_id: BED-1
title: "BED-1: house beds: sleeper, bed USE, owner wake, settlement"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/bed-1
base_sha: 5742f30f3
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

## 2026-10-16 D1122 checkpoint — NOT_FULLY_QUALIFIED

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

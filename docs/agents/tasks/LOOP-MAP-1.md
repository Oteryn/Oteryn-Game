# LOOP-MAP-1 — map loop: Cyclopedia discovery, donations and respawn (skeleton task record)

```yaml
task_id: LOOP-MAP-1
title: "LOOP-MAP-1: map loop: Cyclopedia discovery, donations and respawn"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row LOOP-MAP-1; §7 'LOOP-MAP-1' (CYCLOPEDIA0 child graph)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_CYCLOPEDIA0_MAP_DISCOVERY_AND_AREA_DONATIONS_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_VIEWPORT_BUDGET_2026-10-06.md
  - docs/architecture/reviews/OTERYN_GAME_BANK_FEE0_FEES_FROM_THE_BANK_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/loop-map-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Close the map loop: the client-visible map runs within the viewport budget, Characters discover map areas durably (Cyclopedia), donate to areas through the bank fee path and see respawn facts, with parity against the reference.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

CYC-CONTENT-1 -> CYC-DISCOVERY-1 (CHAR-REV-SEQ-1) -> CYC-DONATE-1 (BANK-FEE-0) -> CYC-RESPAWN-1 (SPAWN-1b) -> CYC-WIRE-1 (Reg) -> CYC-PARITY-1. MAP-VIEWPORT-OPT-1 / MAP-VIEWPORT-LIMIT-1 are handled separately and are NOT part of this draft.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| #1942 client proof | BLOCKING (draft PR open) |
| MAP-VIEWPORT-OPT-1 | BLOCKING (in hand, W3a2) |
| MAP-VIEWPORT-LIMIT-1 | BLOCKING (not merged; in hand) |
| acceptance of CYCLOPEDIA0 (CYC-* children) | BLOCKING |
| MAP-LOAD-1 | MERGED (archive MAP-LOAD-1) |
| MAP-OVERLAY-1a / 1b | MERGED (#1800, #1809) |
| MAP-OVERLAY-1c | BLOCKING (Wave 3, not merged) |
| VIS-2 | MERGED (archive OTV2-20260930-vis-2) |
| CHAR-REV-SEQ-1 (CYC-DISCOVERY-1) | MERGED (archive OTV2-20261003-char-rev-seq1) |
| BANK-FEE-0 (CYC-DONATE-1) | BLOCKING (CANDIDATE decision) |
| SPAWN-1a / SPAWN-1b (CYC-RESPAWN-1) | MERGED (archive OTV2-20261011-spawn-1b) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `content/cyclopedia/** (sources only; generated outputs are slot-only)`
- `apps/game-server/src/domain/cyclopedia.rs`
- `apps/game-server/src/durability/character_cyclopedia.rs`
- `apps/game-server/src/durability/character_cyclopedia_tests.rs`
- `apps/game-server/src/gameplay_transport/cyclopedia.rs`
- `apps/game-server/src/gameplay_transport/cyclopedia_tests.rs`
- `crates/protocol-oteryn/src/cyclopedia.rs`
- `apps/game-server/tests/cyclopedia_postgres.rs`
- `docs/agents/evidence/LOOP-MAP-1-*.md`
- `docs/agents/tasks/LOOP-MAP-1.md` (this record: status and notes only)

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

- connection.rs dispatch chain (CYCLOPEDIA intent next to CHAT_INTENT l.2735)
- capabilities.rs `GATED` l.165 (CYCLOPEDIA capability not offered until CP decides)
- crates/protocol-oteryn/src/lib.rs `pub mod` l.2545-2568
- durability/mod.rs registration; map/view.rs read-only for discovery hook

## Inseparable bundles (never split across PRs)

- CYC-DISCOVERY-1: migration + discovery writer + character revision sequence bump
- CYC-DONATE-1: donation debit + bank fee audit in one transaction
- CYC-WIRE-1: wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] Discovered areas persist per Character and survive restart; duplicate discovery is idempotent.
- [ ] Donation debits the bank once and records the area total atomically; concurrent donations serialize.
- [ ] Respawn facts come from SPAWN-1b data, not client input.
- [ ] Map stays within the MAP-VIEWPORT budget (evidence MAP-VIEWPORT-MEASURE-1).
- [ ] Older clients receive no Cyclopedia domain.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server cyclopedia
cargo test -p oteryn-game-server map
cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_CYCLOPEDIA0_MAP_DISCOVERY_AND_AREA_DONATIONS_DECISION_2026-10-03.md
- docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_VIEWPORT_BUDGET_2026-10-06.md §1.2
- docs/agents/evidence/MAP-VIEWPORT-MEASURE-1-viewport.md
- PR #1942 (client proof, open draft)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Historical REPORT_ONLY input qualification and implementation handoff — 2026-10-15

This is a concrete input/admission packet, not implementation or full LOOP-MAP-1
acceptance. The then-current write lease was this task record, at
`skeleton/loop-map-1` source `3732d7b0434ecb5cbb5f62626f332c6d19c7a33a`.
Protected-main `ac9f172836f00226b68088aa04db95ccf1ab994e` contains the **ACCEPTED**
CYCLOPEDIA0 decision; its original skeleton/dependency history above is retained.
The local QUEUE_READINESS nomination is REPORT_ONLY_INPUT_GATE. Historical PR
and viewport status rows are not fresh lifecycle evidence; viewport optimization
and limit tasks remain outside this packet. No new authority is reconstructed.

### Actual source findings

Sparse filesystem omissions are not missing repository inputs. The following
were read from the exact local Git objects with lazy fetching disabled:

| Existing source | Verified facts | What it does not establish |
|---|---|---|
| `imports/cipsoft-staticdata/map/{areas,subareas,markers}/manifest.json` and their shards | 465 area observations; 28 parents; 441 child references to 437 unique IDs; 158 positioned areas; 1,270 named markers; 1,157 layers = 209 subarea masks +741 satellite +207 minimap | The manifests explicitly say SOURCE_OBSERVATIONS_ONLY. Markers are not a POI pool. Raw flags and names do not prove discoverability/donation eligibility. |
| Pinned `content/assets/files/map-c54dfeb8f3880e9534f1330fa547f06d99c5afa55f641b875ceee23cc04e3496.dat` | Raw SHA-256 equals the staged pin; native existing `read_map` reproduces the area/mask/marker counts | Its undocumented fields require the accepted semantic cross-check; a matching hash is not product admission. |
| The 209 named mask assets and existing `tools/content-schema/world-authoring/client_map_reader.py` | All compressed file hashes match staged observations; existing Tibia framing/LZMA/BMP reader verifies each decoded-BMP filename hash and shape. Streaming one mask at a time yields 5,723,063 pixels, 1,901,667 member tiles, zero overlaps across these masks | No atlas/membership content is emitted. This does not check Wiki examples, canonical discoverable set, actual served-world reachability or POI validity. 228 of 437 raw child IDs have no mask; none may be made discoverable by guessing a rectangle/anchor. |
| `content/world/areas/{regions,cities}/areas-*.json`, `tools/content-schema/area-authoring/{README.md,build_areas.py}` | 28 regions +415 subregions +22 cities =465; source_id and source-record SHA-256 crosswalk matches all staged observations | Candidate Area catalogue deliberately does not model shapes or activate a runtime. It is useful identity provenance, not admitted CYC membership. |
| `content/world/areas/{hunting-places,islands}/...json` | 445 hunting declarations have names/point locations/source bindings; 60 island declarations include anchors/footprints | Neither a point nor an island bounding footprint supplies CYC subarea pixels, discoverability, or POI pools. Do not substitute them for the client-source crosswalk. |

One specific graph ambiguity must be resolved by CYC-CONTENT-1 rather than hidden:
raw source_id **422 (Tibiadrome)** is listed by **five** parents (source IDs
2, 7, 14, 20, 24). Thus 441 references are not 441 distinct subareas. The generic
Area builder's `parent_of` comprehension (`build_areas.py:297`) keeps one last
parent; that implementation is not authority for CYC area percentages or
completion. Preserve the raw five-parent observation and obtain a canonical CYC
mapping through the accepted client/Wiki cross-check. No resolution is asserted
here. Likewise the raw parent `field_6` distribution (20 zero, 8 one) is an
observation, not permission to treat zero as “discoverable”.

The existing decoder is reusable input infrastructure; do not author another
BMP/LZMA parser. Its `read_mask`/`Mask.contains` already describe pixel membership.
The source stager intentionally never decodes images, while the world-authoring
README states that its area/mask reader has no current caller. Neither fact
means decoded, admitted CYC content currently exists.

### Admission gate and owner handoff

CYC-CONTENT-1 must return all of the following as one reviewable source/report
and public-map-definition revision; concrete schema/encoding is owned by the
existing map/content format owners, not allocated by this document:

1. A stable, revisioned client `source_id` → `MapAreaId`/canonical Area crosswalk,
   canonical parent membership and the discoverable top-level/subarea set. Reuse
   existing identities where compatible; do not mint IDs from names, tile zones
   or encounter anchors. Cross-check the 20 discoverable areas against the pinned
   Wiki table; disagreement is CONFLICT and not admitted. Explicitly resolve the
   Tibiadrome multi-parent case and report every unresolved/maskless subarea.
2. World-scoped immutable tile-to-subarea membership derived from the pinned
   masks, with at most one subarea per tile, raw→project→native coordinate-frame
   provenance, map/content revisions and decoder/input hashes. Reuse the actual
   compiler conversion `project.rs:121`: project legacy z maps to native **-z**,
   not the client UI's level convention. Check real Wiki membership examples,
   World extent and actual served-map tiles; source-only zero overlap is one
   prerequisite, not semantic admission.
3. A revisioned POI pool for every admitted discoverable subarea: pinned TibiaMaps
   observation with coverage evidence, or the accepted deterministic reachable-
   tile fallback with an explicit deviation report. Validate uniqueness, same
   floor/membership/reachability and at most256 positions. Refuse insufficient
   eligible positions after excluding the actual current game window. The 1,270
   arbitrary marker records and generic city/hunting point coordinates are not
   replacement POIs. No admitted POI pool was found in the traced runtime/public
   map formats; opaque historical archives are not silently promoted.
4. Explicit bounds (areas≤64, subareas≤512, pool≤256), complete provenance and
   source/licensing disposition; no truncation, silent last-parent winner,
   unresolved flag promotion or default geometry. CYC-PARITY-1 must evidence the
   generated POI count (>7), found radius, speed values, donation-eligible set,
   donation Premium/minimum policy, draw ordering, factor combinations and 33%
   official confirmation. §8 is an activation gate; speed stays explicitly inert
   at0 until its values are qualified (§4.5). This packet sets none of them.
5. A two-revision compatibility report per §4.2a: tile/pool-only normalization
   preserves found positions/count and redraws obsolete unfound positions;
   structural changes to identities/membership/discoverability/eligible sets or
   reward thresholds require a product decision wherever durable state exists.
   Old discovery, grants, donation value and the current epoch's pinned tiles
   cannot be reduced/reinterpreted. A “no durable state” assertion must be read
   under the actual freeze/staging protocol, not inferred from a content file.

### Existing consumers and exact integration seams

| Owner / real source | Required implementation boundary |
|---|---|
| Content: `apps/game-server/src/content/project/v2.rs:141` | `Area` is a generic **candidate-only** declaration; its presence is not executable lowering. `content/project.json` still lists Area among next-population families. Add qualified CYC authoring/admission through the existing owner, not a fake admitted-area callback or permissive generic fields. |
| Bundle: `crates/world-bundle/src/{bundle,sector,spawn}.rs`, `tools/world-bundle-compiler/src/project.rs` | Actual v4 carries tile zones, draft_areas, spawn and NPC frames, but no CYC crosswalk/membership/pool frame. `draft_areas` must be empty in production; numeric tile zones are not MapAreaId. The fail-closed format rejects unknown fields, so a real format-owner extension/version plus compiler/reader/pin changes is a CP schema/integration seam. Do not add a parallel runtime sidecar authority. |
| Map/position: `apps/game-server/src/map/{boot,view}.rs`, `foundation/runtime_actor_carrier.rs` | Reuse pinned WorldBase/map revision, native `WindowLayout` and current-owner `borrow_movement_position().read(ExactActorRef)` provenance. Bind the accepted “current game window” semantics explicitly. Detect POIs against committed positions and exact exploration/pool revisions; request coordinates or cached actor snapshots are not fresh position/session authority. No new discovery-by-walking rule. |
| Discovery: `durability/character_revision_sequencer.rs`, `crates/simulation-determinism/src/lib.rs` | Add the actual CYC Character writer through the existing revision slot/sequencer, session-generation fence and expected revision; no second sequencer. Persist append-only discovered subareas and at most one active exploration row (pinned pool revision, generated positions, found mask, start revision). Start/shuffle request occurrences use the existing deterministic decision primitive with `cyclopedia_poi`; they replace/reset only the active progress. |
| Completion/grants | Commit the found bit before effect195/progress. Seventh find inserts discovery, removes active exploration and records completion in one Character transaction. Area completion records and consumes the real Account achievement grant in that transaction; missing catalogue key rolls everything back. Derive percentages/30–70–100% unlocks/full-area count/titles; do not store drifting copies. Charos outfit thresholds10/15/20 use the actual Account outfit owner/NPC lane; raids use RAID-1; speed uses its own contribution owner. |
| Donations: `durability/item_fee_burn.rs:63`, bank fee boundary | Existing closed FeeBurnCause currently exposes CharmUnassign, not AreaDonation. Add the accepted closed cause/audit amendment through its owner, then reuse coin-first/BANK-FEE payment (junior coins only), fixed occurrence/intent and native fee/bank locking. Check active eligibility/current improved epoch after pool lock; burn+bank debit+pool revision increment+fenced Character receipt remain one transaction, never an independent donation counter update. |
| Reset/content activation | Implement §4.2a World freeze row with shared locks for **every** CYC write/grant and exclusive staging/reset changes. Increasing staging identity, own-staging-only unstage/lift, RETIRING refusal/recovery and activation must share the actual World-reset owner. The inspected map boot explicitly says nothing durable names the bundle; it is not proof of an ACTIVATED reset epoch. Selection needs that independently current ACTIVATED epoch/digest, before any channel admission. |
| Selection/respawn: `ai/spawn.rs`, `runtime_actor_carrier.rs:1751`/`spawn_timing` | Lock all World pools in canonical area order; consume the target bundle/previous epoch, draw under existing deterministic occurrences and record selections/pool revisions/digest once per World+epoch. Keep losing pools; reset winner only; never redraw on replay. Existing channel-local PointTiming/RespawnSchedule/placement cells remain the owner: join real spawn cells to the epoch's admitted membership, apply accepted factor2 only to eligible spawn-home creatures, exclude bosses/specials/raids/summons; no new scheduler/shared creatures. |
| Client/wire: `apps/client/src/cyclopedia.rs`, protocol/session/transport owners | Existing Cyclopedia is Bestiary/Charms only; shared CyclopediaWireError names do not implement a map domain. CYC-WIRE adds the server-authoritative capability and typed views/intents/events with owner-assigned numbers, registry and handler together. Discovery query carries found count, not hidden coordinates. POI positions go only to the owning Character, same floor and in effect range; reconnect/resync must include already-in-range unfound POIs even without a boundary crossing. Older/no-capability clients receive no domain. |

The freeze and reset rules are mandatory transaction design, not a suggestion
for a read-only staging flag. All writes re-check active revision and current
fences/preconditions inside their transactions. World donation/selection state
is shared across Channels; creatures, effects and spawn scheduling are Channel
local; Character discovery remains the accepted per-Character progression scope.
No stored request, row, bundle digest or cached callback reconstructs current
lease/session/reset/staging authority.

### Concrete qualification required when inputs exist

- **Input/admission:** pinned raw/decoded digest failures; malformed framing/BMP;
  canonical crosswalk collision or changed source ID; wrong parent; unknown flag;
  missing mask; overlapping membership; floor-sign/extent mismatch; unresolved
  Wiki discoverability; duplicate/outside/unreachable POI; fallback deviation;
  64/65 areas,512/513 subareas,256/257 pools and generated-count/minus-window
  shortage. These tests must use actual admitted revisions/positions when present.
- **Discovery PostgreSQL:** deterministic start/shuffle retry and request-binding
  mismatch, one active row, concurrent replacement, stale Character/session/lease/
  actor/content/pool revision, wrong Character/World, committed-position radius
  boundary, duplicate bit, finds6→7, atomic achievement failure/rollback, reload
  across restart/reconnect/channel switch. No production test derives supposedly
  current facts solely from the exploration row or request under test.
- **Revision/freeze/reset:** normalize-only redraw preserves found progress;
  structural revision with any discovery/exploration/donation/epoch/grant state
  refuses without loss. Exercise stageA/unstage/stageB, restaging same revision
  with a new ID, stale/duplicate reset worker, write-vs-freeze, crashes before
  RETIRING/during reset/after ACTIVATED-before-selection, wrong active digest,
  rollback and own-staging-only clear. No channel admits before required recovery.
- **Economy/respawn:** coin-first/bank remainder/junior coins-only, insufficient
  funds, amount1/cap/cap+1, current improved area refusal, simultaneous donation
  and selection, threshold just below/at10,000,000, tied pools with bound RNG,
  no eligible area, replay retaining all pool values, World shared/Channel local,
  native spawn-cell membership and excluded creature kinds. No synthetic epoch
  boolean or fake admitted-area source qualifies these cases.
- **Wire/privacy:** requester/other Character, same/different floor, in/out range,
  reconnect already-in-range, found-count-only queries, no hidden pool/position
  leak, gated older clients and viewport regression through the existing owners.

### Evidence and limits

`/workspace/oteryn-prep/loop-map-1-evidence/input-qualification.json` binds263 actual
Git input objects (including all209 source mask assets) and exact acceptance SHA;
`qualify_inputs.py`/its real log plus the trace augmentation and raw multi-parent
report reproduce the read-only checks. Source masks were streamed and discarded;
no generated atlas, coordinates, POI pools, canonical membership or new IDs were
written. All counts above are source observations, not runtime admission.

At that historical checkpoint the sole candidate change was this handoff record. Governance and its
124 tests, owned-path/source binding and whole-diff whitespace checks pass.
No Rust/Cargo, gameplay/PG/runtime/E2E tests, paid resources, product code,
content/schema/registry mutation, commit, push or integration action was run.
Independent architect/format/persistence/economy/protocol review and all child
implementations remain required; this is not approval for production activation.


## D1133 owned source-mask checkpoint

Current status: NOT_FULLY_QUALIFIED. D1133 source authoring supersedes the historical
report-only lease for the owned content/cyclopedia candidate/provenance and
`docs/agents/evidence/LOOP-MAP-1-source-mask-decoding.md`. The candidate retains all209
pinned source-mask decodes as compact source-pixel runs, not admitted native
membership or compiled public-map output. Provenance binds exact source/reader/
crosswalk/extraction hashes; no generator, runtime IDs/flags, canonical parents,
POI pools, registry or pins are added. The old raw graph/ambiguity is preserved.

Independent source review PASS precedes this package. Exact copy/hash,209JSONL
records and whitespace are cheap source checks; native/compiler/runtime checks
remain NOT_RUN. Native frame/served-tile/reachability/overlap and canonical CYC/
donation/POI qualifications remain pending. D1217's7UNKNOWN1CONFLICT parity is
preserved; it does not block this unadmitted source checkpoint. No readiness,
merge, activation or completed CYC-CONTENT claim is made.

Lead independent source/package review PASS: all209 source identities/runs/
counts and null native qualifications checked; pinned reader semantics and
source-format/budget provenance reviewed. The evidence report uses the owned
docs/agents/evidence path. Full runtime/strict/package tests remain NOT_RUN;
source data do not qualify live membership or unblock OWNER parity values.

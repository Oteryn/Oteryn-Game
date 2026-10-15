# BED-REGEN-1 — bed vitals and food settlement (skeleton task record)

```yaml
task_id: BED-REGEN-1
title: "BED-REGEN-1: bed vitals and food settlement (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/bed-regen-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row BED-REGEN-1 (row 121); App. 7 house beds; §5 Wave 5"
class: "impl (persistence review)"
needs_migration: no
needs_registry: no
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/house_bed/regen.rs (new)"
  - "apps/game-server/src/house_bed/regen_tests.rs"
  - "docs/agents/tasks/BED-REGEN-1.md"
slot_only_paths:
  - "none expected (no migration, no registry)"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Settle vitals regeneration and food for a sleeping character on wake per BED0.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| BED-1 | NOT MERGED | **BLOCKING** |
| DUR-02 durable vitals (LOOP-VITALS-1) | PARTIAL: migration 0084_character_last_vitals on main, CHAR-VITALS-1 record still active, LOOP-VITALS-1 open (Wave 2) | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/house_bed/regen.rs (new)
- `apps/game-server/src/house_bed/regen_tests.rs`

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- none expected (no migration, no registry)
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- durable vitals writer (0084_character_last_vitals)
- bed settlement hook in BED-1
- docs/reference/regeneration-food-20261010
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- regen settlement + its tests with the BED-1 settlement hook

## Acceptance criteria

- [ ] regen computed from durable timestamps, capped by food and max vitals
- [ ] deterministic for a fixed clock
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server house_bed::regen
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- `docs/architecture/reviews/OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md`
- `docs/reference/regeneration-food-20261010`
- `docs/agents/tasks/active/OTV2-20261013-char-vitals-1.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.


## Allocated native integration packet (2026-10-15, card only)

Exact allocation: `bed-regen-1-allocation.json`, draft base
`34b3a9c82e07ae4971a2c5f2eb3a6767d08af887`, branch `skeleton/bed-regen-1`.
This round owns **only this card**, superseding the historical free-edit/registration
permission for this round. No Rust, migration, wiring, registry, content or dependency edit;
no merge/rebase, commit or publication by the worker. The original plan, builder CP rules and
all unchecked acceptance criteria above are preserved. This is a concrete REPORT_ONLY
integration packet, not a healing calculator, alternate writer or runtime qualification.

Protected read-only reference main is `bda15eba7a84f65fcb963a86e8b2bf18d69c800a`.
Effective ACCEPT-BATCH #2209 §1 accepts BED0 and OFFLINE0, including their reciprocal
amendments; older CANDIDATE headings on this draft do not override that acceptance.
Acceptance does not install children or waive BED-1 / durable-vitals prerequisites.
`PARITY_PENDING` on BED0's coefficients remains an evidence label: §7.2 already specifies
30 seconds per HP/mana point and 900 seconds per soul point. It is neither permission to
invent rates nor a reason to substitute online vocation rates.

### Actual native owners and prepared boundaries

- `durability/character_build.rs` / migration `0030_character_build_state.sql` own the
  existing build row, root-revision chain, before/after receipt guards and formula validation.
  The closed causes are Training, VocationChoice and Promotion; no-change Training is refused.
  No native Logout/OfflineSettlement marker or accepted OFFLINE columns are installed.
  Existing occurrence replay compares its binding; OFFLINE's marker-key replay-before-binding
  is a required extension, not existing behavior to assume.
- `durability/character_last_position.rs`, migration0084 and accepted CHAR-VITALS0 §§3–3.3
  already persist **HP and mana** in the cell's ordered runtime-state projection. This is not
  a DUR-02 receipt: no CharacterRevision/root/receipt, same session fence and increasing
  `(character_lease_generation, write_sequence)` as position, both values NULL or bounded,
  no pending-respawn or generic Instance write. NULL means start at current maxima. Native
  `gameplay_transport/last_position::admitted_vitals` and `actor_spell` restore only a newly
  created actor, cap to current maxima, and preserve a live reconnect. House coordinates
  do not justify dropping otherwise applicable same-World vitals.
- `spell/food_regeneration.rs` already implements online vocation-dependent fed ticks and
  current-Premium promotion benefits; `spell/actor_conditions.rs` owns food condition time,
  the1200-second cap and lifecycle clearing. Those are real runtime implementations, not
  durable food storage or the BED0 offline recipe. At both exact base and reference main,
  native durability/migrations have no accepted remaining-food column/receipt integration.
  CONDITIONS0 §6.3 requires lease-release receipt save, fresh restore and death reset, using
  the existing H-1 pattern. Eating another item, replaying ItemUse, or simulating online ticks
  is not offline food settlement.
- Soul is runtime-local (`PlayerSpellState::new` initializes it at max); migration0084 and
  `LastVitals` store no soul. Thus0084 proves durable HP/MP only, not the whole BED0 §7.2
  durable-vitals gate. CP/architect must bind the soul owner and atomic HP/MP projection
  composition explicitly before enabling the full food/vitals step. Do not silently treat
  full-at-login soul as a durable before-value or install a separate soul/vitals table here.
- `load_character_cast_facts` derives actual maxima from existing build/progression and the
  retained native vitals profile, bracketing the root revision. Reuse that owner (or its
  transaction-bound native equivalent), not a copied vocation table, client maxima, caller
  `current` booleans or guessed promotion/Premium rates. A consistent read is still not
  current session/house authority; NoVocation/unavailable is not a default class.
- Prepared BED-1 (local deliverable, source `ac3a40da...`, **not merged**) supplies raw
  `StoredSleeper`, Unix-millisecond timestamps, private proposed schema/lifecycle constraints,
  occupancy revision triggers, owner-column fallback and caller-transaction row read/lock.
  It provides no production sleep/free/settlement writer, installed grants or marker proof.
  Prepared LOOP-TRAIN-1 (source `076cef9c...`, **not merged**) supplies noncommitting
  marker/lease/pool/activation plans and native relevel previews. Its StaminaOwner,
  TrainingRates, PercentProjection and MarkerCommitRequired stages remain visible; arbitrary
  DTOs are no authority. Prepared LOOP-VITALS-1 supplies stamina arithmetic only, not durable
  food/soul/regen or a native settlement transaction. None is imported/copied by this task.

### Time, identity and exact settlement values

Native build receipt `committed_at` and prepared BED `slept_at`/`freed_at` are **Unix
milliseconds**. Native terminal lifecycle `decided_at` is seconds; monotonic runtime tick time
and LOOP-TRAIN's proposed integer `*_us` are different carriers. Never reinterpret these
integers. The OFFLINE owner must bind actual DB/FND epoch/precision/provenance and use checked
exact ms→us conversion when that DTO is used; take differences with checked/widened arithmetic
before flooring once to elapsed seconds. OFFLINE marker `as_of=committed_at` must be stamped
and guarded by the real DB trigger, not a runtime clock argument. Reference main release
clocks do not themselves prove lease acquisition/session creation or predecessor relation.

For a fresh admitted lease, after respawn resolution and before playability:

1. Resolve the immediate previous lease/session independently (never generation minus1).
   The latest build-carrying receipt must be that lease's actual logout marker. Only then
   `d=floor(max(0, lease_acquired_at - logout_committed_at))` in seconds, cap1814400 (21days).
   Otherwise crash/first/skipped/older marker means d0, no regen/training, selected skill
   cleared. A same-session/lease reconnect creates no new settlement.
2. Count a sleeper only when Character/World and previous lease match that same latest logout;
   its `slept_at` must bind the creating marker, not merely compare raw lease integers. For
   an eligible open row, `sleep_s=d`; for a freed row,
   `sleep_s=min(d, floor((freed_at-slept_at)/1000))`, with validated nonnegative ordered native
   millisecond times and21day cap. The first free time/cause wins. Delete stale, freed and
   eligible rows alike in this settlement, with their house occupancy revision advancement.
   A freed bed limits only BED vitals: OFFLINE training/stamina/pool still use d.
3. With an actually durable, source-qualified food before-value `0<=f<=1200` seconds and the
   native durable vitals gate resolved, compute `p=floor(min(f,sleep_s)/30)`, `consumed=30*p`,
   `f_after=f-consumed`; HP/MP each gain p, capped independently at current native maxima.
   Thus p<=40, consumed<=min(f,sleep_s), no underflow or negative food, and a remainder<30s
   is retained rather than charged. At either/both HP/MP maxima the accepted formula still
   consumes30*p; do not replace this with consumption based on effective credited HP/MP.
   Soul independently gains `floor(sleep_s/900)` capped at its sourced native maximum,
   including f0. Use checked bounded arithmetic; no guessed vocation multiplier, online
   tick phase or food item is an input. Maxima and durable before-values are captured under
   the same qualified transaction/owner snapshot, not arbitrary detached calculator inputs.
4. If the full durable-vitals/food owner gate is unavailable, keep BED0's explicit temporary
   path: **neither HP/MP nor soul part, no food consumption**. Sleeper deletion and the
   OFFLINE marker/training rules still compose as accepted. Do not ship a partial
   food-debit/volatile-heal path, or silently claim0084 completed the whole prerequisite.

### Ordered physical transaction and current-authority requirements

This is the required composition, not an implementation or a new public transaction API.
All participants must use one native physical transaction/commit proof and retained operation
identity; no nested independent `DurabilityRoot` commit, second food writer, fee-like followup
receipt or post-commit SQL patch is acceptable.

1. Enter the actual native semantic/recovery pass; hold the existing recovery fence and
   `db::lock_admission_relations` in its lexical relation order before domain locks/time.
   Serialize the marker key and return its first historical receipt before binding checks,
   fresh fences or recomputation. Recovery validation remains required even for replay.
   The existing generic build occurrence path must be extended, not falsely documented as
   already providing this marker contract.
2. For a genuinely fresh marker, retain native session→scope assignment/node incarnation→
   admission guards→Character root `FOR UPDATE` order (`assert_session_fence` /
   `assert_gameplay_fence`), then the native progression/build owner rows and pending-respawn
   check/initializer. Current helpers admit Channel only; House integration must come from
   accepted HOUSE-RUNTIME fences, not treating every Instance as a House or downgrading
   WorldId/ChannelId. Admission is committed first, settlement after respawn and before actor
   playability; later XP/build/stance/death fences use the marker's committed revision.
3. Before a sleeper row lock/mutation, determine its actual affected houses and take property
   locks where the owning operation requires them, then house occupancy revision rows in a
   single canonical World+house order, then sleeper row locks. Re-read identity/state after
   locks. This avoids settlement's sleeper→revision trigger order racing wake's
   revision→sleeper order. A preliminary raw lookup discovers keys, never grants authority;
   a changed house/row means retry/refusal. The final shared order with native property/
   disposition writers is CP-owned and must be demonstrated, including old/new houses on
   logout cleanup; this card does not invent an accepted cross-domain lock registry.
4. Load and validate the actual latest marker, predecessor/times, build/progression, durable
   food and applicable ordered last-position/vitals projection. Resolve native maxima at
   the same qualified revision; compute the complete OFFLINE and BED after-values once.
   No writes/credits when any required owner/rate/percent/stamina stage is unresolved.
   The explicitly disabled BED-vitals path above must still leave food and all three vitals
   untouched; it is not a success claim for full BED-REGEN.
5. In the same transaction, write the single OFFLINE build marker/root successor and its
   extended receipt/guards, durable food successor and required receipt composition,
   final HP/MP projection under its existing order, any accepted soul owner successor,
   sleeper deletion and house occupancy revision. No extra root advance merely for a
   projection. No manufactured generic "vitals receipt". Projection `Superseded` or
   `PendingObligation` cannot be silently ignored after charging food/deleting the row;
   resolve the actual owner's monotonic key or fail/retry the whole transaction. Updating
   only health/mana in place at an equal key would violate0080/0084 guards. A same-World
   cell/metadata snapshot must remain coherent with the vitals; no invented placement.
6. Commit once. Only proven commit or historical reconciliation exposes the finalized
   state/revision. Install the committed post-values into a newly created actor before it
   becomes playable, then publish through existing vitals deltas. A replay carries final
   values from history and must neither apply gains nor subtract food again. Preserve
   `training_save` unknown-outcome retention/sequencing, and refresh independently current
   session/guard after historical receipt recovery before any runtime installation.

### Sleep, free and logout integration beyond settlement

BED0 admission has two distinct authority moments. Sleep USE/choice requires Current Premium
(native fail-closed gate, no NotActivated exception), no logout/PZ lock, actual WorldUse reach/
cooldown and the current active reciprocal same-house head/foot resolver. Reserve60s and
revalidate the property/ACL and granting guild/membership revisions on skill choice and marker
insert. OWNER/SUBOWNER/GUEST with an actual entry grant may sleep; role names/caller flags and
stored coordinates alone are no grant. A new access revocation does **not** wake an existing
sleeper: on login the house owner revalidates placement/entrance, then settlement consumes
its row without inventing a fresh Premium/access gate to confiscate historical gains.

After actor removal and final native training/stamina/food/vitals capture, but before terminal
release, the logout marker transaction must insert the qualified sleeper and activation
atomically, delete any old Character sleeper, and advance both affected house revisions.
Take actual property `FOR SHARE` and granting guild/member locks; require accepted property
state/no content fence and current entry revisions. Unique conflict/fenced house means marker
without sleeper **and without activation**. Runtime reservation alone never creates a row.
`FreshAdmissionStore::release` currently owns the terminal lifecycle receipt and final-position
upsert; no bed/build-marker hook exists. Reference main additionally has
`release_logout_session` (LOGOUT-WIRE), absent on this older draft; integrate with its current
controller/session/terminal replay behavior rather than an obsolete release consumer.
Final projection/capture, marker and terminal ordering must be qualified as one coherent
native owner flow, not independent best-effort saves.

Owner-wake and bed-removed remain house-runtime-only; ownership/disposition writers free all
open rows with their accepted cause and database clock. Free does not heal/debit/train or
advance the sleeper's CharacterRevision; it only persists first `freed_at/cause` and the house
occupancy revision, keeping the row for admission. Repeated free is a no-op preserving both.
Prepared BED's generic owner-column trigger must not steal `disposition`: specific-cause free
runs **before** ownership UPDATE in the same transaction, or CP supplies an accepted native
cause seam. It does not fire for ACL-only changes. Settlement vs free must serialize on the
same property/revision/sleeper order and observe either committed first-free row or deletion;
no reinsertion/reopening, lost occupancy revision, retroactive shortening or duplicate award.

### Exact shared deltas and required qualification

`cross_domain_finding` / CP slot, with final architect review:

- Extend native0030/build writer/receipt decode/binding version, marker-key uniqueness,
  trigger-stamped clock/as_of, OFFLINE columns and no-op exemptions. Include death receipt
  before/after equal OFFLINE fields, all-NULL/all-non-NULL group, chain guards and
  `verify_character_integrity`; do not add independent training/bed revision truth.
- Bind CONDITIONS0 durable food ingress/save/restore/death reset and before/after receipt
  composition. Runtime food condition ticks, item use commit and forced-Serene H-1 are existing
  seams, not permission to insert a closed native cause or fabricate durable food facts.
- Bind CHAR-VITALS0's current projection owner/sequence and atomic transaction read/write seam
  with marker recovery. Resolve soul persistence and the BED durable-vitals prerequisite
  with its accepted owner; no new contract/rates or replacement durability model in this card.
- Install BED proposed schema/provenance guards/roles only with actual logout/settlement,
  property/ACL/head/Premium hooks and canonical locks; qualify Channel and admitted House
  paths, latest logout/predecessor and the new reference-main logout consumer together.

Required final native PostgreSQL17.6 and live owner tests (not executed by this docs packet):
final HP/MP/soul/f remain correct after commit/reload and crash between commit and actor
installation; precommit failure at each write rolls back marker/root/build/food/projection/
sleeper/revision together; ambiguous commit reconciles first receipt and applies no gains
again, including changed raw binding/stale live generation on replay. Cover0/29/30/31s food,
899/900s soul, maxima/one-below/current-maxima shrink, f0/1199/1200, cap21days/max+1,
nonnegative ordered ms timestamps, seconds-vs-ms regression, checked ms→us overflow and
acquisition-vs-later-settlement delay. Verify no eligible sleeper/crash d0/stale predecessor
and already-freed rows; training remains over d after early free. Race owner-wake,
owner-change, disposition, bed-removed and settlement; preserve exact first cause/time,
occupancy revision, no-reopen and unique-bed/Character rules. Race stale connection/lease/
scope/node/ACL/grant against insert, death/pending respawn, projection key supersession,
terminal release and same-lease reconnect. Demonstrate current grants/head normalization,
food-condition restore, no volatile partial charge, explicit disabled-path no-consumption,
and actor-visible committed final values rather than only detached arithmetic assertions.

Local checks for this round: governance validator and its tests, whitespace/scope and exact
native/prepared source hashes only. No Rust code, build, PostgreSQL or runtime test is added
or claimed. Evidence is retained in `/workspace/oteryn-prep/bed-regen-1-evidence`; all original
acceptance checkboxes and CP/architect production gates remain open.

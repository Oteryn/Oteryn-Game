# BED-REGEN-1 — native settlement integration and producer gates

```yaml
task_id: BED-REGEN-1
title: "BED-REGEN-1: native settlement integration and producer gates"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/bed-regen-1
base_sha: f010472bcb6794a2cbc56dfaf755bd122c10225e
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

## 2026-10-17 D1370 native producer handoff — preserved inert source

This **card-only** checkpoint extends #2148
`a19773fc99ad8dc5b2f18f636f7a7b4652111175`. The complete helper/caller source
proposal, actual pinned preimages, analysis and real-PG blocker are preserved
in the [reproducible appendix](#reproducible-bedoffline-source-checkpoint).
There is still no `regen.rs`, `regen_tests.rs`, active caller, new schema or
standalone calculator in this PR. Regeneration remains **NOT_IMPLEMENTED** and
formatting/compile/native PG of the recorded proposal remain **NOT_RUN**.
All previous card sections and their historical source refs are retained.

### Concrete actual-owner proposal and boundary cases

The recorded helper belongs to #2147's BED owner and borrows the actual OFFLINE
physical transaction. It pins the real property/revision/sleeper rows, binds
the actual immediate predecessor and latest Logout, derives bounded eligible
sleep from that plan, deletes only the pinned character row and requires one
real revision-hook advance. The historical caller hunk targets #2174's then
actual `training/offline.rs::apply_marker` after plan/fence locks and before
the marker write. It is not applied to that owner's branch by this card.

The native Logout producer retains a microsecond anchor and a millisecond
receipt projection. Correct BED binding is
`slept_at == committed_at_us / 1000`; a multiplication equality would refuse
a legitimate nonzero submillisecond remainder. The consumer must preserve
the actual food millisecond remainder and use its real cap and durable maxima.
An arbitrary client clock or detached arithmetic does not qualify this seam.

The appendix specifies real-native cases using the true Logout/settlement
owner: the submillisecond boundary, rollback after a later marker-owner error,
missing revision hook, stale predecessor/latest Logout and retained-command
replay without touching a newly present sleeper. These cases remain **NOT_RUN**.
They require the real marker schema/guard/fixture and actual native clock;
replacement DDL or fabricated receipt authority would not be equivalent.

### Producer allocation remains unresolved in one exact point

[D1370](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234571417)
accepted the concrete missing producer and reserved migration0118 to #2174,
including real receipt/build columns, chain guards, canonical PG fixture and
independent FND clock/predecessor.
[D1381](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6235225446)
then described migration0120 and moved the native marker writer into
`durability/character_build`. Root's specific
[CP question](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6237062703)
asks whether 0120 replaces 0118 or is separate while retaining the full real
fixture/clock scope. D1403 answered the ordinary House 0121 producer but did
not resolve this 0118/0120 point. **This card chooses neither migration number.**

Read-only PR checks after 16:48 still found #2174 at
`b5576758e5309dffdd8993bd9956e3c798e985ad`, #2196 at
`6fbd70254f323d29a9bad15db74ea0adb9d55a92` and #2167 at
`061c16a6f4ac5f337c56f1189421fa3ecb0cd127`, all draft and unmerged. These
are source bindings for this checkpoint, not substitutes for the required
future actual merge and current producer readback.

The eventual #2147 hook must rebind the actual native writer after #2174 and
#2196 merge. #2148 must then use the complete same-transaction durable
food/HP/mana/soul before-values and mutation participant. Actual #2167 full
active-bundle placement/currentness, canonical property/BED schema and genuine
logout/sleeper provenance are separate prerequisites. A separate Root save
transaction is not this atomic settlement.

**The historical proposal's positive-time participant-required refusal is
inert review material only. It must never be published as a blocking runtime
call before the real food/vitals/soul consumer exists.** No partial award or
food debit is implemented. The old investigation notes retain their pinned
candidate context; this leading disposition governs the current handoff.

Native work also waits for a House window after the MAIL/HUNT order in
[D1399b](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6236882742),
using the canonical D1397/D1397a lock, D1342 and D1329. This card starts no job
and expands no owned path. The PR remains draft / **NOT_FULLY_QUALIFIED**.

## 2026-10-17 D1355 producer disposition — NOT_IMPLEMENTED

Predecessor after CP's ordinary main merge:
`361181051c9b7aba8e8764debc13914e0ba2c6e9`. The owned authoring release is
[D1345](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233992418).
This PR still contains no `regen.rs` or `regen_tests.rs`; this update records the
concrete native integration boundary and CP's producer allocation. It does not
claim an implemented or tested regeneration feature.

Fresh producer readback:

| Producer | Observed head | Required result |
|---|---|---|
| BED #2147 | `b1375be27cf817df99c2e8d7781b6df88523f79b` | Same-TX eligible sleeper consumption bound to the previous lease and actual latest Logout marker; raw stored rows alone are insufficient. |
| OFFLINE #2174 | `b5576758e5309dffdd8993bd9956e3c798e985ad` | The actual `native_marker_writer::apply_marker` transaction, retained build/fence authority, locked clocks, replay and marker owner. |
| Durable vitals #2196 | `6fbd70254f323d29a9bad15db74ea0adb9d55a92` | Actual food, HP/mana and soul before-values/participant in that same physical transaction. The existing pure soul policy does not provide durable soul. |
| Active House placement #2167 | `061c16a6f4ac5f337c56f1189421fa3ecb0cd127` | Full active-bundle House producer and its currentness binding for #2147's real bed resolver. |

[D1355](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234008098)
assigns durable soul to #2196 and explicitly amends #2167 to provide the full
active-bundle producer; scoped-overlay qualification alone is insufficient.
It grants #2147 the BED caller hook inside #2174's `apply_marker` after the
existing plan and fence locks and before `write_offline_marker`, only after
#2174 and #2196 merge. #2148 consumes this actual BED settlement path after
#2147 and #2196; it adds no parallel transaction, authority callback or calculator.

At the native call, BED0's accepted arithmetic uses durable timestamps and state:
eligible sleep is capped by the owner's offline duration and any first freed
timestamp; food-backed HP/mana points are `floor(min(food_s, sleep_s) / 30)`, food
is debited by `30 * points` even at capped vitals, and soul gains
`floor(sleep_s / 900)` up to its actual maximum. BED's Unix milliseconds and
OFFLINE's microseconds require checked conversion and one defined flooring step.
The latest Logout and immediate predecessor are actual owner facts, never lease
generation minus one. Until the complete durable food/vitals/soul gate exists,
BED0 permits neither a partial vitals/soul award nor a food debit.

Native implementation, native compile/test discovery, PostgreSQL execution and
restart qualification remain NOT_RUN/NOT_IMPLEMENTED. The acceptance checklist
stays open. All older allocation and readback sections below are historical and
must not be read as the current producer heads or a continuing authoring hold.

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

## Reproducible BED/OFFLINE source checkpoint

This complete UTF-8 source checkpoint is retained as data inside the owned task card. Its exact length is 32286 bytes and SHA-256 is `217ffb4b5b7c66cfc6d1fbae3415d7c7e2e49a1570b7c08560447b480d08ec5a`. The source inventory, complete patch and analysis/review can be recovered from it; the recovery instructions do not authorize applying a held runtime change.

<!-- card-artifact:bed2148-offline-source-checkpoint.md -->
`````markdown
# BED-REGEN-1 — concrete native handoff and inert source checkpoint

State: **regeneration NOT_IMPLEMENTED; helper/caller proposal unmounted, formatting/compile/PG NOT_RUN**. Published #2148 predecessor: `a19773fc99ad8dc5b2f18f636f7a7b4652111175`. This evidence preserves a concrete cross-owner source proposal and real-native test sequences. It does not add a regeneration calculator, a fixture-only marker schema, a receipt clock or a substitute durable participant.

## Exact scope of the recorded proposal

The helper delta belongs to #2147's BED owner. It borrows the actual OFFLINE owner's physical PostgreSQL transaction, reads and locks the real property/revision/sleeper rows, binds the real immediate predecessor and latest Logout, observes bounded eligible sleep, deletes only that pinned sleeper and verifies exactly one revision-hook advance. It creates no transaction or current-authority witness. The held caller delta was prepared against #2174 `b5576758e5309dffdd8993bd9956e3c798e985ad`, in its then-current `training/offline.rs::apply_marker`, after the plan/fence locks and before the marker write.

For positive eligible BED time the historical proposal has a typed participant-required refusal. **That branch is preserved only as an inert review aid and must never be published as a blocking runtime call.** #2148 still needs the actual #2196 food/vitals/soul before-values and mutation participant in the same transaction. No partial award or food debit is implemented. The embedded two-file patch has different pinned source refs; it is not an applicable production branch update.

## Verified timestamp boundary and concrete PG blocker

The actual pinned OFFLINE producer retains the Logout anchor in microseconds while the stored BED and receipt projection use milliseconds. Its legitimate binding is `slept_at == committed_at_us / 1000`; comparing a multiplied millisecond value to the exact microsecond anchor would reject a nonzero submillisecond remainder. BED's freed interval remains checked in its native units and bounded by the real OFFLINE plan. Actual #2196 food milliseconds must retain their remainder and use its real cap; a detached seconds-only stand-in cannot qualify the consumer.

The source investigation found no real marker migration/guard/PG fixture or independently stamped FND lease-clock/predecessor implementation in the pinned #2174 candidate. The complete concrete blocker, exact inspected refs and real-native cases are preserved below. The boundary case must commit an actual Logout with a known nonzero native microsecond remainder, use the actual successor lease and invoke the real settlement owner. The rollback case must exercise a real later owner failure against the actual receipt table and prove that sleeper/revision/build/marker state rolls back together. Neither case has been executed. An invented clock, schema or authority would not satisfy it.

## Current allocation and held follow-up

[D1370](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234571417) accepted this blocker and assigned the actual migration, native fixture, guards and FND clock/predecessor producer to #2174. [D1381](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6235225446) subsequently assigned the native marker writer to `durability/character_build` and described a new 0120 migration, while the earlier D1370 reservation had named 0118. Root's [precise request](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6237062703) asks CP to reconcile these two numbers and confirm the complete producer/fixture scope. D1403 confirmed the separate ordinary-disposition 0121 producer without resolving 0118 versus 0120. This checkpoint deliberately makes no migration-number decision and supersedes the earlier held card proposal that would choose 0118; the current owned-card disposition explicitly keeps the number unresolved.

Rebind the recorded hook to the actual native writer only after #2174 and #2196 merge and the producer interfaces are available. The real BED schema/logout source provenance, full active-bundle/currentness producer #2167 and actual food/vitals/soul participant remain separate gates. Then author the listed PG boundary, rollback, trigger-failure and replay cases in the real owner harness and run them through D1329's shared-PG wrapper after `cargo fetch --locked`. Merely recovering this source checkpoint does not authorize any shared caller activation.

The embedded investigation notes intentionally retain their original pinned candidate wording. They are historical source evidence, not assertions that those refs remain current after later CP work. The old proposal JSON retained an earlier analysis-note hash; this checkpoint captures the current note bytes under their actual `5adb87f4...` hash without altering that old manifest or the unchanged source patch. The task stays draft with no native implementation or test PASS claim.

## Recovery and integrity

This is inert source evidence preserved verbatim in the existing owned task card. Only that card is proposed for the draft; the embedded Rust changes and dependency sources are not applied or registered. No new repository evidence path or owned_paths expansion is used. Runtime permission, dependency order and qualification gates still apply.

Recover each artifact from the matching section below. Preserve its exact UTF-8 bytes and verify the listed byte length and SHA-256. Reconstruct an isolated review tree from each file's pinned base ref and Git blob in `source-inventory.json`, then check and apply the patch there. Never apply a historical patch to an unspecified current checkout. The checkpoint preparation independently applied the recorded patch to those exact preimages and compared every resulting file byte for byte with the frozen source. That integrity check is not Rust compilation or PostgreSQL qualification.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| `source-inventory.json` | 1043 | `73305eaa0161bd728a93159bba2149b597a5c67ca9d18a886637315fc6f75407` |
| `bed-offline-inert.patch` | 10385 | `87fb666a0334cae4c0101a1ecfe2a2b2aab14c1cbb3ca5991194509f87f8e30f` |
| `original-source-analysis.md` | 7035 | `5adb87f4a89fb03a929b8e1944b5cf89e6df6a18ac125344695b5031dbd76c0e` |
| `real-pg-blocker-and-cases.md` | 6998 | `5191cafbd3f4a30a02a8e4bd7fc9fc880032334cdb06b5d0130218d0bc2f8eff` |

## Embedded artifacts

### source-inventory.json

<!-- checkpoint-artifact:source-inventory.json -->
````json
{
  "pr": 2148,
  "state": "INERT_REVIEW_ONLY_NOT_A_RUNTIME_PATCH",
  "files": [
    {
      "path": "apps/game-server/src/training/offline.rs",
      "base_ref": "b5576758e5309dffdd8993bd9956e3c798e985ad",
      "base_git_blob": "8098f6a6830b6b02a2cf8a125cc05919d6c59897",
      "base_sha256": "2fd70b9357110d7e1f141ad9d5ad742eca439a4d2ebbf15b636b9cf1b8b855b9",
      "result_sha256": "2b8a32f8323b3d488a0a539ad569f396dc6e4a9e7dc361eb110fcca9847c1964",
      "result_bytes": 53433
    },
    {
      "path": "apps/game-server/src/durability/bed_sleeper.rs",
      "base_ref": "87674e53cd4be954b19dfb049a5d41013182fc03",
      "base_git_blob": "d15c64b976f1ce9250eb30d8601546b266bef71d",
      "base_sha256": "7a4d88ce68e80e7ce7638bd329050f006b8dd16d2c77c43cda2dabf9d5f4d39f",
      "result_sha256": "abaea311bcbfacfa4e47aec1a6392516d351214bb0cf3ec12f9d4531caaa8890",
      "result_bytes": 16284
    }
  ],
  "source_patch_owner": 2147,
  "held_caller_owner": 2174,
  "actual_regeneration_consumer": "NOT_IMPLEMENTED",
  "native": "NOT_RUN"
}
````

### bed-offline-inert.patch

<!-- checkpoint-artifact:bed-offline-inert.patch -->
````diff
--- a/apps/game-server/src/training/offline.rs
+++ b/apps/game-server/src/training/offline.rs
@@ -903,6 +903,11 @@
     pub(crate) enum OfflineMarkerError {
         Progression(CharacterProgressionError),
         Backend(DurabilityError),
+        Bed(crate::durability::bed_sleeper::SleeperSettlementError),
+        /// D1355 requires the actual #2196 food/vitals/soul participant before
+        /// positive eligible BED time can commit. The owning TX rolls back,
+        /// preserving the sleeper and pending marker for a qualified retry.
+        BedVitalsParticipantRequired,
         InvalidTimeOrClockBinding,
         InvalidState,
         /// No guessed top-vocation rate or percent rule is substituted. This refusal
@@ -1149,6 +1154,25 @@
                 let SettlementDisposition::Plan(plan) = planned else {
                     return Err(OfflineMarkerError::InvalidState);
                 };
+                // INERT D1355(c) proposal: mount only after #2174/#2196 and the
+                // ordered BED schema/property owners. This is the actual TX,
+                // not a pool-based callback or a reconstructed clock/authority.
+                let bed_sleep_s =
+                    crate::durability::bed_sleeper::consume_sleeper_for_offline_marker(
+                        tx,
+                        &fence,
+                        &clocks_read,
+                        latest,
+                        &plan,
+                    )
+                    .await
+                    .map_err(OfflineMarkerError::Bed)?;
+                if bed_sleep_s.is_some_and(|seconds| seconds > 0) {
+                    // No guessed food, soul, maxima or native participant is
+                    // supplied. Replace this refusal with the actual #2148/
+                    // #2196 same-TX consumer before production activation.
+                    return Err(OfflineMarkerError::BedVitalsParticipantRequired);
+                }
                 let after_build =
                     apply_offline_training(&plan.training, formula, training_facts, &locked.build)?;
                 let stamina = SettlementStaminaParticipant::prepare(
--- a/apps/game-server/src/durability/bed_sleeper.rs
+++ b/apps/game-server/src/durability/bed_sleeper.rs
@@ -1,8 +1,11 @@
-//! BED0 §5.2 private schema/read preparation, not a standalone sleep/wake writer.
+//! INERT D1355 source proposal: BED0 private storage and caller-TX settlement.
 //! No grants, production migration or runtime registration. Insert must eventually
 //! share the actual logout-marker transaction; delete must share settlement (or
-//! the next logout insert). These hooks and current property/grant authority are
-//! not provided by this module. A row lock is not authority to wake its character.
+//! the next logout insert). The proposed delete participant is called only after
+//! OFFLINE's actual marker/fence/plan checks; positive regeneration still refuses
+//! until its actual same-TX food/vitals/soul participant is composed. Sleep/logout
+//! provenance and current property/grant authority remain external prerequisites.
+//! A row lock is not authority to wake its character.
 //! The private owner-change fallback only follows the native ownership column.
 //! CP must free disposition/specific-cause rows BEFORE that ownership UPDATE;
 //! Times use native build committed_at Unix milliseconds.
@@ -18,6 +21,11 @@
 use sqlx::postgres::{PgConnection, PgRow};
 use sqlx::{Postgres, Row, Transaction};
 use std::num::NonZeroU64;
+
+use crate::durability::character_progression::CurrentCharacterGameplayFence;
+use crate::training::offline::{
+    LatestBuildReceipt, MarkerKind, OfflineLeaseClockRead, SettlementPlan,
+};
 
 /// Delegated private design, qualified only in isolated migration-owner fixtures.
 /// No independently executable insert/free/delete API is supplied. The guards
@@ -154,6 +162,159 @@
     .transpose()
 }
 
+/// Errors abort the actual OFFLINE marker transaction. They must never be
+/// converted to an absent sleeper or an acknowledged settlement.
+#[derive(Debug)]
+pub(crate) enum SleeperSettlementError {
+    Read(SleeperReadError),
+    Storage(sqlx::Error),
+    InvalidBinding,
+    InvalidStoredState,
+    MissingProperty,
+    SourcesChanged,
+    RevisionExhausted,
+    MissingRevisionHook,
+}
+
+impl From<sqlx::Error> for SleeperSettlementError {
+    fn from(error: sqlx::Error) -> Self {
+        Self::Storage(error)
+    }
+}
+
+impl From<SleeperReadError> for SleeperSettlementError {
+    fn from(error: SleeperReadError) -> Self {
+        Self::Read(error)
+    }
+}
+
+/// BED0 §7.1 participant of the actual `apply_marker` transaction. The sole
+/// caller already holds the marker/recovery/current-gameplay/Character locks,
+/// has excluded replay, reconnect and pending respawn, and has just produced
+/// `plan` from the same locked native clock/latest-receipt reads supplied here.
+/// These inputs remain observations, never independent authority grants.
+///
+/// Locks property -> occupancy revision -> sleeper, then deletes only this
+/// character's row through the real per-row revision trigger. An error must
+/// propagate so the caller rolls back this physical transaction. The returned
+/// seconds are transaction-local input for BED-REGEN, not a committed receipt,
+/// food/vitals award, or permission to start another transaction.
+pub(crate) async fn consume_sleeper_for_offline_marker(
+    tx: &mut Transaction<'_, Postgres>,
+    fence: &CurrentCharacterGameplayFence,
+    clocks: &OfflineLeaseClockRead,
+    latest: LatestBuildReceipt,
+    plan: &SettlementPlan,
+) -> Result<Option<u64>, SleeperSettlementError> {
+    if plan.key.kind != MarkerKind::Settlement
+        || plan.key.character != fence.character_id
+        || plan.key.lease != fence.character_lease_generation
+        || clocks.character != fence.character_id
+        || clocks.session != fence.game_session_id
+        || clocks.world != fence.runtime_scope.world_id()
+        || clocks.lease_generation != fence.character_lease_generation
+    {
+        return Err(SleeperSettlementError::InvalidBinding);
+    }
+    // Discover the immutable property identity before taking its ordered lock.
+    // The current Character root lock serializes this character's marker writers.
+    let Some(observed) = read_sleeper(&mut **tx, fence.character_id).await? else {
+        return Ok(None);
+    };
+    let house = &observed.bed.house;
+    let property: Option<i32> = sqlx::query_scalar(
+        "SELECT 1 FROM game_house_properties \
+         WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR SHARE",
+    )
+    .bind(house.world_id().as_bytes().as_slice())
+    .bind(house.house_key())
+    .fetch_optional(&mut **tx)
+    .await?;
+    if property.is_none() {
+        return Err(SleeperSettlementError::MissingProperty);
+    }
+    let revision: Option<String> = sqlx::query_scalar(
+        "SELECT revision::text FROM game_house_bed_revisions \
+         WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR UPDATE",
+    )
+    .bind(house.world_id().as_bytes().as_slice())
+    .bind(house.house_key())
+    .fetch_optional(&mut **tx)
+    .await?;
+    let revision = revision
+        .and_then(|value| value.parse::<u64>().ok())
+        .and_then(NonZeroU64::new)
+        .ok_or(SleeperSettlementError::InvalidStoredState)?;
+    let row = read_sleeper_for_update(tx, fence.character_id)
+        .await?
+        .ok_or(SleeperSettlementError::SourcesChanged)?;
+    if row.bed != observed.bed
+        || row.lease_generation != observed.lease_generation
+        || row.slept_at != observed.slept_at
+        || row.occupancy_revision != revision
+    {
+        return Err(SleeperSettlementError::SourcesChanged);
+    }
+    let eligible = match (clocks.times.predecessor, latest) {
+        (Some(previous), LatestBuildReceipt::Logout(marker)) => {
+            if marker.key.kind != MarkerKind::Logout
+                || marker.key.character != fence.character_id
+                || marker.as_of_us < 0
+                || marker.as_of_us != marker.committed_at_us
+            {
+                return Err(SleeperSettlementError::InvalidBinding);
+            }
+            row.lease_generation.get() == previous.generation
+                && marker.key.lease == previous.generation
+                && marker.session == previous.session
+                && marker.scope.world_id() == house.world_id()
+                && house.world_id() == fence.runtime_scope.world_id()
+                // #2174 load_offline_marker validates this exact projection
+                // against the real build receipt's committed_at millisecond field.
+                && row.slept_at == marker.committed_at_us / 1000
+        }
+        _ => false,
+    };
+    let sleep_s = if eligible {
+        match row.freed {
+            Some(freed) => {
+                let elapsed_ms = i128::from(freed.at) - i128::from(row.slept_at);
+                let elapsed_s = u64::try_from(elapsed_ms / 1000)
+                    .map_err(|_| SleeperSettlementError::InvalidStoredState)?;
+                plan.offline_s.min(elapsed_s)
+            }
+            None => plan.offline_s,
+        }
+    } else {
+        0
+    };
+    let after = revision
+        .get()
+        .checked_add(1)
+        .ok_or(SleeperSettlementError::RevisionExhausted)?;
+    let deleted = sqlx::query(
+        "DELETE FROM game_house_bed_sleepers WHERE character_id=encode($1,'hex')::uuid",
+    )
+    .bind(fence.character_id.as_bytes().as_slice())
+    .execute(&mut **tx)
+    .await?;
+    if deleted.rows_affected() != 1 {
+        return Err(SleeperSettlementError::SourcesChanged);
+    }
+    let advanced: String = sqlx::query_scalar(
+        "SELECT revision::text FROM game_house_bed_revisions \
+         WHERE world_id=encode($1,'hex')::uuid AND house_key=$2",
+    )
+    .bind(house.world_id().as_bytes().as_slice())
+    .bind(house.house_key())
+    .fetch_one(&mut **tx)
+    .await?;
+    if advanced != after.to_string() {
+        return Err(SleeperSettlementError::MissingRevisionHook);
+    }
+    Ok(Some(sleep_s))
+}
+
 fn decode(row: &PgRow, expected: CharacterId) -> Result<StoredSleeper, SleeperReadError> {
     let invalid = || SleeperReadError::InvalidStoredState;
     let world =
````

### original-source-analysis.md

<!-- checkpoint-artifact:original-source-analysis.md -->
````markdown
# Inert BED / OFFLINE source proposal

Status: **not applied, not published, not mounted and not validated**. This source is separate from the published #2147 cleanup packet. It is a concrete proposal for D1355's held caller slot; the prerequisite and final regeneration gaps below prevent activation.

## Exact sources and delta

| Source | Baseline | Proposed change |
|---|---|---|
| `durability/bed_sleeper.rs` | #2147 `87674e53cd4be954b19dfb049a5d41013182fc03`, blob `d15c64b976f1ce9250eb30d8601546b266bef71d` | Caller-TX sleeper read, ordered locks, marker binding, eligible sleep observation, deletion and exact revision-hook verification. Existing schema is unchanged. |
| `training/offline.rs` | #2174 `b5576758e5309dffdd8993bd9956e3c798e985ad`, blob `8098f6a6830b6b02a2cf8a125cc05919d6c59897` | One actual fresh-Settlement call after `SettlementPlan` and the existing fence/root/clock checks, before `write_offline_marker`; explicit typed BED errors. |

The unified patch and full source JSON bind both inputs and outputs by hash. They are source-composition inputs, not a patch for an unspecified checkout. Neither the published cleanup source nor another agent's worktree was modified.

## Actual transaction behavior in the proposal

OFFLINE still owns the semantic transaction, recovery/current gameplay fence, root lock, marker occurrence, clocks, training, stamina and marker commit. Existing marker replay, reconnect and pending-respawn returns all precede the BED call. The helper starts no transaction and cannot acknowledge a committed marker.

The helper discovers the character's stored House identity, takes the canonical `game_house_properties` row `FOR SHARE`, its BED occupancy revision `FOR UPDATE`, then the actual sleeper row `FOR UPDATE`. The immutable binding must survive the preliminary read; a first free that won before these locks remains visible. Deletion needs no current owner/guest grant: BED0 requires a character's next settlement to consume its row even after access revocation.

Eligible time requires the actual immediate predecessor and latest Logout relation supplied by OFFLINE, the same World, the row's exact lease and the marker's real committed timestamp. The code uses the existing `SettlementPlan.offline_s`; it does not recalculate OFFLINE or training duration. First-freed time can shorten only BED time. No food, HP, mana, soul, vocation rates or maximum values are invented.

The helper deletes only the current character's pinned row and checks that the actual `game_house_bed_revision` trigger advanced the pinned house revision by exactly one. Missing hooks, changed bindings, overflow or SQL errors propagate to OFFLINE and roll back its entire transaction.

**The durable regeneration consumer is still absent.** For positive eligible BED time, the proposed caller returns `BedVitalsParticipantRequired`. The outer native owner then rolls back the deletion and any earlier D88 initialization, preserving the sleeper and missing marker for a qualified retry. An absent, stale or zero-time row cannot earn any BED award; its ordinary cleanup may continue in the same owner transaction. This explicit refusal must be replaced by #2148 calling the actual #2196 same-TX participant before production activation. It is not an implemented food/vitals settlement, a release-ready feature or permission to bypass that gate.

## Timestamp detail verified in the real producer

#2174's `character_build::load_offline_marker` reads both `marker_as_of_us` and the existing build receipt's millisecond `committed_at`. It validates `marker_as_of_us / 1000 == committed_at`; `latest_offline_build_receipt` then exports the exact microsecond anchor as `committed_at_us`.

Consequently the correct BED row binding is `slept_at == logout.committed_at_us / 1000`. Requiring `slept_at * 1000 == committed_at_us` would reject legitimate markers with a submillisecond remainder. The freed interval remains the difference between BED's two millisecond columns, widened before division and floored once to seconds. The result is capped by the actual OFFLINE plan duration.

## Unresolved owners and held scope

- D1355(c): the `apply_marker` shared hunk is held until #2174 and #2196 merge. This proposal adds no module mount or dispatch.
- #2137's actual canonical property schema and #2147's installed BED schema/grants are prerequisites. The current private schema fixture is not production installation.
- D1355(a): #2167 at `061c16a6f4ac5f337c56f1189421fa3ecb0cd127` must supply the full active-bundle House producer and currentness binding. The actual sleep/logout writer must establish the source provenance assumed for a legitimate sleeper.
- D1355(b): #2196 at `6fbd70254f323d29a9bad15db74ea0adb9d55a92` must supply actual durable food, HP/mana and soul before-values in the same physical transaction. #2148 must consume those real values and the transaction-local BED time; it must replace the typed refusal before activation.
- Actual FND clock/predecessor source composition and native House scope support remain #2174's prerequisites. This patch neither supplies nor mocks them.

## Required native qualification before any active composition

All checks below are **NOT_RUN**. They must call the actual `commit_character_offline_marker` owner using the real prerequisite sources and D1329's shared PG wrapper after `cargo fetch --locked`.

1. No sleeper: ordinary marker behavior and training/stamina remain unchanged.
2. Stale or mismatched predecessor/latest Logout: no BED award, exactly one deletion and revision increment with the marker commit.
3. Eligible positive BED time with the actual #2196/#2148 participant: one atomic food/vitals/soul/marker result. The proposal's temporary typed refusal must never become a published runtime substitute for that participant.
4. Fractional-millisecond Logout anchor: binding accepts the producer's verified millisecond projection without inventing a new timestamp.
5. First free, zero elapsed time and the OFFLINE cap: the observed BED duration follows accepted BED0 while OFFLINE training duration remains unchanged.
6. Disabled per-row revision trigger and exhausted revision: typed refusal, no committed deletion or marker footprint.
7. A real later training/marker failure, or an owning-harness error injection on the actual receipt INSERT: the prior BED mutation and any initialization roll back in the same physical transaction. The fault is confined to the test's own database and needs no runtime placeholder or replacement receipt schema.
8. Competing owner wake/disposition versus settlement: the actual property/revision/row locks preserve first-free behavior and one transition.
9. Marker replay: return the original committed marker before any BED read/write; never consume a newly present row on a historical retry.

The final #2196/#2148 consumer additionally needs native food-debit/vitals/soul atomicity, caps, restart and replay tests. A standalone calculation over caller numbers cannot qualify those requirements.
````

### real-pg-blocker-and-cases.md

<!-- checkpoint-artifact:real-pg-blocker-and-cases.md -->
````markdown
# BED / OFFLINE PostgreSQL qualification blocker

The new helper cannot yet be qualified with the actual native marker writer and its real receipt schema. No substitute DDL, clock source or receipt fixture has been added. The helper proposal and the sequences below remain **NOT_RUN**.

## Verified source evidence

- Current #2174: `b5576758e5309dffdd8993bd9956e3c798e985ad`. Its six changed files are `character_build.rs`, `lib.rs`, `training/mod.rs`, `training/offline.rs`, `training/offline_tests.rs` and the task card. It contains no migration or native PG fixture file.
- Main `f010472bcb6794a2cbc56dfaf755bd122c10225e` and that #2174 head expose the same 81 migration entries and blob identities. No `0104` or OFFLINE migration is present. The complete readback is retained in `migration-catalog-readback.json`.
- The existing `0030_character_build_state.sql`, blob `666605b810b0185a77661c1823a35bcf84f94c4a`, declares the receipt cause set as `training`, `vocation_choice`, `promotion`. Its schema has no OFFLINE marker identity/timestamp or stamina/pool/skill extension.
- The actual #2174 `load_offline_marker` SQL requires `marker_lease_generation`, `marker_game_session_id`, `marker_world_id`, `marker_channel_id`, `marker_as_of_us` and before/after OFFLINE columns. `write_offline_marker` also writes `logout` or settlement causes and evolves the existing root/progression/build chain. Adding a stand-in receipt table or a few fixture-only columns would not supply that native chain guard.
- Current `training/offline_tests.rs` describes itself as proposed-data transitions and native formula previews; its markers and lease clocks are constructed as test data. It does not invoke the real PostgreSQL marker transaction. The task card's historical private generation-six SQL-clock/schema cases are explicitly noncanonical; later increments say that private migration0104 and earlier PG proof are not recovered, installed or reused.
- `OfflineLeaseClockSource` is an actual sealed native port, but this current source does not provide an independently stamped FND acquisition/creation/predecessor implementation. Constructed `ProposedLeaseTimes` or fixed fixture clocks would not establish the requested positive elapsed marker provenance.

The source is accessible at [#2174](https://github.com/Oteryn/Oteryn-Game/pull/2174), [its native marker owner](https://github.com/Oteryn/Oteryn-Game/blob/b5576758e5309dffdd8993bd9956e3c798e985ad/apps/game-server/src/durability/character_build.rs), [its current tests](https://github.com/Oteryn/Oteryn-Game/blob/b5576758e5309dffdd8993bd9956e3c798e985ad/apps/game-server/src/training/offline_tests.rs), [its task card](https://github.com/Oteryn/Oteryn-Game/blob/b5576758e5309dffdd8993bd9956e3c798e985ad/docs/agents/tasks/LOOP-TRAIN-1.md) and [the existing receipt schema](https://github.com/Oteryn/Oteryn-Game/blob/b5576758e5309dffdd8993bd9956e3c798e985ad/apps/game-server/migrations/0030_character_build_state.sql).

## Concrete real-native cases to author after the owner lands

These cases must use the canonical owner harness and real `commit_character_offline_marker`, not call the new helper with fabricated marker/plan authority. Their shared fixture setup is a real character/recovery/current gameplay grant, actual previous lease and new admission, an actual native Logout receipt, the actual property/BED schema and a matching sleeper from the owning logout path. Each case uses its own D1329 database, the existing network-namespace wrapper and the root's D1342 slot; all errors and assertion panics must clean only that database.

### Submillisecond Logout binding

1. Commit a Logout through the actual native owner. Read its real `marker_as_of_us` and `committed_at` pair. The native owner must expose a deterministic controlled DB-clock test seam or its actual canonical fixture must provide a known nonzero microsecond remainder; an arbitrary client clock is insufficient.
2. Require a nonzero `marker_as_of_us % 1000` fixture precondition and `committed_at == marker_as_of_us / 1000`. Store the sleeper through the real logout participant with that exact `committed_at` value.
3. Transition to the actual independently linked next lease, derive the real latest receipt and settlement plan inside the native TX, and execute BED.
4. Assert that the matching row is eligible; it must not become stale because `slept_at * 1000` differs from the exact microsecond anchor. Verify the returned time against the native plan and first-free bound.
5. A successful positive-time assertion uses the real #2196/#2148 durable consumer in that transaction and waits for those owners. Do not activate the proposal's participant-required branch or supply a stand-in consumer to make this case run. Before that composition exists, this boundary remains a source finding, not a PG PASS.

### Physical-transaction rollback

1. Prepare the same real receipt/lease binding and capture the sleeper, occupancy revision, root/progression/build revisions and existing marker receipts.
2. Exercise a real later training/marker validation error or the owning writer's supported failure seam. A fixture-only trigger on the actual marker-receipt table may raise a deliberate error before its INSERT to prove rollback after BED, if the canonical owner harness permits that fault injection. That trigger belongs only to this case's own database; no replacement receipt schema, authority, food/soul participant or runtime placeholder is supplied.
3. Assert that no deletion, occupancy increment, build/root change or settlement receipt committed. Retry through the actual marker owner and verify that the original state remains available.
4. In a separate own database, disable only `game_house_bed_revision`, execute the real caller, require `MissingRevisionHook`, then verify rollback of the whole footprint. Restore the trigger in that own fixture or drop the own database during cleanup.

### Stale row and replay

Use actual build-carrying receipts to make a prior Logout no longer latest, or use the real predecessor relation to make the sleeper stale. The native settlement must consume exactly that row with one revision increment and no BED award. A subsequent same-key marker retry must return the original receipt before touching BED; a later newly present sleeper must remain untouched.

## Handoff requested from CP / #2174

Publish the actual reserved OFFLINE receipt/build migration and native before/after/chain guards, provide the canonical real-PG marker fixture and FND clock/predecessor source, and identify its deterministic native timestamp/failure seams. Then cite that head for BED's helper cases. D1355's #2196 durable food/vitals/soul participant and #2148 consumer remain separately required before any active positive BED settlement.

The inert source proposal's participant-required branch is a review aid only. It must not be copied into a published runtime caller as a blocking placeholder before the actual food/soul integration.
````

`````

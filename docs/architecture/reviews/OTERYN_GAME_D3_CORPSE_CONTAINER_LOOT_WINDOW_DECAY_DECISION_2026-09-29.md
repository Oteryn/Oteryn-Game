# D3 corpse container, loot window and corpse decay decision

- Decision: `D3-CORPSE-CONTAINER-LOOT-WINDOW-DECAY-V1`
- Status: **CANDIDATE with owner decisions D111-D113 taken (binding, §2) and architecture decisions
  D130-D137 (this document, §4)**. Acceptance requires exact-head validation, independent review
  and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Answers: #162 slice D3 ("corpse container, loot window, corpse decay"), scoped out of B3-2
  explicitly (`docs/agents/tasks/active/OTV2-20260928-b3-2-pickup.md` §"Excluded": "D111-D113
  (loot inside a corpse container, killer-only window, corpse decay, later slice D3)")
- Owner decisions: D111, D112 (#162 comment 5879404970), D113 (#162 comment 5884341120)
- Admission baseline: `main@fb204ea3bac7b89b051e7a60c2036c65e506d7e7`
- Runtime, migration, registry and production authority: **NONE**. This decision resolves
  representation, nesting ceilings, window/decay semantics, content routing and the delivery
  split; each child in §6 needs its own #162 allocation before it may register rows, migrate or
  implement.
- Contract text amended: VSL-COMBAT-01 §9.1 (and its §24.1 pointer) and DUR-03 §39 (a new §39.4
  subsection, alongside the existing B3 and reward-chest amendments) — exact lines in §5.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

D111-D113 require loot to live inside a corpse container (not Ground), a 10 s pickup window
exclusive to the creature's top-damage dealer, and a 60 s corpse decay for the rat corpse
(`oteryn:item.registry.i00005801`). DUR-03 currently treats "corpse association" as provenance
only, never a location (§39.1-§39.2, lines 703-704, 777, 913), and VSL-COMBAT-01 §9.1 (lines
158-162) says a corpse is a runtime projection, "not a second durable item/value store," with
identity "derived from the death occurrence," never a durable identity. §17 and §21 leave corpse
lifetime, loot windows and decay open. Today MINT lands loot directly on Ground with a
`corpse_ref` provenance blob (`combat/death_reward.rs`, migration `0010_item_mint_ground.sql`),
and pickup reads only Ground (`combat/pickup.rs`). This decision fixes the representation, the
location family and nesting ceilings, the window mechanism, the decay semantic and effect, the
content revision, and the delivery split — reconciling D111-D113 with DUR-03's single-location
invariant and VSL-COMBAT-01's "never a second durable location" line without weakening either.

## 2. Owner decisions

| # | Decision | Owner choice |
|---|---|---|
| D111 | Creature loot goes inside the corpse container, not on the ground. | #162 5879404970 |
| D112 | Loot is exclusive for 10 s to the creature's top-damage dealer (party later, once a party system exists), then anyone may loot. | #162 5879404970 |
| D113 | The rat corpse uses corpse item `oteryn:item.registry.i00005801` (a Canary-derived id accepted as content data) and decays after 60 s. | #162 5884341120 |

## 3. Facts

**PROVEN** (main `fb204ea3bac7b89b051e7a60c2036c65e506d7e7`)

- DUR-03 §5.2 (lines 100-125) already defines `Container { parent_item_instance_id, entry }` as
  an accepted typed location family; the B3 decision already proved it for the equipped main
  backpack (migration `0011_item_transfer_backpack.sql`: `game_item_container_slots`,
  `game_item_container_entries`, entry-count ceiling 20, placement depth 1, all enforced by
  `game_item_placement_proven()`).
- DUR-03 §39.1 (lines 666-694): MINT is closed to "established in typed Ground custody with
  applicable corpse association/provenance"; no MINT destination other than Ground is admitted
  except the reward-chest amendment's single named shape (§39.3, lines 937-948), which mints
  directly into a `Container` entry of the *already-existing* equipped backpack, not a fresh item.
  Lines 703-704, 777 and 913 all say corpse association is provenance/projection only, "never a
  competing item location" / "not another immediate location authority."
- VSL-COMBAT-01 §9.1 (lines 158-162): a corpse "may exist as an immediate current-runtime
  world/container projection," is "not a second durable item/value store," and its identity "is
  derived from the death occurrence and exact corpse/content definition revision" — "a corpse
  runtime slot/pointer is not durable identity." §24.1 (line 449-450) repeats: "A corpse is a
  runtime projection, never a second durable location." §17 (line 320) and §21 (lines 385-402)
  leave corpse lifetime, loot windows, decay and "corpse ownership/decay product rules"
  unfrozen "unless required by the first fixture scenario" — D3 is that requirement.
- Current implementation: `combat/death_reward.rs` (`ground_placement`, lines 234-256) derives a
  52-byte `corpse_ref` (WorldId, ChannelId, ownership generation, actor local id and generation)
  as pure provenance on the loot item's own `Ground` row; D2b mints every loot entry straight to
  Ground at the corpse's position. `combat/pickup.rs` (B3-2) resolves `ItemDefinitionFacts` from
  Content and calls B3-1's `freeze_item_transfer`/`commit_item_transfer`, whose only admitted
  TRANSFER source is a live `Ground` row (migration 0011: `game_item_ground_removal_proven`,
  `game_item_ground_insertion_guard`); there is no Container-entry pickup source today.
- **There is no per-creature damage attribution anywhere.** `settle_creature_death_rewards`
  (`combat/death_reward.rs`) already takes a caller-supplied single `reward_principals[0]` for XP
  (VSL-COMBAT-01 §11's single-eligible-principal slice, `COMBAT01-REWARD-PRINCIPALS` = 1); nothing
  in GAME-ABILITY/Combat accumulates or exposes per-attacker damage today.
- `foundation/owner_timer.rs` (AI-1, `OwnerTimerLane`) is a process-local, non-durable timer lane
  already generic over "AI think, respawn, and later spell cooldown/regeneration"; it never
  mutates state itself, requires the caller's live `ScopeRuntimeFence`, and is explicitly
  fence-invalidated on scope handoff. Respawn timers built on it already accept the same posture:
  they are runtime-only and are not recovered from a durable per-timer deadline across restart.
  `occurred_at`/`occurred_at_unix_ms` on every MINT reservation/receipt row
  (`durability/item_mint.rs`) is a durable Postgres `statement_timestamp()` unix-millisecond value
  (migration 0010: `game_item_mint_receipts.occurred_at BIGINT`).
- Content: `oteryn:item.registry.i00005801` (`content/items/definitions/items-05500-05999.json`)
  is `materializable: false`, `stack_class: Unknown`, no capacity, no temporal/decay semantics.
  `oteryn:creature.rat`'s content record (`content/creatures/definitions/creatures-01000-01449.json`)
  **already binds** `authoring.profile.details.corpse_item` to this exact key
  (`{"family":"Item","key":"oteryn:item.registry.i00005801","revision":"definition-r1"}`) and
  already carries a distinct `death_residue.item` (`oteryn:item.registry.i00002781`, the
  blood/pool decal — a separate, unrelated mechanism) and a loot table reference
  (`oteryn:loot.creature.rat`). The creature-to-corpse-item binding is not a new decision; only
  the item definition's own semantics need revision.
- `RESOURCE_LIMITS_REGISTRY.json` already registers `AI01-ACTIVE-ACTORS` = 256 (hard maximum
  concurrent creature actors per scope) and `GAMEITEM01-CONTAINER-ENTRIES-MAX` = 20,
  `GAMEITEM01-PLACEMENT-DEPTH` = 1, `GAMEITEM01-REACHABLE-ITEMS` = 21 (all B3, backpack-scoped).
  DUR-03 RL-07's MINT payload is 6,129 B against a 7,936 B cap (§3.2 of the DUR-03 resource-maxima
  decision) — headroom for one additional 16 B field without re-registration.
- VSL-COMBAT-01 §17: "Cleanup/recovery must never duplicate durable loot or retire live
  acknowledged item value without an accepted DUR-03/domain policy" — permitting retirement
  *with* one. This document is that policy for corpse decay (§4.7).

**UNKNOWN:** the rat's actual loot table entries/probabilities (VSL §21 defers this); Global's
exact corpse capacity and partial-loot-after-decay behavior.

## 4. Architecture decisions (D130-D137)

### 4.1 D130 — Corpse representation

The corpse **is** a durable Ground `ItemInstance` (content key `i00005801`), minted exactly like
any other MINT today — no amendment needed for the corpse item's own location. Creature loot is
minted **directly into a `Container` entry whose parent is that corpse `ItemInstance`** — a new
MINT destination, admitted only for this shape (§5).

- **Corpse MINT.** On every creature death (regardless of whether the loot plan draws any
  entries — a rat always leaves a corpse), one MINT establishes the corpse `ItemInstance` in
  typed Ground custody at the corpse's `GroundPlacement`, exactly the existing MINT shape
  (DUR-03 §39.1/§39.2, unchanged). Its cause reuses the existing loot-cause tuple columns
  verbatim (`death key`, `LootTableDefinitionRef`, `LootEntryOrPurposeKey`, `DeterministicDrawOrdinal`,
  `CREATURE-DEATH-OCCURRENCE-IDENTITY-V1` §4.2) with a reserved sentinel purpose key
  (`CORPSE_MATERIALIZATION`) and `draw_ordinal = 0`, and the corpse's own content identity in
  place of a loot table ref. This needs **no new migration** for its reservation/receipt rows
  (same columns, same non-duplication guarantee as loot) beyond the one addition in §4.3.
- **Loot MINT.** Each loot entry mints directly into `Container { parent_item_instance_id =
  <corpse item>, entry = <ordinal> }` — never onto Ground, never through a separate TRANSFER.
  This is the same shape the reward-chest amendment already established (MINT straight into an
  existing container's entry, no Ground custody, no TRANSFER step), generalized from "the
  already-equipped backpack" to "the corpse this death just materialized." The loot MINT must
  causally follow the corpse's own MINT (same death, same generation); `settle_loot` mints the
  corpse first and reuses its committed `ItemInstanceId` as `parent_item_instance_id` for every
  loot entry, so a retry before the corpse commits safely retries the corpse MINT first.
- **Single-location invariant preserved.** The corpse `ItemInstance` still has exactly one
  authoritative location (Ground). Each loot `ItemInstance` still has exactly one authoritative
  location (`Container`, parent = the corpse). No `ItemInstance` ever carries two locations; DUR-03
  §4's `ItemInstanceId -> exactly one ItemLocationRef` invariant is untouched. What changes is
  only which family a fresh loot MINT is admitted into.
- **Reconciling "never a second durable location."** VSL-COMBAT-01 line 450 bars a corpse from
  being a *second, competing* durable location for an item that is already durably located
  elsewhere, and bars treating a runtime slot/pointer as durable identity. Neither is what this
  decision does: the corpse is the *one* MINTed location its own `ItemInstance` durably has, and a
  loot item's Container parent is a real committed `ItemInstanceId`, never a runtime pointer.
  `corpse_ref`'s 52-byte derived-provenance blob becomes redundant for this shape — a loot item's
  `parent_item_instance_id` already gives exact, referential corpse identity, strictly stronger
  than the current derived-bytes provenance — so the amended shape carries no `corpse_ref` field
  at all on the loot location row (§5).
- **Rejected: keep corpse as pure runtime projection, loot as Ground items "associated" with it
  (today's shape).** This is exactly what D111 rejects: loot on the ground, not inside a
  container. It would need no contract change, but it cannot satisfy the owner decision.
- **Rejected: corpse as a second parallel durable table outside DUR-03's `ItemInstance`
  model.** VSL-COMBAT-01 §9.1 explicitly forbids a second durable item/value store; reusing the
  existing `ItemInstance`/`Container` machinery (already proven by B3-1 for the backpack) is the
  minimum-sufficient path and needs no new location-family concept, only a new admitted parent
  kind and a new MINT-into-container cause shape.

### 4.2 D131 — Location family and nesting ceilings

No new DUR-03 location family: `Container { parent_item_instance_id, entry }` (§5.2) already
covers corpse entries: parent = the corpse `ItemInstance`, entry = placement ordinal, assigned the
same way as B3-1's backpack entries (highest live ordinal + 1, newest-first display, no
renumbering). New `RESOURCE_LIMITS_REGISTRY.json` rows (owner: this document), values chosen as
declared safety ceilings pending real loot-table evidence (VSL §21):

| Row | Value | Basis |
|---|---|---|
| `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` | 8 | Safety ceiling; no loot-table evidence yet exists (§3 UNKNOWN). Set equal to the corpse content definition's own capacity (§4.8), mirroring the backpack pattern where the registered ceiling equals the definition's proven capacity. A proven larger table waits for a new decision, as D82 already established for stack maxima. |
| `GAMEITEM01-CORPSE-PLACEMENT-DEPTH` | 1 | Loot entries are direct children of the corpse item only; no bags inside corpses in this slice (mirrors `GAMEITEM01-PLACEMENT-DEPTH`). |
| `GAMEITEM01-CORPSE-REACHABLE-ITEMS` | 9 | The corpse item (1) plus its 8 entries, mirroring `GAMEITEM01-REACHABLE-ITEMS`. |
| `GAMEITEM01-CORPSES-PER-SCOPE-MAX` | 256 | VSL-COMBAT-01 §19 item 6 ("corpse runtime projections per scope... items per corpse"). Bounded by the already-registered `AI01-ACTIVE-ACTORS` (256): a corpse originates from one committed death of a distinct actor slot, so live corpse count in a scope can never exceed live actor capacity, and 60 s decay (§4.6) keeps the practical count far below this ceiling. No new evidence is invented; this reuses an existing hard maximum as the bound. |

### 4.3 D132 — Top-damage attribution locus

**Computation lives in the Channel owner's runtime state** (not durable per-hit): the owner
already applies every GAME-ABILITY damage effect under its own authority and already tracks
creature HP the same way. It accumulates running per-attacker damage for a live creature actor
exactly as it tracks HP — ephemeral, in-memory, keyed by `ExactActorRef`. Writing a durable row on
every hit would need a new table and a durable write on the hot combat path for a feature whose
entire purpose is a 10 s loot-priority window — disproportionate under the playable-first,
minimum-sufficient doctrine, and inconsistent with FND-03's "owner lane never waits on the
database" (VSL-COMBAT-01 lines 151, 518).

**The single result is captured durably, once, at corpse-materialization commit** — the same
pattern `corpse_ref` already uses today (a runtime-derived fact folded into the corpse's own
committed row): the composition point (`settle_creature_death_rewards`) reads the owner's
accumulated top-damage `CharacterId` for the dying actor at the same moment it already extracts
`(death, corpse)` as owned `Copy` values, and passes it into the corpse's own MINT as one more
field. This is not full attribution history — only the single winning identity is retained,
exactly parallel to XP's existing single-eligible-principal posture (§11).

If the owner scope ends (crash, restart, handoff) before the creature dies, in-progress damage
tracking for that fight is lost with it — the same loss D52 already accepts for undelivered
rewards ("Przepadają, bez duplikatów"): nothing is duplicated, and a creature that never finishes
dying under one generation produces no death, hence no corpse, hence no attribution to lose.

### 4.4 D133 — 10 s exclusivity window

**Durable, restart/scope-move-safe by construction**, because it is derived entirely from data
already committed atomically with the corpse's own MINT, never from live owner state:

- `exclusive_until_unix_ms = <corpse mint receipt>.occurred_at_unix_ms + 10_000`, computed the
  same way `occurred_at` already is (Postgres `statement_timestamp()` at MINT commit).
- `top_damage_character_id` is captured on the same receipt row (§4.3, §5).

Enforcement is a **new DUR-03 admission gate** on the TRANSFER that moves an item out of a corpse
`Container` entry (§4.5, §5): before commit, if wall-clock time is still before
`exclusive_until_unix_ms` and the requesting `CharacterId` is not `top_damage_character_id`, the
TRANSFER is refused (a new `ItemTransferRefusal::CorpseExclusiveWindow`, alongside the existing
`NoMainBackpack`); at or after the deadline, or for the top-damage character, it proceeds under
the same rules as any other corpse-container pickup. Because the check reads only the corpse's own
already-committed receipt row and the current transaction's wall clock, it needs no live owner
state and is correct across a restart or a scope-ownership handoff without any additional recovery
work — a materially cheaper answer than reconstructing an in-memory deadline, and still exactly
correct.

- **Rejected: enforce the window in the runtime owner only (no durable check).** A pickup command
  can arrive at a different node/generation than the one that saw the death (D52's structural
  possibility); a runtime-only check could not see the window at all after a handoff. The durable
  check is strictly cheaper than reconstructing runtime state and closes this gap for free.
- **Rejected: durable per-hit attribution ledger.** Disproportionate to a 10 s cosmetic-priority
  feature; see §4.3.

### 4.5 D134 — Pickup: a new TRANSFER source family

B3-1's TRANSFER admits only a `Ground` source (migration 0011). This decision adds
**`Container { parent = a corpse ItemInstance }` as a second admitted TRANSFER source**,
destination unchanged (`CharacterEquipment` container slot or a `MainBackpack` entry, D80-D83).
Admission (new deferred constraint trigger, symmetric to
`game_item_ground_removal_proven`/`game_item_ground_insertion_guard`):

1. the source entry is a live `Container` row whose parent is a corpse `ItemInstance` (has a
   corpse-materialization receipt, §4.1);
2. the D133 exclusivity gate (§4.4) passes;
3. the existing D80-D83 destination, capacity, stack and merge rules apply unchanged — a corpse
   pickup is otherwise an ordinary TRANSFER, so no new `DUR03-RL-*` row is needed: it reuses the
   already-registered touched-item/participant/work-unit/byte ceilings verbatim (§3, headroom
   noted above).

`combat/pickup.rs` (B3-2) gains a second `GroundPickupRequest`-shaped request variant naming a
corpse-container source instead of a bare Ground `source_item_instance_id`; `resolve_item_definition_facts`
is unchanged (it already resolves from Content by claimed identity, not by source family).

### 4.6 D135 — Decay: time semantic and enforcement

**Time semantic (GAME-ITEM-01 §4.4): durable absolute deadline**, not an active-time budget. A
corpse's decay deadline is `decay_at_unix_ms = <corpse mint receipt>.occurred_at_unix_ms +
60_000` — derived from the same already-durable, already-immutable column the window in §4.4
uses, so **no new mutable field, no clock-drift risk, and no double-decay risk**: the deadline is
a pure function of committed data, computed identically by any owner at any time.

**Enforcement is owner-timer-driven, not database-polling**, consistent with FND-03 and the
existing `OwnerTimerLane` (AI-1) posture: at corpse-MINT commit, the current owner schedules one
decay timer for `decay_at_unix_ms` in its `OwnerTimerLane` (a new `Family`, alongside AI think and
respawn — exactly what the lane's own module documentation already anticipates: "serves AI now and
spell cooldowns and regeneration later"). On drain, the owner applies decay as a normalized input
(§4.7). Because the deadline is durable and derivable, a restart or scope handoff needs **no
persisted timer-recovery table**: during scope (re)admission the new owner queries the bounded set
of corpses currently on Ground in its scope (bounded by `GAMEITEM01-CORPSES-PER-SCOPE-MAX`, §4.2)
via their mint receipts, computes each `decay_at_unix_ms` the same way, and reschedules any not yet
past into its own fresh `OwnerTimerLane` — the same recomputation posture AI-1's respawn timers
already accept, but exact rather than approximate because the deadline itself is durably fixed.
Late decay by at most one owner-cycle after a handoff is possible and accepted (decay is cleanup,
not a value-correctness boundary); early or duplicate decay is not, and cannot happen because the
deadline is a pure function of immutable data.

- **Rejected: durable scheduler table (a `pending_decay` row, polled or notified).** Needs a new
  migration, a new recovery scanner and a new owner/database coupling FND-03 disfavors for
  non-blocking owner work, for a value the owner can already recompute for free from data it
  already has. Not minimum-sufficient.
- **Rejected: active-time budget semantic.** Would require tracking "active" scope time
  separately from wall time, with its own persistence and restart semantics, for no accepted
  requirement (D113 says 60 s from spawn, not 60 s of scope-active time).

### 4.7 D136 — Decay effect on unlooted contents

**Decay retires the corpse `ItemInstance` and every live item in its `Container` entries** — it
does not drop them to Ground. This is the minimal playable choice:

- Dropping to Ground would need a *new* DUR-03 location-transition shape (`Container` →
  `Ground`, never defined; today's DUR-03 transitions are MINT-to-Ground and TRANSFER-between-
  admitted-families only) for a feature with no accepted requirement behind it (D113 says decay,
  not "decay drops to the ground").
- Retirement reuses DUR-03's existing terminal-state shape (§11.4/§11.5: a stack reduced to zero
  retires in the same atomic outcome; an item keeps its identity with quantity 0 and no location).
  A new `DECAY_RETIRE` transaction, keyed by the corpse's own death-derived cause (so a replayed
  decay input is idempotent, exactly like every other DUR-03 transaction), retires the corpse and
  its live entries together, in one atomic outcome, and writes the same audit-envelope evidence
  shape (before/after, cause, conservation summary) every other DUR-03 transaction does.
- VSL-COMBAT-01 §17 explicitly permits retiring live acknowledged item value "with an accepted
  DUR-03/domain policy" — this document is that policy for this named shape only. Every other
  §17/§21 deferral (corpse ownership beyond D112, general cleanup policy, non-rat corpses) stays
  open (§9).
- **Rejected: drop unlooted loot to Ground on decay.** No accepted requirement, needs a new
  location-transition shape DUR-03 does not have, and reintroduces exactly the "loot on the
  ground" outcome D111 rejects — just delayed by 60 s.
- **Rejected: leave decayed corpses live forever (no retirement).** Violates D113's "decays after
  60 s" and would let `GAMEITEM01-CORPSES-PER-SCOPE-MAX` accumulate toward its ceiling instead of
  self-bounding.

### 4.8 D137 — Content revision and routing

`oteryn:item.registry.i00005801` needs a Content-owner package revision (this decision selects no
key, grammar or migration; it specifies exactly what the revision must prove):

- `materializable: true` (it is now a real, minted `ItemInstance`, not identity-only);
- `stack_class: NonStackable` (a corpse is never stacked);
- container semantics: `capacity = 8` (matching `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX`, §4.2);
- temporal/decay capability (GAME-ITEM-01 §4.4): **durable absolute deadline** mode, 60 s from
  materialization (§4.6) — the explicit mode field this decision requires, not a generic duration;
- `oteryn:creature.rat`'s `corpse_item` binding to this key is **already authored** (§3) and needs
  no change.
- **Routing and digest pinning.** This is routed to the Content owner on #162, the same way the
  B3-2 backpack content revision (D114) was withdrawn and routed (`docs/agents/tasks/active/
  OTV2-20260928-b3-2-pickup.md`: "it needs a Content-owner package revision... routed on #162").
  The revision must round-trip with `content/world/definitions/reference.json`, whose digest is
  pinned by the World Project manifest/lock (same requirement B3-2 already states); the baseline
  digests this decision was evaluated against are recorded as evidence:
  `content/items/definitions/items-05500-05999.json` sha256
  `22771d49b19bd290686a126301520a751d9a02547be4c6edfad02664c9ae22c0`,
  `content/world/definitions/reference.json` sha256
  `a28b4d553248ce184496a7c501bc54ec18682451b5fb0c2c85a8250496c2d8b8`. A successor package revision
  must pin its own new digests; this decision does not itself mint or revise content.

## 5. Contract amendments

### 5.1 VSL-COMBAT-01 §9.1 (lines 158-162) and §24.1 (line 449-450)

Text unchanged; a pointer note is added stating that for the single named shape of §4.1 above, the
creature's own corpse is a durable Ground `ItemInstance` — the *one* location that item durably
has, not a second location for anything else. Line 450's "never a second durable location" bars a
corpse from being a competing second location for an item already durably located elsewhere, or
from being treated as durable identity from a runtime slot/pointer; it does not bar the corpse
`ItemInstance` itself from being the one MINTed location a loot item durably receives via
`Container(parent=corpse)`. §17 (line 320) and §21's "corpse ownership/decay product rules" (line
396) are resolved, for this named shape only, by §4.4-§4.7 above; every other §17/§21 deferral is
unchanged. Every other VSL-COMBAT-01 obligation is unchanged.

### 5.2 DUR-03 §39 — new §39.4 "Corpse container amendment (D3)"

Added after the existing "Reward chest amendment" paragraph (after line 948), in the same form as
the B3 and reward-chest amendments:

> **Corpse container amendment.** `D3-CORPSE-CONTAINER-LOOT-WINDOW-DECAY-V1` §4.1 (owner decisions
> D111-D113) admits, for creature-death loot only, a MINT whose first and only location is a new
> entry of the corpse `ItemInstance` that death's own corpse-materialization MINT established (§4.1
> above) — no Ground custody and no TRANSFER for that loot item. The corpse's own MINT is
> unamended: it is an ordinary Ground MINT under the existing §39.1/§39.2 shape, keyed by a
> reserved `CORPSE_MATERIALIZATION` cause. The B3 amendment's `Container` source restriction is
> further extended (§4.5 above) to admit TRANSFER *out of* a corpse-parented `Container` entry,
> gated by the D133 exclusivity window (§4.4 above). A new `DECAY_RETIRE` transaction shape (§4.6
> above) retires a corpse `ItemInstance` and its live entries together under VSL-COMBAT-01 §17's
> "accepted DUR-03/domain policy" clause. Every other §39 obligation (fences, cause, evidence,
> idempotency, current authority, conservation) is unchanged.

Lines 703-704, 777 and 913 ("corpse association is provenance/projection only... never a competing
item location") are **unchanged and remain true for the corpse item's own MINT**, which is still a
Ground MINT exactly as those lines describe; they never applied to a loot item's own MINT
destination, which this amendment is the first to define beyond Ground. No existing sentence is
struck; §39.4 is additive, exactly as the B3 and reward-chest amendments already are.

## 6. Delivery

| Child | Scope | Depends on |
|---|---|---|
| D3-1 | Migration: corpse-materialization receipt column (`corpse_top_damage_character_id`), the corpse-container MINT-into-`Container` destination (extends `game_item_mint_consistency_guard`), `GAMEITEM01-CORPSE-*` and `GAMEITEM01-CORPSES-PER-SCOPE-MAX` registration | This decision |
| D3-2 | `combat/death_reward.rs`: mint the corpse first (`CORPSE_MATERIALIZATION` cause), then mint each loot entry into its `Container` entry instead of Ground; carry the owner's runtime top-damage `CharacterId` into the corpse MINT request | D3-1 |
| D3-3 | GAME-ABILITY/Combat: runtime per-attacker damage accumulation for a live creature actor, exposed to `settle_creature_death_rewards` as the top-damage `CharacterId` at death-commit time (§4.3) | none (parallel to D3-1/D3-2) |
| D3-4 | Migration + `durability/item_transfer.rs`: admit TRANSFER source `Container(parent=corpse)`, the D133 exclusivity admission gate, `ItemTransferRefusal::CorpseExclusiveWindow` | D3-1 |
| D3-5 | `combat/pickup.rs` (B3-2 extension): a corpse-container pickup request variant wired to D3-4 | D3-4 |
| D3-6 | `foundation/owner_timer.rs`: a decay `Family`, scheduled at corpse-MINT commit and rescheduled from durable receipts at scope (re)admission; the `DECAY_RETIRE` transaction (migration + durability module) | D3-1, D3-2 |
| D3-7 | Content: `i00005801` revision (materializable, stack class, container capacity 8, temporal/decay mode) per §4.8, routed to the Content owner with digest pinning | none (parallel); D3-2/D3-6 need it merged before their own tests can use real content |

Each child needs its own #162 allocation and independent review before implementation, per repo
governance; none may merge ahead of D3-1's resource-row registration for the rows it depends on.

## 7. Rejected options (summary)

- Loot stays on Ground, corpse stays a pure runtime projection (rejected: contradicts D111).
- A second durable store outside `ItemInstance`/`Container` for corpses (rejected: VSL-COMBAT-01
  §9.1 forbids a second durable item/value store; unnecessary given the existing model already
  covers it).
- Durable per-hit damage attribution ledger (rejected: disproportionate to a 10 s window, §4.3).
- Runtime-only (non-durable) window enforcement (rejected: cannot survive a node/generation
  handoff, §4.4).
- Durable persisted decay-scheduler table (rejected: unnecessary given the deadline is already a
  pure function of committed data, §4.6).
- Drop unlooted loot to Ground on decay (rejected: no accepted requirement, needs an undefined
  location-transition shape, reintroduces the outcome D111 rejects, §4.7).

## 8. Non-decisions

`DECISIONS_NOT_TAKEN` (explicit, per VSL-COMBAT-01 §21 and this document's own scope):

- exact rat loot table entries, probabilities and quantities (VSL §21; Content-owner evidence);
- corpse content for any creature other than the rat;
- party/shared loot attribution once a party system exists (D112 explicitly defers this);
- what happens to a corpse's `Container` entries if `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` is
  reached mid-drop (rejects the remaining entries, per the existing D81/§7 fail-closed posture;
  not separately re-litigated here since it reuses B3-1's already-accepted full-container refusal);
- whether decay should be configurable per creature/content rather than a fixed 60 s (D113 fixes
  60 s for the rat only; a general decay-duration content field is a later decision if more
  creatures need a different value);
- client corpse/loot UI representation and animation;
- whether a corpse blocks movement/tile occupancy (unrelated to this decision's location/durability
  scope);
- PvP/skull looting-restriction interaction (VSL-COMBAT-01 §21 already defers PvP entirely).

## 9. Decision test

- **Must decide now:** YES. D3 is explicitly excluded from B3-2 and blocks any corpse-container
  pickup implementation; VSL-COMBAT-01 §17/§21 explicitly deferred this exact question "unless
  required by the first fixture scenario," and D111-D113 make it required now.
- **Minimum sufficient:** reuses DUR-03's existing `Container` family and B3-1's proven
  entry/ordinal/merge machinery for corpse entries; reuses the reward-chest amendment's
  MINT-into-container pattern for loot; reuses the existing `OwnerTimerLane` for decay scheduling
  instead of a new scheduler; derives both the window deadline and the decay deadline from a
  column (`occurred_at`) that is already durable and already written, adding exactly one new
  durable field (`corpse_top_damage_character_id`) across the whole slice.
- **Superseding evidence:** proven rat loot-table entries that need more than 8 entries; an owner
  requirement that decay drop loot to Ground instead of retiring it; a party system that reopens
  D112's exclusivity rule; Global evidence that corpse capacity differs from 8.
- **Deliberately not decided:** §8 above.

## 10. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalations: ["#162 5879404970 (D111, D112)", "#162 5884341120 (D113)"]
owner_decisions: [D111, D112, D113]
architecture_decisions: [D130, D131, D132, D133, D134, D135, D136, D137]
amends:
  - docs/architecture/VSL-COMBAT-01_MINIMAL_COMBAT_DEATH_LOOT_CONTRACT_CANDIDATE.md   # §9.1 pointer, §17/§21 resolution for this shape only
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md        # new §39.4, additive only
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md
resource_values_changed: true   # GAMEITEM01-CORPSE-* and GAMEITEM01-CORPSES-PER-SCOPE-MAX values selected here; registration is D3-1's
production_authority_changed: false
cross_repository_authority_changed: false
implementation_lanes: [D3-1, D3-2, D3-3, D3-4, D3-5, D3-6, D3-7]
implementation_may_resume: true   # D3-1 and D3-3/D3-7 may be allocated now on this owner/architecture decision; this text still needs protected integration
required_fresh_allocation: true
required_independent_review: "exact-head independent review (DUR-03 §39.4 amendment, VSL-COMBAT-01 §9.1 pointer, corpse/loot MINT and DECAY_RETIRE conservation, resource rows)"
required_revalidation:
  - "D3-1: corpse MINT commits with no loot custody; a loot MINT commits only with a live corpse Container parent already committed; the 9th corpse entry is rejected; corpse count 257 in one scope is rejected"
  - "D3-2/D3-3: a replayed death produces the same corpse ItemInstanceId and the same top-damage CharacterId; a death with zero loot entries still materializes a corpse"
  - "D3-4/D3-5: pickup by the top-damage character succeeds before 10 s; pickup by any other character is refused before 10 s and succeeds at/after 10 s; a stale/duplicate pickup command transfers at most once"
  - "D3-6: a corpse decays at exactly 60 s from its MINT's occurred_at, not before; a decay retires the corpse and every live entry in one atomic outcome; a restart before decay reschedules the same deadline from the durable receipt, not a new one"
remaining_unknowns:
  - rat loot table entries/probabilities (Content owner)
  - corpse content for other creatures
  - party loot attribution (later, once a party system exists)
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates D3-1, D3-3 and D3-7 (parallel), followed by D3-2, D3-4/D3-5 and D3-6."
```

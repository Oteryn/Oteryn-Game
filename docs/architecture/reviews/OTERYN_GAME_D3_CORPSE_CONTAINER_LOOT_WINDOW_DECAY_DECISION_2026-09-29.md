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
- Contract text amended and **applied in this PR** (not merely described): VSL-COMBAT-01 §9.1,
  §17, §21 and §24.1, and a new DUR-03 §39.4 "Corpse container amendment (D3)" — exact applied
  line ranges in §5.
- Revision note (2026-09-29, post-review, round 1): a Codex review of head `98a76b7` on PR #1198
  found 5 P1s, all fixed — corpse capacity raised to the already-accepted 16 (not an invented 8),
  the corpse-per-scope bound reconciled to the already-accepted `COMBAT01-CORPSES-PER-SCOPE` = 64
  (not a derived 256), the amendment text applied to the contract files instead of only described,
  the window/decay anchor moved off the reservation-time `occurred_at`, and the corpse item itself
  explicitly barred from ever being a TRANSFER source.
- Revision note (2026-09-29, post-review, round 2): a Codex review of head `e11f77d` found 4 more
  P1s and 1 P2, all fixed — `DECAY_RETIRE` restructured into N+1 one-item transactions instead of
  one 17-item transaction, so it fits the existing default `DUR03-RL-01`/`-RL-06` ceiling with no
  new row; the "crash mid-sequence recovers the plan" claim corrected to state the accepted D52
  partial-corpse loss explicitly (retries only within one generation); a new, bounded
  `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` = 16 row closes the previously unbounded per-creature
  contributor map; a deterministic two-rule tie-break is specified for equal top-damage totals; and
  `materialized_at` is now anchored by `clock_timestamp()` in a deferred constraint trigger firing
  at commit, named as the latest *reachable* pre-commit point (not the exact commit/visibility
  instant) with its residual earliness bounded by the existing `DFR-DB-PASS-MS` timeout. See §4
  and §5 for the corrected text.
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
- **`occurred_at` is a reservation-time timestamp, not a commit-time one.**
  `durability/item_mint.rs::freeze_item_mint` (lines 410-422) reads
  `statement_timestamp()` once, at PREPARE/reservation time, and stores it on the `Reservation`
  row; `commit_item_mint` (lines 591-628) reuses that same frozen value verbatim (`frozen.occurred_at_unix_ms`,
  line 625) — it never re-reads the clock at actual commit. A reservation that sits pending for any
  length of time before its commit pass runs would silently shrink a deadline derived from
  `occurred_at`. This is the exact gap the original D3 draft's window/decay formulas had; §4.4/§4.6
  below anchor to a new, deferred-trigger `clock_timestamp()` column instead. Every semantic
  transaction (including this one) already runs under `transaction_timeout`/`statement_timeout`/
  `lock_timeout` set to the remaining time of the already-registered `DFR-DB-PASS-MS` budget
  (2,000 ms, `durability/db.rs::begin_semantic_transaction`, lines 1088-1096) — the named, already
  -configured setting the corrected anchor's residual earliness bound (§4.4) uses.
- **D52's generation-scoped retry rule** (`CREATURE-DEATH-OCCURRENCE-IDENTITY-V1` §4.3,
  `reviews/OTERYN_GAME_DUR03_RESOURCE_MAXIMA_AND_CREATURE_DEATH_IDENTITY_DECISION_2026-09-28.md`):
  "A descendant (loot MINT, XP award) commits only while the death's ownership generation is the
  current assignment... When the generation ends (restart, crash, scope move), uncommitted
  descendants are dropped. No later generation retries them." A multi-entry loot plan is exactly
  such a set of descendants; the earlier D3 draft's "a crash mid-sequence is recovered" language
  contradicted this and is corrected in §4.2/§5.2 below.
- **D57's creature-per-scope envelope** (`AI-RL-11`,
  `reviews/OTERYN_GAME_AI_ACTION_INTEGRATION_FIRST_CREATURE_SLICE_DECISION_2026-09-28.md` §4.9):
  16 spawn sources per scope × 4 creatures per spawn = **64 live creatures per scope**, the hard
  structural ceiling this document's new `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` row (§4.3)
  multiplies against for its worst-case memory statement.
- `RESOURCE_LIMITS_REGISTRY.json` and the accepted VSL resource-rows decision already register,
  and this document must not silently duplicate or contradict:
  `COMBAT01-LOOT-PLAN-ITEMS` = 16 ItemInstances per death
  (`reviews/OTERYN_GAME_VSL_COMBAT_RESOURCE_ROWS_DECISION_2026-09-28.md`),
  `COMBAT01-ITEMS-PER-CORPSE` = 16 direct root only, and **`COMBAT01-CORPSES-PER-SCOPE` = 64
  corpse projections per scope, with its own already-accepted overflow rule: "reject the
  projection; the death still commits and loot follows D52"** (same decision, §4.1 row 6).
  `AI01-ACTIVE-ACTORS` = 256 bounds concurrent creature *actors*, not deaths accumulated over
  time, and is not a valid basis for a corpse-count ceiling (a scope can produce far more than 256
  deaths across its lifetime even with few actors alive at once, via respawn churn).
- Content: `oteryn:item.registry.i00005801` (`content/items/definitions/items-05500-05999.json`)
  is `materializable: false`, `stack_class: Unknown`, no capacity, no temporal/decay semantics.
  `oteryn:creature.rat`'s content record (`content/creatures/definitions/creatures-01000-01449.json`)
  **already binds** `authoring.profile.details.corpse_item` to this exact key
  (`{"family":"Item","key":"oteryn:item.registry.i00005801","revision":"definition-r1"}`) and
  already carries a distinct `death_residue.item` (`oteryn:item.registry.i00002781`, the
  blood/pool decal — a separate, unrelated mechanism) and a loot table reference
  (`oteryn:loot.creature.rat`). The creature-to-corpse-item binding is not a new decision; only
  the item definition's own semantics need revision.
- `RESOURCE_LIMITS_REGISTRY.json` already registers `GAMEITEM01-CONTAINER-ENTRIES-MAX` = 20,
  `GAMEITEM01-PLACEMENT-DEPTH` = 1, `GAMEITEM01-REACHABLE-ITEMS` = 21 (all B3, backpack-scoped).
  DUR-03 RL-07's MINT payload is 6,129 B against a 7,936 B cap (§3.2 of the DUR-03 resource-maxima
  decision) — headroom for two additional small fields (a `CharacterId` and a `BIGINT` timestamp)
  without re-registration.
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
  place of a loot table ref. Its receipt row gets two additive, nullable columns beyond the
  ordinary MINT shape — `corpse_top_damage_character_id` (§4.3) and `materialized_at` (§4.4,
  §4.6), both `NOT NULL` exactly when `loot_purpose_key = 'CORPSE_MATERIALIZATION'` and `NULL`
  otherwise — so no other MINT shape (reward chest, future non-corpse loot) is touched.
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
- **The corpse `ItemInstance` is never itself pickupable.** It leaves Ground only through
  `DECAY_RETIRE` (§4.7); it is never a TRANSFER source, whether or not it currently holds live
  entries (§4.5). This keeps "a corpse lives only on Ground" an invariant the decay-recovery query
  (§4.6) can rely on without also having to watch for a corpse that quietly migrated into someone's
  backpack.
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
renumbering). New `RESOURCE_LIMITS_REGISTRY.json` rows (owner: this document):

| Row | Value | Basis |
|---|---|---|
| `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` | **16** | Not invented: equal by construction to the already-accepted `COMBAT01-LOOT-PLAN-ITEMS` and `COMBAT01-ITEMS-PER-CORPSE` (both 16, `reviews/OTERYN_GAME_VSL_COMBAT_RESOURCE_ROWS_DECISION_2026-09-28.md` §4.1 row 6). This row is the GAME-ITEM-01 §7.1 container-legality mirror of that already-accepted combat-domain ceiling; the two must never diverge, and this document sets both the content capacity (§4.8) and this row to the same value they already imply. |
| `GAMEITEM01-CORPSE-PLACEMENT-DEPTH` | 1 | Loot entries are direct children of the corpse item only; no bags inside corpses in this slice (mirrors `GAMEITEM01-PLACEMENT-DEPTH`). |
| `GAMEITEM01-CORPSE-REACHABLE-ITEMS` | 17 | The corpse item (1) plus its 16 entries, mirroring `GAMEITEM01-REACHABLE-ITEMS`. |
| `COMBAT01-CORPSES-PER-SCOPE` | **64 (already accepted; not re-decided here)** | `reviews/OTERYN_GAME_VSL_COMBAT_RESOURCE_ROWS_DECISION_2026-09-28.md` §4.1 row 6, matching the D57 creature envelope ("64, jak potwory", D78). This is the *one* bound on concurrent corpses per scope; no `GAMEITEM01-CORPSES-PER-SCOPE-MAX` or other competing/derived row is introduced. The 64th-plus-one overflow rule is likewise reused verbatim, not re-decided: "reject the projection; the death still commits and loot follows D52" (exact overflow behaviour below). |

**Whole-plan preflight, not partial commit.** Because a death's loot plan is itself capped at 16
accepted entries (`COMBAT01-LOOT-PLAN-ITEMS`) and a corpse's own capacity is also 16, an accepted
plan can never exceed corpse capacity by construction — but the composing caller still checks the
plan's full accepted entry count against `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` once, before
freezing the corpse or any loot entry, exactly mirroring how `COMBAT01-INFLIGHT-LOOT-MINTS-PER-SCOPE`
is "checked once per death... before any entry of its loot plan is frozen... the whole plan is
rejected up front rather than minting some entries and stopping partway" (§3). The corpse's entry
count is read under a row lock on the corpse (the same `FOR UPDATE`-before-count pattern
`game_item_placement_proven()` already uses for the backpack), so concurrent loot MINTs for one
corpse serialize instead of racing the count. Each entry still commits as its own DUR-03
transaction (§39.1: MINT sequences are never combined into one commit) — **this preflight prevents
only a capacity-caused partial commit, not a generation-ending one.** Per D52
(`CREATURE-DEATH-OCCURRENCE-IDENTITY-V1` §4.3): a loot entry's MINT retries only while the death's
ownership generation is still the current assignment; if that generation ends (restart, crash,
scope move) before every preflight-admitted entry has committed, the remaining entries are dropped
terminally — no later generation retries them. A corpse may therefore durably hold fewer live
entries than its own accepted plan's count; this is the exact, already-accepted D52 loss
("Przepadają, bez duplikatów"), stated here explicitly rather than implied: **never a duplicate,
never silently completed by a later generation, and never a capacity overrun** (the preflight above
already rules that case out completely).

**`COMBAT01-CORPSES-PER-SCOPE` overflow behaviour (exact, not re-derived).** This decision keeps
the already-accepted rule verbatim rather than inventing an alternative (such as retiring the
oldest corpse early to make room): on the 65th concurrent corpse in one scope, **the new corpse's
MINT is refused before it or any of its loot entries freeze** (`CapacityExceeded`, checked first,
before the corpse-materialization cause is even reserved). Because D111 gives loot no destination
other than the corpse it belongs to, refusing the corpse refuses its whole loot plan with it; the
creature's death itself still commits (death and loot are independent descendants, §11/§24.1), and
the lost loot is exactly the class of loss D52 already accepts for a death whose descendant never
commits — **never duplicated, never silently retried into a different corpse or onto the ground**.
No already-committed corpse or its already-committed loot is ever touched by another death's
overflow: this is a refusal of the *new* arrival, never an eviction of an existing one, so no
already-materialized loot is ever lost outside this one stated rule. A future party/shared-loot or
higher-throughput slice that finds 64 insufficient re-decides `COMBAT01-CORPSES-PER-SCOPE` itself
(owned by the VSL resource-rows decision); this document does not raise it.

### 4.3 D132 — Top-damage attribution locus

**Computation lives in the Channel owner's runtime state** (not durable per-hit): the owner
already applies every GAME-ABILITY damage effect under its own authority and already tracks
creature HP the same way. It accumulates running per-attacker damage for a live creature actor
exactly as it tracks HP — ephemeral, in-memory, keyed by `ExactActorRef`. Writing a durable row on
every hit would need a new table and a durable write on the hot combat path for a feature whose
entire purpose is a 10 s loot-priority window — disproportionate under the playable-first,
minimum-sufficient doctrine, and inconsistent with FND-03's "owner lane never waits on the
database" (VSL-COMBAT-01 lines 151, 518).

**`COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` = 16 (new row, owner: this document).** The
per-creature contributor map (`CharacterId -> running total`) is otherwise unbounded — every
distinct attacker of a long fight would grow it without limit. Ceiling: at most 16 distinct
`CharacterId` contributors tracked per live creature actor. Overflow rule: once a creature's map
holds 16 distinct contributors, a *new* (17th) distinct attacker's damage is applied to the
creature exactly as normal (GAME-ABILITY/SIM combat math is completely unaffected) but is **not
added to the contributor map**, so that attacker cannot become, or affect who becomes, the
top-damage winner; every already-tracked contributor keeps accumulating normally. This is a
narrow, cosmetic-priority-only cap (fail-open for combat, fail-closed only for extra attribution
slots), not a gameplay restriction. Worst-case memory: at most 64 live creatures can exist in one
scope at once (D57, `AI-RL-11`: 16 spawn sources × 4 creatures per spawn,
`reviews/OTERYN_GAME_AI_ACTION_INTEGRATION_FIRST_CREATURE_SLICE_DECISION_2026-09-28.md` §4.9), so
the whole scope's contributor-map state is bounded at 64 × 16 = 1,024 `CharacterId` entries,
entirely in-memory and entirely ephemeral (§4.3 below).

**Deterministic tie-break among equal top totals (P2, boundary-tested by D3-3).** Ties are possible
(two attackers dealing identical total damage). Resolution, in order:

1. the contributor whose running total **first reached** the tied maximum value wins — tracked as
   each contributor's own last-updated ordinal in the owner's already-deterministic damage-
   application order (FND-03/SIM); since a contributor's total only increases, "first reached" is
   simply the smaller of the two contributors' last-update ordinals at the moment their total last
   changed to that value;
2. if that still ties (simultaneous application within the same deterministic ordinal, or two
   contributors whose totals both last changed at the same ordinal), the contributor with the
   lexicographically **lowest `CharacterId` byte sequence** wins.

Both rules are pure functions of already-deterministic owner state (the same ordering FND-03/SIM
already guarantees for damage application), so a replayed fight resolves the same tie the same way
every time. D3-3 must prove an explicit equal-damage boundary test (two contributors reaching the
identical total, resolved by rule 1; a same-ordinal tie, resolved by rule 2).

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
already committed atomically with the corpse's own MINT, never from live owner state — and
**anchored to the latest pre-commit timestamp reachable, not the existing reservation-time
`occurred_at`** (§3: `occurred_at` is frozen at PREPARE and only replayed at commit,
`item_mint.rs:410-422` / `:591-628`, so it under-measures any window/deadline by however long the
reservation sat pending). The corpse's own receipt row instead carries a new `materialized_at
BIGINT` column, set from `NULL` to `clock_timestamp()` by a new deferred (`AFTER INSERT ...
DEFERRABLE INITIALLY DEFERRED`) constraint trigger that fires immediately before the transaction's
own commit finalizes — never a value the Rust candidate carries, and never `statement_timestamp()`,
which is fixed per-statement and so can itself precede the actual commit (§5.2). This is **not
claimed as the exact commit/visibility instant**: the residual gap between this read and actual
commit is bounded by whatever remains of the transaction's `DFR-DB-PASS-MS`-derived timeout budget
(2,000 ms, §5.2) at that point, so the window (and decay, §4.6) may be **early by up to that bound
in the worst case, never late — accepted, not a defect**, since a few seconds' early expiry is
immaterial to either a 10 s window or a 60 s decay.

- `exclusive_until_unix_ms = <corpse mint receipt>.materialized_at + 10_000`.
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

### 4.5 D134 — Pickup: a new TRANSFER source family, and the corpse item's own exclusion

B3-1's TRANSFER admits only a `Ground` source (migration 0011). This decision adds
**`Container { parent = a corpse ItemInstance }` as a second admitted TRANSFER source, for loot
entries only**, destination unchanged (`CharacterEquipment` container slot or a `MainBackpack`
entry, D80-D83). Admission (new deferred constraint trigger, symmetric to
`game_item_ground_removal_proven`/`game_item_ground_insertion_guard`):

1. the source entry is a live `Container` row whose parent is a corpse `ItemInstance` (has a
   corpse-materialization receipt, §4.1);
2. the D133 exclusivity gate (§4.4) passes;
3. the existing D80-D83 destination, capacity, stack and merge rules apply unchanged — a corpse
   pickup is otherwise an ordinary TRANSFER, so no new `DUR03-RL-*` row is needed: it reuses the
   already-registered touched-item/participant/work-unit/byte ceilings verbatim (§3, headroom
   noted above).

**The corpse `ItemInstance` itself is never a legal TRANSFER source**, for its own Ground row,
regardless of whether it currently has live entries. `plan_transfer`'s existing
`ContainerNotEmpty` refusal (`item_transfer.rs:405-407`) only blocks moving a *non-empty*
container; once a corpse's loot is fully picked out (or its plan drew zero entries), that check
no longer applies and generic Ground TRANSFER would otherwise admit picking up the empty corpse
itself into a backpack — which would delete its Ground row via the ordinary TRANSFER path and
leave any not-yet-fired decay timer pointed at a location that no longer exists. A new refusal,
`ItemTransferRefusal::CorpseNotPickupable`, closes this: TRANSFER rejects any source item that
carries a live `CORPSE_MATERIALIZATION` receipt, checked independently of `ContainerNotEmpty` and
enforced by a DUR-03 constraint trigger (not only the Rust admission function), so a corpse's
Ground row cannot be deleted by TRANSFER by construction. As a second, independent closure at the
content layer (§4.8), the corpse definition declares no `container`-slot equip pattern, so even a
bypassed source check would still fail `NotContainerSlotEquippable` for the `ContainerSlot`
destination; `MainBackpack` has no equivalent independent closure, which is why the TRANSFER-level
refusal above is the binding one, not merely a content-layer side effect.

**Other paths checked and closed (siblings of this finding):**

- A corpse can never be a D83 merge/top-up *receiver*: receivers are matched by identity/
  definition compatibility against an item already resident in the backpack (DUR-03 §13); a
  corpse never enters a backpack in the first place (the refusal above), so it can never appear as
  a receiver candidate either.
- A loot MINT cannot target an arbitrary parent to "materialize" straight into a character's own
  container, bypassing pickup entirely: DUR-03 §39.4 requires the parent to carry a live
  `CORPSE_MATERIALIZATION` receipt for the *same* death, which no character-owned container ever
  has.
- The corpse's own MINT cause is unique per death (the existing reservation/receipt primary key,
  §4.1), so no second corpse MINT can ever occur for one death, and no double-corpse escape path
  exists.
- `DECAY_RETIRE` is the only transaction that removes a corpse's Ground row (§4.7); it is not
  itself a leaving-Ground-while-still-live path, since retirement is terminal, not a relocation.

`combat/pickup.rs` (B3-2) gains a second `GroundPickupRequest`-shaped request variant naming a
corpse-container source instead of a bare Ground `source_item_instance_id`; `resolve_item_definition_facts`
is unchanged (it already resolves from Content by claimed identity, not by source family).

### 4.6 D135 — Decay: time semantic and enforcement

**Time semantic (GAME-ITEM-01 §4.4): durable absolute deadline**, not an active-time budget. A
corpse's decay deadline is `decay_at_unix_ms = <corpse mint receipt>.materialized_at + 60_000` —
the same latest-pre-commit-anchor column the window (§4.4) uses, not the reservation-time
`occurred_at` (§3), so **no new mutable field beyond `materialized_at` itself, no clock-drift
risk, and no double-decay risk**: the deadline is a pure function of committed data, computed
identically by any owner at any time, early by at most the same bounded amount §4.4 already states
(never late, never a correctness issue).

**Enforcement is owner-timer-driven, not database-polling**, consistent with FND-03 and the
existing `OwnerTimerLane` (AI-1) posture: at corpse-MINT commit, the current owner schedules one
decay timer for `decay_at_unix_ms` in its `OwnerTimerLane` (a new `Family`, alongside AI think and
respawn — exactly what the lane's own module documentation already anticipates: "serves AI now and
spell cooldowns and regeneration later"). On drain, the owner applies decay as a normalized input
(§4.7). Because the deadline is durable and derivable, a restart or scope handoff needs **no
persisted timer-recovery table**: during scope (re)admission the new owner queries the bounded set
of corpses currently on Ground in its scope (bounded by the already-accepted
`COMBAT01-CORPSES-PER-SCOPE` = 64, §4.2) via their mint receipts, computes each `decay_at_unix_ms`
the same way, and reschedules any not yet past into its own fresh `OwnerTimerLane` — the same
recomputation posture AI-1's respawn timers already accept, but exact rather than approximate
because the deadline itself is durably fixed. Late decay by at most one owner-cycle after a
handoff is possible and accepted (decay is cleanup, not a value-correctness boundary); early or
duplicate decay is not, and cannot happen because the deadline is a pure function of immutable
data.

**The recovery query is Ground-only and excludes already-retired corpses.** Per §4.1/§4.5, a
corpse's own location is always Ground and it is never a TRANSFER source, so the scope-admission
query reads only live (`lifecycle = 1`) `game_item_ground_locations` rows joined to a
`CORPSE_MATERIALIZATION` receipt — never a `Container` row, and never a corpse already retired by
`DECAY_RETIRE` under a different (possibly prior) owner, which is excluded by the `lifecycle = 1`
filter and therefore never double-scheduled. Because `DECAY_RETIRE` is a sequence of separate
per-item steps (§4.7) and the corpse's own row stays `lifecycle = 1` until its own final step
commits, this same query is exactly what resumes a partially-drained corpse: it looks identical to
a fresh, never-started decay, and re-issuing the remaining steps is safe by each step's own
idempotent cause.

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
- Retirement reuses DUR-03's existing terminal-state shape (§11.4/§11.5: an item keeps its identity
  with quantity 0 and no location once retired). **A full corpse is up to 17 `ItemInstance`s
  (corpse + 16 entries) — past `DUR03-RL-01`/`-RL-06`'s existing 1-2 touched-item ceiling, so
  `DECAY_RETIRE` is *not* one N-item atomic transaction.** It is **N+1 separate one-item
  transactions**: one per live entry currently parented to the corpse, each keyed by that entry's
  own idempotent cause, then one final transaction for the now-empty corpse, keyed by its own
  `CORPSE_MATERIALIZATION` cause and admitted only once zero live entries remain under it (§5.2).
  Each step fits the existing default `DUR03-RL-01`(1)/`DUR03-RL-06`(1 participant/3 work units)
  ceilings exactly, so **no new `DUR03-RL-*` row is registered** — the restructuring is preferred
  over inventing a corpse-sized limit. A corpse mid-drain stays refused as a TRANSFER source the
  whole time (`CorpseNotPickupable` does not check entry count, §4.5), and decay is **resumable
  from durable state alone**: the §4.6 recovery query finds any corpse still live on Ground past
  its deadline — whether decay never started or stopped partway — and simply continues issuing the
  remaining per-entry steps and then the corpse step; no in-memory "decay in progress" marker is
  needed, because "still on Ground past its deadline" already is that state, durably.
- VSL-COMBAT-01 §17 explicitly permits retiring live acknowledged item value "with an accepted
  DUR-03/domain policy" — this document is that policy for this named shape only. Every other
  §17/§21 deferral (corpse ownership beyond D112, general cleanup policy, non-rat corpses) stays
  open (§9).
- **Rejected: drop unlooted loot to Ground on decay.** No accepted requirement, needs a new
  location-transition shape DUR-03 does not have, and reintroduces exactly the "loot on the
  ground" outcome D111 rejects — just delayed by 60 s.
- **Rejected: leave decayed corpses live forever (no retirement).** Violates D113's "decays after
  60 s" and would let `COMBAT01-CORPSES-PER-SCOPE` accumulate toward its 64 ceiling instead of
  self-bounding.

### 4.8 D137 — Content revision and routing

`oteryn:item.registry.i00005801` needs a Content-owner package revision (this decision selects no
key, grammar or migration; it specifies exactly what the revision must prove):

- `materializable: true` (it is now a real, minted `ItemInstance`, not identity-only);
- `stack_class: NonStackable` (a corpse is never stacked);
- container semantics: `capacity = 16` (matching `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX`, §4.2,
  itself equal to the already-accepted `COMBAT01-LOOT-PLAN-ITEMS`/`COMBAT01-ITEMS-PER-CORPSE`);
- **no `container`-slot equip pattern** (closes the `ContainerSlot`-destination sibling path,
  §4.5);
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

## 5. Contract amendments (applied)

Both amendments below are applied to the contract files **in this PR**, not merely described. They
are additive: no existing sentence in either file is struck or reworded.

### 5.1 VSL-COMBAT-01 — applied at four points

`docs/architecture/VSL-COMBAT-01_MINIMAL_COMBAT_DEATH_LOOT_CONTRACT_CANDIDATE.md`:

- **§9.1, after the existing "runtime slot/pointer is not durable identity" sentence** (new
  paragraph "D3 corpse-container amendment"): states that for this named shape the corpse is the
  one durable Ground `ItemInstance`, that "never a second durable location" bars a *competing*
  second location (not the corpse's own single MINTed one), and that the corpse is never itself a
  TRANSFER source.
- **§17, after the existing "unless required by the first fixture scenario" sentence** (new
  paragraph "D3 resolution (rat corpse only)"): names D111-D113 as that required scenario and
  points to §4.4/§4.6/§4.7 of this decision for the window, decay and decay-effect values.
- **§21's non-decisions list**: the existing "corpse ownership/decay product rules" bullet gains an
  inline qualifier — resolved for the rat corpse only by D3; open for every other creature.
- **§24.1, after the existing "A corpse is a runtime projection, never a second durable location"
  sentence** (new paragraph "D3 chain amendment"): states the alternate chain for creature-death
  loot (corpse MINT to Ground, then loot MINT-into-container, then a gated TRANSFER out — no
  separate TRANSFER for the corpse itself) and that every non-corpse MINT keeps the unamended
  Ground-then-TRANSFER chain.

### 5.2 DUR-03 — new §39.4 "Corpse container amendment (D3)"

`docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`, inserted after the
existing "Reward chest amendment" paragraph and before "Expected bindings versus current
authority" (same position the B3 and reward-chest amendments already occupy). It states, in full:
the corpse's own MINT is unamended (ordinary Ground MINT, `CORPSE_MATERIALIZATION` cause); a loot
entry's MINT establishes only a `Container(parent=<that corpse>)` entry, and only when that parent
carries a live `CORPSE_MATERIALIZATION` receipt for the same death; the whole-plan preflight and
its locking (§4.2); the new commit-time `materialized_at` column and why `occurred_at` cannot be
reused for it (§3, §4.4, §4.6); that the corpse `ItemInstance` is never a legal TRANSFER source,
with the new `CorpseNotPickupable` refusal and its content-layer closure (§4.5); the new
`Container(parent=corpse)` TRANSFER-out source gated by the D112/D133 window; the already-accepted
`COMBAT01-CORPSES-PER-SCOPE` = 64 bound and its exact, unchanged overflow rule (§4.2); the
Ground-only, `lifecycle = 1`-filtered recovery query (§4.6); and the new `DECAY_RETIRE` transaction
under VSL-COMBAT-01 §17's "accepted DUR-03/domain policy" clause (§4.7).

Lines 703-704, 777 and 913 ("corpse association is provenance/projection only... never a competing
item location") are **unchanged and remain true for the corpse item's own MINT**, which is still a
Ground MINT exactly as those lines describe; they never applied to a loot item's own MINT
destination, which §39.4 is the first to define beyond Ground.

## 6. Delivery

| Child | Scope | Depends on |
|---|---|---|
| D3-1 | Migration: two additive nullable columns on the corpse's own `game_item_mint_receipts` row (`corpse_top_damage_character_id`, `materialized_at`, both `NOT NULL` only for the `CORPSE_MATERIALIZATION` receipt), the `materialized_at`-writing deferred constraint trigger and its narrow immutability exception, the corpse-container MINT-into-`Container` destination and its parent-carries-a-live-receipt check (extends `game_item_mint_consistency_guard`), the whole-plan preflight's corpse-row locking, `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX`/`-PLACEMENT-DEPTH`/`-REACHABLE-ITEMS` registration (16/1/17) | This decision |
| D3-2 | `combat/death_reward.rs`: mint the corpse first (`CORPSE_MATERIALIZATION` cause), then mint each loot entry into its `Container` entry instead of Ground; carry the owner's already-tie-broken top-damage `CharacterId` (D132) into the corpse MINT request | D3-1 |
| D3-3 | GAME-ABILITY/Combat: runtime per-attacker damage accumulation for a live creature actor (capped at `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` = 16, new row registered by this decision, overflow = untracked, not refused), the D132 deterministic tie-break, exposed to `settle_creature_death_rewards` as the top-damage `CharacterId` at death-commit time (§4.3); max/max+1 and equal-damage tie-break boundary tests | none (parallel to D3-1/D3-2) |
| D3-4 | Migration + `durability/item_transfer.rs`: admit TRANSFER source `Container(parent=corpse)` gated by the D133 window (`ItemTransferRefusal::CorpseExclusiveWindow`), and the new `ItemTransferRefusal::CorpseNotPickupable` refusal for the corpse item itself as a source (§4.5) | D3-1 |
| D3-5 | `combat/pickup.rs` (B3-2 extension): a corpse-container pickup request variant wired to D3-4 | D3-4 |
| D3-6 | `foundation/owner_timer.rs`: a decay `Family`, scheduled at corpse-MINT commit from `materialized_at` and rescheduled from durable, `lifecycle = 1`-filtered Ground receipts at scope (re)admission (naturally resuming a partially-drained corpse, §4.6); the `DECAY_RETIRE` transaction as N+1 one-item steps — one per live entry, then the corpse (§4.7) — each fitting the existing default `DUR03-RL-01`/`-RL-06` ceilings, no new resource row (migration + durability module) | D3-1, D3-2 |
| D3-7 | Content: `i00005801` revision (materializable, stack class, container capacity 16, no `container`-slot equip pattern, temporal/decay mode) per §4.8, routed to the Content owner with digest pinning | none (parallel); D3-2/D3-6 need it merged before their own tests can use real content |

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
- An invented, independent `GAMEITEM01-CORPSES-PER-SCOPE-MAX` derived from `AI01-ACTIVE-ACTORS`
  (rejected: `AI01-ACTIVE-ACTORS` bounds concurrent actors, not deaths accumulated over a scope's
  lifetime, so it is not valid evidence for a corpse-count ceiling; the already-accepted
  `COMBAT01-CORPSES-PER-SCOPE` = 64 is reused instead, §4.2).
- Retiring the oldest corpse early to admit a new one past `COMBAT01-CORPSES-PER-SCOPE`
  (rejected: not the already-accepted rule; refusing the new arrival, not evicting an existing
  one, is the exact behaviour already decided and is strictly simpler and safer, §4.2).
- Anchoring the window/decay deadlines to the existing `occurred_at` column (rejected: it is a
  reservation-time timestamp, replayed unchanged at commit, so it under-measures both deadlines by
  however long the reservation was pending, §3/§4.4).
- Relying only on `ContainerNotEmpty` to keep a corpse out of a backpack (rejected: it stops
  admitting only a *non-empty* container, so an emptied or never-populated corpse would still pass
  it; an explicit `CorpseNotPickupable` refusal closes the gap regardless of entry count, §4.5).
- A single N-item `DECAY_RETIRE` transaction touching the corpse and all its entries at once
  (rejected: up to 17 touched items, far past `DUR03-RL-01`/`-RL-06`'s existing 1-2 ceiling; would
  need a new, corpse-sized resource row purely to retire, not create, value. Restructuring into
  N+1 one-item steps fits the existing default ceilings exactly and needs no new row, §4.7).
- "A crash mid-sequence recovers the plan," implying cross-generation retry of uncommitted loot
  entries (rejected: contradicts D52, under which a generation that ends drops its uncommitted
  descendants terminally; corrected to state the accepted partial-corpse loss explicitly, §4.2).
- An unbounded per-creature damage-contributor map (rejected: unbounded memory growth over a long
  fight with many distinct attackers; capped at `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` = 16,
  fail-open for combat and fail-closed only for extra attribution slots, §4.3).
- A non-deterministic or unspecified top-damage tie-break (rejected: would make a replayed fight
  resolve a tie differently across replays; the two-rule deterministic order in §4.3 is a pure
  function of already-deterministic owner state).
- Anchoring `materialized_at` to the `INSERT` statement's own `statement_timestamp()`, or to any
  value the Rust candidate carries (rejected: `statement_timestamp()` is fixed per statement, not
  per transaction, and can itself precede the actual commit; a deferred constraint trigger reading
  `clock_timestamp()` immediately before commit is the latest anchor reachable from inside the
  transaction and does not depend on `insert_mint` being the transaction's last statement, §5.2).
- Claiming `materialized_at` as the exact commit or durable-visibility instant (rejected: it is the
  latest *reachable* pre-commit anchor; the residual gap is named and bounded by the existing
  `DFR-DB-PASS-MS`-derived timeout instead of asserted away, §4.4).

## 8. Non-decisions

`DECISIONS_NOT_TAKEN` (explicit, per VSL-COMBAT-01 §21 and this document's own scope):

- exact rat loot table entries, probabilities and quantities (VSL §21; Content-owner evidence);
- corpse content for any creature other than the rat;
- party/shared loot attribution once a party system exists (D112 explicitly defers this);
- what happens to a corpse's `Container` entries if `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` is
  reached mid-drop (structurally unreachable in this slice, since it equals
  `COMBAT01-LOOT-PLAN-ITEMS`, §4.2; a future larger loot table that could exceed 16 waits for a new
  decision, not a silent partial-plan behaviour);
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
  instead of a new scheduler; reuses the already-accepted `COMBAT01-LOOT-PLAN-ITEMS`/
  `-ITEMS-PER-CORPSE` (16), `COMBAT01-CORPSES-PER-SCOPE` (64) and D57's 64-creature-per-scope
  envelope instead of inventing independent ones; restructures `DECAY_RETIRE` into N+1 one-item
  steps rather than registering a new corpse-sized resource row; adds exactly two new durable,
  nullable, corpse-scoped fields (`corpse_top_damage_character_id`, `materialized_at`) and exactly
  one new bounded, ephemeral, in-memory row (`COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` = 16)
  across the whole slice, with `materialized_at` the one genuinely new durable capability needed (a
  bounded-early, not exact-commit, anchor, since `occurred_at` cannot serve that role, §3/§4.4).
- **Superseding evidence:** proven rat loot-table entries that need more than 16 (re-decides
  `COMBAT01-LOOT-PLAN-ITEMS`/`-ITEMS-PER-CORPSE`, owned by the VSL resource-rows decision, not this
  one); an owner requirement that decay drop loot to Ground instead of retiring it; a party system
  that reopens D112's exclusivity rule; sustained scope throughput that needs more than 64
  concurrent corpses (re-decides `COMBAT01-CORPSES-PER-SCOPE`, same owner); a fight regularly
  exceeding 16 distinct attackers where an untracked 17th contributor's exclusion from the window
  is judged unacceptable (re-decides `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE`, owned by this
  document).
- **Deliberately not decided:** §8 above.

## 10. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalations: ["#162 5879404970 (D111, D112)", "#162 5884341120 (D113)"]
owner_decisions: [D111, D112, D113]
architecture_decisions: [D130, D131, D132, D133, D134, D135, D136, D137]
amends:
  - docs/architecture/VSL-COMBAT-01_MINIMAL_COMBAT_DEATH_LOOT_CONTRACT_CANDIDATE.md   # applied: §9.1, §17, §21, §24.1 (§5.1)
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md        # applied: new §39.4, additive only (§5.2)
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md
resource_values_changed: true   # GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX/-PLACEMENT-DEPTH/-REACHABLE-ITEMS (16/1/17) and COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE (16) are new; COMBAT01-CORPSES-PER-SCOPE (64) is reused unchanged; no new DUR03-RL-* row (DECAY_RETIRE restructured to fit the existing default 1-2 item ceiling); registration is D3-1's (items) / D3-3's (contributors)
production_authority_changed: false
cross_repository_authority_changed: false
implementation_lanes: [D3-1, D3-2, D3-3, D3-4, D3-5, D3-6, D3-7]
implementation_may_resume: true   # D3-1 and D3-3/D3-7 may be allocated now on this owner/architecture decision; this text still needs protected integration
required_fresh_allocation: true
required_independent_review: "exact-head independent review (DUR-03 §39.4 amendment including the restructured N+1-step DECAY_RETIRE and the materialized_at deferred-trigger mechanism and its bounded-earliness claim, VSL-COMBAT-01 §9.1/§17/§21/§24.1 amendments, corpse/loot MINT conservation, the D52-aligned partial-corpse-loss statement, resource rows including COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE)"
required_revalidation:
  - "D3-1: corpse MINT commits with no loot custody; materialized_at is set only by the deferred constraint trigger's clock_timestamp(), never by the Rust candidate, and stays NULL until then; a loot MINT commits only with a live corpse Container parent already committed for the same death; the 17th corpse entry is rejected under a row-locked count; corpse count 65 in one scope is rejected before any of its entries freeze"
  - "D3-2/D3-3: a replayed death (same generation) produces the same corpse ItemInstanceId and the same top-damage CharacterId; a generation that ends mid-plan leaves the remaining entries terminally uncommitted, never retried by a later generation, and the corpse holds only the entries that did commit; a death with zero loot entries still materializes a corpse; the 17th distinct damage contributor is not tracked and cannot win the window while the creature's damage taken is unaffected; two contributors tied at the same total resolve to the one that reached it at the earlier owner ordinal, and a same-ordinal tie resolves to the lower CharacterId"
  - "D3-4/D3-5: pickup by the top-damage character succeeds before 10 s; pickup by any other character is refused before 10 s and succeeds at/after 10 s, both within materialized_at's accepted early-by-at-most-DFR-DB-PASS-MS bound; a stale/duplicate pickup command transfers at most once; TRANSFER of the corpse item itself is refused (CorpseNotPickupable) whether or not it currently has live entries"
  - "D3-6: a corpse decays at materialized_at + 60 s, never later, and early by at most the accepted DFR-DB-PASS-MS bound; DECAY_RETIRE commits as separate one-item steps, never one multi-item transaction; the corpse's own step is admitted only once zero live entries remain under it; a restart or handoff after only some entries retired resumes and completes the remaining steps from the durable Ground-only recovery query alone, with no in-memory progress state; an entry legitimately picked up before decay reaches it is never retired a second time"
remaining_unknowns:
  - rat loot table entries/probabilities (Content owner)
  - corpse content for other creatures
  - party loot attribution (later, once a party system exists)
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates D3-1, D3-3 and D3-7 (parallel), followed by D3-2, D3-4/D3-5 and D3-6."
```

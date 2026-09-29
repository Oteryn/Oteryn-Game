# D4 multi-hit damage receipt decision

- Decision: `D4-MULTI-HIT-DAMAGE-RECEIPT-V1`
- Status: **CANDIDATE, no new owner decision (§2)**. Acceptance requires exact-head validation,
  independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Answers: #162 slice D4 ("multiple committed damage occurrences per creature generation")
- Amends: no accepted contract text; this decision applies already-accepted policy
  (VSL-COMBAT-01 §7, §13; GAME-ABILITY-01 §7.1, §12, §13) to a carrier gap those contracts already
  govern but the implementation has not yet closed
- Admission baseline: `main@f611ffe3e44a02c6549b00670473e7bfc6174fba`
- Runtime, registry and production authority: **NONE**. This decision resolves the retention
  bound, bound behaviour while alive, the owner damage-application ordinal and lethal-receipt
  identification; the single implementation child in §5 registers the new resource row and applies
  the code change.
- Revision note (2026-09-29, control-plane rejection, round 1): the control plane rejected the
  original D141 (hard fail-closed reject at 16 receipts, no eviction) — a live creature became
  permanently unkillable after 16 non-lethal hits from even one attacker, a real gameplay bug on
  the playable path (misses, chip damage against the D116 rat), not the boss-scale edge case the
  original text assumed. D141 is rewritten below: zero-damage occurrences never reach the receipt
  list at all (`InvalidDamage` already gates them, §4.2); no ordered occurrence identity reaches
  the carrier today (finding, §4.2) so the caller must supply a new explicit per-attacker monotonic
  sequence; retained receipts are now always evicted oldest-first to admit a new occurrence, guarded
  by a per-attacker high-water mark (reusing D132's bounded 16-attacker map) so an evicted
  occurrence's replay is refused, never re-applied; the bound is now unreachable for admission and
  reachable only for a narrow, explicitly stated residual case (§4.2). D140, D142, D143 and D144 are
  unchanged by this revision.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

`apps/game-server/src/foundation/runtime_actor_carrier.rs`'s `Slot::CreatureOccupied` keeps
`committed: Option<OwnerCommitRecord>` (line 655) — room for exactly one committed damage
occurrence per creature generation. `commit_creature_damage_inner` (line 1771) checks it first: if
`committed` already holds a record and the incoming `occurrence` bytes do not match that one
record's, the call fails `CarrierError::OccurrenceConflict` (line 1817) **before it ever reads
current health**. A creature can therefore take only one committed hit, ever, in its generation —
lethal or not. That blocks the playable path (killing the D116 rat fixture over several hits, each
its own ability cast/occurrence) and blocks D3-3 (PR #1215, `D3_CORPSE_..._DECISION_2026-09-29.md`
§4.3, D132): its own commit message states plainly that "today's carrier still commits exactly one
hit per creature generation (the separately routed 'multiple hits per creature generation' P1, not
addressed here)" and its overkill-credit test seeds a prior contributor directly on the slot to
work around the single-record limit rather than driving it through two real commits. Which bound
replaces the single-record ceiling, what happens at that bound while the creature is still alive,
how the owner damage-application ordinal D132 already references informally is introduced, what
stays out of scope, and how the lethal receipt stays identifiable?

## 2. Owner decisions

No new owner decision. The two facts this decision turns into a resource row and a carrier rule
are both already accepted, not chosen here:

- the new retention ceiling reuses the already-accepted family value of 16 (`COMBAT01-LOOT-PLAN-ITEMS`,
  `COMBAT01-ITEMS-PER-CORPSE`, D77; `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE`, D132) rather than
  inventing a new number;
- the fail-closed behaviour at that ceiling is GAME-ABILITY-01 §12's already-mandatory policy
  ("Further uncommitted descendants are deterministically rejected/terminated under the owning
  fail-closed policy... The engine MUST NOT roll back history, silently disable the safety limit...
  or continue unbounded"), applied here rather than re-decided.

## 3. Facts

**PROVEN** (main `f611ffe3e44a02c6549b00670473e7bfc6174fba`)

- `Slot::CreatureOccupied.committed: Option<OwnerCommitRecord>` (line 655); `OwnerCommitRecord {
  binding: Box<[u8]>, damage: i64, result: OwnerDamageResult }` (line 378). `admit_inner`
  initializes it `None` (line 1674, inside `fn admit_inner`, line 1618).
- `commit_creature_damage_inner`'s replay/conflict branch (lines 1811-1826):
  ```
  if let Some(prior) = committed {
      if prior.binding.split(|byte| *byte == 0).next() != Some(occurrence) {
          return Err(CarrierError::OccurrenceConflict);
      }
      if prior.binding.as_ref() != binding || prior.damage != damage {
          return Err(CarrierError::PlanConflict);
      }
      return Ok(OwnerDamageResult { applied: false, ..prior.result });
  }
  if *health == 0 {
      return Err(CarrierError::CreatureNotActionable);
  }
  ```
  A *different* occurrence than the one already committed is rejected unconditionally, whether or
  not the creature is still alive; the `health == 0` check is only reached for a creature with no
  committed record at all. `PlanConflict` already correctly separates "same occurrence, different
  content" from this; `OccurrenceConflict` is the exact bug.
- The existing test
  `lethal_damage_disables_actions_but_administrative_remove_is_not_death`
  (`foundation/channel_owner_ability_commit_tests.rs:162`) asserts a *second, different* occurrence
  after a lethal hit returns `OccurrenceConflict` — today's only committed occurrence is already
  the lethal one, so this happens to read as correct, but the assertion is exercising the same
  branch that also blocks a second **non-lethal** hit; `channel_owner_combat_death_tests.rs:279`
  has the equivalent assertion in the death/corpse composition test. Neither test currently drives
  a creature through two real distinct commits.
- VSL-COMBAT-01 §7 (lines 113-141): "one creature lifecycle generation produces at most one logical
  death occurrence"; "replay/duplicate delivery of the lethal effect cannot create a second death".
  Nowhere does §7 say a generation may commit at most one *damage* occurrence — only at most one
  *death* occurrence. §13 (lines 280-297) makes the deterministic, bounded `VSL_COMBAT_FIXTURE_PROFILE`
  test/evidence-only and explicitly not Reference or Evolved product policy by itself — it does not
  itself fix hit count, so it neither requires nor forbids multi-hit; the current one-record ceiling
  is an implementation artifact, not something §13 mandates.
- GAME-ABILITY-01 §7.1 (lines 108-115): "A mechanic requiring intentional sequential or partial
  resolution MUST represent it as explicit deterministic ordered sub-occurrences/commit groups...
  This applies to multi-hit, multi-target and other ordered mechanics when prior commits are
  allowed to influence later results." This governs ordering *within* one ability's own commit
  group (one cast producing several ordered hits) — a GAME-ABILITY-owned concern, already textually
  scoped to the ability/commit-group layer, not the carrier's per-creature receipt storage.
  §12 (lines 209-224): every reactive/future-work dimension needs bounded descendants and, at the
  bound, "the parent and already committed descendants remain committed. Further uncommitted
  descendants are deterministically rejected/terminated under the owning fail-closed policy and
  bounded evidence is emitted." §234 (§13, "Resource-limit contract"): "every externally/content-
  controlled work or allocation dimension used by that implementation MUST have an explicit hard
  maximum... in the accepted resource-limit mechanism" before executable acceptance. An unbounded
  or single-slot damage-receipt dimension satisfies neither.
- `OTERYN_GAME_VSL_COMBAT_RESOURCE_ROWS_DECISION_2026-09-28.md` §4.1 registers twelve combat
  dimensions and has no row for damage receipts or damage occurrences per creature generation.
- D52 (`CREATURE-DEATH-OCCURRENCE-IDENTITY-V1` §4.3): runtime state not committed before the
  generation ends is dropped, never retried by a later generation — the damage-receipt list this
  decision adds is exactly this class of state: ephemeral, per-generation, never durable.
- D132 (`OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md` §4.3) already
  describes, informally, "each contributor's own last-updated ordinal in the owner's already-
  deterministic damage-application order" as the first tie-break rule for top-damage attribution.
  D3-3 (branch `claude/d3-3-top-damage`, PR #1215, **not yet merged to main**) implements exactly
  this as a private `DamageContributors { entries: Vec<DamageContributor>, next_ordinal: u64 }`
  boxed on the slot (`damage_contributors: Box<DamageContributors>`), with `record()` assigning
  `self.next_ordinal` at the mutation boundary and only when an `attacker: Some(CharacterId)` is
  supplied — so today an attacker-less commit never advances it. D132/D3-3 names this value only as
  an internal `last_update_ordinal`; no document has yet named it, as this decision's charge asks,
  "the owner damage-application ordinal" as a first-class, generally assigned value.
- `commit_creature_damage_inner`'s own write-time recheck (lines 1935-1955) already re-validates
  generation, target identity, `health == result.health_before` and `committed.is_some()` at the
  mutation boundary independent of the earlier read — the pattern this decision's widened check
  reuses unchanged.
- `size_of::<Slot>() == 192` is asserted today (test at line 3359); D3-3 boxes its new
  `DamageContributors` state specifically to keep this a one-pointer growth (192 -> 200) rather
  than an inline one, following the same convention `target_identity: Arc<[u8]>` already set.
- `committed_lethal_receipt_inner` (line 1908) and `validate_lethal_receipt` (line 1950) — read by
  `project_committed_lethal_inner`, which `CreatureDeathOccurrenceRef`, D2b's reward occurrence and
  the D3 corpse decision's corpse MINT all depend on — read the slot's single `committed: Some(committed)`
  field directly and require `committed.result.applied && committed.result.health_before > 0 &&
  committed.result.health_after == 0`. Nothing about their external contract (`CreatureDeathOccurrenceRef`,
  its `death_key()`, `commit_binding()`, `damage()`, `health_before()`) depends on there being only
  one slot in `committed`; it depends only on there being exactly one **matching** record.

- `commit_creature_damage_inner`'s own damage validation (line ~1789) already rejects `damage <= 0`
  with `CarrierError::InvalidDamage` **before it ever loads the slot or looks at `committed`**. A
  zero-damage (miss) occurrence therefore never reaches the receipt list, replay logic or the bound
  at all today, under any design; it needs no retention decision (§4.2).
- **No ordered occurrence identity reaches the carrier today.** `AbilityOccurrenceId`
  (`ability/occurrence.rs:4`) is an opaque, caller-chosen string atom (`valid_atom`-validated only);
  nothing constrains it to be monotonic, content-addressed or attacker-scoped. The one production
  bridge that would supply it to the carrier, `ability::commit::commit_exact_owner_damage`
  (`ability/commit.rs:255`), is documented in its own comment as "a real typed Ability->Foundation
  bridge, compiled into the game-server library but never composed into live gameplay" — it passes
  `plan.occurrence().id().as_str().as_bytes()` straight through as `occurrence`, with no `CommandRef`
  or sequence involved. Separately, `CommandRef = (GameSessionId, CommandId)` with `CommandIngress`'s
  strictly-increasing `CommandId` (`foundation/mod.rs:93,477-483`, `next_command_id: Some(CommandId(1))`,
  non-zero, monotonic) **is** existing, already-proven per-session ordered infrastructure, used today
  for exactly this kind of duplicate-safe ordering by WO-0 local-object commands (`world_runtime.rs`)
  and B3-2 pickup (`combat/pickup.rs`, VSL-COMBAT-01 §12: "duplicate CommandRef/interaction child
  never transfers the same item twice") — but nothing wires it into the ability/combat damage-commit
  path, and this decision does not itself prove that a `GameSessionId`'s `CommandId` sequence stays
  monotonic across a reconnect (`DISCONNECT-PROTECTION-V1`'s `ControlLossMark` keeps the same
  committed player actor reconnectable, but does not by itself prove `CommandIngress` continuity
  across that reconnect).

**UNKNOWN:** measured hits-to-kill for shipped non-fixture content above the rat; whether any
planned creature needs more than 16 distinct committed occurrences in one generation; whether a
`GameSessionId`'s `CommandId` sequence is guaranteed monotonic across a reconnect mid-fight.

## 4. Decision (D140-D144)

### 4.1 D140 — Retention bound

`Slot::CreatureOccupied.committed` widens from `Option<OwnerCommitRecord>` to a bounded, boxed
collection of up to 16 records, one per distinct committed occurrence this generation:

```
committed: Box<DamageReceipts>,
```
```
struct DamageReceipts {
    entries: Vec<OwnerCommitRecord>,   // len() <= COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX
    next_ordinal: u64,                 // D142
}
```
`OwnerCommitRecord` gains one field, `ordinal: u64` (D142). Boxing keeps the per-slot footprint
growth to one pointer, the same convention `target_identity: Arc<[u8]>` and D132/D3-3's
`damage_contributors: Box<DamageContributors>` already use; the implementation child updates the
`size_of::<Slot>()` regression-guard test to whatever value it measures, with a comment, exactly as
D3-3's own 192 -> 200 change did.

**New resource-limit row** (registered by the implementation child, single-writer lease):

| Row | Hard maximum | Basis |
|---|---|---|
| `COMBAT01-DAMAGE-RECEIPTS-PER-CREATURE-GENERATION` | **16** distinct committed damage occurrences retained per live creature actor, per generation | Reuses the already-accepted family value (D77 `COMBAT01-LOOT-PLAN-ITEMS`/`COMBAT01-ITEMS-PER-CORPSE` = 16; D132 `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` = 16), not invented. Distinct from `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE`: that row bounds distinct **attacker `CharacterId`s**; this row bounds distinct **occurrence identities** (hits) — one attacker landing 16 hits consumes 16 receipt slots and one contributor slot. |

Worst case: `AI01_SPAWN_SOURCES_PER_SCOPE_MAX * AI01_SPAWN_POPULATION_MAX` = 64 live creatures per
scope (D57), each retaining at most 16 receipts of at most `MAX_OWNER_COMMIT_BINDING_BYTES` = 4,096
B binding each — an existing per-record bound this decision does not change, only multiplies by up
to 16 per creature, entirely in-memory and entirely ephemeral (D52).

`16` still bounds how many receipts are **retained at once**; it no longer bounds how many distinct
occurrences a creature may accept over its life (revised D141, §4.2, below) — the row's hard maximum
and basis are unchanged, only what happens when a 17th distinct occurrence arrives.

### 4.2 D141 — Behaviour at the bound while alive (revised, see header revision note)

**1. Zero-damage occurrences.** They need no retention decision: `damage <= 0` already fails
`InvalidDamage` before the slot or `committed` is ever touched (§3). Re-applying 0 damage is not
merely idempotent, it is unreachable — a miss never creates, consumes or contends for a receipt
slot under this or any design. Nothing here changes that check.

**2. Identity finding.** §3 found no ordered occurrence identity reaches the carrier today:
`AbilityOccurrenceId` is an opaque, unordered string atom, and the one bridge that would supply it
is not composed into live gameplay. `CommandRef`/`CommandId` is existing, already-proven per-session
monotonic infrastructure used elsewhere (WO-0, B3-2 pickup) but not threaded into ability damage
commits, and its continuity across a reconnect is not proven here. This decision therefore does not
assume ordering is already available; it adds a new, explicit, optional input the caller must supply
to get eviction-safe treatment, and defines safe behaviour when it is absent.

**3. The mechanism: bounded eviction guarded by a per-attacker high-water mark.**
`commit_creature_damage_inner` gains a new parameter alongside D3-3's existing `attacker:
Option<CharacterId>`: `attacker_sequence: Option<u64>` — the caller's promise that, for this exact
`(creature actor, attacker)` pair within this generation, successive calls supply a non-decreasing
value (the natural fit is that attacker's own `CommandId`, once the ability bridge is composed into
live gameplay and wired through `CommandRef`; a fixture/test caller may supply any monotonic
counter it controls). D132/D3-3's bounded 16-entry `DamageContributor` map (already keyed by
`CharacterId`, `COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX`) gains one field per entry,
`high_water_sequence: Option<u64>`, reusing the same bounded structure rather than adding a second
one. The lookup widens:

- **Match found** (a retained record's occurrence equals the incoming one): unchanged
  idempotent-replay/`PlanConflict` logic, checked per-record instead of against a single field.
- **No match, health == 0:** unchanged `CreatureNotActionable`.
- **No match, health > 0, attacker tracked and `attacker_sequence <= that attacker's
  high_water_sequence`:** refuse with a new `CarrierError::StaleAttackerSequence`, never mutating
  HP or the receipt list. This is provably a replay, not a new hit: sequences from one attacker are
  promised non-decreasing, so anything at or below the recorded high-water mark was already resolved
  once, whether or not its own receipt is still retained — refusing it can never double-apply
  damage, because the damage it represents was already applied (or never will be, if it was itself
  ever rejected) the first time its sequence was seen.
- **No match, health > 0, otherwise (a genuinely new occurrence):** apply the damage; if
  `attacker_sequence` is `Some` and the attacker is tracked (or has room to be, in D132's bounded
  contributor map, above), raise that attacker's `high_water_sequence`; append a new record with the
  next ordinal (D142). If
  the retained set is already at 16, **evict the oldest *evictable* record first** (lowest ordinal
  among records whose attacker has a tracked `high_water_sequence`, i.e. whose future replays are
  already provably refusable by the check above) to make room, rather than refusing the new
  occurrence. Every distinct new occurrence from a tracked attacker is therefore always admitted —
  **a single attacker can land any number of hits without ever being blocked**, which is the exact
  bug the control plane flagged; eviction only ever removes a receipt whose own stale replay is
  already safe to refuse.
- **No match, health > 0, retained set at 16, and no evictable record exists** (every retained
  receipt is either from an attacker with no `attacker_sequence` supplied, or from a 17th-or-later
  distinct attacker past `COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX` = 16 and therefore untracked
  in D132's map): refuse with `CarrierError::DamageReceiptCapacityExceeded`, fail-closed, exactly
  as GAME-ABILITY-01 §12 requires — HP and every retained receipt stay untouched. Unlike the
  original D141, this is now the narrow fallback, not the common case (residual limitation, below).

`CarrierError::OccurrenceConflict` is still removed as unreachable (§4.2 original reasoning
unchanged); its two test sites are rewritten to assert `CreatureNotActionable`, as before.

**Replay window.** Exactly the currently retained receipts (up to 16) replay byte-identically; an
occurrence whose receipt was evicted, if it is ever resubmitted, is either provably stale (refused,
never re-applied, if it carried a tracked `attacker_sequence`) or — for an untracked attacker only —
was never eligible for eviction in the first place, so it is still retained and still replays
byte-identically. No retained-but-untracked receipt is ever silently dropped.

**Why eviction is now safe (unlike the originally rejected FIFO option, §6).** Plain FIFO eviction
was rejected because evicting a receipt with no ordering signal could let a later replay of it be
mistaken for new. The high-water mark closes exactly that gap for tracked attackers: refusing a
stale sequence is a function of the attacker's own monotonic counter, not of whether that specific
receipt is still retained, so eviction of a *tracked* attacker's older receipts can never cause a
double-apply.

**Residual, explicitly accepted limitation.** `DamageReceiptCapacityExceeded` remains reachable only
when at least 16 of the retained receipts are simultaneously untracked — either genuinely
attacker-less (AI/environment-sourced, no `CharacterId`) or from the 17th-or-later distinct attacker
past D132's own already-accepted 16-attacker cardinality ceiling (`COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE`,
whose own overflow rule already accepts that a 17th+ attacker's damage is applied but not tracked for
attribution; this decision extends the same accepted ceiling to eviction-safety). Reaching the bound
now requires at least 16 simultaneously untracked receipts, not merely 16 total hits from one
attacker — narrower than, and consistent with, D132's already-accepted boundary, and not reachable by
the rat/playable path (every player attacker supplies a `CharacterId`). Raising this further (e.g. a
larger or unbounded high-water-mark map for untracked sources) is a later decision if content needs
it (§7).

**Open question for the owner/coordinator (not blocking this decision, §7).** Whether a
`GameSessionId`'s `CommandId` sequence is guaranteed monotonic across a reconnect is not proven by
this document (§3); the implementation child that wires `attacker_sequence` from live gameplay must
either prove that continuity or derive the sequence from something already proven durable across
reconnect. Until the ability bridge is actually composed into live gameplay (§3: it is not, today),
no production caller supplies `attacker_sequence` at all, so no production behaviour regresses while
this is resolved.

### 4.3 D142 — The owner damage-application ordinal

**Introduced formally here** (D132 only referenced it informally as a contributor's "last-updated
ordinal"): a `u64` counter, `DamageReceipts::next_ordinal`, assigned by the **Channel owner** — the
same authority executing `commit_creature_damage_inner`'s mutation boundary, under its existing
compare-and-recheck pattern — to every newly committed **distinct** occurrence for a creature,
**never to a replay**. It starts at 0 on `admit_inner` (fresh per creature admission, matching
`DamageContributors`'s own fresh-per-admission initialization) and increments by exactly one per
newly accepted occurrence, monotonically for the life of the creature's current local generation;
it is never reset except by a fresh admission into the slot (a new generation), and is dropped with
the rest of runtime state at generation end (D52). Each newly appended `OwnerCommitRecord` stores
the ordinal value assigned to it (`ordinal: u64`); a replay returns the previously stored value
unchanged (it does not read `next_ordinal` again), so ordinals stay a gap-free, strictly increasing
sequence of *new* commits only.

**How it feeds D132.** D132/D3-3's `DamageContributors::record` currently self-generates its own
`next_ordinal`, only when an `attacker` is supplied — a second, independent counter that can drift
from this one and silently skips attacker-less commits. This decision supersedes that private
counter: `DamageContributors::record` takes the already-assigned ordinal as a parameter from the
same mutation boundary instead of incrementing its own, so D132's tie-break input
(`last_update_ordinal`) and this decision's receipt ordinal are provably the same sequence, and an
attacker-less commit (if one is ever wired) still advances it correctly. Landing order with PR
#1215 is not fixed by this decision:

- if #1215 lands first, this decision's implementation child changes `DamageContributors::record`'s
  signature to accept the ordinal (mechanical) and removes its private `next_ordinal` field;
- if this decision lands first, #1215's rebase consumes `DamageReceipts::next_ordinal` instead of
  adding its own.

### 4.4 D143 — Scope: carrier multi-occurrence only

This decision is **runtime-carrier multi-occurrence retention only** — how many independently
committed occurrences one creature's slot may hold across its whole generation, regardless of which
ability, cast or attacker produced each one. It is explicitly **not** GAME-ABILITY-01 §7.1's
"explicit deterministic ordered sub-occurrences/commit groups" for multi-hit **within one ability**
(one cast producing several ordered hits against one or more targets under one commit group,
possibly with prior commits influencing later results). §7.1 already fully owns that mechanic at
the ability/commit-group layer, textually distinct from the carrier's per-creature receipt storage
this decision widens. Each ability-level sub-occurrence, whether from a single-hit cast or a future
ordered multi-hit commit group, already supplies its own distinct `occurrence` identity to
`commit_creature_damage_inner`; this decision does not need to, and does not, invent occurrence
identity scoping, sub-occurrence ordering or commit-group failure semantics — it only lets the
carrier accept and retain more than one such identity per creature generation instead of exactly
one. No GAME-ABILITY-01 ordered-sub-occurrence machinery is required, built or assumed here.

### 4.5 D144 — The lethal receipt stays exactly one

`committed_lethal_receipt_inner`/`validate_lethal_receipt` change their lookup from "read the one
`committed` field" to "find the one retained record, among up to 16, whose `result.applied &&
result.health_before > 0 && result.health_after == 0`" — a linear scan of a bounded (<=16), already
in-memory `Vec`. **There can never be more than one such record**, structurally, not by convention:
health only ever decreases and clamps at 0 (`next = health.checked_sub(damage)...max(0)`, unchanged);
the instant one record drives health to 0, every subsequent *new* distinct occurrence is refused by
D141's `CreatureNotActionable` branch before it can mutate health again, so no second record can
ever be created with `health_before > 0 && health_after == 0`. VSL-COMBAT-01 §7's "one creature
lifecycle generation produces at most one logical death occurrence" is therefore preserved
structurally by D141 alone; D144 only relocates where the one lethal record is found.

**External contract byte-for-byte unchanged.** `CreatureDeathOccurrenceRef` (`actor`,
`commit_binding`, `damage`, `health_before`, `death_key()`), `committed_lethal_receipt_inner`'s
`CommittedLethalReceipt`/`RuntimeCorpseProjection` output, D2b's `reward_occurrence_inner` and the
D3 corpse decision's corpse MINT (`settle_creature_death_rewards`, `top_damage_character_inner`)
all consume the same fields with the same meaning; none of them are touched by this decision,
because the found record is copied into exactly the same shapes they already read.

## 5. Delivery

One implementation child is sufficient; this is a single-file semantic change plus its direct
tests, no migration, no protocol, no cross-domain wiring.

| Child | Scope |
|---|---|
| `#162` allocation, D4 (working label `OTV2-<date>-d4-multi-hit-damage-receipt`) | Owned paths: `apps/game-server/src/foundation/runtime_actor_carrier.rs` (widen `committed`, `commit_creature_damage_inner` — new `attacker_sequence: Option<u64>` parameter, high-water-mark check, eviction — `committed_lethal_receipt_inner`, `validate_lethal_receipt`, remove `OccurrenceConflict`, add `StaleAttackerSequence`/`DamageReceiptCapacityExceeded`, add `DamageContributor::high_water_sequence`, update the `size_of::<Slot>()` guard); `apps/game-server/src/foundation/channel_owner_ability_commit_tests.rs`; `apps/game-server/src/foundation/channel_owner_combat_death_tests.rs`; `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` (register `COMBAT01-DAMAGE-RECEIPTS-PER-CREATURE-GENERATION`, max and max+1 tests); reconciliation with PR #1215's `DamageContributors`/`DamageContributor` shape per D142/D141, in whichever direction landing order requires. |

Required tests (this child, white-box against the carrier unless noted):

- **Multi-hit kill, single attacker, past the old bound.** One attacker with a monotonic
  `attacker_sequence` lands more than 16 distinct occurrences against a creature whose HP survives
  all but the last: every occurrence applies, none is ever refused by capacity, and the final one
  drives health to 0 and becomes the one lethal receipt found by D144 — the exact scenario the
  control plane flagged, proven fixed.
- **Replay of each occurrence, retained and evicted.** Replaying a still-retained occurrence returns
  the original `applied: false` result byte-identical. Replaying an occurrence whose receipt has
  since been evicted (same attacker, an old `attacker_sequence`) returns `StaleAttackerSequence` and
  mutates nothing — proving eviction never double-applies.
- **Conflict.** A replay of an already-committed occurrence with a different `binding`/`damage`
  still returns `PlanConflict` (unchanged). A genuinely new, distinct occurrence after the creature
  is already dead returns `CreatureNotActionable` (replacing the two `OccurrenceConflict` sites
  named in §4.2).
- **The bound at max and max+1, tracked vs. untracked.** 16 distinct *tracked* (attacker with a
  supplied `attacker_sequence`) occurrences retain; a 17th tracked, non-lethal occurrence evicts the
  oldest and still applies (never `DamageReceiptCapacityExceeded`). Separately, 16 distinct
  *untracked* occurrences (no `attacker_sequence`, or from the 17th-plus distinct attacker past
  `COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX`) retain; a 17th untracked, non-lethal occurrence
  returns `DamageReceiptCapacityExceeded`, proving the residual limitation is exactly as narrow as
  §4.2 states.
- **The ordinal feeding D3-3's accumulator end-to-end.** Two distinct attackers land hits in a
  known interleaved order against one creature; the ordinal D142 assigns at each new commit is
  asserted strictly increasing and gap-free across *both* attackers' commits (not just one
  attacker's), and the resulting `DamageContributors::top_damage_character()` tie-break (equal
  totals, resolved by D132's rule 1) is proven to use exactly that shared ordinal — wherever
  `DamageContributors`/`damage_contributors_tests.rs` lives at implementation time (main, if this
  child lands first, or the reconciled state, if PR #1215 has already merged).

## 6. Rejected options

- **Plain FIFO eviction with no ordering signal** (the originally rejected option, and the original
  D141's own reasoning for rejecting it): evicting the oldest receipt with nothing to prove a later
  replay of it is safe to refuse risks double-applying that replay's damage — never acceptable.
  Superseded, not simply rejected: the revised D141 (§4.2) evicts, but only ever a receipt whose
  attacker carries a tracked, monotonic `high_water_sequence` that already makes its own stale
  replay provably refusable — the two options differ exactly in that guarantee.
- **Hard fail-closed reject at the bound, no eviction at all** (the original D141). Rejected by the
  control plane: it makes a live creature permanently unkillable by a *new* occurrence identity once
  16 are retained, reachable on the ordinary playable path (many small or 0-damage-adjacent hits
  against one creature), not only boss-scale content.
- **Unbounded `Vec<OwnerCommitRecord>`.** Rejected: GAME-ABILITY-01 §13 requires an explicit hard
  maximum for every content/externally-controlled allocation dimension before executable
  acceptance; an attacker can otherwise grow one creature's receipt list without bound.
- **An unbounded or much larger high-water-mark map, independent of `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE`.**
  Would close the residual >16-untracked-attacker edge case (§4.2) entirely, but adds a second,
  differently-sized bounded dimension for a scenario the rat/playable path cannot reach; reusing
  D132's already-accepted 16-attacker ceiling is the minimum-sufficient choice, revisited if
  boss/raid content needs it.
- **A durable per-hit receipt table.** Rejected for the same reason D132 already rejected a durable
  per-hit attribution ledger: disproportionate to an ephemeral, generation-scoped feature; VSL-
  COMBAT-01 nowhere requires damage receipts to survive a generation.

## 7. Decision test

- **Must decide now:** YES. Both the playable rat-kill path (any creature whose HP survives one
  hit, including many small/chip hits from one attacker) and D3-3/PR #1215's own production wiring
  (`commit_damage_for_attacker`) are blocked without this; a hard reject at the bound reintroduces
  the same class of bug with a higher threshold, not a fix.
- **Minimum sufficient:** reuse the already-accepted 16 for both the retained-receipt cap and the
  high-water-mark map (no new resource row for the latter) rather than measuring new numbers; reuse
  GAME-ABILITY-01 §12's fail-closed policy for the one residual, narrow bound case rather than
  inventing a new failure mode; change one file's semantic shape plus its direct tests.
- **Superseding evidence:** proof that a `GameSessionId`'s `CommandId` stays monotonic across a
  reconnect (closes the open question in §4.2 outright); measured shipped content needing more than
  16 simultaneously untracked contributors (closes the residual limitation).
- **Deliberately not decided:** how the ability bridge derives `attacker_sequence` once it is
  composed into live gameplay (its own later decision; not blocking, since no production caller
  exists today, §3); raising `COMBAT01-DAMAGE-RECEIPTS-PER-CREATURE-GENERATION` or
  `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` above 16; any change to GAME-ABILITY-01 §7.1 ordered
  sub-occurrences/commit groups.
- **Open question (not blocking):** `CommandId` monotonicity across reconnect (§4.2) — the
  implementation child either proves it or sources `attacker_sequence` from something already proven
  durable across reconnect; owner/coordinator input welcome but not required to accept this decision.

## 8. Handback

```yaml
result: RESOLVED_WITHOUT_NEW_OWNER_DECISION
source_escalation: "#162 slice D4 (multiple committed damage occurrences per creature generation)"
owner_decisions: []   # reuses already-accepted D77/D132 value and already-mandatory GAME-ABILITY-01 §12 policy
architecture_decisions: [D140, D141, D142, D143, D144]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_D4_MULTI_HIT_DAMAGE_RECEIPT_DECISION_2026-09-29.md
resource_values_changed: true   # COMBAT01-DAMAGE-RECEIPTS-PER-CREATURE-GENERATION, registered by the implementation child
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true
required_fresh_allocation: true
required_independent_review: "exact-head independent review (bounded eviction, high-water-mark safety, ordinal, lethal-receipt lookup, PR #1215 reconciliation)"
implementation_lanes: [Combat-D4]
required_revalidation:
  - "multi-hit kill: one attacker lands more than 16 distinct occurrences against a creature that survives all but the last; none is ever refused by capacity; the last is the one lethal receipt"
  - "replay: a still-retained occurrence replays byte-identical; a replay of an evicted occurrence returns StaleAttackerSequence and mutates nothing"
  - "conflict: PlanConflict unchanged; CreatureNotActionable replaces OccurrenceConflict for a new occurrence against a dead creature"
  - "bound: tracked occurrences past 16 always evict-and-admit, never DamageReceiptCapacityExceeded; 16 simultaneously untracked occurrences plus a 17th untracked one does return it"
  - "ordinal: strictly increasing and gap-free across attackers; D132's tie-break proven to consume the same sequence"
remaining_unknowns:
  - measured hits-to-kill for non-fixture/boss content
  - CommandId monotonicity across reconnect (open question, §4.2)
  - PR #1215 landing order relative to this decision's implementation child
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates the D4 implementation child."
```

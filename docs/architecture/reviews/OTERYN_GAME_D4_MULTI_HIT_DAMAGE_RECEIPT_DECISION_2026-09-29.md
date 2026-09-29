# D4 multi-hit damage receipt decision

- Decision: `D4-MULTI-HIT-DAMAGE-RECEIPT-V1`
- Status: **CANDIDATE, no new owner decision (§2)**. Acceptance requires exact-head validation,
  independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Answers: #162 slice D4 ("multiple committed damage occurrences per creature generation")
- Amends: no accepted contract text; this decision applies already-accepted policy
  (VSL-COMBAT-01 §7, §13; GAME-ABILITY-01 §7.1, §12, §13; FND-04B §16, §21) to a carrier gap those
  contracts already govern but the implementation has not yet closed
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
- Revision note (2026-09-29, control-plane rejection, round 2): the control plane closed the round-1
  open question in this decision itself rather than leaving it deferred: the high-water mark is now
  keyed `(GameSessionId, sequence)`, not `sequence` alone. A command from a different, *current*
  session replaces the mark outright (a reconnected session's low starting sequence is never treated
  as stale); ordering stays sequence-monotonic only within one session. This is grounded, not
  assumed: `FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md` §16 already keeps `CommandId`
  continuous across a fast, in-grace reconnect (same `GameSessionId`, no reset), and §21 already
  makes a post-grace `GameSessionId` terminal and non-revivable, with only a genuinely new
  `GameSessionId` able to resume control — so a lower sequence under a differing session id is never
  a smuggled-through replay of the old one, and the already-wired connection-generation fence
  (`ConnectionFence`/`StaleConnectionGeneration`, §3) keeps a superseded session's commands from
  reaching the carrier at all. D140, D142, D143 and D144 remain unchanged.
- Revision note (2026-09-29, Codex review, PR #1218 head `df69c9c`, round 3): two P1s, both valid,
  fixed. **(a)** The replay check keyed "Match found" on the caller-supplied, opaque
  `AbilityOccurrenceId` bytes alone, independent of session. A current session could present an old
  occurrence id whose receipt had already been evicted, paired with a fresh sequence of its own, and
  have it applied again — a real double-apply, because the two identity systems (opaque bytes for
  exact replay, `(session, sequence)` for stale-vs-new) disagreed once eviction removed the only
  record of the opaque match. Fixed: an attributed commit's occurrence identity is now *derived by
  the carrier* from the command itself, `(CharacterId, GameSessionId, sequence)`, never accepted as a
  free caller-supplied id; a caller cannot present one attacker/session/sequence's identity under
  another's bytes, because the bytes no longer exist independently of that triple. **(b)** Eviction
  eligibility was inferred from "is this attacker currently tracked at all," not from whether *this
  specific receipt's own* `(session, sequence)` is still covered by the attacker's current
  high-water mark — so a receipt from a since-superseded session could be evicted only because the
  same `CharacterId` is tracked again under a newer session, discarding the one record that could
  have caught a genuine resubmission of the old session's command as stale. Fixed: each retained
  receipt now carries immutable origin metadata and is evictable only when that metadata is present
  and matches the attacker's *current* high-water session with a covered sequence; an unsequenced
  receipt is never evictable. Both fixes are in §4.2; D140, D142, D143 and D144 remain unchanged.
- Revision note (2026-09-29, control-plane correction, round 4): round 3's orphan rule was backwards.
  A receipt whose origin session has been *superseded* is the **safest** to evict, not the least
  safe: that session is terminal and non-revivable (FND-04B §21), and the session-generation fence
  (§3) refuses its commands before they ever reach the carrier, so no command from it can ever arrive
  to replay that receipt. Never evicting orphaned receipts let up to 16 reconnect-orphaned receipts
  from one attacker permanently occupy the pool and block new hits — reintroducing the unkillable-
  creature bug round 1 fixed, in a new guise. Corrected rule (§4.2 point 4): a sequenced receipt is
  evictable iff its attacker is currently tracked and either its origin session differs from the
  attacker's current session (superseded, hence fenced, hence safe) or it is covered by the current
  session's high-water mark (the ordinary same-session case); an unsequenced receipt remains never
  evictable. The reachability statement and the round-3 regression test are corrected to match (§5).
  D140, D142, D143 and D144 remain unchanged.
- Revision note (2026-09-29, Codex review, PR #1218 head `74490e1`, round 5, final): one more P1.
  One command can produce several ordered damage sub-occurrences against the same creature
  (GAME-ABILITY-01 §7.1 multi-hit; `SubOccurrenceRef`, `ability/plan.rs`), and the round-4 triple
  `(CharacterId, GameSessionId, sequence)` cannot distinguish them — a second sub-occurrence of the
  same command collapses onto the first's identity and is rejected `PlanConflict` (differing
  binding/damage) instead of committing as its own hit, silently losing damage the ability already
  computed. Fixed: the replay key widens to `(CharacterId, GameSessionId, sequence, sub_ordinal)`.
  `sub_ordinal` is a carrier-validated `u16`, starting at 0, bounded by the already-registered
  `ABILITY01-EFFECT-PLAN-ENTRIES` = 2 (`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`,
  `ability/mod.rs:33`, `MAX_EFFECT_PLAN_ENTRIES`) — that row already bounds exactly how many typed
  entries (and so how many `SubOccurrenceRef::ordinal` values, `ability/plan.rs:225-230`) one Effect
  Plan may have; no new row is registered here. The high-water mark and per-record origin metadata
  now carry `(sequence, sub_ordinal)` compared lexicographically within one session. §4.2 point 3/4,
  the reachability restatement and the tests are updated to match (§5). D140, D142, D143 and D144
  remain unchanged.
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

- **The session-generation character fence already exists and is production-wired**, unlike the
  ability bridge: `ConnectionFence` (`foundation/mod.rs:793-829`, `current: ConnectionGeneration`,
  `accepts()`, `rebind()`) is consumed by `durability/{mod,schema,db}.rs`, `foundation/admission.rs`/
  `admission_recovery_inner.rs` and `lib.rs`; `gameplay_transport/connection.rs` returns
  `FoundationProtocolError::StaleConnectionGeneration` at multiple live command-handling sites
  (lines 512, 653, 1523, 1663). The test `reconnect_advances_generation_and_fences_stale_transport`
  (`foundation/mod.rs:1390`) proves the exact shape this decision relies on: after `rebind`, the
  fence accepts only the new generation and `!fence.accepts(first)` for the old one — a superseded
  session's commands are rejected before any downstream consumer, including the carrier, ever sees
  them. This is the concrete evidence behind AGENTS.md's durable invariant "Character writes remain
  session-generation fenced."
- **`FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md` fixes exactly how `GameSessionId`/`CommandId`
  behave across a reconnect, resolving the round-1 open question directly, not by inference:**
  §16 "Same-GameSession continuity" — a fast, in-grace reconnect **preserves `GameSessionId`** and
  **"preserves FND-02 CommandId order, server_sequence and typed domain revisions"**; `CommandId`
  never resets while `GameSessionId` is unchanged. §21 "Grace expiry and post-grace existing-actor
  recovery" — once grace expires, "GameSessionId cannot revive; reconnect proof cannot resurrect it;
  old prepared candidates cannot commit"; the only route back to control is reauthenticated recovery
  that "creates new canonical GameSessionId" with `connection_generation = 1`. A lower `CommandId`
  for the same attacker can therefore only ever legitimately appear under a *different*
  `GameSessionId` — never under the same one — and the old `GameSessionId` is contractually incapable
  of producing another command afterward.

- **Sibling check (Codex round 3 instruction): another opaque, caller-chosen replay identity
  exists, out of this decision's scope.** `ability/commit.rs`'s separate fixture `AbilityEngine`
  (`committed: BTreeMap<AbilityOccurrenceId, CommitRecord>`, distinct from `commit_exact_owner_damage`
  and never on its path) keys its own idempotent-commit map purely on `AbilityOccurrenceId` equality,
  with no session-awareness at all. It has no live gameplay caller either (§ above) and never reaches
  the carrier or creature HP, so it cannot double-apply damage — but it is the same class of gap this
  decision closes at the carrier boundary, and whoever composes it into live gameplay must give it
  the same `(CharacterId, GameSessionId, sequence)` treatment before it is session-aware. Flagged,
  not fixed here (D143: carrier-scope only; the ability engine's own identity model is its own,
  later decision).

- **One command can carry more than one sub-occurrence, and a registered bound on how many already
  exists.** `EffectPlan.effects: Vec<Effect>` (`ability/plan.rs`) is capped at `MAX_EFFECT_PLAN_ENTRIES
  = 2` (`ability/mod.rs:33`), enforced in `EffectPlan::new` (`ability/plan.rs:148`,
  `TooManyEffectPlanEntries`) and registered as `ABILITY01-EFFECT-PLAN-ENTRIES` = 2, hard maximum,
  `configurable_range` 1-2 (`RESOURCE_LIMITS_REGISTRY.json:950-965`, owner
  `GAME-ABILITY-01_WHOLE_GATE_OWNER_ACCEPTANCE_BASELINE.md`). `EffectPlan::sub_occurrence(ordinal)`
  (`ability/plan.rs:225-230`) derives one `SubOccurrenceRef` per effect index, so this already-
  registered row is exactly the structural ceiling on how many distinct `sub_ordinal` values one
  committed command can ever produce — GAME-ABILITY-01's own "multi-hit snapshot policy" (which
  effects, in what order, may reference prior results) is separately "Deliberately not decided"
  (`GAME-ABILITY-01_EFFECT_COMPOSITION_DAMAGE_HEAL_OWNER_BASELINE.md` line 88), but the carrier needs
  only the count, not that policy, to size its own replay key.

**UNKNOWN:** measured hits-to-kill for shipped non-fixture content above the rat; whether any
planned creature needs more than 16 distinct committed occurrences in one generation.

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
commits. This decision therefore does not assume ordering is already available; it adds a new,
explicit, optional input the caller must supply to get eviction-safe treatment, and defines safe
behaviour when it is absent. Round 2 (header revision note) closes the reconnect question this
finding originally left open, rather than deferring it.

**3. The identity: derived from the command, never a free caller-supplied id — and wide enough for
every sub-occurrence one command can carry (round 3, P1 fix a; widened round 5, P1 fix).** For an
attributed commit, the occurrence identity a receipt is stored and matched under is not the caller's
opaque `AbilityOccurrenceId` bytes at all — it is the **carrier's own canonical encoding of
`(CharacterId, GameSessionId, sequence, sub_ordinal)`**, the command's own identity, one component
per distinct sub-occurrence a single command can produce (§3: `SubOccurrenceRef`, GAME-ABILITY-01
§7.1 multi-hit). `commit_creature_damage_inner` gains new parameters alongside D3-3's existing
`attacker: Option<CharacterId>`: `attacker_session: Option<GameSessionId>`, `attacker_sequence:
Option<u64>` and `attacker_sub_ordinal: Option<u16>` (all four `Some` together or all `None`).
`attacker_sub_ordinal` is carrier-validated: `0 <= attacker_sub_ordinal <
ABILITY01_EFFECT_PLAN_ENTRIES_MAX` (= 2, mirroring the already-registered
`ABILITY01-EFFECT-PLAN-ENTRIES` row, §3 — not a new bound); a value at or past it is refused before
any other check (a new `CarrierError::SubOrdinalOutOfRange`). When present, the carrier computes the
occurrence identity itself from the full quadruple and the caller's `occurrence` bytes are not read
as identity for this commit — only `binding` (the ability-specific plan/content payload, unchanged
in shape) is still caller-supplied, and it must be prefixed by the carrier-derived identity exactly
as `binding.starts_with(occurrence)` already requires today, just with `occurrence` now
carrier-computed rather than caller-asserted. This closes two things at once: a caller can no longer
make a session-B command carry session-A's occurrence identity (round 3), and two distinct
sub-occurrences of the *same* `(character, session, sequence)` command — a real, GAME-ABILITY-01
§7.1-accepted shape, e.g. an ordered two-hit ability — no longer collapse onto one identity and
conflict with each other; each `sub_ordinal` commits and replays as its own receipt. **Only an
unattributed commit** (`attacker`/`attacker_session`/`attacker_sequence`/`attacker_sub_ordinal` all
`None` — no command ref available, e.g. AI/environment-sourced) still uses the caller's opaque
`occurrence` bytes as its identity, exactly as today; such a receipt is "unsequenced" and, per the
next point, never evictable.

**4. The mechanism: bounded eviction, guarded by immutable per-receipt origin metadata (now
including `sub_ordinal`, round 5) and the attacker's current session-scoped high-water mark (round
3, P1 fix b; corrected round 4; widened round 5, see header revision note).** Each retained
`OwnerCommitRecord` gains an immutable field set once, at creation, never mutated: `origin:
Option<(CharacterId, GameSessionId, u64, u16)>` — `Some((attacker, attacker_session,
attacker_sequence, attacker_sub_ordinal))` for an attributed commit (point 3, above), `None` for an
unsequenced one. D132/D3-3's bounded 16-entry `DamageContributor` map (already keyed by
`CharacterId`, `COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX`) gains one field per entry,
`high_water: Option<(GameSessionId, u64, u16)>`, reusing the same bounded structure rather than
adding a second one. Every `(sequence, sub_ordinal)` pair below is compared **lexicographically**
within one session (sequence first, sub_ordinal as the tiebreak — matching the ordering
GAME-ABILITY-01 §7.1 already requires for an ordered commit group's own sub-occurrences: within one
`sequence`, `sub_ordinal` 0 precedes 1). The lookup widens:

- **Match found** (a retained record's occurrence identity — carrier-derived for an attributed
  commit, point 3, now including `sub_ordinal` — equals the incoming one): unchanged
  idempotent-replay/`PlanConflict` logic, checked per-record instead of against a single field. Two
  sub-occurrences of the same `(character, session, sequence)` with different `sub_ordinal` are two
  distinct records here, never one.
- **No match, health == 0:** unchanged `CreatureNotActionable`.
- **No match, health > 0, attacker tracked with a recorded `high_water = Some((session, seq,
  sub))`, and the incoming `attacker_session == session` and `(attacker_sequence,
  attacker_sub_ordinal) <= (seq, sub)` lexicographically:** refuse with a new
  `CarrierError::StaleAttackerSequence`, never mutating HP or the receipt list. This is provably a
  replay, not a new hit: `(sequence, sub_ordinal)` pairs within one session are promised
  non-decreasing lexicographically, so anything at or below the recorded mark for *that same
  session* was already resolved once, whether or not its own receipt is still retained.
- **No match, health > 0, attacker tracked with `high_water = Some((session, _, _))` and the
  incoming `attacker_session != session`:** **not stale — treated as the next branch.** A different
  `GameSessionId` for the same attacker only ever occurs after the old one is over: FND-04B §16
  (§3) keeps `CommandId` continuous for as long as `GameSessionId` is unchanged, so a differing
  session id is never an in-grace reconnect replaying the same commands under the same identity; it
  is FND-04B §21's post-grace recovery, which mints a genuinely new, previously-unseen
  `GameSessionId` while the old one "cannot revive" and its "old prepared candidates cannot commit."
  A reconnected session's `(sequence, sub_ordinal)` starting low is therefore never mistaken for
  stale.
- **No match, health > 0, otherwise (a genuinely new occurrence — including the reconnect case and
  a second sub-occurrence of the same command above)**: apply the damage; if `attacker_session`/
  `attacker_sequence`/`attacker_sub_ordinal` are `Some` and the attacker is tracked (or has room to
  be, in D132's bounded contributor map, above), **set** (not monotonically raise — a differing
  session always replaces) that attacker's `high_water` to `(attacker_session, attacker_sequence,
  attacker_sub_ordinal)`; append a new record, storing its own `origin` (point 3) and the next
  ordinal (D142). If the retained set is already at 16, **evict the oldest evictable record first**
  to make room, rather than refusing the new occurrence — where **a record with `origin =
  Some((character, session, sequence, sub_ordinal))` is evictable if and only if** the attacker's
  *current* `high_water` is `Some((session', seq', sub'))` for that same `character`, **and
  either**:
  - **its origin session is not the attacker's current session** (`session' != session`) — the
    record's own session has been *superseded*, and a superseded `GameSessionId` is fenced (FND-04B
    §21, `ConnectionFence`/`StaleConnectionGeneration`, §3): no command from it can ever reach the
    carrier again, so a stale replay of this exact record is not merely refusable — it is
    structurally impossible. This makes an orphaned record the **safest** to evict, not the least
    safe (corrected round 4; the round-3 text had this backwards); or
  - **it is covered by the current session's high-water mark** (`session' == session` and
    `(seq', sub') >= (sequence, sub_ordinal)` lexicographically) — the ordinary same-session,
    same-or-later-sub-occurrence progression case.

  A record with `origin = None` is never evictable, whatever its attacker's later state. In
  practice every record whose attacker is currently tracked at all satisfies one of the two branches
  (a same-session record is always covered, since `high_water`'s `(sequence, sub_ordinal)` for one
  session can only ever equal or lexicographically exceed an already-accepted earlier record from
  that same session; a different-session record is always superseded-hence-safe) — a record fails
  to be evictable only when its attacker has no current `high_water` entry at all (untracked: a
  17th-or-later distinct attacker past `COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX`, or never
  wired with attacker metadata) or the record itself has `origin = None`.
- **No match, health > 0, retained set at 16, and no evictable record exists** (every retained
  receipt has `origin = None`, or an `origin` whose attacker is not currently tracked at all —
  D132's bounded contributor map has no entry for that `CharacterId`, whether because it is a
  17th-or-later distinct attacker past `COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX` = 16 or
  because attribution was never wired for that commit): refuse with
  `CarrierError::DamageReceiptCapacityExceeded`, fail-closed, exactly as GAME-ABILITY-01 §12
  requires — HP and every retained receipt stay untouched. **Restated and confirmed unreachable on
  the rat/playable path:** reaching it needs 16 simultaneously non-evictable receipts, which now
  requires genuinely unsequenced (attacker-less) sources or a 17th-plus distinct attacker —
  reconnect-orphaned receipts no longer count toward this, because they are evictable (above), and
  neither does widening the identity to include `sub_ordinal`: an ability's own sub-occurrence count
  is capped at `ABILITY01_EFFECT_PLAN_ENTRIES_MAX` = 2 (point 3, §3), so one command against the rat
  can add at most 2 receipts, not a new unbounded source of non-evictable state; every ordinary
  player hit against the D116 rat carries a `CommandRef` (point 3) from a tracked attacker, so the
  bound is not reached by any number of hits, sub-occurrences or reconnects.

`CarrierError::OccurrenceConflict` is still removed as unreachable (§4.2 original reasoning
unchanged); its two test sites are rewritten to assert `CreatureNotActionable`, as before.

**Replay window.** Exactly the currently retained receipts (up to 16) replay byte-identically. An
occurrence whose receipt was evicted, if it is ever resubmitted, is handled by the stale-vs-new
check (above), not by an exact-match lookup: a resubmission under the *same, still-current* session
and a sequence the mark already covers is refused `StaleAttackerSequence` (never re-applied); a
resubmission under the record's *original* session, once that session has been superseded, cannot
occur at all — the session-generation fence (§3) refuses it before it reaches the carrier, which is
exactly why an orphaned record is safe to evict in the first place. A resubmission under a genuinely
different, current session is a new player action (§4.2 point 4's "differs" branch), never a replay
of a prior one. No record's eviction ever removes protection a live session could still need: either
the resubmission is structurally impossible (fenced), or it is still caught by the stale check
against the live session's own mark.

**Why eviction is now safe (unlike the originally rejected FIFO option, §6, and unlike the round-2
shape Codex found unsound on PR #1218).** Plain FIFO eviction was rejected because evicting a
receipt with no ordering signal at all could let a later replay of it be mistaken for new. The
round-2 shape ("attacker tracked at all") and the round-3 shape ("same session and covered") both
under- or over-restricted eviction; round 4 ties it to one consistent trust boundary the design
already relies on elsewhere (§4.2 point 4's "differs" branch): the session-generation fence (§3),
already proven and already wired, is what makes *both* directions safe — trusting a differing
session id as legitimately current (admission) and trusting a superseded session id as never
resubmittable (eviction) are the same trust, applied symmetrically, not two different assumptions.
A same-session record is protected by the ordinary stale check for as long as its session lives; an
orphaned record needs no protection at all, because nothing can ever resubmit it.

**Residual, explicitly accepted limitation.** `DamageReceiptCapacityExceeded` remains reachable only
when at least 16 of the retained receipts are simultaneously non-evictable — `origin = None`
(genuinely attacker-less, AI/environment-sourced), or an `origin` whose attacker has no current
`high_water` entry at all (a 17th-or-later distinct attacker past D132's own already-accepted
16-attacker cardinality ceiling, `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE`, whose own overflow
rule already accepts that a 17th+ attacker's damage is applied but not tracked for attribution; this
decision extends the same accepted ceiling to eviction-safety). Reconnects no longer contribute to
this limitation (round 4): a reconnected attacker's pre-reconnect receipts are evictable, not stuck.
Reaching the bound needs 16 simultaneously non-evictable receipts, never merely 16 total hits, any
number of reconnects, or any number of sub-occurrences from one attacker (round 5: a tracked
attacker's multi-hit sub-occurrences are exactly as evictable as its single-hit ones, and are
themselves capped at `ABILITY01_EFFECT_PLAN_ENTRIES_MAX` = 2 per command, §4.2 point 3) — narrower
than, and consistent with, D132's already-accepted boundary, and not reachable by the rat/playable
path (every player attacker supplies a `CharacterId`, §4.2 point 4). Raising this further (e.g. a
larger or unbounded high-water-mark map for untracked sources) is a later decision if content needs
it (§7).

**Reconnect, closed (round 2).** The round-1 open question — whether a `CommandId` sequence stays
monotonic across a reconnect — is closed by FND-04B itself, not inferred: there are only two cases,
and both already have an accepted answer (§3). A **fast, in-grace** reconnect (FND-04B §16) keeps
the same `GameSessionId` and keeps `CommandId` continuous — no reset, so the ordinary same-session
check (above) already covers it correctly, unchanged. A **post-grace** reconnect (FND-04B §21) mints
a new `GameSessionId` precisely because the old one is terminal and "cannot revive" — its `CommandId`
starting low is not a violation of monotonicity, because it is not the same sequence continuing; it
is a different sequence, under a different, previously-unseen identity, that the mark's session key
now distinguishes rather than conflates. Keying by `(GameSessionId, sequence)` therefore needs no new
property of `CommandId` itself and no cross-session comparison at all — it only needs the two
properties FND-04B already establishes: same-session continuity, and old-session non-revival. Until
the ability bridge is actually composed into live gameplay (§3: it is not, today), no production
caller supplies `attacker_session`/`attacker_sequence` at all, so no production behaviour changes
while that composition work happens; this decision only fixes the shape it must arrive in.

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
possibly with prior commits influencing later results). §7.1 already fully owns *that* mechanic —
which effects run, in what order, and what happens on a partial commit-group failure — at the
ability/commit-group layer, textually distinct from the carrier's per-creature receipt storage this
decision widens; none of that is decided, built or assumed here.

**Consistency note (round 5).** This boundary is narrower than the round-1/round-3 text stated: the
carrier is not fully passive about sub-occurrence identity. §4.2 point 3 has the carrier accept a
`sub_ordinal: u16` alongside `attacker`/`attacker_session`/`attacker_sequence` and validate it
against the already-registered `ABILITY01-EFFECT-PLAN-ENTRIES` bound (§3) before deriving the
occurrence identity itself — a caller-supplied `AbilityOccurrenceId` is never trusted as identity for
an attributed commit (round 3), so the carrier cannot stay silent about *how many* sub-occurrences
one command may carry either, or two of them would collide. This is still not §7.1's ordering/
commit-group machinery: the carrier does not decide which `sub_ordinal` runs first, does not
sequence effects, and does not define partial-failure semantics for a commit group — it only needs
one already-accepted count (not a new one, §3) to keep each sub-occurrence's replay identity
distinct. Reusing an existing GAME-ABILITY-01 row for this, rather than inventing a carrier-owned
one, keeps the carrier-vs-ability boundary intact: the *number* comes from GAME-ABILITY-01's own
accepted structure, not from a new carrier opinion about multi-hit.

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

One implementation child is sufficient; this is a carrier-scoped semantic change plus its direct
tests and one required signature change at the ability seam, no migration, no protocol, no
cross-domain authority.

| Child | Scope |
|---|---|
| `#162` allocation, D4 (working label `OTV2-<date>-d4-multi-hit-damage-receipt`) | Owned paths: `apps/game-server/src/foundation/runtime_actor_carrier.rs` (widen `committed`, `commit_creature_damage_inner` — new `attacker_session: Option<GameSessionId>`/`attacker_sequence: Option<u64>`/`attacker_sub_ordinal: Option<u16>` parameters, `ABILITY01_EFFECT_PLAN_ENTRIES_MAX: u16 = 2` mirroring the registered `ABILITY01-EFFECT-PLAN-ENTRIES` row (§3, no new row), carrier-derived occurrence identity for an attributed commit §4.2 point 3, immutable per-record `origin` metadata and origin/session-matched eviction with lexicographic `(sequence, sub_ordinal)` comparison §4.2 point 4 — `committed_lethal_receipt_inner`, `validate_lethal_receipt`, remove `OccurrenceConflict`, add `StaleAttackerSequence`/`DamageReceiptCapacityExceeded`/`SubOrdinalOutOfRange`, add `OwnerCommitRecord::origin: Option<(CharacterId, GameSessionId, u64, u16)>`, add `DamageContributor::high_water: Option<(GameSessionId, u64, u16)>`, update the `size_of::<Slot>()` guard); **`apps/game-server/src/ability/commit.rs`'s `commit_exact_owner_damage` (line 255) — the exact, identified seam: it must gain a `CommandRef` (or the equivalent `(CharacterId, GameSessionId, CommandId)` triple) parameter and pass it, plus the effect's own index as `sub_ordinal`, through as `attacker`/`attacker_session`/`attacker_sequence`/`attacker_sub_ordinal` instead of deriving `occurrence` from `plan.occurrence().id()`; this signature change is a required input of this child, even though no production caller exists yet to supply the `CommandRef` (§3) — that composition remains later, separate work (§7)**; `apps/game-server/src/foundation/channel_owner_ability_commit_tests.rs`; `apps/game-server/src/foundation/channel_owner_combat_death_tests.rs`; `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` (register `COMBAT01-DAMAGE-RECEIPTS-PER-CREATURE-GENERATION`, max and max+1 tests; `ABILITY01-EFFECT-PLAN-ENTRIES` is already registered and not re-registered here); reconciliation with PR #1215's `DamageContributors`/`DamageContributor` shape per D142/D141, in whichever direction landing order requires. |

Required tests (this child, white-box against the carrier unless noted):

- **Multi-hit kill, single attacker, past the old bound.** One attacker with a monotonic
  `attacker_sequence` lands more than 16 distinct occurrences against a creature whose HP survives
  all but the last: every occurrence applies, none is ever refused by capacity, and the final one
  drives health to 0 and becomes the one lethal receipt found by D144 — the exact scenario the
  control plane flagged, proven fixed.
- **Replay of each occurrence, retained and evicted.** Replaying a still-retained occurrence returns
  the original `applied: false` result byte-identical. Replaying an occurrence whose receipt has
  since been evicted (same attacker, same session, an old `attacker_sequence`) returns
  `StaleAttackerSequence` and mutates nothing — proving eviction never double-applies.
- **Reconnect (round 2, §4.2).** One attacker lands a hit under `attacker_session = A`, sequence 5
  (mark becomes `(A, 5)`); resubmitting sequence 5 (or lower) still under `A` is refused
  `StaleAttackerSequence` — the ordinary within-session case, unaffected by the session-keyed
  redesign. The attacker then reconnects: a hit under a *different* `GameSessionId` `B`, sequence 1
  (lower than `A`'s recorded 5), is **accepted**, not `StaleAttackerSequence` — proving a
  reconnected session's low starting sequence is never compared against, or blocked by, the old
  session's mark. This carrier-level test does not, and does not need to, prove that session `A`'s
  own commands stop arriving after the reconnect — that guarantee is the session-generation
  character fence's job (§3), already proven by its own
  `reconnect_advances_generation_and_fences_stale_transport` test, upstream of the carrier.
- **Cross-session opaque-id reuse cannot double-apply (round 3, P1 fix a).** Attacker lands a hit
  under session `A`, sequence 1, with some caller-supplied `occurrence`/`binding` bytes; that
  receipt is later evicted (16 other distinct occurrences push it out). A new commit under session
  `B`, sequence 1, is submitted with a `binding` engineered to reuse `A`'s old literal
  `occurrence`/`binding` bytes as its own claimed identity: it is accepted as session `B`'s own new
  occurrence (its stored identity is the carrier-derived `(character, B, 1)`, not the reused bytes),
  proving the caller's literal bytes cannot make one session's commit alias another's. Separately,
  an exact resubmission of the *same* `(character, session, sequence, sub_ordinal)` still replays
  byte-identical regardless of what `occurrence` bytes the caller passes for it, since only the
  quadruple is read as identity for an attributed commit.
- **Two sub-occurrences of one command commit as distinct receipts (round 5, P1 fix).** One
  attacker's single command, `attacker_session = A`, `attacker_sequence = 1`, submits
  `attacker_sub_ordinal = 0` (e.g. the plan's first `Effect::Damage`) and, separately,
  `attacker_sub_ordinal = 1` (its second): both apply, both retain their own receipt, and neither is
  treated as a replay or a `PlanConflict` of the other — proving the round-4 triple's collapse is
  fixed. Replaying `sub_ordinal = 0` alone returns its own original result unchanged; a third
  `attacker_sub_ordinal = 2` (at `ABILITY01_EFFECT_PLAN_ENTRIES_MAX` = 2) is refused
  `SubOrdinalOutOfRange` before any other check, never silently truncated or accepted.
- **Reconnect-orphaned receipts are the safest to evict, so they are evicted (round 4, corrected).**
  Attacker lands a hit under session `A`, sequence 1 (retained, `origin = Some((attacker, A, 1))`);
  the attacker reconnects under session `B` and lands hits until the retained set is at 16 and a 17th
  `B`-origin occurrence needs room: the `A`-origin receipt (now superseded, hence the *most*
  evictable, not the least) is evicted, never the reverse — proving a reconnect cannot let one
  attacker's stale-session receipts pile up and starve new hits, the exact regression round 3
  introduced. A second assertion proves the fence side of the same guarantee, not the carrier's own
  job to re-verify (§3): a late command still carrying session `A` and any sequence, submitted after
  `B` has taken over, is refused upstream by the session-generation fence
  (`reconnect_advances_generation_and_fences_stale_transport`, §3) before it ever reaches
  `commit_creature_damage_inner` — so even though the carrier itself would (correctly, per §4.2
  point 4's "differs" branch) treat a differing session as a new, current one if it ever arrived, no
  such command from a genuinely superseded session ever does.
- **Conflict.** A replay of an already-committed occurrence with a different `binding`/`damage`
  still returns `PlanConflict` (unchanged). A genuinely new, distinct occurrence after the creature
  is already dead returns `CreatureNotActionable` (replacing the two `OccurrenceConflict` sites
  named in §4.2).
- **The bound at max and max+1, evictable vs. non-evictable.** 16 distinct occurrences from one
  attacker, mixing same-session progression and at least one reconnect (each still evictable by the
  next, per §4.2 point 4's corrected rule) retain; a 17th such occurrence evicts the oldest —
  reconnect-orphaned or not — and still applies (never `DamageReceiptCapacityExceeded`). Separately,
  16 distinct *non-evictable* occurrences (any mix of `origin = None` and 17th-plus distinct
  attackers past `COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX`, with no reconnect involved) retain;
  a 17th non-evictable occurrence returns `DamageReceiptCapacityExceeded`, proving the residual
  limitation is exactly as narrow as §4.2 now states.
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
  Superseded, not simply rejected: the current D141 (§4.2) evicts, but only ever a record whose own
  immutable origin still matches its attacker's *current* high-water session — the two options
  differ exactly in that guarantee.
- **Eviction eligibility inferred from "is this attacker currently tracked at all," with no
  per-record origin metadata to justify it** (the round-2 shape). A Codex P1 on PR #1218 (header
  revision note round 3): with no immutable record of what session/sequence a receipt was actually
  created under, there was nothing to audit *why* a given eviction was safe — round 3 added the
  missing per-record `origin` metadata this decision now keeps.
- **Eviction restricted to same-session-and-covered records only, treating a superseded session's
  records as permanently non-evictable** (the round-3 shape). Rejected by the control plane (header
  revision note round 4): this had the safety direction backwards — a superseded session cannot ever
  resubmit anything (FND-04B §21, the session-generation fence, §3), making its records the *safest*
  to evict, not the least safe. Treating them as permanently protected let up to 16 reconnect-orphaned
  records from one attacker occupy the pool forever, reintroducing an unkillable-creature bug. The
  corrected rule (§4.2 point 4) keeps round 3's per-record `origin` metadata but evicts a superseded
  record precisely because it is superseded, not in spite of it.
- **The caller's opaque `AbilityOccurrenceId` as the authoritative replay identity for an attributed
  commit** (the round-1/round-2 shape). A second Codex P1: nothing tied that opaque identity to the
  session/sequence the stale check reasons about, so a current session could present an evicted
  receipt's old occurrence bytes under its own fresh sequence and have it applied again. The round-3
  fix makes the carrier derive the occurrence identity itself from `(CharacterId, GameSessionId,
  sequence)`, so the two identity systems can no longer disagree — they are the same system.
- **Replay identity as `(CharacterId, GameSessionId, sequence)`, with no `sub_ordinal`** (the round-4
  shape). A third Codex P1 on PR #1218 (header revision note round 5): one command can produce more
  than one ordered damage sub-occurrence against the same creature (GAME-ABILITY-01 §7.1 multi-hit),
  and without `sub_ordinal` they collapse onto one identity, so a second sub-occurrence of the same
  command is rejected `PlanConflict` instead of committing — silently losing damage the ability
  already computed. The round-5 fix widens the identity, the high-water mark and each record's
  origin to the quadruple, comparing `(sequence, sub_ordinal)` lexicographically.
- **A new, carrier-owned resource row for the sub-occurrence bound**, instead of reusing
  `ABILITY01-EFFECT-PLAN-ENTRIES`. Rejected: that row already bounds exactly the same structural
  dimension (`EffectPlan.effects.len()`, hence `SubOccurrenceRef::ordinal`'s range, §3); a second row
  for the same count would duplicate, and could silently drift from, an already-accepted ceiling.
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
- **A high-water mark keyed by `sequence` alone, ignoring `GameSessionId`** (round 1's shape).
  Rejected by the control plane: a reconnected session's `CommandId` restarting at 1 would compare
  as `<=` almost any pre-reconnect mark and be refused `StaleAttackerSequence` forever after a
  reconnect — the exact bug this round closes, and worse than the original bound bug because no
  further hits from that attacker could ever land again in the generation.
- **Comparing sequences across sessions directly, hoping `CommandId` stays globally monotonic for
  one attacker.** It does not, and FND-04B never claims it does (§3): a post-grace reconnect mints
  a new `GameSessionId` and a new `CommandId` sequence by design (§21). Keying by
  `(GameSessionId, sequence)` and relying on FND-04B §16/§21's already-accepted continuity/
  non-revival guarantees is the correct reading of the existing contract, not a cheaper
  approximation of a harder proof.

## 7. Decision test

- **Must decide now:** YES. Both the playable rat-kill path (any creature whose HP survives one
  hit, including many small/chip hits from one attacker) and D3-3/PR #1215's own production wiring
  (`commit_damage_for_attacker`) are blocked without this; a hard reject at the bound reintroduces
  the same class of bug with a higher threshold, not a fix.
- **Minimum sufficient:** reuse the already-accepted 16 for both the retained-receipt cap and the
  high-water-mark map, and the already-accepted `ABILITY01-EFFECT-PLAN-ENTRIES` = 2 for the
  sub-occurrence bound (no new resource row for any of the three) rather than measuring new numbers;
  reuse GAME-ABILITY-01 §12's fail-closed policy for the one residual, narrow bound case rather than
  inventing a new failure mode; derive occurrence identity from data the carrier already validates
  (`(CharacterId, GameSessionId, sequence, sub_ordinal)`) rather than adding a second identity
  system; store one small immutable field per receipt rather than a separate tracking structure.
- **Superseding evidence:** measured shipped content needing more than 16 simultaneously
  non-evictable receipts (closes the residual limitation, §4.2), or more than 2 ordered damage
  sub-occurrences per command (would need a new, separately-decided GAME-ABILITY-01 row before this
  decision's `sub_ordinal` bound could widen); evidence that the session-generation character fence
  (§3) does not in fact cover every path that could reach `commit_creature_damage_inner` with an
  attacker-attributed command (would require reinstating an open question round 2 closed); a fourth
  identity- or eviction-safety gap found by a future review (round 3-5 fixed the three Codex found on
  PR #1218; the sibling in `ability/commit.rs`'s fixture engine is flagged, not fixed, §3).
- **Deliberately not decided:** how the ability bridge derives the `CommandRef` it must pass to
  `commit_exact_owner_damage` (§5) once it is composed into live gameplay (its own later decision;
  not blocking, since no production caller exists today, §3); the sibling `AbilityEngine` fixture's
  own identity model (§3); raising `COMBAT01-DAMAGE-RECEIPTS-PER-CREATURE-GENERATION` or
  `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` above 16; any change to GAME-ABILITY-01 §7.1 ordered
  sub-occurrences/commit groups.

## 8. Handback

```yaml
result: RESOLVED_WITHOUT_NEW_OWNER_DECISION
source_escalation: "#162 slice D4 (multiple committed damage occurrences per creature generation)"
owner_decisions: []   # reuses already-accepted D77/D132 value and already-mandatory GAME-ABILITY-01 §12 policy
architecture_decisions: [D140, D141, D142, D143, D144]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_D4_MULTI_HIT_DAMAGE_RECEIPT_DECISION_2026-09-29.md
resource_values_changed: true   # COMBAT01-DAMAGE-RECEIPTS-PER-CREATURE-GENERATION, registered by the implementation child; ABILITY01-EFFECT-PLAN-ENTRIES reused, not changed
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true
required_fresh_allocation: true
required_independent_review: "exact-head independent review (carrier-derived occurrence identity including sub_ordinal, per-receipt origin metadata and origin/session-matched eviction, lexicographic (sequence, sub_ordinal) high-water-mark safety, reliance on the session-generation fence, ordinal, lethal-receipt lookup, ability/commit.rs seam signature, PR #1215 reconciliation)"
implementation_lanes: [Combat-D4]
required_revalidation:
  - "multi-hit kill: one attacker lands more than 16 distinct occurrences against a creature that survives all but the last; none is ever refused by capacity; the last is the one lethal receipt"
  - "replay: a still-retained occurrence replays byte-identical; a replay of an evicted occurrence (same origin session) returns StaleAttackerSequence and mutates nothing"
  - "reconnect: a stale sequence resubmitted within the same session is refused; a lower sequence under a new GameSessionId for the same attacker is accepted, never StaleAttackerSequence"
  - "cross-session identity: a session's commit cannot be made to carry another session's occurrence identity via caller-supplied bytes (round 3, P1 a); exact same-origin resubmission still replays regardless of caller-supplied bytes"
  - "reconnect-orphaned receipts: a receipt whose origin session no longer matches its attacker's current high-water session IS evicted before a same-session-covered one is needed (round 4, corrected); a late command from that superseded session is refused by the session-generation fence before it reaches the carrier"
  - "sub-occurrences: two sub_ordinal values of the same (character, session, sequence) commit as distinct receipts, neither a replay nor a PlanConflict of the other; a sub_ordinal at or past ABILITY01_EFFECT_PLAN_ENTRIES_MAX is refused SubOrdinalOutOfRange (round 5)"
  - "conflict: PlanConflict unchanged; CreatureNotActionable replaces OccurrenceConflict for a new occurrence against a dead creature"
  - "bound: evictable occurrences past 16 always evict-and-admit, never DamageReceiptCapacityExceeded; 16 simultaneously non-evictable occurrences plus a 17th does return it"
  - "ordinal: strictly increasing and gap-free across attackers; D132's tie-break proven to consume the same sequence"
remaining_unknowns:
  - measured hits-to-kill for non-fixture/boss content
  - PR #1215 landing order relative to this decision's implementation child
  - the sibling AbilityEngine fixture's own opaque-identity model (§3), not fixed here
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates the D4 implementation child."
```

# Oteryn Evolved PvE Post-Window Reclaim — Owner Baseline

- Status: **OWNER_ACCEPTED FIRST-GENERATION DEFAULT**
- DecisionStatus: `ACCEPTED_WITH_PLAYABLE_EVIDENCE_REVISIT`
- Date: 2026-09-09
- Coordination issue: `#220`
- Source type: `USER_SOURCE`
- Protected source base: `main@9afb7cbb538674408bc7d2eaaaaa1e8917b04640`
- Scope: ordinary Oteryn Evolved PvE corpse-bound item custody after the protected full recovery window
- Runtime/client/server/protocol/DDL/migration/Platform/Atlas/production authority: **NONE**

## 1. Owner decision

For the first playable Oteryn Evolved PvE implementation direction, corpse-bound items that remain unrecovered when the protected full recovery window ends must not be silently destroyed and must not remain as an indefinitely authoritative spatial corpse.

The accepted first-generation custody flow is:

```text
eligible corpse-bound item
  -> DEATH_CORPSE_CUSTODY
  -> direct corpse recovery during the protected full-recovery window
  -> if still unrecovered at full-window expiry:
       RECLAIM_SALVAGE_CUSTODY
```

This decision is `EVOLVED` ordinary PvE only.

It does not alter Oteryn Reference and does not decide PvP, theft, public-loot or skull/fair-fight disposition.

## 2. Evidence-revisitable status

This is intentionally an **owner-accepted first-generation default**, not a claim that the final shipped recovery experience is known before the game exists in playable form.

The owner explicitly expects this area to be re-evaluated after real playable-game evidence exists.

```text
REVISIT_AFTER_PLAYABLE_GAME_EVIDENCE = true
```

Relevant future evidence includes at minimum:

- whether returning to the corpse still creates meaningful tension;
- whether reclaim makes ordinary death feel too safe or too punitive;
- player comprehension of corpse versus reclaim state;
- abuse or free-storage incentives;
- repeated-death behavior;
- real item/economy values;
- reclaim congestion/capacity behavior;
- disconnect/outage/failover experience;
- interaction with blessings and DeathExhaustion;
- measured support/UX friction from lost or apparently missing items.

A future change based on such evidence is explicitly allowed and does not represent an architectural failure.

However, until an explicit superseding owner-reviewed decision is protected, runtime code must not silently drift away from this baseline.

## 3. XP and item truth remain separate

Protected Evolved death architecture keeps XP recovery and item custody separate.

At the protected full recovery deadline:

```text
recoverable XP claim may expire according to the active XP policy
```

while independently:

```text
remaining corpse-bound items must retain one authoritative item truth
```

Therefore XP expiry must not imply item destruction.

Likewise, moving an item into reclaim custody must not restore expired XP.

## 4. Authoritative custody transition

The transition from corpse custody to reclaim custody is one logical DUR-03 disposition operation.

It must preserve:

- one authoritative item identity;
- one authoritative location/custody state;
- conservation of item/value truth;
- idempotency under retry;
- revision/generation fencing;
- deterministic reconciliation after ambiguous outcomes.

An item must never be authoritative in both:

```text
DEATH_CORPSE_CUSTODY
```

and:

```text
RECLAIM_SALVAGE_CUSTODY
```

at the same time.

## 5. Failure and ambiguity

If the reclaim/durability authority is unavailable, or the transition result is ambiguous, the system must not guess the final location.

The same logical transition remains pending/reconcilable under its existing identity and fence.

Until the durable transition is proven committed:

- no duplicate reclaim copy may be created;
- no cleanup routine may delete the item as if reclaim succeeded;
- no stale corpse/runtime writer may overwrite a newer disposition;
- retry must reconcile the same operation rather than create another move.

Runtime presentation may fail closed or temporarily hide inaccessible state when necessary, but durable truth must remain singular and recoverable.

## 6. Spatial corpse lifecycle

The accepted product direction is not to preserve an authoritative corpse in world space forever merely because items remain inside it.

Once the durable reclaim transfer is committed:

- the spatial corpse no longer owns those transferred items;
- runtime corpse cleanup/despawn may proceed under the owning runtime/content rules;
- later reclaim access resolves the durable reclaim custody, not the old spatial corpse.

This prevents permanent world clutter and avoids treating runtime corpse lifetime as durable item storage authority.

## 7. Reclaim is bounded, not a free warehouse

`RECLAIM_SALVAGE_CUSTODY` is a recovery mechanism, not generic unlimited storage.

A later implementation contract must define finite resource bounds before production admission, including whichever dimensions are exercised by the chosen design, such as:

- maximum reclaim entries/items;
- maximum aggregate retained item count/value-bearing entries;
- maximum retained age or another terminal disposition trigger;
- bounded pending-transition count;
- bounded per-Character/account/world reclaim state;
- any applicable payload/manifest size limits.

Exact maxima are deliberately not invented by this owner decision.

If required maxima are not yet accepted when implementation reaches that boundary, the implementation must fail closed rather than choose production values locally.

## 8. Fees, inconvenience and economy

This baseline does not make reclaim free or frictionless.

It deliberately leaves open whether reclaim later requires one or more of:

- gold/value fee;
- location/NPC/service interaction;
- cooldown or staged availability;
- limited reclaim slots/capacity;
- escalating repeated-use friction;
- blessing-sensitive reclaim conditions;
- DeathExhaustion-sensitive recovery friction.

Those are economy/gameplay balance decisions and should be selected using real Oteryn economy and playable evidence rather than copied blindly from Global Tibia.

Current blessing prices likewise remain deferred.

## 9. Long-term unclaimed reclaim

This baseline chooses the immediate post-window custody move only.

It does **not** choose the terminal outcome if reclaim custody itself remains unclaimed for a much longer period.

Later valid dispositions may include, after separate acceptance:

- a bounded value-conserving depot route;
- explicit salvage conversion;
- an explicit economy sink;
- another typed terminal custody/disposition state.

Silent disappearance caused only by runtime cleanup, timer overflow or service restart is forbidden.

## 10. Item classification remains separate

This decision applies only to items that the future accepted Evolved death rules classify as corpse-bound for the owning ordinary PvE death.

It does not decide the exact boundary between:

- equipped/secured long-term value;
- `UNSECURED_EXPEDITION_LOOT`;
- consumables;
- containers and nested contents;
- quest/unique/bound items;
- currency or ledger value;
- PvP-lootable value.

That classification remains a separate owner/gameplay decision under GAME-ITEM/DUR-03 authority.

## 11. Blessing relationship

Protected Evolved blessing baselines remain unchanged.

Blessings do not reduce the nominal `15% * LevelXPSpan` death envelope.

A later explicit decision may let blessings improve reclaim conditions or item protection, but this baseline does not select such percentages, fees or exceptions.

No blessing effect may create duplicate item authority or bypass one-location conservation.

## 12. Reference and PvP isolation

This baseline is not authority for:

- Oteryn Reference corpse/item behavior;
- Optional-PvP/Open-PvP/other PvP corpse ownership;
- player theft/public-loot windows;
- skull/fair-fight consequences;
- Twist-of-Fate-like disposition;
- PvP blessing consumption/protection.

Those require their own profile/world-mode decisions.

## 13. Future implementation and test consequences

A later authorized implementation must prove at minimum:

1. unrecovered eligible ordinary-Evolved-PvE corpse-bound items transition to typed reclaim custody after the active full-recovery boundary;
2. XP-claim expiry and item-custody transition are independent;
3. no item is simultaneously authoritative in corpse and reclaim custody;
4. retry/replay/failover cannot duplicate the transition or item;
5. ambiguous transition outcomes reconcile the same operation;
6. stale corpse/runtime state cannot overwrite a newer reclaim disposition;
7. committed reclaim transfer allows old spatial corpse item authority to end safely;
8. missing reclaim authority fails closed without deleting value;
9. reclaim resource usage is bounded by accepted hard maxima before production admission;
10. Reference fixtures prove this Evolved policy cannot activate in Reference;
11. PvP paths prove this ordinary-PvE disposition cannot silently activate in PvP;
12. telemetry/evidence exists to support the planned post-playable re-evaluation.

## 14. Planned playable-evidence review

Once Oteryn Evolved has a physically playable death/recovery loop with representative items and economy, architecture/product review should explicitly re-evaluate this baseline.

The review may retain, tune or supersede:

- the existence of reclaim at all;
- timing relative to corpse recovery;
- reclaim friction;
- fees;
- capacity;
- retention;
- item-class eligibility;
- blessing interaction;
- DeathExhaustion interaction;
- terminal salvage behavior.

Changes require explicit superseding authority and migration/disposition semantics for any already-persisted reclaim state.

## 15. Deliberately deferred

This baseline does not freeze:

- exact corpse-bound item classification;
- reclaim fee;
- reclaim access location/UX;
- reclaim capacity/hard maxima;
- reclaim retention duration;
- long-term terminal salvage policy;
- blessing-based item protection/reclaim effects;
- DeathExhaustion reclaim effects;
- PvP disposition;
- persistence schema;
- protocol/UI representation;
- runtime implementation allocation;
- live deployment authority.

## 16. Acceptance boundary

Accepted now:

```text
profile                                  = Oteryn Evolved
scope                                    = ordinary PvE
post-window corpse-bound item outcome    = RECLAIM_SALVAGE_CUSTODY
silent destruction at XP/window expiry  = forbidden
indefinite authoritative world corpse    = not the first-generation default
XP expiry implies item destruction       = no
custody transition                       = one fenced/idempotent DUR-03 move
reclaim                                  = bounded, exact limits deferred
prices/fees                              = deferred
PvP                                      = deferred
Reference                                = unchanged
playable evidence revisit                = explicitly required/allowed
silent runtime drift before supersession = forbidden
```

`IMPLEMENTATION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`

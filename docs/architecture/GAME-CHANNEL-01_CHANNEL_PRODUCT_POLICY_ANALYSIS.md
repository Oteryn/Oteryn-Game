# GAME-CHANNEL-01 — Channel Product Policy Analysis

- Date: 2026-08-12
- Gate: `GAME-CHANNEL-01`
- Delivery task: `OTV2-20260812-game-channel-01-architecture`
- Delivery PR: #209
- Status: **paper-only candidate analysis; nonbinding until delivery merge + lifecycle closeout**
- ImplementationStatus: **NOT_STARTED**
- Runtime/client authority: **NONE**
- PostgreSQL DDL/migration authority: **NONE**
- Production authority: **NONE**

## 1. Problem

Oteryn already accepts a technical multichannel foundation:

```text
WorldId
-> one economy/community/profile boundary
-> multiple ChannelId public-world simulations
-> one logical mutation owner per ChannelRuntime
```

The missing gate is product policy: player selection, queue/co-location, switch/anti-hopping, source/reward multiplicity, PvP/social consequences, same-Channel recovery and the qualitative conditions under which public Channels may be created, drained, drain-aborted or retired.

Without this gate a technically correct runtime could still create product/economy defects: implicit reward multiplication, private-party Channel creation, farming-driven expansion, PvP escape, session-local cooldown bypass, drain continuing during renewed demand, or retirement while authoritative obligations still exist.

## 2. Binding accepted inputs

### ADR-0001 + FND-ID-01

`PROVEN`:

- World is one product/economy/community boundary;
- Channel is one independent public-world simulation;
- canonical `ChannelRef = WorldId + ChannelId`;
- display labels/ordinals are not identity;
- channel-local position/creatures/combat/ground runtime is separate from world/Character shared durable state;
- completed Channel switch already means safe source exit plus fresh destination admission and fresh GameSessionId;
- combat/trade/item/protected-event/unsafe-instance transitions already block switch;
- retired ChannelId is never reused; same semantic Channel recovery preserves ChannelId.

### ADR-0009 + FND-03

`PROVEN`:

- GameNode/ChannelRuntime lifecycle and one-writer ownership are already accepted;
- dynamic activation/closure is expected but exact algorithms and capacity values are not GAME-CHANNEL-owned;
- failed semantic Channel recovery preserves same ChannelId;
- no silent alternate-Channel failure continuation;
- PERF/OPS evidence owns numeric capacity and orchestration.

### FND-04

`PROVEN`:

- directory/Gateway may advertise one bounded attempt;
- final game-domain admission remains authoritative;
- admission material binds exact WorldId+ChannelId and revisions;
- no silent target retarget;
- GameSessionId is game-issued only after successful admission.

### GAME-VISION-01

`PROVEN`:

- solo viable, party rewarded;
- PvP secondary pillar;
- conservation before tuning;
- no hidden macro economy tuning;
- duplicated group/reward eligibility rejected;
- channel friction and economy effects are measurable product concerns.

### DUR-03

`PROVEN`:

- durable value mutation is transactionally idempotent/conserved;
- stale authority cannot commit value;
- reward/source occurrences can be idempotent;
- Game Intelligence is read-only.

GAME-CHANNEL owns product eligibility/multiplicity/lifecycle predicates, not durable transaction mechanics.

## 3. Historical GAME-CHANNEL evidence reconciled

The current source audit also consumes:

- `GAME-CHANNEL-01_PREDECISION_ANALYSIS.md`;
- `GAME-CHANNEL-01_PREDECISION_CAPACITY_TRIGGERS_ADDENDUM.md`.

These were **pre-decision framework inputs**, not accepted target semantics. The candidate adopts the addendum's qualitative lifecycle predicate vocabulary because the current gate explicitly owns channel create/remove/capacity-trigger product semantics.

It does **not** adopt any historical numeric utilization thresholds, player counts, windows, headroom percentages, timers, hysteresis values or technology assumptions. Those remain PERF/OPS evidence-owned.

## 4. Decision timing

**Must decide now: YES.** The owner-accepted programme ordering requires GAME-CHANNEL before multichannel becomes a product feature.

Blocked work includes directory/selection UX, queues, co-location, voluntary switching, anti-hopping, public Channel lifecycle semantics, multichannel event/reward policy and VSL multichannel product proof.

Changing these late can require migration/rework of switch guard state, queue/Gateway contracts, reward keys, lifecycle/control-plane rules, client UX, support and E2E.

Supersession requires named playtest, economy, PvP abuse, recovery/availability, privacy/security, PERF/OPS or explicit product-owner evidence.

## 5. Player entry options

### Auto-only

Good balancing, weak player agency and co-location. Rejected as sole model.

### Manual-only

Good agency, weak balancing and unnecessary queues. Rejected as sole model.

### Recommendation + explicit eligible override

Recommended.

Control plane returns current eligible Channels plus a recommendation. Player may accept or choose another visible eligible target. Recommendation never replaces FND-04 admission authority.

If an explicit target becomes full/draining/recovering/unavailable/stale/incompatible, the attempt fails closed and requires a fresh offer/grant; same authorization is never retargeted silently.

## 6. Queue and co-location

Recommended queue: optional bounded target-Channel **pre-admission** queue.

Queue is not GameSession, CharacterLease, runtime ownership, durable value reservation or guaranteed admission. Long queue state does not hold a stale short-lived admission grant; final admission revalidates current facts.

First generation does not reserve a destination while Character remains authoritative in the source Channel.

Party/friend co-location is a privacy-bounded recommendation/target hint. Every Character admits independently. No party-owned Channel, all-or-nothing group admission or implicit migration is accepted.

## 7. Switch and anti-hopping

Same-Channel reconnect/recovery is not a switch. Completed switch consumes the accepted ADR-0001 safe-exit/fresh-destination-session flow.

Hard blockers remain: combat/PvP, direct trade, unresolved DUR-03 mutation, protected event/encounter, unsafe instance/house transition, pending authority/checkpoint/handoff, stale lease/session/runtime evidence and ineligible destination.

A GameSession-local cooldown is rejected because relog could reset it.

Candidate semantic state:

```text
scope: CharacterId + WorldId
owner: GAME-CHANNEL / game-domain world channel-policy authority
lifetime: may outlive GameSession/connection/GameNode
revision: world_policy_revision
state includes: last successful ChannelRef or equivalent unambiguous prior placement
```

First admission establishes baseline without counting as switch. Same prior Channel fresh login is not a switch. Different Channel fresh login is a switch even after the previous GameSession ended.

Initial anti-hopping mechanism is time-based cooldown + hard locks. Exact duration remains an evidence/owner decision before implementation activation.

Different-Channel destination playable authority and remembered-Channel/guard advancement must be one recovery-safe semantic outcome; no crash/retry path may expose destination gameplay while the guard update is absent/ambiguous.

No mandatory new ChannelSwitchId is justified yet.

## 8. Source multiplicity problem

Channel-local runtime placement cannot imply per-Channel durable source/reward repeatability.

Every value-producing source/encounter family whose durable output changes with active Channel count must explicitly compile/validate one supported class. Missing classification fails closed.

Supported vocabulary:

- `CHANNEL_LOCAL_REPEATABLE`;
- `CHANNEL_LOCAL_SHARED_ELIGIBILITY`;
- `WORLD_SCOPED_UNIQUE`;
- `EXPLICIT_EVENT_POLICY_REQUIRED`.

Reward-bearing events keep `simulation_scope`, `eligibility_scope`, reset/repeat policy and stable reward/source occurrence semantics distinct. ChannelId cannot silently enter a shared Character/Account/World reward reset key.

No hidden inverse spawn/loot scaling by active Channel count is accepted.

## 9. Dynamic lifecycle ownership problem

The earlier draft delegated activation/deactivation too broadly to OPS/PERF. That leaves a product-policy hole because OPS could not tell whether a Channel is being created for legitimate demand or merely to give one party a private spawn copy.

Correct split:

```text
GAME-CHANNEL -> qualitative product predicates
PERF-01       -> numeric service objectives/capacity/headroom evidence
OPS-CHANNEL   -> windows, hysteresis, timers, orchestration, placement, algorithms
```

This is the key cycle-3 independent-review repair.

## 10. Qualitative lifecycle trigger vocabulary

### `DEMAND_PRESSURE`

Eligible public demand exceeds currently healthy/selectable capacity for a **sustained** evidence window under accepted service-objective/headroom policy.

The numeric meaning of sustained/headroom/service objective is PERF/OPS-owned.

### `RECOVERY_PRESSURE`

World public capacity is unavailable/insufficient because a Channel/hosting capacity failed or cannot safely serve demand.

This never permits actors bound to failed Channel A to continue silently in new Channel B. Same semantic Channel recovery preserves ChannelId when possible.

### `LOW_LOAD_CONSOLIDATION_CANDIDATE`

A Channel has sustained low utilization and may be consolidated only if retained eligible Channels can absorb load with required headroom and no event/recovery/product policy requires it.

### `CHANNEL_UNHEALTHY`

A Channel is not safely routable/authoritative under runtime health/fencing/recovery policy. It maps primarily to Recovering/Unavailable, not automatic migration drain.

## 11. New semantic Channel creation predicate

A new public ChannelId may be created/activated only when all applicable conditions hold:

1. trigger is `DEMAND_PRESSURE`, or `RECOVERY_PRESSURE` requires additional general public capacity beyond safe same-Channel recovery/current healthy capacity;
2. demand-driven creation proves sustained eligible demand and no current eligible healthy Channel can satisfy service objective while keeping required headroom;
3. recovery-driven creation does not reinterpret failed Channel A actors as new Channel B actors;
4. added Channel multiplicity does not violate World/ruleset/content/event/reward policy;
5. safe infrastructure capacity exists under PERF/OPS authority;
6. new Channel reaches accepted Ready/compatible state before becoming Selectable or receiving ordinary admission;
7. topology authority issues canonical ChannelId.

Forbidden sole create triggers:

- one party/friend group wants a private copy;
- a player wants an emptier Channel;
- rare spawn/loot/resource farming;
- PvP avoidance;
- operator preference without demand/recovery/product justification.

These preferences may influence selection among existing Channels but cannot mint new public capacity by themselves.

## 12. Recovery pressure versus semantic Channel identity

Important distinction:

```text
replace hosting/runtime for failed Channel A
-> same ChannelId A when recovery is possible

create new semantic Channel B for additional World capacity
-> new ChannelId B
-> no automatic movement of affected A actors
```

Recovery pressure can justify infrastructure replacement/additional capacity but does not supersede ADR-0009 identity semantics.

## 13. Drain predicate

Low-load consolidation drain requires all applicable conditions:

1. sustained `LOW_LOAD_CONSOLIDATION_CANDIDATE`;
2. retained eligible Channels can absorb expected demand with required headroom/service objective;
3. no World/event/reward/recovery/reserved-capacity policy requires the candidate;
4. new ordinary admissions/switches into it stop;
5. incumbents can reach bounded safe exit/session boundary without session/item/value correctness violation;
6. drain does not become a reward/source reset mechanism.

`CHANNEL_UNHEALTHY` is not equivalent to low-load drain; unhealthy follows recovery/fencing semantics.

## 14. Drain abort/hold predicate

Before terminal retirement, drain aborts or holds if:

- demand pressure returns and retained capacity cannot meet service objective/headroom;
- a retained Channel becomes unhealthy/unavailable;
- recovery pressure needs the candidate capacity;
- World/event/reward/product policy requires the Channel;
- continued drain risks GameSession/CharacterLease, item/value, encounter, checkpoint or recovery correctness.

A reversed drain preserves ChannelId and requires revalidated runtime readiness plus fresh directory/queue/admission evidence before becoming Selectable again.

## 15. Terminal retirement/removal predicate

Semantic Channel retirement requires all applicable conditions:

- no authoritative player session/Character placement remains;
- no active Instance/encounter/event obligation depends on the scope;
- no unresolved item/value transaction, handoff, checkpoint or recovery obligation depends on it;
- required durable checkpoint/audit/evidence finalization completed;
- old owner/generation fenced;
- stale routing/queue/admission references invalidated/removed;
- policy confirms permanent semantic retirement rather than temporary stop/inactivity.

Retired ChannelId is never reused. Temporary stop/reactivation of the same semantic Channel preserves ChannelId.

## 16. Numeric lifecycle decisions deliberately deferred

Not frozen here:

- demand/load windows;
- utilization percentages;
- queue-latency objective;
- headroom percentages;
- players/Channel thresholds;
- min/max Channels;
- drain grace;
- scaling cooldown/hysteresis timing;
- GameNode placement capacity.

These require PERF/OPS evidence but cannot weaken GAME-CHANNEL qualitative predicates.

## 17. PvP/community/recovery consequences

PvP execution remains local; World/Character-scoped consequences survive Channel/GameSession changes. Failure does not move actors to another combat simulation and client disconnect claims do not create switch exceptions.

One World remains one guild/economy/ranking/community boundary; local speech/combat/position remains Channel-local.

Player-facing availability classes are `SELECTABLE`, `CAPACITY_LIMITED`, `DRAINING`, `RECOVERING`, `UNAVAILABLE`.

Failure recovery is same-ChannelId first. Alternate Channel requires proven safe actor state plus fresh selection/admission. Failure never erases combat/reward/value consequences.

## 18. Policy revision and World boundary

GAME-CHANNEL uses existing `world_policy_revision`. Stale offers/grants cannot silently apply superseded Channel policy. Guard/lifecycle state retains enough revision context for deterministic migration/interpretation.

WorldId change is not Channel switch and cannot bypass World-scoped value/profile/Character lifecycle rules.

## 19. Security/resource/analytics review

Future implementation must prevent client-forged eligibility/capacity/queue priority, queue bearer authority, silent retarget, relog guard reset, lost prior Channel, destination authority without guard update, display identity authority, dual Character placement, stale owner durable writes, switch-lock bypass, implicit per-Channel value output, preference/farming/PvP-only creation, unsafe continuing drain, premature retirement, shared reward resets, hidden inverse rate tuning, silent failure failover and privacy bypass.

All attacker/load-influenced directory/queue/switch/lifecycle/multiplicity structures are bounded before implementation acceptance. Architecture deliberately does not guess numbers.

Game Intelligence may measure population, queue/co-location/switch behavior, lifecycle trigger decisions, source/sink composition, reward conflicts and recovery outcomes, but never mutate channel/value authority automatically.

## 20. Recommendation

Accept the companion contract with:

- recommendation + explicit eligible target;
- bounded target pre-admission queue;
- privacy-bounded co-location hints;
- durable Character+World prior-Channel/switch guard;
- recovery-safe destination admission + guard update;
- fail-closed authored source multiplicity classification;
- qualitative `DEMAND_PRESSURE`, `RECOVERY_PRESSURE`, `LOW_LOAD_CONSOLIDATION_CANDIDATE`, `CHANNEL_UNHEALTHY` lifecycle vocabulary;
- strict demand/recovery creation predicate;
- strict low-load drain + abort/hold + terminal retirement predicates;
- PERF-owned numbers and OPS-owned implementation/hysteresis;
- same-Channel failure recovery;
- no runtime/DDL/production authority.

## 21. Deliberately not decided

Numeric cooldowns, queue limits, demand windows, percentages, channel-count/capacity thresholds, scaling/drain hysteresis, service topology, exact client UI, privacy matrix, exact PvP/boss/reward/spawn/loot/economy formulas, physical DB/control-plane schemas, protocol/ANL IDs, production admin exception implementation and monetization/paid priority remain downstream.

## 22. Acceptance consequence

If the companion contract passes exact-head self-review, required independent review, exact-head CI, unchanged merge and separate lifecycle closeout:

```text
GAME-CHANNEL-01
DecisionStatus       = ACCEPTED
DeliveryStatus       = LIFECYCLE_CLOSED
ImplementationStatus = NOT_STARTED
runtime/client authority = NONE
DDL/migration        = NONE
production authority = NONE
```

Architecture acceptance would unblock later multichannel product implementation/VSL planning, not authorize or prove runtime behavior.

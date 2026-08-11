# GAME-CHANNEL-01 — Capacity Trigger Pre-Decision Addendum

- Status: **PRE-DECISION ANALYSIS / NOT ACCEPTED**
- Date: 2026-08-11
- Parent gate: `GAME-CHANNEL-01`
- Parent analysis: `GAME-CHANNEL-01_PREDECISION_ANALYSIS.md`
- Decision owner: product owner
- Purpose: define the missing semantic create/drain/remove trigger conditions while leaving numeric thresholds and orchestration to `PERF-01` / `OPS-CHANNEL-01`
- Does not authorize: autoscaling implementation, orchestrator choice, numeric limits, production scaling or acceptance of recommendations below

## 1. Why this addendum is required

The parent analysis correctly defers numeric capacity thresholds, hysteresis and orchestration technology, but `GAME-CHANNEL-01` still needs to freeze **what kinds of conditions mean capacity should expand, drain or remain stable**. Saying only “in response to measured demand” is too weak for deterministic routing, queue behavior and `VSL-MULTICHANNEL-01` acceptance.

This addendum is therefore part of the `GAME-CHANNEL-01` owner decision dossier. It supplements the parent `channel creation/removal and capacity` section and its owner decision packet without selecting any numeric thresholds.

## 2. Accepted constraints

### PROVEN

- Exact player/channel capacity claims require measured `PERF-01` evidence.
- External orchestration owns process/container creation/replacement; a `GameNode` remains one game-server process under ADR-0009.
- A channel has a stable logical `ChannelId` and one authoritative mutation owner under generation fencing.
- Recovery/replacement of the same logical channel is distinct from adding another public capacity channel.
- Draining/full/unhealthy channels already affect admission/switch eligibility.
- Players must not be silently moved to another healthy live channel merely as a capacity optimization.

## 3. Trigger vocabulary

The future accepted channel policy should distinguish these semantic trigger classes. Exact metric names and numeric boundaries remain later implementation contracts.

### `DEMAND_PRESSURE`

New ordinary admission demand cannot be served within the accepted service objective by the currently eligible healthy public channels **without consuming reserved safety headroom**.

Evidence may include semantically equivalent signals such as:

- sustained admission queue growth or queue-age objective violation;
- all eligible healthy channels reaching the configured open-new-channel operating envelope;
- party/social co-location requests repeatedly failing for capacity when another safe channel could materially reduce pressure.

One transient spike must not by itself force uncontrolled channel creation; numeric persistence windows belong to `PERF-01`/`OPS-CHANNEL-01`.

### `RECOVERY_PRESSURE`

A failure/reconnect event temporarily needs capacity or headroom to recover existing authority safely.

This **must not be treated as ordinary demand growth by default**. Recovery capacity has priority semantics defined by FND-04/OPS policy and may justify suppressing new ordinary admissions rather than creating gameplay copies that change channel identity.

### `LOW_LOAD_CONSOLIDATION_CANDIDATE`

More than one healthy public channel exists and measured load remains low enough that at least one channel could be drained **while all retained channels still satisfy required safety headroom and service objectives**.

Low population alone does not authorize removal if consolidation would break party/social placement, active encounter semantics, recovery commitments or accepted headroom.

### `CHANNEL_UNHEALTHY`

The channel or its owner is unhealthy, fenced or lost.

This triggers recovery/replacement semantics for the **same logical ChannelId** where supported. It is not a product-level signal to move the population into another already-live channel and is not equivalent to `DEMAND_PRESSURE`.

## 4. Recommended create trigger semantics

### RECOMMENDATION — owner decision required

An additional ordinary public channel may be requested only when all of the following semantic conditions are true:

1. `DEMAND_PRESSURE` is sustained according to later measured policy;
2. no existing eligible healthy channel can satisfy the demand within the accepted service objective while preserving required headroom;
3. creating another channel does not violate world/ruleset/content compatibility or event/reward policy;
4. the infrastructure owner reports enough host/GameNode capacity to start/host the channel safely;
5. the new channel can become fully ready before receiving players; partial initialization is not admission authority.

A single party wanting a private copy, a rare-spawn farming request, PvP avoidance or manual player preference is **not** by itself a channel-create trigger.

## 5. Recommended drain trigger semantics

### RECOMMENDATION — owner decision required

A public channel may enter `DRAINING` only when all of the following semantic conditions are true:

1. it is a `LOW_LOAD_CONSOLIDATION_CANDIDATE`;
2. retained healthy channels have enough post-consolidation capacity and safety headroom;
3. no active policy requires the channel to remain available for world event, encounter, recovery or reserved-capacity reasons;
4. the system can stop assigning new ordinary admissions and manual switches to that channel before removal;
5. incumbents can finish, logout, switch when safely eligible, or follow explicit later drain policy without forced cross-channel combat/loot/state migration.

Entering `DRAINING` must be reversible if conditions change.

## 6. Recommended drain-abort semantics

### RECOMMENDATION — owner decision required

A draining channel returns to ordinary eligible service when any accepted condition makes consolidation unsafe, including semantically equivalent cases such as:

- demand pressure returns and retained channels would lose required headroom;
- target/retained channels become unhealthy or unavailable;
- recovery pressure requires the channel capacity;
- a world/event policy requires the channel to remain available;
- the drain cannot finish without violating safe session/encounter/transaction semantics.

Numeric hysteresis and minimum drain duration are deferred, but the **ability to abort drain safely is not**.

## 7. Recommended remove/close trigger semantics

### RECOMMENDATION — owner decision required

A drained logical channel may be removed from public availability only when terminal safety conditions are satisfied:

1. no authoritative player session remains bound to it;
2. no active instance/encounter/transaction/checkpoint/recovery obligation requires that channel authority;
3. required durable/checkpoint/audit/evidence publication boundaries for shutdown are satisfied by their owning contracts;
4. the channel owner/generation is fenced so stale work cannot regain authority after closure;
5. World Registry/directory state can remove or mark the channel unavailable without admitting a client to stale routing evidence.

Exact shutdown ordering belongs to FND-03/OPS contracts, but these semantic predicates are product/system requirements.

## 8. Deterministic routing/E2E consequences

Once accepted, `VSL-MULTICHANNEL-01` can test trigger semantics without hard-coding production numbers by injecting threshold predicates or test capacity states.

Required scenario classes should include:

1. one channel has headroom -> new login stays on existing capacity; no unnecessary channel creation;
2. sustained demand pressure with no eligible headroom -> create/ready a second channel before routing new players there;
3. transient spike below the configured persistence rule -> no oscillating create/remove loop;
4. low-load candidate with safe retained headroom -> mark one channel draining;
5. new demand during drain -> abort drain when consolidation would violate policy;
6. draining channel rejects new ordinary admissions/switches;
7. active unsafe encounter/recovery obligation prevents terminal removal;
8. empty, fully settled, fenced channel -> removal becomes legal;
9. channel owner failure -> recover/replace same `ChannelId`, not silently route incumbents into another active channel;
10. recovery pressure may reserve capacity and reject/defer new ordinary admission rather than changing authoritative channel identity.

## 9. Additional owner decision required for GAME-CHANNEL-01

The parent owner packet's capacity-lifecycle item must be read as requiring confirmation of these semantic triggers:

10. **Capacity lifecycle and trigger semantics:** confirm that:
   - new ordinary channels are created for sustained `DEMAND_PRESSURE` after existing eligible capacity/headroom is insufficient;
   - recovery pressure is distinct and does not silently create another gameplay identity;
   - drain begins only for safe low-load consolidation with retained headroom;
   - drain can abort when demand/health/recovery conditions change;
   - removal requires no remaining authoritative session/encounter/transaction/recovery obligation plus fencing/settlement;
   - exact numeric thresholds, persistence windows, hysteresis and orchestration remain `PERF-01`/`OPS-CHANNEL-01` decisions.

## 10. Evidence for later supersession

Later measured evidence may refine or supersede trigger policy using:

- admission latency/queue age distributions;
- per-channel tick/queue/resource saturation;
- GameNode headroom and failure-domain evidence;
- party co-location failure rate;
- drain duration and abort frequency;
- reconnect/recovery storm behavior;
- channel churn and startup cost;
- player-visible disruption;
- economy/event side effects from channel multiplicity.

No observed metric may autonomously create/remove authoritative capacity unless the later accepted OPS contract explicitly authorizes the bounded control loop and its safety/failure behavior.

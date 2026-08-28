# Disconnect Forensic Evidence Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: disconnect protection evidence, security analytics, Game Intelligence, later `FND-03`, `FND-04`, `ANL-01`, `ANL-03`, persistence and QA/E2E contracts
- Related: ADR-0006, ADR-0009, `LAG_DISCONNECT_PROTECTION_OWNER_BASELINE.md`, `LAG_DISCONNECT_REENTRY_ACTION_POLICY_OWNER_BASELINE.md`, `DISCONNECT_LIVENESS_AND_CRASH_EVIDENCE_OWNER_BASELINE.md`, `CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md`

## Purpose

Record the owner-accepted server-side forensic evidence model for disconnect protection and disconnect-abuse investigation.

This baseline supersedes the earlier simplified idea that a durable `DisconnectEpisodeSummary` alone could be the primary evidence record. The summary is derived analysis only; canonical server-side transition evidence remains the durable source material.

This is architecture only. It does not authorize runtime, protocol, persistence, database, telemetry backend, Game Intelligence implementation or production collection.

## 1. Four-layer evidence model

The accepted target shape is:

```text
heartbeat / ACK / transport progress
            |
            v
     Live Liveness State
          RAM only
            |
            +--> bounded rolling forensic buffer
            |       ephemeral by default
            |
            v
material disconnect/protection transition
            |
            v
canonical DisconnectEpisode transition evidence
            |
            +--> bounded forensic context snapshot
            +--> gameplay/risk context
            +--> network/liveness context
            `--> GameNode/runtime-health context
            |
            v
Game Intelligence / Security Analytics
            |
            +--> DisconnectEpisodeSummary (derived)
            +--> deterministic detectors
            +--> longitudinal pattern analysis
            +--> infrastructure correlation
            `--> read-only AI-assisted investigation
```

The four layers have different durability and authority. They must not be collapsed into one generic telemetry stream.

## 2. Live liveness state is not a durable heartbeat log

Heartbeat, acknowledgement, transport-progression, RTT, jitter, packet-loss, reorder and related liveness signals are evaluated in the live networking/runtime path.

Binding direction:

- ordinary heartbeat/ACK samples are not individually written as durable PostgreSQL security records;
- liveness evaluation remains cheap, bounded and suitable for every connected player;
- operational aggregates may expose low-cardinality health evidence;
- raw heartbeat processing remains implementation detail of later `FND-03` / `FND-04` contracts;
- gameplay/security evidence begins at semantically meaningful transitions, not at every packet.

This avoids turning disconnect protection into full network event sourcing.

## 3. Bounded rolling forensic buffer

Each active authoritative gameplay session may maintain a small, fixed-capacity rolling forensic buffer containing only allowlisted structured evidence needed to reconstruct the immediate context around a material incident.

The buffer is ephemeral and continuously overwritten when no incident occurs.

Candidate evidence classes include, subject to later exact resource/privacy contracts:

- recent liveness-progression summaries;
- RTT/jitter/loss buckets rather than unrestricted packet capture;
- transport generation and stale-generation observations;
- GameNode/server tick delay or event-loop health summaries;
- combat-state transitions;
- bounded HP/resource deltas;
- recent incoming damage and healing summaries;
- bounded movement/zone transitions rather than full continuous trails;
- committed high-value gameplay actions relevant to the incident;
- encounter/boss mechanic state where needed for correct reconstruction.

The buffer must have explicit limits for time horizon, entries, bytes and event classes in `RESOURCE_LIMITS_REGISTRY.json` or its accepted successor.

Normal operation must not persist the rolling buffer indefinitely.

When a material disconnect incident is created, an allowlisted bounded slice may be retained as forensic context under the incident's security/privacy retention class.

No unrestricted network packet capture, arbitrary client file collection or unlimited movement history is accepted by this baseline.

## 4. Canonical disconnect episode evidence is the durable source material

A material disconnect/protection incident produces canonical, versioned server-side transition evidence correlated under one incident/episode identity concept.

The exact final identifier name and type (for example `DisconnectEpisodeId`) remain owned by the relevant identifier/event contract and are not frozen here.

The event family must be capable of representing transitions conceptually equivalent to:

```text
PROTECTION_ACTIVATED
TRANSPORT_STALE
TRANSPORT_FENCED
RECONNECT_ATTEMPTED
RECONNECT_ACCEPTED
REENTRY_STARTED
REENTRY_ENDED
EPISODE_RESOLVED
```

Exact public event names are not frozen here.

A noisy pre-threshold state such as every transient `LIVENESS_DEGRADED` observation should not automatically become durable security audit. The durable incident begins when a contract-defined material boundary is crossed, including the accepted PvE protection boundary at `elapsed >= 2.0 s`, or another later-defined security-relevant transition requires evidence.

Events must use the common versioned event envelope required by ADR-0006/`ANL-01` where applicable, including stable event identity, time/tick correlation, world/channel/instance/node/session/generation context, build/content/ruleset/protocol revisions, privacy class, retention class and durability class.

## 5. Protection-activation context snapshot

`PROTECTION_ACTIVATED` or its final equivalent must carry a bounded server-side context snapshot sufficient for later reconstruction without requiring the full simulation history.

Candidate required fields include, subject to `ANL-01` / `ANL-03` schema ownership:

- incident/episode correlation identity;
- restricted `CharacterId` and/or pseudonymous `AnalyticsActorId` according to access class;
- `GameSessionId` and authoritative transport generation;
- `WorldId`, `ChannelId`, `InstanceId`, `NodeId`;
- wall-clock timestamp;
- authoritative simulation tick/sequence;
- monotonic elapsed duration since last sufficient liveness evidence;
- HP/max HP and relevant resource state;
- combat/PZ/logout state;
- bounded recent incoming-damage/risk summary;
- bounded recent healing/resource-use summary where materially relevant;
- monster/hostile-pressure summary;
- boss/encounter context;
- `Area` / `Subarea` / `EncounterZone` or other accepted semantic geography;
- client/server/protocol/content/ruleset/build revisions where known;
- liveness classification/reason produced from server-observed evidence;
- GameNode/runtime-health context needed to determine whether server-side degradation contributed to the event.

The context snapshot is evidence of what the authoritative server observed. It is not a client-provided declaration of lag or abuse innocence/guilt.

## 6. Server health context is mandatory forensic evidence

Disconnect analysis must be able to distinguish player-side connectivity loss from Oteryn-side degradation.

For each material incident, the retained server-side context must be able to correlate relevant bounded evidence such as:

- GameNode tick/event-loop latency;
- queue saturation/backpressure state;
- process/resource pressure;
- host/container restart/OOM/signal context when applicable;
- database/shared-service degradation;
- channel-local overload;
- deployment/build regression context;
- correlated disconnects affecting other players on the same node/channel/region.

A player incident must not be classified as a player-network problem solely because liveness stopped progressing while the authoritative server itself was stalled.

Mass or correlated infrastructure incidents must be identifiable so they do not accumulate as independent adverse player evidence.

## 7. Three clocks / ordering domains

Forensic and gameplay correctness must not rely on one wall-clock timestamp alone.

Later contracts must preserve three distinct concepts where applicable:

1. **wall clock** — cross-system and operator correlation;
2. **authoritative simulation tick/sequence** — deterministic gameplay ordering;
3. **monotonic elapsed time** — exact duration boundaries such as `2 s`, `5 s`, `15 s` and `4 s`.

System-clock/NTP adjustments must not cause protection timers or reconnect/re-entry durations to jump backward or forward incorrectly.

Wall-clock timestamps are therefore correlation evidence, not the sole timer authority.

## 8. Reconnect and re-entry evidence

A reconnect/re-entry incident must retain enough server-side evidence to determine what happened after control returned.

Candidate evidence includes:

- outage duration;
- reconnect attempt and admission result;
- old/new transport generation and fencing result;
- authoritative actor continuity;
- re-entry protection start/end;
- HP/resource state at return;
- bounded movement/repositioning outcomes;
- self-heal and health/mana/resource consumable use during re-entry;
- rejected/suspended offensive attempts during the four-second window;
- encounter/boss state at return;
- recovery exit/instance-destruction result when applicable.

Offensive inputs rejected during re-entry remain non-buffered, consistent with the accepted re-entry policy.

## 9. Movement/privacy boundary

Precise continuous player movement history is not a default durable retention requirement.

For ordinary disconnect evidence, prefer bounded semantic evidence such as:

- position/zone at protection activation;
- position/zone at valid reconnect;
- position/zone at re-entry end;
- `Area` / `Subarea` / `EncounterZone` transitions;
- a small allowlisted forensic movement window only when materially needed.

This follows ADR-0006's minimization direction and avoids turning security diagnostics into permanent full-session surveillance.

Finer location evidence requires an accepted security purpose, access policy, retention rule and resource bound.

## 10. `DisconnectEpisodeSummary` is derived, not canonical authority

Game Intelligence may derive a `DisconnectEpisodeSummary` or equivalent analytical projection from canonical transition evidence plus authorized contextual data.

The derived summary may include features such as:

- outage duration;
- HP/risk before protection;
- recent incoming DPS/pressure;
- encounter/boss context;
- use and outcome of re-entry protection;
- healing/resource recovery after reconnect;
- escape/repositioning result;
- repeated disconnects in one encounter;
- repeated low-HP disconnect -> protection -> reconnect -> heal/escape patterns;
- correlation with other players, GameNode, region or infrastructure incidents;
- historical frequency and pattern features.

The summary is **not** the sole source of truth and must not replace canonical transition evidence.

Reasons:

- derivation logic can contain bugs;
- detector definitions will evolve;
- model/rule versions will change;
- old incidents must remain re-analysable under newer read-only detectors;
- a human reviewer must be able to trace an analytical conclusion back to named evidence.

Derived summaries, scores and features must retain their detector/derivation version and evidence references.

## 11. Client crash evidence remains corroborating only

Client crash diagnostics may strengthen diagnosis, for example by correlating a client renderer/device failure with server-observed loss of liveness.

They never replace canonical server-side evidence.

Example correlation:

```text
server: liveness lost / protection activated
client: GPU_DEVICE_REMOVED near the same incident
GameNode: healthy
other players: no correlated outage
```

This may provide stronger evidence of a genuine client crash, but the client report remains untrusted diagnostic input.

If client diagnostics are disabled or unavailable, the server-side incident remains fully visible to Oteryn Security Analytics.

The privacy opt-out itself remains non-adverse evidence under `CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md`.

## 12. Durability classes

The accepted direction is:

```text
individual heartbeat/ACK samples        -> live runtime / operational evidence
rolling forensic ring                   -> bounded ephemeral evidence
material transition events              -> durable security evidence
retained incident forensic slice        -> bounded durable security evidence when triggered
DisconnectEpisodeSummary / features     -> derived analytics projection
security case                           -> durable reviewed case evidence
```

This preserves ADR-0006's separation of operational observability, best-effort telemetry and durable security audit.

The exact persistence/outbox semantics for each disconnect event family remain for `ANL-01`, `DUR-02` and owning runtime/session contracts. This baseline does not require full event sourcing of gameplay.

## 13. Security and anti-abuse use

Disconnect abuse must be investigated primarily through server-generated evidence and longitudinal patterns.

A single disconnect remains insufficient proof of abuse.

The evidence model must support deterministic and statistical investigation of patterns such as:

```text
high combat risk / low HP
    -> authoritative liveness loss
    -> disconnect protection
    -> reconnect / re-entry protection
    -> heal / escape
    -> repeated similar episodes
```

Any score/model output remains investigative. Consistent with ADR-0006, Game Intelligence and AI cannot autonomously ban, mutate gameplay, revoke/grant protection, rollback state or deploy fixes.

Human-review and later enforcement-policy contracts remain required for sanctions.

## 14. Failure behavior

Evidence collection must not introduce an unbounded synchronous remote dependency into the authoritative combat path.

Later contracts must define:

- in-memory ring capacity and overflow behavior;
- event-envelope/payload size limits;
- durable queue/outbox behavior;
- evidence retention classes;
- schema incompatibility handling;
- observable evidence-loss conditions;
- privacy/redaction failure handling;
- replay/deduplication semantics;
- GameNode crash behavior before unpublished evidence leaves the failed process.

Durable security evidence must not silently downgrade to ordinary best-effort telemetry.

At the same time, this baseline does not require every heartbeat or movement step to block gameplay on a remote database write.

## 15. Required future tests

Future contracts and implementation must prove at minimum:

1. ordinary heartbeat/ACK traffic does not create unbounded durable per-packet records;
2. an idle healthy session produces no disconnect incident merely because no gameplay command is sent;
3. the rolling forensic buffer remains hard-bounded and overwrites/evicts according to deterministic policy;
4. a material disconnect freezes only an allowlisted bounded forensic slice;
5. `elapsed >= 2.0 s` uses monotonic/server-authoritative timing rather than mutable wall clock;
6. canonical transition evidence can reconstruct activation, fencing, reconnect and re-entry ordering;
7. `DisconnectEpisodeSummary` can be regenerated from canonical evidence under a newer detector version;
8. GameNode stall/overload can be distinguished from player-side liveness failure where evidence exists;
9. correlated mass incidents can be distinguished from isolated player incidents;
10. precise continuous movement is not retained by default;
11. missing client crash evidence does not make the server-side incident invisible;
12. client diagnostic opt-out does not become adverse evidence;
13. stale transport generations cannot create valid reconnect/re-entry evidence;
14. evidence consumers remain read-only and cannot mutate authoritative gameplay.

## Programme effect

Accepted now:

```text
raw heartbeat/ACK != durable audit stream
live liveness state -> bounded runtime state
rolling forensic buffer -> bounded + ephemeral by default
material disconnect/protection transition -> canonical durable server evidence
server health context -> part of disconnect forensics
wall clock + simulation order + monotonic elapsed -> separate time domains
DisconnectEpisodeSummary -> derived projection, never sole source of truth
client crash report -> corroborating evidence only
security analysis -> server-side and functional without client diagnostics
precise movement history -> not default durable retention
```

No runtime, protocol, persistence, database, telemetry backend, Game Intelligence implementation or production collection is authorized by this baseline.

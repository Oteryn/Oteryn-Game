# OTV2-20260807-disconnect-forensic-evidence-analysis

```yaml
task_id: OTV2-20260807-disconnect-forensic-evidence-analysis
title: Continue Disconnect Protection forensic evidence architecture
mode: ARCHITECTURE_ANALYSIS_ONLY
status: owner_accepted_checkpoint_current
repository: blakinio/Oteryn-v2
base_branch: main
owner: Oteryn project owner
updated_at: 2026-08-07T19:07:00+02:00
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
parent_task: docs/agents/tasks/active/OTV2-20260807-lag-disconnect-protection-analysis.md
canonical_contracts:
  - docs/architecture/LAG_DISCONNECT_PROTECTION_OWNER_BASELINE.md
  - docs/architecture/LAG_DISCONNECT_REENTRY_ACTION_POLICY_OWNER_BASELINE.md
  - docs/architecture/DISCONNECT_LIVENESS_AND_CRASH_EVIDENCE_OWNER_BASELINE.md
  - docs/architecture/CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md
  - docs/architecture/DISCONNECT_FORENSIC_EVIDENCE_OWNER_BASELINE.md
accepted_prs_before_this_package:
  - PR #75
  - PR #76
  - PR #78 / merge 7f3046c1614de750c8a0bca62bce33ccf25f8aa7
implementation_authorized: false
```

## Purpose

Preserve the owner-accepted refinement of disconnect evidence after the developer-level re-evaluation of the earlier simplified `DisconnectEpisodeSummary` idea.

This file is a continuation overlay for the existing Lag / Disconnect Protection architecture discussion. The canonical decision is `docs/architecture/DISCONNECT_FORENSIC_EVIDENCE_OWNER_BASELINE.md`.

No runtime, protocol, persistence, database, client or Game Intelligence implementation is authorized.

## Owner-accepted correction

Do **not** treat a derived `DisconnectEpisodeSummary` as the primary or sole durable evidence source.

Accepted hierarchy:

```text
raw heartbeat / ACK / transport progress
    -> live bounded liveness state

recent forensic context
    -> bounded rolling in-memory ring, ephemeral by default

material disconnect/protection transitions
    -> canonical durable server-side evidence

DisconnectEpisodeSummary / detector features
    -> derived Game Intelligence projection
```

The derived summary must be reproducible from canonical transition evidence and must retain derivation/detector version plus evidence references.

## Accepted forensic ring direction

A small fixed-capacity rolling forensic buffer may retain only allowlisted structured context for a short bounded horizon.

It is not:

- unrestricted packet capture;
- full client logging;
- permanent movement surveillance;
- an unbounded per-player queue.

Normal evidence is overwritten. A material disconnect incident may freeze only an allowlisted bounded slice under the incident's security/privacy retention rules.

Exact time horizon, entry count, byte limit and event classes remain later `ANL-01` / resource-limit work.

## Accepted durable episode evidence direction

A material incident must be correlatable under one disconnect-episode/incident identity concept. Exact final ID name/type remains unresolved.

Canonical transition evidence must be capable of representing semantically equivalent states such as:

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

Exact public names are not accepted yet.

Do not persist every transient liveness degradation as security audit. The normal durable incident boundary begins at a contract-defined material transition, including the accepted `elapsed >= 2.0 s` PvE protection activation.

## Accepted incident context

A material protection incident must preserve a bounded authoritative snapshot sufficient to reconstruct the risk and environment around the event.

The later schema must consider:

- session/transport generation and world/channel/instance/node identity;
- wall clock, simulation tick/order and monotonic elapsed time;
- HP/resources and combat/PZ/logout state;
- bounded recent incoming damage/risk and recovery context;
- hostile pressure and boss/encounter context;
- semantic geography (`Area`/`Subarea`/`EncounterZone` as applicable);
- build/protocol/content/ruleset revisions;
- server-observed liveness reason/context;
- GameNode/runtime-health context.

## Three time domains

Do not use mutable wall clock as the sole gameplay timer authority.

Later contracts must distinguish:

```text
wall clock              -> cross-system/operator correlation
simulation tick/sequence -> authoritative gameplay ordering
monotonic elapsed time   -> 2 s / 5 s / 15 s / 4 s duration boundaries
```

Clock/NTP adjustment must not distort disconnect protection or reconnect/re-entry durations.

## GameNode/runtime evidence

Disconnect forensics must be able to identify when the server contributed to liveness loss.

Retained incident evidence must correlate bounded GameNode/runtime health such as tick/event-loop latency, queue/backpressure state, resource pressure, host/container termination, shared-service degradation and correlated same-node/channel/region disconnects where applicable.

Do not classify a player incident as a player-network problem merely because client liveness stopped while the server itself was stalled.

## Privacy/minimization

Precise continuous movement history is not a default durable evidence stream.

Prefer protection/reconnect/re-entry positions or semantic zone transitions and only a small bounded forensic movement window when materially necessary.

Client crash diagnostics remain optional corroborating evidence. Server-side disconnect evidence remains available even when the user disables client diagnostics, and the opt-out itself is never adverse evidence.

## Required downstream owners

This checkpoint is mandatory input to:

- `FND-03` runtime/liveness timers and bounded forensic-memory behavior;
- `FND-04` reconnect/admission/transport-generation transitions;
- `ANL-01` event envelope, durability, retention and resource limits;
- `ANL-03` security analytics, episode reconstruction and detectors;
- `DUR-02` where durable transition/outbox persistence requires transaction semantics;
- QA/E2E for liveness, server stall, reconnect and evidence reconstruction scenarios.

## Still unresolved

Do not silently freeze:

- final `DisconnectEpisodeId` / incident identifier naming and ownership;
- exact canonical event names and schemas;
- exact forensic ring duration/entries/bytes;
- exact snapshot fields and numeric aggregation windows;
- exact retention periods;
- exact rules for preserving a forensic slice across a GameNode crash before publication;
- exact detector features/thresholds;
- enforcement and appeal policy;
- PvP disconnect semantics;
- post-15-second held/suspended persistence/admission semantics.

## Guardrails

- ARCHITECTURE / ANALYSIS ONLY until explicit implementation authorization.
- Canonical server evidence is authoritative; client diagnostic claims are corroborating only.
- No full gameplay event sourcing requirement is introduced.
- No durable per-heartbeat/per-packet logging requirement is introduced.
- No permanent full-session movement surveillance is introduced.
- Game Intelligence remains read-only/investigative and cannot autonomously ban or mutate gameplay.
- Continue future owner decisions one concrete unresolved question at a time.

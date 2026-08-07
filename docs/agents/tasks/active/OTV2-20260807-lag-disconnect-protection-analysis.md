# OTV2-20260807-lag-disconnect-protection-analysis

```yaml
task_id: OTV2-20260807-lag-disconnect-protection-analysis
title: Continue Lag / Disconnect Protection architecture analysis
mode: ARCHITECTURE_ANALYSIS_ONLY
status: owner_discussion_checkpoint
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-lag-disconnect-protection-handover
pr: 74
base_sha: af094d4d75d3a19db63714810f263059c78f7b3a
owner: Oteryn project owner
created_at: 2026-08-07T14:40:00+02:00
updated_at: 2026-08-07T14:44:00+02:00
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-lag-disconnect-protection-analysis.md
public_contracts: []
depends_on:
  - PR #72 / merge 97b29e5c927f319ed03fb5583614d5fe0366d134 — healthy combat/PZ/logout-locked incumbent cannot be kicked by duplicate login
  - PR #73 / merge af094d4d75d3a19db63714810f263059c78f7b3a — reconnect continuity, transport-generation fencing, 15-second grace window including combat/PZ/logout lock
blocks:
  - later FND-04 liveness/reconnect/admission state machine details affected by disconnect protection
  - later gameplay protection policy if an emergency controller is accepted
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Purpose

Preserve the exact architecture-discussion boundary so a new agent can resume without relying on chat history.

The owner has moved the discussion from **reconnect identity/session semantics** into a distinct design area: **protection against lag, network stalls and genuine Internet disconnection**.

Do not treat the candidate mechanisms below as accepted architecture unless they are explicitly marked `OWNER-ACCEPTED`.

Do not implement runtime/game code. Continue architecture analysis with the owner, one concrete decision at a time. Accepted decisions may later be recorded through their own bounded documentation-only change.

## Canonical prerequisites already accepted

### Duplicate-login / combat protection — OWNER-ACCEPTED

A second authenticated client must not forcibly disconnect, fence, revoke or steal control from a healthy incumbent gameplay session while its character has an active combat/PZ/logout blocker.

If the incumbent session is genuinely unavailable, same-character recovery may reconnect to the same in-world actor without resetting gameplay state. A different character remains blocked while mandatory world presence exists.

### Reconnect identity and fencing — OWNER-ACCEPTED

A short eligible reconnect does not create a new logical gameplay session.

During the reconnect grace window:

```text
same GameSessionId
+ newer transport/connection generation
```

The newer generation fences old transports and delayed/stale commands.

### Reconnect grace duration — OWNER-ACCEPTED

Initial policy:

```text
reconnect_grace_window = 15 seconds
```

The same 15-second value applies whether or not the character currently has a combat/PZ/logout lock.

The 15 seconds are a tunable server policy, not an immutable protocol constant.

Expiry of reconnect grace does **not** itself clear combat/PZ state, remove a character that still has mandatory world presence, or create a safe-logout exploit.

The exact server-authoritative start/measurement instant is still FND-04 work.

## New architecture area: Lag / Disconnect Protection

The owner explicitly identified the next topic as a **mechanism protecting the player against lag and Internet disconnection**.

This must be designed separately from reconnect grace:

- reconnect grace answers whether the old logical `GameSessionId` can be resumed;
- Lag / Disconnect Protection answers how the server detects loss/degradation of player control and what happens to the existing in-world actor while control is degraded or absent.

The two mechanisms interact but must not be conflated.

## Candidate connection-health state model — NOT YET ACCEPTED

A possible server-authoritative state machine discussed so far:

```text
HEALTHY
  -> DEGRADED
  -> UNRESPONSIVE
  -> DISCONNECTED
  -> RECOVERING
  -> HEALTHY
```

Names and exact transitions are only working terminology.

The system should not trust a client statement such as "I have lag" or "I disconnected" as authoritative proof.

Candidate server-side evidence includes:

- heartbeat/liveness observations;
- time since last valid inbound traffic;
- sequencing and acknowledgment progress;
- RTT/jitter/packet-loss trends where observable;
- command-stream progress;
- Gateway observation;
- GameNode observation;
- transport closure as a signal, but not necessarily sufficient proof by itself;
- current session lease/fencing state.

Exact liveness proof remains unresolved.

## Hard anti-abuse / state-preservation requirements discussed

These are strong candidate invariants consistent with already accepted reconnect/combat rules. They should be challenged and then individually owner-accepted before being frozen into a contract.

Lag/disconnect protection must not become an escape or reset primitive. In particular it should not automatically:

- grant invulnerability;
- clear combat/PZ/logout lock;
- teleport or reposition the character to safety;
- heal HP/mana/resources;
- clear conditions, damage-over-time, debuffs, exhaustion or cooldowns;
- reset aggro/threat/combat attribution;
- reset encounter or instance state;
- rollback committed authoritative actions;
- allow stale/delayed command batches to execute after control is recovered;
- create a second copy/actor of the same character;
- admit another character from the same account while the first still has mandatory world presence.

The character should remain the **same authoritative in-world actor** throughout a genuine disconnect/recovery path unless normal gameplay rules later remove it.

## Candidate layered protection architecture — NOT YET ACCEPTED

The discussion identified three conceptually separate layers.

### 1. Network protection

Possible responsibilities:

- transport-generation fencing;
- stale/delayed packet rejection;
- bounded command queues;
- expiry of time-sensitive input;
- reconnect credential validation;
- rapid safe rebinding to the same actor;
- protection against duplicate execution after retransmit/replay.

Much of the fencing direction is already compatible with PR #73, but detailed policies remain unresolved.

### 2. Movement / input protection

Possible responsibilities once the server has sufficient evidence that control is degraded or lost:

- do not continue an old movement/autowalk instruction indefinitely;
- do not replay a backlog of obsolete directional/input commands after a stall;
- define which previously committed movement/action sequences may still finish;
- distinguish committed server actions from stale client intent.

Exact semantics are unresolved.

### 3. Emergency gameplay protection / controller

A possible but **controversial and NOT ACCEPTED** direction is a tightly constrained server-side emergency controller that may perform only limited defensive behavior after genuine loss of player control.

Examples considered only as discussion material:

- defensive healing;
- limited defensive ability use;
- possibly stopping dangerous movement;
- possibly maintaining only previously allowed low-risk behavior.

This carries a major abuse risk:

```text
dangerous situation
-> player deliberately cuts network
-> server emergency controller plays better / saves character
```

Therefore any such controller, if accepted at all, would need strong anti-abuse constraints, deterministic limited capability, server-authoritative disconnect proof and full observability/auditability.

No emergency-controller behavior has been owner-accepted yet.

## Key distinction after 15-second reconnect expiry

If the 15-second reconnect grace expires while the character still has combat/PZ/logout mandatory world presence:

- expiry may end eligibility to preserve the old logical `GameSessionId` according to the eventual FND-04 state machine;
- it must not by itself remove or protect the in-world character;
- the character's continued world behavior is exactly where Lag / Disconnect Protection and combat/logout gameplay policy now need to be designed;
- post-grace same-character recovery/admission semantics are still unresolved.

This is the immediate architecture boundary.

## Questions still unresolved

Do not answer all at once. Work through these with the owner one decision at a time:

1. Should genuine loss of player control trigger only network/input safety, or also a limited server-side emergency gameplay controller?
2. What exact evidence is sufficient to classify `DEGRADED`, `UNRESPONSIVE` and genuinely `DISCONNECTED` without making deliberate cable-pulling exploitable?
3. Should stale movement/autowalk stop immediately, after a bounded timeout, or at the next safe server action boundary?
4. Which already-issued commands remain committed and which client intents expire during lag?
5. If an emergency controller exists, which actions are permitted and prohibited?
6. Should protection behavior differ in PvE, PvP, bosses, instances or high-value activities?
7. What happens after the 15-second reconnect grace expires while combat/PZ mandatory world presence continues?
8. How does post-grace same-character re-entry work without allowing two authorities, reset, teleport or combat escape?
9. Which parts belong in FND-04 session/liveness versus later combat/gameplay architecture?
10. What telemetry/audit events are required to detect deliberate disconnect abuse and tune false positives?

## Current recommendation, not decision

Keep Lag / Disconnect Protection modular:

```text
Connection Health Monitor
        |
        v
Session Liveness / Fencing
        |
        v
Disconnect Protection Policy
        |
        +--> Network/Input Safety
        |
        +--> optional Emergency Character Controller
```

The `Emergency Character Controller` should remain optional in the architecture until the owner explicitly decides whether such gameplay automation is desirable at all.

## Guardrails for the next agent

- ARCHITECTURE / ANALYSIS ONLY. Do not implement runtime code.
- Do not mix client and server ownership. Detection/authority/protection state is server-side; the client may provide observations but cannot authoritatively grant itself protection.
- Do not reopen accepted PR #72/#73 semantics unless a concrete contradiction is discovered.
- Do not silently make candidate items above canonical decisions.
- Surface abuse cases and race conditions before recommending a mechanism.
- Prefer fail-safe behavior that never creates dual authority or combat escape.
- Ask one high-leverage product/architecture question at a time.
- Save newly owner-accepted decisions through bounded documentation-only architecture changes.

## Context checkpoint

```yaml
status: owner_discussion_checkpoint
branch: docs/OTV2-20260807-lag-disconnect-protection-handover
head_sha: 2a0394a86d403e3fac34ffb30a2967a1556f2253
pr: 74
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-lag-disconnect-protection-analysis.md
public_contracts: []
last_progress: Owner redirected the discussion from reconnect grace into a distinct Lag / Disconnect Protection mechanism and requested that all current considerations be saved for a new agent.
validation_state: PR #74 created from main@af094d4d75d3a19db63714810f263059c78f7b3a; this metadata refresh advances the branch after the recorded content head, so the next agent must resolve the live PR head before mutation
audit_state: candidate concepts explicitly separated from owner-accepted prerequisites
e2e_state: not applicable; no implementation authorized
ci_generation: PR #74
run_ids: []
counters:
  waits: 0
  retries: 0
blocker: none
next_action: Ask the owner whether genuine disconnect protection should be limited to network/input safety or may include a tightly constrained emergency gameplay controller.
```

## Resume instruction for the next agent

Read this entire task, then verify live `main`, PR #74/head/merge state, and the accepted prerequisite documents from PR #72 and PR #73.

Do **not** restart the identity/reconnect discussion.

Resume exactly at the first unresolved product decision:

> Should a genuinely disconnected character receive only network/input safety, or may the server run a tightly constrained emergency defensive controller for that existing in-world actor?

# Disconnect Client/OS Forensics Privacy and Decision-Timing Refinement

- Status: Architecture consistency reconciliation
- Date: 2026-08-08
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Refines: `DISCONNECT_CLIENT_OS_FORENSICS_OWNER_DIRECTION.md`
- Preserves: `CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md`, `DISCONNECT_FORENSIC_EVIDENCE_OWNER_BASELINE.md`, ADR-0006 and `ARCHITECTURE_DECISION_DISCIPLINE.md`
- Applies to: later native-client diagnostics, Launcher/Guardian evaluation, disconnect incident capsules, Game Intelligence, `FND-03`, `FND-04`, `ANL-01`, `ANL-03` and QA/E2E
- Does not authorize: client/runtime implementation, a mandatory Launcher/Guardian process, a Guardian heartbeat, production telemetry, automatic sanctions, kernel drivers or invasive anti-cheat

## Purpose

Resolve two review ambiguities in the disconnect client/OS forensic direction without changing the accepted four-second re-entry gameplay rule or creating a new telemetry entitlement.

This document is a consistency refinement. It binds the new forensic direction to privacy and decision-timing rules that were already accepted on `main`.

## 1. Existing client-diagnostics privacy control remains binding

`CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md` remains authoritative for automatic client-originated diagnostic upload.

For disconnect forensics, **client-originated diagnostic evidence** includes a bounded incident capsule assembled from any combination of:

- native game-client observations;
- allowlisted OS/network/process evidence collected by the client;
- post-boot client-side evidence;
- Launcher/Guardian observations when such a component exists;
- a companion component submitting the same client-side incident capsule on behalf of the game client.

The implementation location does not change the privacy class. Moving collection or transmission into a launcher, guardian or helper process does not turn client-originated diagnostics into server-side authoritative evidence and does not create a new permission to upload it.

## 2. Global diagnostics opt-out cannot be bypassed

When the accepted global client-diagnostics setting is disabled:

- no automatic client-originated disconnect diagnostic capsule may be uploaded;
- no Launcher/Guardian/helper may automatically upload such a capsule merely because it runs in a different process;
- queued automatic retries for those client-originated capsules remain disabled while the setting remains disabled;
- post-reconnect or post-boot automatic submission of those capsules remains disabled;
- the user's choice is not silently reset by restart, update, relogin or process separation;
- server-side gameplay, liveness, runtime, audit and security evidence remains unaffected.

Whether opted-out evidence may remain locally for manual export/troubleshooting is still governed by the existing privacy baseline and later retention design. This refinement does not decide it.

## 3. Missing client evidence is not adverse evidence

The following remain binding:

- diagnostics opt-out is not itself suspicious behavior;
- absence of a client/OS/Launcher/Guardian capsule is not itself adverse evidence;
- enabling diagnostics does not create a presumption of innocence;
- disabling diagnostics does not increase an abuse/risk score or enforcement priority;
- Game Intelligence may represent evidence availability, but availability itself is not behavioral guilt evidence;
- server-generated evidence must remain sufficient for incident visibility and abuse investigation when optional client diagnostics are absent.

This prevents privacy choice from becoming an anti-cheat scoring feature.

## 4. Server authority remains independent from optional client diagnostics

The server remains authoritative for:

- gameplay state;
- liveness and transport-generation evidence;
- protection activation and expiry;
- reconnect/re-entry eligibility;
- authoritative combat/risk context;
- GameNode/runtime-health correlation;
- durable server-side incident evidence required by the owning contracts.

Optional client diagnostics may improve classification confidence but cannot be required synchronously for the player to receive mechanically valid protection.

A player whose client diagnostics are disabled or unavailable is still subject to the same authoritative gameplay/session rules and the same server-side security/audit evidence as any other player.

## 5. Launcher/Guardian wording is an extension point, not a mandatory first implementation

The phrase in `DISCONNECT_CLIENT_OS_FORENSICS_OWNER_DIRECTION.md` that a lightweight independent Launcher/Guardian is an accepted architectural direction must be interpreted narrowly:

> the architecture should preserve the ability to add an independent user-space diagnostic observer later without redesigning server authority, privacy or incident correlation.

It does **not** currently require:

- a separate executable or service;
- a permanently running background process;
- any specific privilege level;
- startup persistence;
- a separate network transport;
- a Guardian-to-server connection;
- a heartbeat;
- continuous OS monitoring;
- a production anti-cheat agent.

The first client/runtime implementation may therefore omit a separate Launcher/Guardian process entirely while still complying with the accepted architecture.

## 6. Guardian heartbeat remains a separately gated candidate

A direct Guardian diagnostic heartbeat is not authorized by PR #96.

Before such a mechanism may become part of the production architecture, its owning contract must define at minimum:

- exact security/diagnostic purpose;
- whether the signal is optional client-originated diagnostics or separately justified required service/security telemetry;
- privacy classification and user-facing disclosure;
- lawful/operational justification appropriate to deployment jurisdiction;
- data fields and explicit exclusions;
- cadence, bandwidth and resource limits;
- authentication, replay and spoofing limits;
- retention and access controls;
- failure behavior and evidence-loss semantics;
- relation to the global diagnostics opt-out;
- confirmation that it cannot become gameplay authority or synchronously stall the authoritative combat/runtime path;
- measurements showing that the additional process/network complexity provides material diagnostic value.

A future contract may not silently classify the heartbeat as required telemetry solely to bypass the existing diagnostics opt-out. If it proposes a different privacy/control model, that difference must be explicit, separately reviewed and justified rather than inferred from this forensic direction.

## 7. Mandatory architecture decision-timing test

### Must decide now — YES

The following boundaries are required now because `FND-03`/`FND-04` consume disconnect/reconnect semantics:

- server evidence remains authoritative;
- client/OS/Launcher/Guardian evidence is corroborating only;
- protection eligibility does not synchronously depend on client telemetry;
- client-originated diagnostics remain subject to existing privacy controls;
- a missing diagnostic record is not adverse evidence;
- any client forensic buffer/capsule is bounded and purpose-limited;
- no automatic sanction follows from one event or one analytical score.

### Must decide now — NO

The following remain deliberately deferred because they do not block the next safe runtime contract:

- whether a separate Launcher/Guardian process exists at all;
- process topology and lifecycle;
- privileges;
- OS API/provider/event selections;
- heartbeat existence, cadence and transport;
- local storage/retention details beyond existing privacy constraints;
- exact telemetry backend and ingestion path;
- detector thresholds and sanction policy.

### Concrete downstream work blocked

`FND-03` and `FND-04` need the authority, timing and asynchronous-evidence boundaries so runtime/session state machines do not depend on untrusted client telemetry. They do not require a concrete Guardian implementation.

Later native-client diagnostics and `ANL-*` contracts require the privacy and evidence-classification boundary before they can select implementation details safely.

### What becomes harder if decided incorrectly now

Making a client helper process mandatory too early would create avoidable packaging, startup, privacy, support and cross-platform coupling.

Making client evidence synchronous or authoritative would create a security and availability dependency in the reconnect/combat path and would be expensive to remove later.

### Evidence that may justify supersession

A later decision may change the deferred implementation direction only with named evidence such as:

- measured inability of in-process diagnostics to distinguish materially important incident classes;
- demonstrated diagnostic improvement from an independent observer;
- quantified bandwidth/CPU/memory/privacy cost;
- security findings showing a proposed mechanism is unsafe or ineffective;
- platform-specific feasibility evidence;
- changed product, legal or operational requirements.

## 8. Required future proof

Later implementation must prove, as applicable, that:

1. disabling client diagnostics blocks automatic upload of disconnect capsules regardless of which client-side process assembled them;
2. helper/Launcher/Guardian process separation cannot bypass that control;
3. missing or opted-out client evidence is represented as unavailable evidence, not adverse behavior;
4. server-side incident evidence remains complete enough for authoritative protection and investigation without optional client diagnostics;
5. any accepted direct Guardian heartbeat has its own explicit privacy/resource/security contract and measured justification;
6. no client diagnostic mechanism can synchronously block the authoritative runtime or become gameplay authority;
7. no separate Launcher/Guardian process is required merely to satisfy the current foundation architecture.

## Programme effect

The PR #96 forensic direction is therefore interpreted as:

```text
server-side evidence -> authoritative and independent of client diagnostics
client/OS/Launcher/Guardian incident evidence -> optional corroborating diagnostics
existing global client diagnostics opt-out -> applies to automatic client-originated incident-capsule upload
opt-out or missing diagnostics -> not adverse evidence
independent observer capability -> preserved extension point
mandatory separate Launcher/Guardian process -> not decided / not required now
direct Guardian heartbeat -> separately gated candidate only
FND-03/FND-04 -> consume authority/timing boundaries, not a concrete client diagnostic topology
```

No runtime, protocol, persistence, database, client diagnostics, Launcher/Guardian, telemetry backend, Game Intelligence detector, enforcement or production implementation is authorized by this refinement.

# Disconnect Liveness and Crash Evidence Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: disconnect protection, client/GameNode diagnostics, later `FND-02`, `FND-03`, `FND-04`, operations, QA/E2E and Game Intelligence contracts
- Related: `LAG_DISCONNECT_PROTECTION_OWNER_BASELINE.md`, `LAG_DISCONNECT_REENTRY_ACTION_POLICY_OWNER_BASELINE.md`, `CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md`, ADR-0006, ADR-0009

## Purpose

Record the owner-accepted direction for deciding when a gameplay connection has lost usable control/liveness and for collecting and analysing crash evidence after client, GameNode or infrastructure failures.

This is architecture only. It does not authorize runtime, protocol, client, persistence, Platform, telemetry backend or production implementation.

## 1. Disconnect protection uses server-authoritative control liveness

The owner accepts that Disconnect Protection is driven by a dedicated **server-authoritative control/liveness signal**, not by the absence of gameplay commands.

The server must evaluate whether the authenticated gameplay transport is still making sufficient forward progress. Conceptually this may use authenticated heartbeat/acknowledgement progression, transport progress and other server-observed evidence accepted by the later protocol/admission contracts.

The following are binding invariants:

- a player standing still, reading chat, looking at UI or otherwise sending no gameplay action must not be classified as disconnected merely because no movement/combat command was received;
- absence of gameplay commands is not sufficient liveness-loss evidence;
- a TCP/socket state that still appears open is not by itself sufficient evidence that the player retains usable gameplay control;
- a client assertion such as `I am lagging`, `I disconnected` or `activate protection` is never authoritative;
- only server-observed evidence may start, continue, cancel or recover the disconnect-protection timer;
- the accepted PvE protection boundary remains `elapsed >= 2.0 s` from the last sufficient server-authoritative control/liveness evidence;
- recovery of valid liveness must be fenced by the currently authoritative transport/connection generation so delayed packets from an older transport cannot regain control.

The exact heartbeat cadence, message representation, acknowledgement scheme, piggybacking strategy, packet-loss/jitter tolerance, grace/hysteresis rules and transition names remain later `FND-02` / `FND-03` / `FND-04` work.

## 2. Liveness detection must tolerate ordinary network variation

The final contract must distinguish loss of usable control from ordinary latency, jitter and isolated packet loss.

It must therefore define bounded policy for:

- heartbeat/ack progression rather than one missing packet;
- packet loss and reordering;
- transient latency spikes;
- duplicate acknowledgements;
- stale acknowledgements from an older transport generation;
- server event-loop or GameNode overload so local server stalls are not misclassified as player disconnects;
- correlated infrastructure incidents.

The liveness mechanism must remain cheap enough to operate for every connected player and must not introduce an unbounded timer, queue or per-client allocation path.

## 3. Crash evidence is diagnostic evidence, not the real-time protection authority

A crash report or diagnostic log may strengthen later classification of what happened, but Disconnect Protection must not wait for a crash report.

Reasons:

- a crashed client cannot upload a report while it is dead;
- a power outage may prevent any local report from being transmitted;
- a network outage may leave the client healthy but unable to communicate;
- a malicious client could forge or suppress client-side evidence.

Therefore:

```text
real-time protection decision
    <- server-authoritative liveness evidence

post-event diagnosis and correlation
    <- crash/log/telemetry evidence
```

Missing client crash evidence must never by itself revoke otherwise valid disconnect protection.

## 4. Client crash evidence

When the native client crashes, hangs fatally or is terminated by an internal fatal condition, the client-side diagnostics layer should preserve a bounded local crash package when technically possible.

On a later successful client start, an approved diagnostics uploader **automatically submits** eligible pending crash packages to the Oteryn diagnostics/Game Intelligence ingestion boundary when client diagnostics upload is enabled. Per-crash interactive confirmation is not required.

The owner also accepts a global privacy opt-out for client-originated diagnostics. The detailed privacy behavior is defined by `CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md`.

Automatic submission is owner-accepted only under the following safety boundary:

- local redaction/filtering happens before upload;
- the upload schema is allowlisted and versioned rather than accepting arbitrary files or arbitrary log directories;
- report, stack, log-ring and attachment sizes are hard-bounded before network transmission;
- secrets, reusable credentials, tokens, private chat content and unrelated personal data are excluded before upload;
- the client records whether a package was successfully accepted so the same crash does not upload indefinitely;
- transient upload failure may use bounded retry/backoff, but crash upload must never block normal client startup or gameplay admission indefinitely;
- client crash uploads remain untrusted diagnostic evidence and never become gameplay authority;
- disabling client diagnostic upload prevents transmission of client-originated crash packages but does not disable required server-side operational or authoritative evidence.

The package may contain, subject to later privacy and resource-limit contracts:

- exact client build/artifact revision;
- operating system and architecture;
- renderer/backend and relevant GPU/driver/device-loss information;
- crash/panic/exception signature and bounded stack information;
- timestamps and monotonic-time correlation data where available;
- last known protocol/content/ruleset revisions;
- bounded connection/session correlation references required to associate the crash with server evidence;
- bounded structured diagnostic events immediately preceding the crash;
- integrity/hash metadata for the submitted evidence.

The client must not upload arbitrary user files or entire unrestricted log directories. Secrets, reusable credentials, tokens, private chat content and unrelated personal data must not enter crash packages.

Client-submitted crash evidence is **untrusted diagnostic input**. It must be schema-validated, size/depth bounded, redacted and treated as evidence rather than authoritative truth.

## 5. GameNode/server crash evidence

A GameNode that has already crashed cannot reliably perform its own post-crash analysis. Crash collection must therefore survive outside the failed process.

The production architecture must allow an external supervisor, orchestrator or crash/log collector to retain bounded evidence such as:

- `NodeId` / process-incarnation context;
- exact server build and deployment revision;
- panic/fatal-error signature and bounded stack/core/minidump metadata where supported;
- structured logs immediately preceding failure;
- runtime queue/latency/resource-health observations;
- active world/channel assignments and ownership generation context required for recovery analysis;
- database, dependency and publication health immediately preceding the failure;
- operating-system/container termination reason such as OOM, signal or host loss where available.

Crash evidence must not exist only on an ephemeral container filesystem that disappears with the failed GameNode.

The authoritative recovery/fencing path defined by ADR-0009 remains independent from whether diagnostic analysis has completed.

## 6. Network and infrastructure evidence

The diagnostic system must also correlate failures that are not application crashes, including:

- ISP or regional network incidents;
- router/path loss visible only from one side;
- GameNode overload or event-loop stall;
- host/container restart;
- database or shared-service degradation;
- deployment or release regression;
- correlated mass disconnects across players/channels/nodes.

This prevents a mass infrastructure incident from being misclassified as many independent suspicious player disconnects.

## 7. Automated analysis belongs outside authoritative simulation

The system should automatically analyse collected crash and disconnect evidence, but expensive analysis must not run synchronously in the authoritative channel mutation path.

The accepted ownership direction is:

```text
Client diagnostics / GameNode supervisor / operational telemetry
        |
        v
bounded diagnostics ingestion
        |
        v
Oteryn Game Intelligence / diagnostics analysis
        |
        +--> deterministic signature and invariant analysis
        +--> regression/build/content correlation
        +--> incident clustering
        +--> read-only AI-assisted investigation
        `--> human/operator evidence package
```

Automated analysis should first use deterministic evidence where possible, for example:

- known crash signatures;
- panic/exception class;
- renderer/device-loss signature;
- OOM or process termination reason;
- build/revision regression clustering;
- correlated Node/region outage;
- protocol/session-generation anomalies;
- repeated identical crash sequence;
- mismatch between authoritative server evidence and client-reported evidence.

Consistent with ADR-0006, AI/investigation tooling may correlate and summarize evidence or propose hypotheses, but it remains read-only and must not autonomously:

- mutate authoritative gameplay state;
- grant or remove disconnect protection;
- ban or sanction a player;
- roll back gameplay;
- deploy a fix;
- declare an unverified client report to be authoritative truth.

## 8. Crash/disconnect incident correlation

Later contracts must provide enough stable correlation to reconstruct one incident across available evidence without using high-cardinality identities as Prometheus labels.

Correlation should be capable of connecting, where authorized and applicable:

- client diagnostic report;
- logical `GameSessionId` and transport generation;
- `CharacterId` under restricted access;
- `WorldId`, `ChannelId`, `InstanceId` and `NodeId`;
- exact client/server/protocol/content/ruleset revisions;
- disconnect-protection activation/recovery timestamps;
- server liveness transitions;
- GameNode crash/restart/recovery evidence;
- infrastructure incident context.

Exact incident/report identifier types remain for the owning identifier/analytics contracts and are not frozen here.

## 9. Security, privacy and retention

Crash and disconnect diagnostics are security- and privacy-sensitive.

Later contracts must define:

- accepted fields and redaction rules;
- maximum report, stack, log-ring and attachment sizes;
- compression/decompression limits;
- upload authentication and abuse/rate limits;
- retention classes and deletion/anonymization behavior;
- role-based access for operators, developers and security investigators;
- integrity/deduplication of repeated reports;
- treatment of potentially malicious client payloads;
- separation from low-cardinality operational metrics;
- user-facing privacy controls and disclosures for client-originated diagnostics.

Raw player/session identifiers must not be exported as ordinary Prometheus labels.

## 10. Failure behavior

Diagnostics failure must not become gameplay authority.

If crash-log upload, storage or analysis is unavailable:

- server-authoritative liveness and disconnect protection continue according to their own contract;
- GameNode recovery and fencing continue according to their own contract;
- diagnostics loss/backlog is observable;
- durable security evidence follows the durability rules of ADR-0006 where applicable;
- normal client startup/gameplay admission is not indefinitely blocked by diagnostic upload failure;
- the system must not invent a crash classification when evidence is missing.

## 11. Required future tests

Future contracts and implementation must prove at minimum:

1. a completely idle but healthy client does not trigger disconnect protection merely because it sends no gameplay commands;
2. an apparently open but non-progressing transport can eventually be classified as lacking sufficient control/liveness evidence;
3. a client cannot self-declare lag/disconnect protection;
4. stale heartbeat/ack evidence from an older transport generation cannot recover authority;
5. isolated packet loss/jitter does not deterministically trigger protection before the accepted liveness policy says it should;
6. client crash evidence can be retained locally and is automatically submitted after restart when available and permitted by the client privacy setting;
7. automatic upload applies local redaction, allowlisted schema and hard size limits before transmission;
8. upload failure cannot indefinitely block normal client startup/gameplay admission and retry behavior remains bounded;
9. disabling client-originated diagnostics prevents automatic client crash-package upload without disabling server-authoritative protection/evidence;
10. absence of client crash evidence does not revoke valid protection;
11. GameNode crash evidence survives process/container death through an external collector/supervisor path;
12. correlated mass infrastructure failures can be distinguished from isolated client incidents;
13. automated diagnostic/AI analysis remains outside authoritative mutation and cannot ban, alter gameplay or decide protection.

## Programme effect

Accepted now:

```text
Disconnect Protection authority = server-observed control/liveness progress
lack of gameplay commands != disconnect evidence
socket-open state alone != sufficient liveness proof
client self-report != authority
2.0-second PvE protection timer starts from last sufficient server-authoritative liveness evidence
crash/log evidence is collected and analysed after/around the incident when available
client crash -> bounded local evidence -> automatic upload after restart when possible and enabled
per-crash user confirmation is not required
client diagnostics -> global privacy opt-out available
upload -> local redaction + allowlisted/versioned schema + hard size limits + bounded retry
crash-upload failure must not indefinitely block normal startup/gameplay admission
client opt-out != disabling required server-side operational/authoritative evidence
GameNode crash -> external supervisor/collector preserves evidence
network/infrastructure incidents -> correlated with player and node evidence
Game Intelligence performs deterministic + read-only AI-assisted analysis outside ChannelRuntime
missing crash evidence never invalidates otherwise valid protection
```

No runtime, protocol, client, persistence, Platform, telemetry-backend or production implementation is authorized by this baseline.

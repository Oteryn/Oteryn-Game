# Disconnect Client/OS Forensics Owner Direction

- Status: Owner-accepted architecture direction
- Date: 2026-08-08
- Decision owner: Oteryn project owner
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Refines: `DISCONNECT_FORENSIC_EVIDENCE_OWNER_BASELINE.md`
- Related: `DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md`, ADR-0006, ADR-0009, `CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md`
- Applies to: disconnect-abuse investigation, reconnect forensics, Game Intelligence, later client diagnostics, `FND-03`, `FND-04`, `ANL-01`, `ANL-03`, QA/E2E
- Does not authorize: runtime implementation, kernel driver, invasive anti-cheat, unrestricted Event Log collection, automatic sanctions, or production telemetry collection

## Purpose

Record the accepted direction for using bounded client-side and operating-system evidence to improve classification of disconnect incidents without weakening the server-authoritative forensic model.

The goal is not to prove from one event that a player intentionally disabled a network adapter, killed the client, removed power or caused a crash. The goal is to collect multiple independent, privacy-bounded signals that allow Oteryn Security Analytics and Game Intelligence to distinguish incident classes and detect repeated abuse patterns with materially higher confidence.

The primary abuse concern motivating this direction is automation that intentionally causes loss of connectivity or termination of the game process during dangerous combat in order to obtain the accepted defensive reconnect window. The architecture therefore focuses on observable incident evidence and longitudinal behavioral correlation rather than attempting to infer human intent from one low-level event.

## 1. Authority model

The binding trust order is:

```text
server-generated gameplay / liveness / runtime evidence -> authoritative evidence
client / launcher / guardian / OS evidence              -> corroborating evidence
analytics score / classification                         -> derived investigative evidence
human-reviewed enforcement decision                      -> separately governed
```

Client or OS evidence must never become the sole source of truth for whether protection was valid, whether abuse occurred, or whether a sanction is applied.

A missing client-side diagnostic record is not adverse evidence by itself.

## 2. Incident classification is evidence, not a claim of intent

The later analytics model should be able to classify an incident into concepts equivalent to:

```text
GRACEFUL_CLIENT_EXIT
GAME_PROCESS_CRASH
GAME_PROCESS_ABRUPT_EXIT
NETWORK_INTERFACE_LOST
NETWORK_INTERFACE_ADMIN_STATE_CHANGED
NETWORK_PATH_LOST
SYSTEM_CRASH
SYSTEM_POWER_LOSS_OR_HARD_RESET
SERVER_OR_INFRASTRUCTURE_FAILURE
UNKNOWN
```

These names are conceptual and do not freeze public protocol or event enums.

Each classification must carry evidence references and confidence rather than pretending to know user intent from one symptom.

For example, `GAME_PROCESS_ABRUPT_EXIT` means that the game process disappeared without an accepted graceful-exit path while other evidence suggests the operating system remained alive. It does not by itself mean `USER_KILLED_PROCESS`.

Likewise, `NETWORK_INTERFACE_ADMIN_STATE_CHANGED` means the operating system exposed a local administrative/interface-state transition correlated with the incident. It does not by itself prove that a player deliberately disabled the adapter to obtain protection.

## 3. Preferred client evidence shape

The client should not upload the full Windows Event Viewer or arbitrary machine logs.

The preferred direction is a bounded, allowlisted, normalized incident capsule containing only data relevant to the disconnect episode.

Candidate evidence includes:

- game-process lifecycle state;
- graceful shutdown marker when available;
- crash-diagnostic marker when available;
- local network-interface operational-state transitions;
- local network-interface administrative-state transitions where safely observable;
- local network-connectivity transitions;
- local monotonic timestamps around process/network changes;
- client build/revision identity;
- boot/session continuity evidence sufficient to correlate a likely system restart;
- narrowly selected OS diagnostic event metadata around the incident;
- optional launcher/guardian observations if that component is accepted by the later client architecture contract.

The client should prefer normalized fields over raw log text.

## 4. Windows evidence direction

For Windows, later implementation may use native operating-system facilities rather than scraping the Event Viewer UI.

Candidate sources include:

- native interface/network state notifications for live changes;
- bounded current interface-state queries;
- narrowly filtered Windows Event Log queries around the incident time;
- Windows Error Reporting / application-crash evidence where available;
- system unexpected-shutdown / crash evidence after restart where available;
- process-lifecycle observations from a separate launcher/guardian process if accepted.

Exact providers, event IDs, APIs, permissions and retention are implementation details for a later Windows diagnostics contract and are not frozen here.

The implementation must not depend on one Windows event ID as a universal proof of cause.

The Windows Event Log is a potential bounded evidence source; the Event Viewer UI itself is not an architectural dependency.

## 5. No full Event Log ingestion

The accepted privacy and minimization rule is:

```text
no unrestricted Event Viewer upload
no unrestricted Windows Event Log export
no arbitrary Windows log collection
no unrelated application/system event collection
```

If Event Log evidence is used, the client must query only an allowlisted set of providers/events and only a bounded time window around the disconnect incident.

Prefer normalized evidence such as:

```text
local_event_time
provider_class
event_class
interface_state_before
interface_state_after
interface_admin_state_changed
process_state_before
process_state_after
boot_continuity_changed
client_transport_loss_time
client_reconnect_time
```

Raw event message text, SSIDs, MAC addresses, unrelated device inventory and unrelated log payloads are not collected by default.

Any additional field requires a named security purpose, privacy classification, retention class and resource bound.

## 6. Client-side rolling incident buffer

The preferred client design mirrors the accepted server-side bounded forensic ring.

The client may keep a small in-memory rolling buffer covering only the recent, allowlisted network/process/system transitions needed to reconstruct a disconnect.

Normal operation overwrites the buffer and does not persist it indefinitely.

When a material disconnect occurs, the relevant bounded slice may be frozen locally and sent after a valid reconnect, subject to privacy and telemetry policy.

A later implementation should prefer capturing live state transitions into this bounded ring and use post-incident Event Log queries only as complementary evidence rather than depending entirely on retrospective log scraping.

Candidate horizons and byte/count limits remain for the later resource contract.

## 7. Lightweight launcher / guardian direction

A lightweight process independent from the main game process is an accepted architectural direction worth preserving for later client design.

Its purpose is diagnostic separation, not kernel-level control.

A future `Oteryn Launcher/Guardian` may observe whether:

- the game process exited gracefully;
- the game process crashed or disappeared abruptly;
- the operating system and guardian process remained alive after game-process loss;
- local network connectivity remained available after game-process loss;
- both game and guardian connectivity disappeared together;
- the machine subsequently presents evidence consistent with an unexpected reboot or system failure.

The exact process topology, privileges, transport, heartbeat cadence and startup lifecycle are not frozen here.

This direction does **not** authorize a kernel driver or invasive anti-cheat component.

## 8. Independent guardian heartbeat is a candidate, not yet a frozen contract

A later client/security contract may evaluate a very lightweight guardian-to-server diagnostic heartbeat separate from the main gameplay transport.

This could distinguish evidence patterns such as:

```text
game transport lost + guardian alive + network alive
    -> evidence consistent with game-process loss

game transport lost + guardian transport lost + OS/interface alive
    -> evidence consistent with network-path loss

game transport lost + guardian lost + interface down
    -> evidence consistent with local interface loss

game transport lost + guardian lost + later boot continuity break
    -> evidence consistent with system crash/power interruption
```

This is a diagnostic correlation mechanism only.

The exact necessity, cadence, bandwidth, abuse resistance and privacy cost must be measured before acceptance into runtime architecture.

The guardian heartbeat must never become a hidden second gameplay authority or a prerequisite that can stall the authoritative combat loop.

## 9. Force-close versus genuine crash

The architecture should preserve enough evidence to distinguish, where evidence exists, between:

- a graceful game exit;
- a game process that crashed and produced normal crash evidence;
- a game process that disappeared abruptly while the OS/guardian/network remained healthy;
- a simultaneous wider machine/network failure.

An abrupt process disappearance with healthy independent guardian/network evidence is materially more suspicious than an outage in which the entire machine or path disappears.

However, a single abrupt exit is still not automatic proof of deliberate abuse.

Longitudinal repetition and server-side combat context remain necessary for high-confidence investigation.

The absence of ordinary crash evidence may increase suspicion for an abrupt process-loss episode, but absence alone is not proof because crash reporting can itself fail or be unavailable.

## 10. Power loss, hard reset and system crash

A true power interruption or hard reset can remove the game, guardian and network path at the same moment, leaving little or no real-time client evidence.

The architecture therefore permits bounded post-boot corroboration after the next launch/reconnect.

Candidate evidence includes:

- boot/session continuity change;
- narrowly selected unexpected-shutdown evidence;
- narrowly selected system-crash / bugcheck / error-reporting evidence where available;
- absence of an accepted graceful client/system shutdown marker.

Such evidence supports the classification `SYSTEM_CRASH` or `SYSTEM_POWER_LOSS_OR_HARD_RESET` with an appropriate confidence level; it must not overclaim the exact physical cause when the OS itself cannot establish it.

A genuine system-wide interruption is expected to have a different evidence shape from a game-only force-close because the independent guardian, network path and machine continuity may disappear together.

## 11. Network-interface abuse investigation

The architecture should be able to preserve evidence of local network-interface state changes around a disconnect.

A repeated pattern such as:

```text
high combat risk / low HP
    -> local interface administrative/operational transition
    -> server liveness loss
    -> protection activation
    -> rapid interface recovery
    -> reconnect
    -> four-second recovery window
    -> repeated similar episodes
```

is a strong investigative signal even though one incident does not prove that the user intentionally disabled the interface.

The same principle applies to cable removal, Wi-Fi loss or other local link transitions: classify the observable state change, not the unobservable intent.

A physically removed cable and an administratively disabled interface can produce different local evidence on some systems, but the architecture does not assume those differences are universally available or impossible to spoof.

## 12. Longitudinal disconnect-abuse model

Game Intelligence should combine client/OS evidence with canonical server evidence and historical patterns.

Candidate features include:

- combat risk immediately before disconnect;
- HP/resource level;
- recent incoming damage / hostile pressure;
- whether disconnects cluster around a stable HP or danger threshold;
- protection use after reconnect;
- self-healing/potion usage during the four-second window;
- movement/escape outcome;
- frequency of abrupt client exits;
- frequency of local interface-state transitions;
- reconnect latency distribution;
- incident frequency inside versus outside combat;
- correlation with other affected players, GameNode or infrastructure incidents;
- consistency of client/guardian/OS corroborating evidence;
- detector/model version.

Repeated, unusually deterministic timing may raise suspicion of automation, for example repeated disconnects occurring near similar HP/risk thresholds or shortly after sharply increasing incoming damage. Analytics should look for repeated behavior patterns that are difficult to explain as ordinary random outages while remaining aware that correlation is not proof of intent.

The goal is to detect **abuse of disconnect protection**, not necessarily to identify the exact physical or software mechanism used to cause the disconnect.

Analytics output remains investigative evidence rather than autonomous enforcement authority.

## 13. Infrastructure and population correlation

Player-side classification must be compared with Oteryn-side and population-wide evidence.

Later analysis should correlate, where available:

- GameNode tick/event-loop health;
- channel/runtime overload and backpressure;
- deployment/restart/OOM/failure context;
- database/shared-service degradation;
- simultaneous disconnects on the same node/channel/region;
- broader clusters of affected players;
- repeated isolated incidents limited to one actor versus correlated infrastructure events.

A mass or correlated Oteryn-side/regional failure must not accumulate as independent adverse evidence against affected players.

## 14. Protection must not depend on client evidence arriving in time

The accepted four-second re-entry protection remains governed by authoritative server-side reconnect/liveness rules.

Client/OS evidence may arrive only after connectivity is restored and therefore cannot be required synchronously to decide whether a live disconnect transition exists.

This prevents a real outage from losing protection merely because the machine could not transmit diagnostics while disconnected.

Client evidence is for correlation, classification and abuse investigation after the fact.

An episode may therefore receive the mechanical protection and later become suspicious through forensic analysis; protection and retrospective abuse investigation are deliberately separate concerns.

## 15. Anti-tamper limits

The user controls the client machine.

Therefore:

- client evidence can be missing, delayed, modified or suppressed;
- a launcher/guardian can itself be terminated;
- local logs can be altered or unavailable;
- a sophisticated attacker can attempt to imitate a genuine outage.

The system must assume these limitations.

Authenticating a client build or signing a diagnostic capsule can establish provenance/integrity of what a genuine component sent, but does not turn local observations into server-authoritative truth.

Server-side evidence and longitudinal behavior remain primary.

## 16. Privacy and security constraints

Any later implementation must satisfy:

- explicit allowlists for OS/client evidence classes;
- bounded retention and byte/count limits;
- no arbitrary file collection;
- no unrestricted process inventory;
- no unrestricted Event Log export;
- no secret/key/token collection;
- no unrelated SSID/MAC/device identifiers by default;
- explicit access control for security evidence;
- pseudonymous analytics identities where possible;
- observable evidence-loss conditions;
- client diagnostics cannot silently become a general surveillance subsystem.

Collection should be incident-scoped and purpose-limited rather than a continuous broad system-audit feed.

## 17. Enforcement boundary

No single client/OS event may automatically ban or punish a player.

No single disconnect episode, including an abrupt process exit or network-interface administrative transition, is sufficient by itself to establish deliberate abuse.

No AI or Game Intelligence model may autonomously:

- ban or suspend an account;
- revoke disconnect protection;
- mutate gameplay state;
- rollback inventory/economy state;
- deploy code or policy changes.

Sanctions require a separately accepted enforcement policy and human-review boundary consistent with ADR-0006.

## 18. Required future proof

Later contracts and implementation should prove at minimum that:

1. normal gameplay does not upload arbitrary Windows Event Logs;
2. only allowlisted, bounded incident evidence is collected;
3. client evidence remains non-authoritative and optional for server-side incident visibility;
4. a graceful client exit is distinguishable from an abrupt process disappearance where evidence exists;
5. a game-process disappearance while guardian/OS/network remain alive can be represented separately from wider machine/network failure;
6. local network-interface transitions can be correlated with server-side liveness loss;
7. unexpected system restart/crash evidence can be attached after the next launch where available;
8. a missing client diagnostic capsule is not itself adverse evidence;
9. incident capsules are hard-bounded by time, count and bytes;
10. client-side evidence cannot synchronously block the authoritative combat loop;
11. protection eligibility does not require client telemetry to arrive during the outage;
12. longitudinal analysis can trace any risk score back to named canonical and corroborating evidence;
13. unusually deterministic combat-correlated disconnect patterns can be represented without hard-coding one simplistic threshold as guilt;
14. Oteryn-side or correlated regional failures can be separated from isolated actor evidence where evidence exists;
15. no single client event can trigger an automatic sanction;
16. privacy/redaction and evidence-loss behavior are observable and testable.

## Programme effect

Accepted now:

```text
server evidence remains authoritative
client/OS evidence is corroborating only
classify observable incident evidence, not presumed intent
focus on abuse-of-protection patterns, not exact physical disconnect mechanism
no full Event Viewer/Event Log ingestion
bounded allowlisted incident capsule
client-side rolling evidence buffer is preferred
live local network/process observations may supplement bounded post-incident OS evidence
lightweight launcher/guardian is an accepted design direction
separate guardian heartbeat remains a candidate to benchmark/contract later
crash / abrupt process loss / NIC loss / admin-state change / path loss / system crash-power loss are separate investigative classes
post-boot evidence may corroborate machine failure
repeated combat-correlated and unusually deterministic patterns feed Game Intelligence disconnect-abuse analysis
infrastructure/population correlation prevents Oteryn-side failures being blamed on players
mechanical protection and retrospective abuse analysis remain separate
no single client event -> automatic sanction
no kernel driver required by this decision
```

No runtime, protocol, persistence, database, launcher, guardian, Windows diagnostics, telemetry backend, Game Intelligence implementation or production collection is authorized by this direction.

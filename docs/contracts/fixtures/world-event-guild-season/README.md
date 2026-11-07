# PLAT3 world-event-guild-season evidence and conformance

Normative companion to `../../OTERYN_GAME_WORLD_EVENT_OUTCOMES_AND_GUILD_SEASON_V1.md`. Fixture JSON is documentation only;
wire string values exclude JSON fixture framing and its final LF. Hash exact UTF-8 wire
strings, never pretty-printed parsed objects. Synthetic identifiers are not deployment trust.

## 5. Watermark, resync, restart and audit

PublishLeagueWatermarkV1 exact fields:
`contract_version,operation,source_authority,family,projection_epoch,complete_through,observed_at`.
family is world_event_outcome or guild_season. PP §5.1 applies: complete_through is monotone
within epoch, <=observed_at, no future time beyond 1 s uncertainty; publish every <=10 s.
S=30 s, `now-complete_through+1 <=30` on every serve/cache/submission. A watermark never passes
an unacknowledged revision/source cut or incomplete guild contribution scan. Stale means 503 /
submissions closed, not zero score or no events. Past final outcomes remain history only;
staleness cannot reopen predictions or fabricate new results.

Per-subject revision advances atomically with source interpretation, full snapshot, audit and
outbox. Duplicate pair/equal content accepts; unequal content quarantines and alarms. A gap
signals missed publications. Because snapshots are complete, installation repairs the subject,
but scoring stays suspended until correction lineage/evidence is reconciled. Missing correction
history must be replayed from immutable audit/decision records before consuming a corrected
result; a current snapshot alone cannot justify repeated point changes.

Full resync enumerates every retained event/season, including VOID/CLOSED and empty tombstones,
under a committed cut; publishes full snapshots and required correction lineage, then the
watermark. Platform stages the new epoch and atomically switches only on complete enumeration.
Unknown old Platform subjects remain hidden until reconciled. A new epoch is operator-controlled,
fenced by nonrollback storage, never minted on ordinary restart. Restored unreconciled state
publishes nothing. Reconcile lost outcomes/corrections using authoritative native receipts and
recovery generation; if a result cannot be proven, Game explicitly voids before freeze or
blocks post-freeze repair. Platform never chooses a result from its old cache.

Crash before commit emits nothing; crash after commit replays same EventId/payload/hash and
snapshot pair. Existing EventEnvelope owns transaction identity, occurrence time in milliseconds,
causation and recovery-scoped provenance; wire seconds are never used as a total order.
WORLD_EVENT_OUTCOME_DECIDED and GUILD_SEASON_STANDINGS_DECIDED use the registered protobuf
payloads with TransactionEventRef: world outcomes carry exact records/hash; season audits
carry the hash/season identity referencing the retained complete authoritative snapshot.
Existing ANL payload/envelope ceilings remain unchanged; no oversized audit copy is admitted. Audit is P90D
under PLAT3_AUTHORITY_AUDIT_RETENTION_V1. Serving history is at most P90D from final freeze;
retries do not refresh retention. Retention expiry cannot resurrect a closed question or allow
repeat contribution: minimal non-reuse/final-state control evidence is separate authority state.
Logs contain family, operation, result class and timing only, no private raw evidence.

Use existing registry vocabulary for failures: PLATFORM_REQUEST_MALFORMED (5002),
PLATFORM_UNAUTHENTICATED (5003), PLATFORM_CONFLICT (5004), PLATFORM_RATE_LIMITED (5005),
PLATFORM_RESPONSE_INVALID (5006), PLATFORM_UNAVAILABLE (5007). These are diagnostics, not
new terminal outcome values; wire failures remain PP empty bodies. No command/refund namespace.

## 6. Fixtures and acceptance evidence

`fixtures/world-event-guild-season/snapshots.json` supplies exact canonical wire strings,
record hashes, settlement/correction/void/freeze and tied-guild ranking. Required future tests:
lock equality, late schedule, pre-lock leaked result denial; duplicate/equal-pair conflict;
raid timeout vs boss death; daily reset replay; absent source never settled; corrections and
correction-of-correction applied once; stale watermark/clock failure; channel aggregation;
guild membership cut and tie-break; full snapshot bounds; final freeze rejects late contribution;
restart/restore at each commit boundary; complete resync before dependent serving resumes.
Documentation fixtures demonstrate interpretation, not runtime qualification of G-12.

Game source/scoring owners accept mappings and emitters first; build Game outboxes and Platform
read-model adapters behind off-by-default flags, then qualify exact producer/consumer heads
with T88/T89 dependencies. Only a separately approved environment rollout may enable T94.
No accepted outcome source exists until this candidate is reviewed/merged/locked and implemented.

Before producer implementation, register measured family limits through resource-limit fragments
(as PP §8 requires); existing ANL hard ceilings remain mandatory. Candidate bounds here do not
change runtime configuration or authorize a larger envelope.

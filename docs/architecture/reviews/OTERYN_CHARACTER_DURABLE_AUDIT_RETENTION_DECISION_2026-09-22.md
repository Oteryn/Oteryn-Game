# Oteryn Character Durable-Audit 90-Day Retention Decision

- Status: `OWNER_ACCEPTED ARCHITECTURE DECISION` after merge to protected `main`
- Date: 2026-09-22
- Repository: `Oteryn/Oteryn-Game`
- Parent control plane: Issue #162
- Source-readiness programme: Issue #319
- Governing owner decision: Issue #220 comment `5780141258`
- Consumed by: WP5 #414 Character Authority durable audit/outbox
- Implementation authority granted by this decision: `NONE`
- Production authority: `NONE`

## 1. Decision timing

**Must decide now?** `YES`.

**Concrete downstream work blocked:** ANL-01 requires every concrete durable-audit event type to bind a finite accepted retention profile before production collection. WP5 #414 cannot activate Character owner/world/lifecycle mutations while the first Character audit family has no finite retention policy.

**What becomes harder later if decided too broadly now:** inventing analytics warehousing, indefinite forensic storage or generalized anonymization would expand the first server slice and create unnecessary privacy/storage coupling.

**Evidence that may justify superseding this decision:** legal/regulatory requirements, incident-response evidence, actual support/security investigation needs, measured storage cost, privacy findings, or a later independently approved analytics profile.

**Deliberately not decided:** analytics warehouse/broker, public history, detector/AI retention, economy/item event retention, storage vendor, or retention periods for unrelated event families.

## 2. Selected profile

The selected first profile is:

`CHARACTER_AUTHORITY_DURABLE_AUDIT_RETENTION_V1`

with:

`ordinary_retention_ceiling = 90 days`

Retention is rolling per record from its authoritative event timestamp/commit according to the implementation contract. It is not a global 90-day cycle and does not require server shutdown or manual renewal.

## 3. Purpose and privacy

Purpose:

- prove and reconcile authoritative Character owner/world/lifecycle mutations;
- support bounded security, operations and administrative investigation.

Privacy floor:

`RESTRICTED_PLAYER_LINKED`

or stricter when a specific future event requires it.

Ordinary public/analytics projection must not expose raw `AccountId` to alternate-character relationships.

## 4. Access, export and expiry

Ordinary readers: none.

Permitted readers are only explicitly authorized Character Authority operations/support/security roles. Access must be purpose-bound and auditable.

Case-scoped authorized export must redact unrelated player-linked fields.

At ordinary expiry, the retained player-linked event/envelope/payload is deleted. Expiry must not silently convert the event into an indefinite pseudonymous analytics record. Separately derived aggregates require their own accepted profile.

## 5. Legal hold

Legal hold is an exception, not ordinary retention.

A hold requires an explicit case/legal authorization with at least:

- reason;
- authorizing actor;
- start time;
- affected records/scope;
- audited access.

Release of a hold returns the record to the ordinary 90-day expiry/deletion policy. Legal hold does not modify event semantics, gameplay authority or Character state.

## 6. Policy revision and existing events

The profile uses positive, forward-only policy revisions.

A retained event remains governed by the policy revision bound to it unless an explicit reviewed migration/supersession states otherwise. Event payload bytes and immutable `EventId` semantics are never rewritten merely to fit a later retention revision.

## 7. Implementation consequences

The later #414 implementation must:

- register this real retention profile before enabling the first Character durable-audit event types;
- bind every enabled first-slice Character audit event type to the profile;
- commit Character mutation, revision, receipt, immutable audit event and durable outbox atomically;
- keep mutation disabled when retention/event registration is missing or contradictory;
- provide bounded automatic expiry/cleanup without gameplay/server downtime;
- prove cleanup cannot delete records under active legal hold;
- prove expiry does not replay or alter Character authority.

The exact scheduler/job mechanism and batch size are implementation details and must remain bounded. No requirement exists to stop the game server merely to perform ordinary expiry.

## 8. Non-decisions and authority

This decision selects only the first Character operational durable-audit retention profile. It does not authorize registry mutation, runtime code, SQL/migration, production collection, external analytics, or deployment.

Work must separately serialize the registry/event-type application and material #414 implementation. Normal independent review, canonical CI, Merge Queue and protected-main readback remain mandatory.

`IMPLEMENTATION_AUTHORITY: NONE_BY_THIS_DECISION`
`LIVE_DEPLOYMENT_AUTHORITY: NONE`

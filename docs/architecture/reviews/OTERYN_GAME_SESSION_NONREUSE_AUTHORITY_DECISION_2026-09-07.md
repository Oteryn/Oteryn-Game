# Oteryn Game — Durable GameSession Nonreuse Authority Decision

- Date: 2026-09-07
- Source escalation: Issue #162, comment `5570865467`, Decision A
- Affected work: #353 / PR #361 (WP2 Foundation), #329 / PR #335 (WP4 Durability), downstream #247 Server Seam
- Decision ID: `FND-DUR-GAMESESSION-NONREUSE-V1`
- Exact decision base: `main@5ce7457a6441b7c727c549acc18d1cfad2920ceb`
- Status: **CANDIDATE — ACCEPTED ONLY AFTER REVIEWED PROTECTED-MAIN INTEGRATION**
- Runtime / SQL execution / Cargo / workflow / production authority: **NONE**

## 1. Architecture-resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
source_issue: 162
source_comment: 5570865467
source_decision: A
main_sha: 5ce7457a6441b7c727c549acc18d1cfad2920ceb
accepted_decision: FND-DUR-GAMESESSION-NONREUSE-V1
facts:
  proven:
    - after S0 -> S1 -> S2, the current Foundation representation cannot prove that intermediate S1 is retired
    - exact-head review finding 3948631113 on PR 361 is accepted and remains unresolved
    - current released PostgreSQL migration 0001 already retains replacement edges in game_durability_session_replacements
    - current WP4 candidate migration 0002 has immutable fresh-admission receipts but no complete durable used-GameSession membership authority
    - RESOURCE_LIMITS_REGISTRY.json requires an explicit absolute maximum for a retained count before implementation acceptance
  derived:
    - copying an ever-growing retired-ID history into GameSessionAuthoritySnapshot would couple Foundation authorization to unbounded retained history and duplicate Durability ownership
    - a candidate-specific sealed observation from one durable Game-owned membership authority is sufficient if the transaction revalidates membership revision, current predecessor and all applicable authority fences before commit
  unknown:
    - production lifetime distribution of new GameSession creation per CharacterId
    - final PostgreSQL table/index names and internal Rust helper layout
    - any future exact-membership-preserving compaction representation
  conflict: []
resource_values_changed_by_decision: true
resource_registry_file_changed_by_this_candidate: false
protocol_changed: false
stable_identity_format_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_authority_granted: false
independent_review:
  required: true
  reason: session/recovery/persistence authority and a new hard capacity consequence
next_action: >-
  Oteryn: work coordinator qualifies this architecture candidate, applies the separately
  serialized registry/contract amendment defined in section 8, and integrates accepted
  architecture through normal protected review/check/Merge Queue controls before issuing
  any widened WP2/WP4 implementation allocation.
```

This decision resolves **Decision A only** from the current #162 escalation packet. It does not resolve Decision B/WP3, WP5 producer obligations, #308/WP1 holds, G0/G1, or the full #162 programme.

## 2. Decision timing

**Must decide now: YES.**

Concrete downstream work blocked: WP2 cannot truthfully reject an intermediate retired `GameSessionId` after more than one replacement, WP4 cannot persist/reload a complete authority that WP2 can consume, and #247 Server Seam must remain blocked from consumer activation across this boundary.

What becomes harder later if chosen incorrectly: copying history into Foundation creates duplicated authority and growing snapshots; TTL/eviction permits a previously used ID to become apparently fresh; transactionally separate membership/session updates can omit a committed predecessor or admit a reused candidate after a race.

Evidence that may justify superseding this decision: measured lifetime session-cardinality data, PostgreSQL storage/latency evidence, a formally exact compact membership representation, or a reviewed new identity/authority contract proving the same nonreuse property with lower retained-state cost. Supersession may change the bound or physical representation, but must never make an already used `GameSessionId` reusable.

Deliberately not decided: physical table/index names, compaction algorithm, production database sizing, retention of ancillary audit payload, deployment topology, credentials, new wire fields, new `GameSessionId` format, or any WP3/WP5 design.

## 3. Selected authority: sealed lookup, not copied history

Define one Game-owned durable logical resource:

`GameSessionUseLedgerV1`

Its semantic membership key is the existing globally unique `GameSessionId`. Each committed membership is permanently associated with its owning `CharacterId`. The ledger is the canonical fact that a `GameSessionId` has already participated in a successful GameSession creation. A used ID remains used forever within this authority version.

Foundation does **not** receive the ledger or a copied retired-ID collection. It consumes only a sealed, candidate-specific current observation equivalent to:

```text
GameSessionUseObservationV1
  version = 1
  CharacterId
  candidate GameSessionId
  expected current/predecessor GameSessionId or explicit no-current origin
  source identity/version
  membership revision
  completeness = COMPLETE
  candidate membership result
  applicable current authority/fence binding
```

The exact Rust names/field ordering are implementation details. The semantics above are mandatory.

The observation is eligibility evidence, not authorization escrow. It must be constructed only from the registered Game owner after asynchronous loading. Caller-filled history, an immutable operation receipt, the latest replacement anchor, UUID ordering, probabilistic membership, or a record-derived convenience helper cannot substitute for it.

Missing source, incomplete history, stale revision, conflicting source identity, revision rollback, changed predecessor/current session, or incompatible authority fences reject fail-closed.

## 4. Atomic membership and replay semantics

A new canonical `GameSessionId` may be committed only when the same durability transaction proves all of the following under its serialization protections:

1. the ledger history for the owning `CharacterId` is `COMPLETE`;
2. the candidate `GameSessionId` is globally absent from committed membership;
3. the expected current/predecessor session and all applicable AccountPresence, CharacterLease, runtime/source and operation fences still match;
4. the per-CharacterId membership revision and capacity state still match the sealed observation;
5. the operation is not a conflicting replay.

On successful **fresh admission**, ledger membership for the initial `GameSessionId`, the immutable fresh operation receipt, current session and applicable claim/lease/transport effects commit atomically.

On a successful operation that creates a **new GameSession** — including Terminal replacement, CompleteReconnect `EarlyTerminalReplacement`, or PostGrace recovery/adoption where the accepted contract creates a new session — the candidate membership and the current-session/claim/receipt transition commit atomically. The predecessor is already a used member and is never removed.

A same-GameSession reconnect that preserves `GameSessionId` does not consume another membership entry.

Exact operation replay is classified against its original immutable binding **before** treating an already-used candidate as a new-session request. Reconciliation of the exact previously committed operation returns its stable committed disposition without inserting a second membership row, incrementing the membership revision, consuming another capacity unit, or re-aging authority. The same used candidate under a different operation/binding is rejected.

Membership revision arithmetic is checked. Overflow, rollback, contradiction, or loss of the accepted revision floor rejects; no wrapping is permitted.

## 5. Retention and capacity decision

### Accepted hard maximum

The logical resource is bounded at:

```text
65,536 permanently retained committed GameSessionId memberships per CharacterId
```

This count includes the initial committed fresh session and every later operation that commits a new `GameSessionId` for that character. It excludes same-session reconnect attempts and failed/uncommitted candidates.

Rationale: this is a protective architecture ceiling, not a prediction of normal player behavior. It makes retained cardinality finite while remaining orders of magnitude above the already accepted per-loss reconnect-attempt budget. The raw 16-byte identifiers alone are bounded to 1 MiB per CharacterId at the ceiling; PostgreSQL row/index overhead is explicitly not inferred from that number and remains an operational measurement concern.

The lack of current production lifetime-cardinality measurements is **UNKNOWN**, not hidden. Therefore this value must be observable in implementation and may be superseded by a later reviewed resource decision if measured lifetime projections justify a different ceiling. Raising or lowering it requires an accepted registry/contract amendment; lowering must not discard existing membership.

### Capacity consequence

There is **no TTL, expiry, eviction, LRU, age-based deletion or 'make room' reuse** of membership.

At 65,536 committed memberships for a `CharacterId`, any otherwise new-session-producing operation is rejected with the existing `CAPACITY_EXCEEDED` resource category before it commits membership, a new current session, claims, receipt or authority effect. Existing current authority and all prior membership remain unchanged. An exact replay of an already committed operation remains idempotently reconcilable even at the ceiling and consumes no new unit.

This deliberately chooses nonreuse safety over future admission availability at exhaustion.

Ancillary payload may be compacted only when exact membership, owner association, completeness and revision/floor semantics remain lossless. Probabilistic filters are not sufficient authority because false negatives would violate nonreuse and false positives would create unreviewed availability behavior.

## 6. Restore, migration and completeness

Admission/new-session creation must not be enabled until the same membership authority and its accepted revision/floor state are restored.

For a new empty database initialized under this contract, an explicitly versioned empty `COMPLETE` ledger state is valid. Thereafter every successful new GameSession commit extends that complete state atomically.

For pre-contract persisted state, known IDs may be reconstructed only from authoritative durable rows whose identity and relationship are provable. Existing `game_durability_reconnect_sessions`, `game_durability_session_replacements`, and — when present on the accepted WP4 lineage — immutable fresh-admission receipts are evidence inputs, not automatic proof of historical completeness.

If migration can observe only a subset such as initial/current state, or cannot prove that all historical session-creation transitions are represented, it must preserve known membership but mark the ledger `INCOMPLETE` (or equivalently fail its completeness proof). It must **not invent a missing intermediate S1** and must not mark a partial history complete merely because the current S2 is known.

While completeness is unknown/incomplete or a source revision/floor is rolled back, all operations that would create a new `GameSessionId` for the affected character remain closed until an authoritative reconciliation establishes complete state. Existing safe terminal/history reads may continue if their own contracts permit them.

## 7. WP2 / WP4 custody and sequencing

This architecture decision grants no runtime writer lease. After protected acceptance, the existing Work control plane must issue exact allocations in this order.

### WP2 — Foundation semantic surface

WP2 owns only its already allocated Foundation source/test/task surfaces, amended precisely enough to:

- define the sealed candidate-specific `GameSessionUseObservationV1` semantics;
- require it consistently in Terminal, CompleteReconnect `EarlyTerminalReplacement`, and PostGrace authorization/adoption paths that create a new GameSession;
- reject missing/incomplete/stale/conflicting observations;
- preserve exact replay as reconciliation, not a new replacement;
- avoid activating consumers until WP4 persistence/reload qualification is accepted.

WP2 receives no SQL, migration, Durability codec, resource-registry, Cargo, workflow or Server Seam custody from this decision.

### WP4 — Durability persistence/reload

WP4 alone owns the persistence implementation under its existing bounded surfaces:

- `apps/game-server/src/durability/fresh_admission.rs`
- `apps/game-server/src/durability/admission_authority_guards.rs`
- `apps/game-server/src/durability/admission_journal.rs`
- `apps/game-server/src/durability/db.rs`
- `apps/game-server/src/durability/mod.rs`
- `apps/game-server/src/durability/schema.rs`
- `apps/game-server/migrations/0002_fresh_admission_authority.sql`
- `apps/game-server/tests/durability_postgres.rs`
- `apps/game-server/tests/support/postgres.rs`
- the existing WP4 allocation/task records explicitly amended by Work

WP4 may implement the logical ledger and revision/completeness state inside the still-unreleased forward `0002` lineage only after Work confirms that exact migration remains unreleased and writable. Released `0001` stays immutable.

Any additional production/source/migration/export path must be named in a fresh allocation amendment before mutation.

### Dependency order

```text
accepted architecture
  -> serialized RESOURCE_LIMITS_REGISTRY amendment
  -> exact WP2 semantic/API allocation and implementation, consumers inactive
  -> legitimate WP3 prerequisite where still required by #162
  -> exact WP4 persistence/reload allocation and implementation
  -> joint PostgreSQL 17.6 + Foundation qualification
  -> consumer activation
  -> coordinator re-evaluates #247 Server Seam
```

There is no B-first integration, no silent WP4 -> WP2 reversal, and no release of #335/#361 merely because this candidate exists.

## 8. Required resource-registry amendment

Before implementation acceptance, Work must serialize the following semantic entry into `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` under normal contract review. The registry file is intentionally not mutated by this architecture-author candidate.

```json
{
  "id": "FND04-GAMESESSION-USED-IDS-PER-CHARACTER",
  "owner_contract": "../architecture/reviews/OTERYN_GAME_SESSION_NONREUSE_AUTHORITY_DECISION_2026-09-07.md",
  "resource": "Permanently retained committed GameSessionId memberships for one CharacterId",
  "unit": "entries per CharacterId",
  "hard_maximum": 65536,
  "configurable_range": {"minimum": 65536, "maximum": 65536},
  "failure_category": "CAPACITY_EXCEEDED",
  "allocation_impact": "Under the same serialized durability decision, check complete membership, candidate absence, current membership revision and count before inserting any new membership or committing new-session authority. At the hard maximum reject the new-session operation before membership/session/claim/receipt authority effects. Exact replay of an already committed binding consumes no additional entry. Never evict membership to make room.",
  "client_visible": true,
  "boundary_tests": [
    "65,535 committed memberships plus one unused candidate may commit as entry 65,536 when all authority predicates pass",
    "65,536 committed memberships plus a distinct unused candidate is rejected as CAPACITY_EXCEEDED before any new authority effect",
    "exact replay at the ceiling reconciles without consuming an additional entry",
    "a GameSessionId already used by a different binding is rejected regardless of remaining per-character capacity",
    "restart/reload preserves exact membership count, revision, completeness and rejection behavior"
  ]
}
```

No smaller configurable value is allowed in V1 because an operational configuration change must not silently introduce a lower availability ceiling than the reviewed contract.

## 9. Required proof before consumer activation

The joint WP2/WP4 qualification must prove at minimum:

- `S0 -> S1 -> S2 -> candidate S1 rejected` for Terminal, CompleteReconnect `EarlyTerminalReplacement`, and PostGrace;
- each family through direct, resumed and reconciled/reloaded entry;
- fresh unused candidate positive controls;
- same-candidate concurrent races and competing candidate/CAS races;
- exact replay and conflicting replay;
- lost response followed by reconciliation;
- transaction rollback without phantom membership;
- stale/missing/substituted membership source and stale revision negatives;
- incomplete migration/reload keeps new-session creation closed;
- 65,535 -> 65,536 success and 65,536 -> new candidate `CAPACITY_EXCEEDED` with no partial mutation;
- exact replay at capacity remains idempotent;
- full-u64 revision/fence edges and checked overflow rejection;
- real configured PostgreSQL 17.6 process/reload recovery.

Each negative authority case must change one relevant invariant while keeping unrelated facts valid. Immutable records may prove expected historical binding but may not substitute for independently current authority.

## 10. Rejected alternatives

- copying the full retired/used ID set into `GameSessionAuthoritySnapshot`;
- retaining only the latest predecessor/candidate anchor;
- UUIDv7 ordering or timestamp comparison as nonreuse authority;
- account-local uniqueness in place of the existing global `GameSessionId` namespace;
- caller-supplied history or immutable receipt as current membership authority;
- TTL/expiry/LRU/age-based deletion of used IDs;
- probabilistic membership as authoritative acceptance evidence;
- guessing missing migration history from S0/current S2;
- wrapping membership/source revision on overflow;
- blocking the logical writer on SQL merely to obtain the sealed observation;
- activating WP2 consumers before WP4 reload/persistence qualification.

## 11. Player/producer and future impact

Player-visible benefit: a retired session can never regain control merely because it fell out of a local snapshot or process memory. Retry/lost-response behavior remains stable rather than creating a second session.

Player-visible cost: after the explicit lifetime ceiling is exhausted for a character, operations that require a new `GameSessionId` fail closed until a later accepted architecture/operational remedy; an existing current session is not destroyed by the denial.

Producer/operational cost: Durability retains permanent exact membership and one bounded per-character revision/count authority. Storage and lookup behavior must be measured before production launch. The selected candidate-specific API prevents Foundation memory/snapshot growth from tracking lifetime history size.

A future exact compact representation may replace physical rows only after proving lossless membership, reload, revision/floor, concurrency and migration behavior. It must preserve this decision's external semantic contract unless explicitly superseded.

## 12. Architecture-author closeout boundary

This candidate is complete when its exact head has been read back, its diff has been self-reviewed, repository architecture/governance checks for that head are green or their exact failures are reported, and the #162 escalation receives a candidate-return comment pointing to the PR/head.

Acceptance requires a non-author independent review appropriate to the session/recovery/persistence risk, normal required CI, protected Merge Queue integration, and protected-main readback. The architecture author must not treat PR creation, green partial checks, or Issue closure alone as acceptance.

# PLAT3 account-deletion evidence and conformance

Normative companion to `../../OTERYN_GAME_ACCOUNT_DELETION_FINALIZATION_V1.md`. Fixture JSON is documentation only;
wire string values exclude JSON fixture framing and its final LF. Hash exact UTF-8 wire
strings, never pretty-printed parsed objects. Synthetic identifiers are not deployment trust.

## 5. Recovery, retention and conformance

| Failure | Required behavior |
|---|---|
| Crash during kick/detach | Resume manifest steps, keep admission closed, no success proof yet |
| Crash after proof before publication | Republish original receipt and projection; no repeat detach |
| Platform crash after receiving proof | Requery same operation; worker still checks cancellation/blockers |
| Platform commits cancellation/finalization then crashes | Its resolution outbox retries exact bytes; never issue both decisions |
| Domain partial failure | Stay pending and quarantined; CANCEL runs compensating restoration, not speculative retry under a new termination |
| Restore/stale fence | CC §5.4 generation-selected reopening, nonrollback reconcile before publish/admission; invalidate lost proofs and requery same identity |

Retain minimal closed-account/operation/generation and non-reuse evidence as authoritative
control state, separately from finite ANL audit. Quarantined personal gameplay state is retained
only until cancellation resolution/final disposition, not copied to analytics. Audit uses
PLAT3_AUTHORITY_AUDIT_RETENTION_V1 (P90D), restricted case-scoped access; delete names/text and
unneeded account linkage at expiry, preserve only required authoritative proof/replay state.
Legal holds follow that registry profile. Physical financial/economy evidence follows its own
accepted retention; this contract does not erase it or reset retention on retry.

Additional registered codes: 5014 CHAR_CMD_ACCOUNT_CLOSE_STALE_GENERATION;
5015 CHAR_CMD_ACCOUNT_CLOSE_OBLIGATION; 5016 CHAR_CMD_ACCOUNT_CLOSE_RESOLUTION_CONFLICT.
Use existing CHAR_CMD_INTENT_INVALID/EXPIRED/UNSUPPORTED/OPERATION_CONFLICT for common failures.
Unknown dependencies are PENDING; a definitely impossible reversible disposition is OBLIGATION.
A resolution contradicting the recorded winner is RESOLUTION_CONFLICT, no state change.

Fixtures: `fixtures/account-deletion/closure.json`, including cancellation-before-close and
proof requery hashes. Required integration scenarios: live kick versus admission; incoming
transfer versus closing portfolio; partial detach then restart/cancel; duplicate receipt;
blocked house/guild/market; LCFA lag; cancellation at/after deadline; cancellation versus worker
under one Platform lock; lost resolution acknowledgement; restore at each phase. New adapters
must preserve termination_tests.rs semantics and keep finalization pending on unknown evidence.
Implement Game first, then qualify Platform boundary/projection adapters; qualify exact
heads before separate testing/preproduction enablement. No production permission.

Before producer implementation, register measured family limits through resource-limit fragments
(as PP §8 requires); existing ANL hard ceilings remain mandatory. Candidate bounds here do not
change runtime configuration or authorize a larger envelope.

## Implemented Game producer

Owner decision 2026-11-06 enables the Game producer by default. See
[CONTRACT-NOTES.md](CONTRACT-NOTES.md), especially **PLATFORM CONFIG**, and
`producer.example.toml`. `lifecycle.json` adds exact authenticated current-lifecycle
read vectors; all original `closure.json` wire bytes and hashes remain unchanged.
Unapproved owner disposition remains configurable/PENDING, never fabricated success.

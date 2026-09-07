# Oteryn Game — FND-04C GameSession Ledger Capacity Error Amendment

- Date: 2026-09-07
- Parent decision: `FND-DUR-GAMESESSION-NONREUSE-V1`
- Parent PR: #383
- Source review: PR #383 review `5132433524`
- Amends: `docs/architecture/FND-04C_ERROR_DIAGNOSTICS_FAILURE_COMPATIBILITY_CONTRACT.md`, Section 4 only for permanent `GameSessionUseLedgerV1` capacity exhaustion
- Status: **CANDIDATE — ACCEPTED ONLY AFTER REVIEWED PROTECTED-MAIN INTEGRATION**
- Runtime / protocol implementation / SQL execution / production authority: **NONE**

## 1. Purpose and precedence

`FND-DUR-GAMESESSION-NONREUSE-V1` introduces one permanent, non-evicting lifetime membership ceiling for committed `GameSessionId` values. The pre-existing FND-04C catalogue has only `ADMISSION_CAPACITY_EXCEEDED`, whose accepted semantics are `RETRYABLE` bounded transient capacity. Those semantics are false at a permanent lifetime-ledger ceiling because retry cannot create capacity while membership is never removed.

When this amendment is accepted, it is an additive supersession of the FND-04C Section 4 completeness claim **only** for the four new-`GameSessionId` operation families below. All other FND-04C codes and semantics remain unchanged.

The existing `ADMISSION_CAPACITY_EXCEEDED` remains canonical for retryable/transient admission capacity and MUST NOT be emitted for `GameSessionUseLedgerV1` lifetime exhaustion.

## 2. Common lifetime-exhaustion semantics

For every governed family below:

```text
category = CAPACITY_EXCEEDED
progression = TERMINAL for this new-GameSession-producing operation under the current ledger version/bound
public_class = SESSION_UNAVAILABLE
```

Retry authority is intentionally narrow:

- no independent retry, new candidate, new grant or new recovery attempt can create capacity while the same `GameSessionUseLedgerV1` version remains at its accepted ceiling;
- an **exact replay/reconciliation of an operation that already committed** remains valid and returns its stable committed disposition; it never consumes another membership unit and must not be remapped to capacity exhaustion;
- a later attempt may become admissible only after a separately accepted architecture/resource supersession changes the capacity policy or exact representation while preserving every already-used `GameSessionId` as used;
- operator cleanup, restart, process failover, TTL, LRU, age, retry delay or a fresh credential is not capacity authority.

The denial itself commits no new membership, candidate session, claim, lease, transport, receipt or successful nonce/grant-consumption effect. Existing current/terminal actor and session authority remains exactly as it was at revalidation.

## 3. Canonical FND-04C additions

| Operation family | Canonical code | Category | Progression | Retry authority | Mutation / idempotency | Public class | Redacted diagnostic | Safe correlation |
|---|---|---|---|---|---|---|---|---|
| Fresh admission creating initial `GameSessionId` | `ADMISSION_GAMESESSION_LEDGER_EXHAUSTED` | `CAPACITY_EXCEEDED` | `TERMINAL` | no independent retry under the same ledger version/ceiling; exact previously committed replay may reconcile | `NO_AUTHORITY_MUTATION` | `SESSION_UNAVAILABLE` | `game session lifetime capacity exhausted` | operation-family + capacity-policy/version class; no raw character/session count |
| Terminal replacement creating successor `GameSessionId` | `TERMINAL_REPLACEMENT_GAMESESSION_LEDGER_EXHAUSTED` | `CAPACITY_EXCEEDED` | `TERMINAL` | no new candidate/replacement retry under the same ledger version/ceiling; exact previously committed replay may reconcile | `CURRENT_AUTHORITY_PRESERVED`; no candidate authority effect | `SESSION_UNAVAILABLE` | `game session lifetime capacity exhausted` | operation-family + capacity-policy/version class; no raw character/session count |
| CompleteReconnect `EarlyTerminalReplacement` creating successor `GameSessionId` | `EARLY_TERMINAL_REPLACEMENT_GAMESESSION_LEDGER_EXHAUSTED` | `CAPACITY_EXCEEDED` | `TERMINAL` | no new candidate/replacement retry under the same ledger version/ceiling; exact previously committed replay may reconcile | `CURRENT_AUTHORITY_PRESERVED`; no candidate authority effect | `SESSION_UNAVAILABLE` | `game session lifetime capacity exhausted` | operation-family + capacity-policy/version class; no raw character/session count |
| PostGrace recovery/adoption creating successor `GameSessionId` | `POST_GRACE_RECOVERY_GAMESESSION_LEDGER_EXHAUSTED` | `CAPACITY_EXCEEDED` | `TERMINAL` | no new-session recovery retry under the same ledger version/ceiling; exact previously committed replay may reconcile | `CURRENT_AUTHORITY_PRESERVED`; no candidate authority effect | `SESSION_UNAVAILABLE` | `game session lifetime capacity exhausted` | operation-family + capacity-policy/version class; no raw character/session count |

`CURRENT_AUTHORITY_PRESERVED` means exactly the FND-04C vocabulary: whatever authority is current at revalidation remains current; PREPARE-time or historical state is not restored. This amendment does not invent a new takeover, recovery or reconnect authority.

## 4. Progression and client behavior

`SESSION_UNAVAILABLE` is selected instead of `TEMPORARILY_UNAVAILABLE` or `RETRY_LOGIN` because neither waiting nor reissuing a credential creates ledger capacity. The client may return to a non-session UI or surface an operator/support-facing failure according to product UX, but it MUST NOT be instructed to bounded-backoff retry the same lifetime-exhausted transition as though capacity were transient.

The API does not expose the exact retained count, `CharacterId`, raw ledger membership, private fence values or a capacity side channel beyond the canonical public class and safe diagnostic class.

## 5. Required compatibility proof

Before consumer activation, implementation qualification must prove for all four families:

- exact family code, `CAPACITY_EXCEEDED`, `TERMINAL`, `SESSION_UNAVAILABLE` and the matching mutation disposition at the ceiling;
- no membership/session/claim/lease/transport/receipt/successful grant-or-nonce effect on denial;
- existing current/terminal authority is byte/semantically unchanged by the denial;
- a fresh credential, new attempt reference or process restart does not turn the permanent ceiling into a retryable success;
- exact replay of a previously committed binding at the ceiling returns the stable committed result, not a lifetime-exhaustion error;
- one-below-ceiling positive control can commit the final allowed membership;
- the pre-existing `ADMISSION_CAPACITY_EXCEEDED` remains retryable only for its pre-existing transient-capacity meaning and is never used for this ledger ceiling;
- public diagnostics reveal no raw per-character lifetime count or membership contents.

## 6. Supersession boundary

This amendment changes only FND-04C error/progression/public-class handling for `GameSessionUseLedgerV1` lifetime exhaustion. It does **not** change:

- the 65,536 bound itself;
- `GameSessionId` identity format or global nonreuse semantics;
- FND-04A credential/authentication semantics;
- FND-04B reconnect/recovery authority predicates except for representing this newly accepted capacity denial;
- the existing retryable meaning of `ADMISSION_CAPACITY_EXCEEDED` for transient capacity;
- WP2/WP4 implementation custody;
- SQL/migration implementation authority;
- Server Seam, production or external-repository authority.

A future supersession may change the capacity bound or exact storage representation only through a reviewed resource/architecture decision that preserves all already-used `GameSessionId` membership and updates this error contract consistently.
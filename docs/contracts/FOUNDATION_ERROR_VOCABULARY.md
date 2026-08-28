# Foundation Error Vocabulary

Status: normative foundation vocabulary; concrete numeric codes remain owned by their accepted contracts.

## Purpose

Provide one cross-layer shape for failures without forcing protocol, Gateway, admission, runtime and persistence to invent incompatible semantics.

Every public or cross-component error must define:

- a stable machine category and contract-owned code;
- whether it is `RETRYABLE`, `TERMINAL` or `SECURITY_TERMINAL`;
- whether retry requires the same command/session, a new session or owner intervention;
- a redacted diagnostic message separate from client-facing presentation;
- correlation/trace fields that contain no credentials, tickets or private payloads;
- idempotency and partial-mutation outcome;
- mapping from internal causes to the bounded public category.

## Foundation categories

- `INVALID_INPUT` — malformed, out-of-range or non-canonical input; terminal for the rejected operation.
- `UNSUPPORTED_REVISION` — family/version/schema/ruleset/content mismatch; terminal with no silent downgrade.
- `AUTHENTICATION_FAILED` — credential or proof invalid; security-terminal for the attempt.
- `SESSION_REJECTED` — expired, replayed, consumed, wrong-audience or wrong-bound session.
- `STALE_GENERATION` — stale session, writer, entity or revision fence; no mutation committed.
- `CONFLICT` — a current authoritative owner/state prevents the requested transition.
- `CAPACITY_EXCEEDED` — registered queue/entity/frame/resource limit reached.
- `DEPENDENCY_UNAVAILABLE` — required external service unavailable; retry policy is contract-specific and bounded.
- `TIMEOUT` — a named total-operation or lifecycle deadline expired.
- `CANCELLED` — operation intentionally cancelled with documented cleanup state.
- `INTERNAL_UNAVAILABLE` — safe fail-closed response for an unexpected internal condition; diagnostic details remain internal.

Contracts may add narrower codes but must map them to one category and must not expose secrets or unstable implementation text as API behavior.

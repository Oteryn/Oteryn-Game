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

## Code space

Amended by ARCH-ERROR-CODES-0 (`docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md`); accepted on merge.

- Every Game error code is one u32 in one space, written `E<number>` with at least four digits and shown with its SCREAMING_SNAKE name. Code 0 is invalid.
- Blocks: 1000–1999 are wire codes, owned by `PROTOCOL_OTERYN_V1_REGISTRY.json` (FND-02 §18): 1000–1099 foundation, 1100–1199 admission, 1200–1999 future wire. 2000–2999 node lifecycle, 3000–3999 durability, 4000–4999 runtime internals, 5000–5999 Platform integration, 6000–6999 ops tooling, 7000–7999 client, 8000–8999 tools and CI. 9000–9999 are reserved. These codes are owned by `OTERYN_GAME_ERROR_CODE_REGISTRY.json`.
- Numbers and names are unique across both registries. A registered number is never reused or renumbered; a retired code stays registered as `RETIRED`, and a retired code is never reactivated. A registered code's name, category, progression and (protocol registry) `default_disposition` never change, and its `public_class` never changes once present. A change of meaning takes a new code and retires the old one.
- Every code maps to exactly one category above. Every code in the Game registry also has one progression. A protocol-registry code keeps the fields its registry defines. Where an entry has no `progression` member (the existing 1001–1050 codes), its progression and retry requirement are derived from `default_disposition` by this fixed table and are normative for every public failure: `TRANSPORT_FATAL` and `SESSION_FATAL` → `TERMINAL`, retry needs a new connection and session; `OPERATION_TERMINAL` → `TERMINAL` for the operation, the session continues and a new command may be sent; `RESYNC_REQUIRED` → `RETRYABLE` in the same session after the resync the disposition requires. An explicit `progression` member (the N8 admission entries) is authoritative. A disposition outside this table fails the registry validator.
- A failure that crosses a boundary (the wire, a log line at warn or above, a process exit, a Platform call, or a tool or CI failure) carries its registered code. For a process exit, the diagnostic line written before exit carries the registered code; the OS exit status keeps its accepted coarse value (node `BootError` 10–21, ops 2–7) and is never the registered code. Only 1000–1999 codes are sent on the wire. When the root cause has a non-wire code, the wire carries the public code that the owning contract assigns to the wrapper, and the diagnostic line records the root code as `code` and the wire code as `wire`.
- Diagnostic lines carry `ts` (UTC Unix milliseconds), `code`, `name`, `cat` and a `trace` CorrelationId (ANL-01), and `wire` when a different wire code was sent. The quoted `detail` value is escaped (`\\`, `\"`, `\r`, `\n`, other control characters as `\u{XX}`), so a line stays single and its fields cannot be forged. Untrusted client input never becomes the trace. Scope fields are limited to `world`, `channel` and `session_gen`. A player-linked identifier (AccountId, CharacterId, GameSessionId and the others of ANL-01 §18) never appears in a diagnostic line; it is correlated through `trace` only in stores that ANL-01 permits to hold it.
- Every Game process names its build (`<version>+<sha12>`, or `<version>+<sha12>.local` when the build cannot prove its exact source) in its first diagnostic line. An unexpected Rust panic writes one diagnostic line with code `E4001 PANIC` and no formatted panic payload, and is otherwise handled as FND-03 §24 requires.

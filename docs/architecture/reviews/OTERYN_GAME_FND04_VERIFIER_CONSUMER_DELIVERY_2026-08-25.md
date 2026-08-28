# FND-04 verifier/consumer delivery

## Scope

This delivery adds only the Game-owned FND-04 verifier/consumer seam.  It
does not bind a listener, create/revive/rebind a `GameSession`, consume a
grant nonce, implement durable replay storage, select key infrastructure, or
mutate Platform or another repository.

## Security boundary

`verify_fresh_grant` and `verify_recovery_grant` select their issuer,
audience, profile, purpose, type and trust context from their typed caller
context, never from a token.  Both paths perform bounded Compact-JWS parsing,
fixed-context Ed25519 verification, exact authenticated claim validation,
binding/profile validation, NumericDate validation, evidence freshness and
non-rollback-floor checks, then current target/revision checks. Signing trust
is a per-`kid` decision: a revoked key cannot inherit a different key's
trusted state. The verifier obtains key and account decisions only through
the `Fnd04EvidenceAuthority` interface, scoped to the fixed fresh/recovery
profile, exact account or `kid`, and verification time. The interface requires
its implementation to authenticate Platform evidence, preserve durable
per-scope non-rollback floors and decision identities, reject equal-revision
contradictions, and fail closed after restart if those floors cannot be
reconstructed. Raw timestamps, revisions, floors and allow/deny flags are not
accepted by the public verifier API.

Fresh verification returns the existing `FreshAdmissionFacts`; the existing
authority boundary remains solely responsible for atomic replay consumption
and session creation.  Recovery verification returns `VerifiedRecoveryFacts`,
which carries no session, controller, placement or commit authority.

The former partial parser/signature helpers are no longer public API.  This
prevents a future caller from treating successful signature verification as
admission authorization.

## Local validation

- RED: the initial full fresh-consumer test failed because no typed consumer,
  context or evidence API existed.
- RED: explicit current trust revocation initially returned stale-evidence;
  the final implementation distinguishes current explicit revocation from
  unavailable/stale evidence.
- GREEN: a revoked `kid` is rejected even while a separate current `kid`
  remains usable.
- GREEN: evidence below its durable non-rollback floor fails closed.
- GREEN: a fresh signing key cannot be reused in the recovery evidence scope.
- GREEN: `cargo test --locked --workspace`.
- GREEN: `cargo fmt --check`.
- GREEN: `cargo clippy -p oteryn-game-server --locked --all-targets -- -D warnings`.
- GREEN: `git diff --check`.

## Remaining release gates

This report is not an admission authorization or production deployment
record. Merge authority remains conditional on an independent non-authoring
security review and all repository-required exact-head checks, including
`game-gate`, for the frozen remote PR head.

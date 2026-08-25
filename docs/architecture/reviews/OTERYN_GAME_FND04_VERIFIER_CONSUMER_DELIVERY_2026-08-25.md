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
non-rollback-floor checks, then current target/revision checks.

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
- GREEN: `cargo test --locked --workspace`.
- GREEN: `cargo fmt --check`.
- GREEN: `cargo clippy -p oteryn-game-server --locked --all-targets -- -D warnings`.
- GREEN: `git diff --check`.

## Remaining release gates

This report is not an admission authorization or production deployment
record.  The exact remote PR head still requires an independent non-authoring
security review and all repository-required exact-head checks, including
`game-gate`, before merge.

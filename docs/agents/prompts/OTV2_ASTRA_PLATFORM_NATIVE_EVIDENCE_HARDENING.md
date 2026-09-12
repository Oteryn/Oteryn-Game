# OTV2 Platform Native Evidence Hardening

Short invocation:

```text
Oteryn: astra platform native evidence hardening
```

## Outcome

Harden the existing Oteryn Platform native-evidence producer so it is ready for truthful real Game<->Platform interoperability qualification required by WP5, without transferring Game, PKI, deployment or production authority.

Terminal task outcome: `PLATFORM_NATIVE_EVIDENCE_READY_FOR_REAL_INTEROP`.

## Repository/authority boundary

- Target repository: `Oteryn/Oteryn-Platform`.
- This file in Game is a cross-repository task specification/handoff only. It does not itself grant Platform mutation authority.
- When the owner launches this alias for Platform work, first resolve Platform's live `AGENTS.md`, META binding, Platform bootstrap, live task/PR ownership and path overlaps, then create/reuse the repository-native task lineage required there.
- Game remains read-only unless the launched task separately authorizes the exact Game inspection needed for compatibility evidence.
- No production deployment, secret/certificate mutation, live account/session mutation or Game repository write is implied.

## Required hardening families

### Witness durability

Qualify the actual PHP/filesystem profile required for high-water durability. The current audited write path conditionally calls `fsync`; do not silently claim the same durability when the runtime/filesystem cannot provide the accepted guarantee. Test file and directory durability, restart, lost/unavailable witness storage and DB/witness ordering.

### Limiter semantics

Decide explicitly whether `GAME_AUTH_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE` is an approximate throughput limiter or a hard admission ceiling. If hard, prove atomic reservation before expensive producer work under concurrency. Do not describe check-then-callback-then-hit behavior as a strict concurrent reservation proof.

### Security-transaction recovery

Qualify whole workflows where native game authorization revocation is nested inside larger security transactions, including applicable password change/reset, recovery, MFA/email and termination families. Cover witness advance -> later transaction failure -> restart -> reconciliation without weakening anti-rollback.

### Witness provenance / restore

Prove how deployment distinguishes genuine first activation from accidental replacement/loss of the independently retained witness volume. A new empty writable directory must not become authority merely by being writable when retained history should exist elsewhere.

### Signing-trust lifecycle

Separate normal key rotation, key revocation and profile revocation. Define the safe recovery/next-version procedure for terminal profile revocation under the accepted fixed issuer/profile/key-purpose contracts; never clear revocation or invent a new profile ad hoc.

### Real interoperability readiness

Prepare the exact non-production qualification boundary for Platform route `/internal/v1/game-auth/native-evidence` using real TLS 1.3 mutual authentication/terminator metadata provenance, exact four producer operations, bounded request/response behavior and exact compatible Game consumer fixtures. Application server-variable injection alone is not final mTLS evidence.

## Acceptance

- Existing producer anti-rollback semantics remain fail-closed.
- No private signing material is moved into this producer.
- AccountId/native-generation contract remains compatible with accepted Game consumer semantics.
- Real database/filesystem/concurrency tests cover the repaired boundaries where applicable.
- Cross-repository evidence names exact Game and Platform revisions and does not promote local PASS to composed PASS.
- Any production or credential action remains separately gated.
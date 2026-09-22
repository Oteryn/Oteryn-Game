# WP5 S3-A real Platform↔Game interoperability qualification allocation
Coordinator: #162. Source readiness: #319. Remediation programme: #364.
Preparation record: #319 comment `5773772641`.

## Status and authority
Prospective Game-only qualification allocation. **NOT_ACTIVE**.
```yaml
allocation_id: OTV2-WP5-REAL-INTEROPERABILITY-QUALIFICATION-20260922
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
source_issue: 319
programme_issue: 364
allocation_state: NOT_ACTIVE
worker_launch: NONE_UNTIL_PROTECTED_WORK_APPLICATION
platform_write_authority: FORBIDDEN
production_authority: FORBIDDEN
live_data_authority: FORBIDDEN
secret_rotation_authority: FORBIDDEN
foundation_composition_authority: FORBIDDEN
```
This packet allocates no current path lease, worker, branch, workflow execution,
environment, credential or publication. Protected integration alone does not
activate it. Work must first re-read protected main, live ownership and collisions,
then prove the exact execution, private-source read, authoring/publication and
required-validation routes.

## Accepted sources
- Platform protected source:
  `Oteryn/Oteryn-Platform@623435ec1b907d6d9770b767806c90300252a71c`;
  governing `docs/agents/evidence/OTERYN-20260912-platform-native-evidence-hardening/REAL_INTEROP_QUALIFICATION.md`,
  blob `4a6f2131b1a10e91b9f02e035705b44ad6eb2f59`.
- Platform producer protected by PR #1389; repository result
  `PLATFORM_NATIVE_EVIDENCE_READY_FOR_REAL_INTEROP`, not composed proof.
- Game S1 protected by PR #735 as
  `c59d8b25f9e3017d013d578a5a4bc9d93fa49e1d`; preparation readback Game
  `main@bbda147eedc7b01c39827ab4784b5760032b1237`.
- Existing `apps/game-server/tests/native_admission_source_transport.rs` proves the
  private S1 plus codec path-import pattern without Cargo or `lib.rs` changes.

Execution must refresh and bind the exact Game and Platform revisions actually
used. These preparation revisions are evidence, not permission to skip that pin.
## Prospective exact owned paths
```text
apps/game-server/tests/native_admission_source_real_interop.rs
.github/workflows/wp5-s3a-real-interop.yml
tools/qualification/wp5_s3a/compose.yml
tools/qualification/wp5_s3a/platform-fpm.Dockerfile
tools/qualification/wp5_s3a/nginx.conf
tools/qualification/wp5_s3a/run.sh
```
No other path is implied. PR #739 owns its seven explicit paths; it does not hold
a blanket workflow-directory lease. Work must nevertheless refresh exact
workflow-path custody before application and serialize any real collision.

## Launch prerequisites
Before releasing one sole qualification writer, Work must prove:

1. fresh protected-base and exact six-path ownership/collision readback;
2. authenticated read access to the exact private cross-repository Platform
   source needed to build the pinned producer, without granting Platform writes;
3. a retained runner with Docker Engine, Docker Compose (record exact qualified
   version), Rust/Cargo 1.94.0, PHP 8.5 FPM, nginx, MariaDB 11.8 and evidence tools;
4. an executable TLS 1.3 mTLS nginx-to-FastCGI/PHP-FPM provisioning/validation
   route with trusted TLS metadata and public-header-substitution controls;
5. an isolated synthetic Platform database and separately retained fsync-capable
   witness volume outside the database restore unit;
6. executable runner controls for file/directory `fsync`, restart, database
   restore, replacement witness and injected persistence failure;
7. ephemeral non-production PKI and synthetic identities only, with private keys
   excluded from source, logs, artifacts and retained evidence;
8. a permitted exact-head publication route and applicable repository validation,
   independent review, canonical CI, Merge Queue and protected-main readback.

Generic GitHub CI availability does not prove these capabilities. Current Game CI,
Platform PHP/MariaDB CLI CI and the Synology reverse-proxy topology do not execute
the required composed FastCGI/mTLS/witness procedure and cannot be relabeled.

## Minimal implementation and proof
The Rust harness must compile the exact S1 transport and codec, consume only
explicit non-secret configuration plus ephemeral key/certificate file paths, and
emit bounded sanitized evidence. It must not export S1 through `lib.rs`, add
dependencies, publish Foundation state or become a production client.

Capability preflight binds exact tool/image versions and proves every provision,
probe, failure, restart, restore, capture and cleanup route is executable and
authorized. It does not claim an acceptance result.

One exact composed run must satisfy the governing Platform procedure:

- real TLS 1.3 mTLS and rejection of no certificate, wrong root, lower TLS and
  wrong exact client identity before producer authority;
- all four operations decoded by exact Game code without fixture rewriting;
- exact operation/subject/key/purpose bindings; malformed, nested, duplicate and
  unknown-field rejection; 1,024-byte request and 8,192-byte response bounds;
- the Platform producer's hard two-in-flight/no application-queue proof, without
  using its rate limiter as atomic evidence; preserve Game S1's separate
  two-active plus eight-queued client limits;
- bounded unavailable response after relational query failure;
- retained-witness process restart, database restore, replacement-witness
  rejection, account rollback/forward reconciliation, signing-trust conservative
  revocation/successor recovery and file/directory-sync failure behavior;
- explicit `BLOCKED` for simultaneous loss or rollback of database history and
  the independent witness authority.

Evidence must bind exact Game, Platform, harness, topology, image and configuration
revisions and contain no private key, bearer material, live identity or full
secret. Result classes are `COMPOSED_PASS`, `COMPOSED_FAIL`, `BLOCKED` and
`NOT_RUN`.

## Boundaries and downstream use
S3-A is non-production interoperability evidence only. It creates no Platform
source-owner decision, Foundation publication/composition, S2 PostgreSQL evidence,
#414 Character Authority, #415 process-registration or scope-assignment
implementation, production deployment, live-data access, credential provisioning
or secret rotation.

Game PostgreSQL is not required for this independent evidence stage. A protected
`COMPOSED_PASS` may be reused as one S3-B prerequisite; it is never S3-B, WP5 G0
or Server Seam release by itself. #415 remains blocked on the separate accepted
architecture decision requested in #220 comment `5773031968`.

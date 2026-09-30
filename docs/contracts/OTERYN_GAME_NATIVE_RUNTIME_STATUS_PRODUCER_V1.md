# Oteryn Game Native Runtime Status Producer v1

- Status: Candidate. Acceptance by the architect and the owner (#162 Q15a). Implementation, configuration and activation are not authorized by this document.
- Contract ID: `oteryn-game-native-runtime-status-v1`
- Coordination: #162 (owner decisions Q14–Q18, comments 5899892092 and 5899942821); Oteryn/Oteryn-Platform#1419.
- Owner decision: **Q16b** — game nodes report runtime status automatically; there is no manual operator source.
- Producers: `Oteryn/Oteryn-Game` — the `oteryn-game-server serve` node (runtime reports) and the scope ownership authority `oteryn-game-ops` (assignment reports). Consumer: `Oteryn/Oteryn-Platform` (Game Gateway route selection and runtime-status read model).
- Platform consumer rules: `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` §7 (Oteryn/Oteryn-Platform PR #1420, candidate) under the accepted Platform `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`.
- Consumes: `docs/architecture/reviews/OTERYN_GAME_NODE_BOOT_COMPOSITION_DECISION_2026-09-24.md` (D1, D3 steps 8 and 10, D5), ADR-0003 (the World Registry owns routes), `docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md` §5, §11, §15, `docs/contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json`, `apps/game-server/src/native_admission_source/`.
- Does not change: the readiness publication, scope assignment semantics, admission, the FND-04 profile or any registry row.

## 1. Purpose

Platform may issue a fresh-entry grant for a `(WorldId, ChannelId)` only from fresh, current-owner runtime evidence, and the grant must carry the scope's route, runtime, ownership and gameplay revisions (FND-04 §11). Today the node publishes readiness only to its own durability root, and the revisions must be configured by hand on the Platform side (node-boot D5, "Cross-repository coordination"). This contract makes Game the automatic source of that evidence for Platform.

## 2. Division of authority

- **World Registry (Platform)** owns the gameplay route of each scope: endpoint host, port, TLS server name, ALPN, transport profile and the `route_revision` bound to them (Platform contract §7.3). A node never reports an endpoint, SNI name or route record.
- **Scope ownership authority (Game, `oteryn-game-ops`)** owns which node serves a scope under which ownership generation (the #415 assignment). It reports each committed assignment (§5).
- **Node** owns its readiness and the revisions it serves. It reports them (§4). Platform accepts a node report only when it matches the latest assignment report (§7).

## 3. Transport and identities

- HTTP/1.1 over TLS 1.3 with a client certificate, through the existing `native_admission_source` client code (`http1_mtls::exchange`, ALPN `http/1.1`), with the node's `[platform]` endpoint, `peer_name` and trust roots.
- **A separate client certificate per purpose.** Each purpose uses its own `ProducerDescriptor` built from its own certificate and key files; none reuses the native-evidence certificate:
  - runtime status: one certificate per node host (new configuration keys, for example `[platform.runtime_status] client_certificate_file`, `client_key_file`); Platform lists the scopes each node-host identity may serve;
  - assignment reports: one ownership-authority certificate held only by `oteryn-game-ops`;
  - Character projection: Character Authority hosts only (projection candidate §3).
- Closed operations with compiled paths (not configurable):
  - `ReportRuntimeStatusV1` → `POST /internal/v1/game-auth/native-runtime-status` (node);
  - `ReportScopeAssignmentV1` → `POST /internal/v1/game-auth/native-scope-assignments` (ownership authority).
- Bounds: connect 1 s, handshake 2 s, exchange 3 s (existing constants); request body at most 2048 bytes; response body at most 256 bytes.
- Capacity: one in-flight runtime report per node and no queue; a newer report supersedes a pending one (latest wins). A report never takes an admission `TransientCapacity` slot.

## 4. Runtime report (node)

A report is a projection of the node's **committed** Runtime guard readiness publication for its assigned scope (node-boot D3 step 8) plus a heartbeat observation time. The node reports nothing it has not published and never a prepared or ambiguous publication.

| Field | Source |
|---|---|
| `world_id`, `channel_id` | the assigned scope (FND-ID-01 canonical lowercase UUIDv7) |
| `node_id` | this incarnation's `NodeId`, canonical text form |
| `source_authority` | `[readiness].source_authority` (node-boot D1) |
| `assignment_epoch` | the ownership authority's current epoch (§6), carried by the assignment |
| `scope_ownership_generation` | the #415 assignment ownership generation used in the publication |
| `source_revision`, `decision_identity`, `ready`, `published_at` | the committed publication's source revision, decision identity, `ready` value and `source_observed_at` |
| `observed_at` | node clock when this report is built (clock uncertainty 0, as in D3 step 8.6) |
| `protocol_major`, `transport_profile` | fixed by the build (`1`, `1`) |
| `route_revision` | the Registry's `route_revision` for this scope, deployed as the node-boot D5 declared value |
| `runtime_observation_revision`, `ruleset_revision`, `content_revision`, `map_revision`, `world_policy_revision`, `offer_revision` | the publication's revisions (node-boot D5, declared until the D5 supersession trigger replaces content, map and ruleset with measured values) |

Wire, exact member set, no unknown, duplicate or `null` members, nesting at most 1:

```json
{
  "contract_version": 1,
  "operation": "ReportRuntimeStatusV1",
  "source_authority": "oteryn:runtime:world-1:channel-1",
  "world_id": "01934f10-7c02-7001-805b-3b1122334401",
  "channel_id": "01934f10-7c03-7001-805b-3b1122334401",
  "node_id": "<NodeId>",
  "assignment_epoch": "1",
  "scope_ownership_generation": "3",
  "source_revision": "7",
  "decision_identity": "runtime-readiness:<node hex>:3:7:true",
  "ready": true,
  "published_at": "1790000000",
  "observed_at": "1790000015",
  "protocol_major": 1,
  "transport_profile": 1,
  "route_revision": "rt.4.0f3a9c1d2b7e4a5f6c8d9e0a1b2c3d4e",
  "runtime_observation_revision": "observation-1",
  "ruleset_revision": "ruleset-1",
  "content_revision": "content-1",
  "map_revision": "map-1",
  "world_policy_revision": "policy-1",
  "offer_revision": "offer-1"
}
```

Revision strings satisfy the FND-04 §5 grammar (`[A-Za-z0-9._:-]{1,64}`); epochs, generations and revisions are canonical non-zero decimal uint64 strings; times are canonical non-negative decimal Unix seconds.

Success response (`200`, exact): `{"contract_version":1,"result":"accepted"}`, where `result` is `accepted` (new publication), `refreshed` (same publication, newer `observed_at`) or `superseded`. Failures are empty bodies: `400` malformed, `401` unauthenticated, wrong purpose or scope not allowed, `409` conflict with the current assignment or an equal key with different content, `429` rate limited, `503` unavailable. The node treats every failure as "not delivered", never as a statement about its readiness.

## 5. Assignment report (ownership authority)

After an assignment (#415) commits, `oteryn-game-ops` reports it with the ownership-authority certificate:

```json
{
  "contract_version": 1,
  "operation": "ReportScopeAssignmentV1",
  "assignment_epoch": "1",
  "world_id": "01934f10-7c02-7001-805b-3b1122334401",
  "channel_id": "01934f10-7c03-7001-805b-3b1122334401",
  "ownership_generation": "3",
  "node_identity": "<certificate subject of the assigned node host's runtime-status identity>",
  "assigned_at": "1789999990"
}
```

- `node_identity` comes from the operator's assignment request and must be one of the node-host identities configured for that scope; the ops tool rejects any other value before reporting.
- Delivery is retried until Platform returns a definite result; an ambiguous delivery is replayed with the same content. A failed report never changes the Game assignment, which remains authoritative.
- Revocation or replacement of an assignment is reported the same way with the new generation.

## 6. Restore reset (`assignment_epoch`)

`assignment_epoch` is a positive value owned by the scope ownership authority. It is raised only by an operator after a restore of the Game durability root that could lower ownership generations. Platform orders every scope by `(assignment_epoch, ownership_generation, source_revision)`; on the first assignment report in a higher epoch it invalidates all runtime state from lower epochs, and a scope routes again only after a new assignment report and a matching node report. Where the epoch is stored and how the operator raises it is a Game follow-up (U-RS5).

## 7. Ordering, freshness and failover (Platform acceptance)

- A node report is accepted only when its `(assignment_epoch, scope_ownership_generation)` equals the latest accepted assignment for the scope **and** its TLS client identity equals that assignment's `node_identity`. A node therefore cannot raise its own generation or report for a scope it was not assigned.
- Within one assignment, reports are ordered by `source_revision`; an equal revision with different content is a conflict and the scope routes nowhere until a newer publication.
- `observed_at` later than Platform time plus its configured uncertainty is invalid; the node never future-dates (D3 step 8.6 already waits out a clock gap of up to 5 s and fails boot on a larger gap).
- A report is evidence for routing only. Final admission still checks the grant's generation and revisions against current authoritative state (FND-04 §11, §12); a stale Platform view can cause a refused admission but never an admission.

## 8. When the node reports

1. **After a definite publication commit.** Right after `ready = true` commits at boot (D3 step 8), and right after the `ready = false` shutdown publication commits (D3 step 10.1), within the shutdown budget, best effort.
2. **Heartbeat.** Every H seconds (proposed 5 s, U-RS1) while all hold: the node is serving; its latest publication for the scope is definite and `ready = true`; the durability root reports ready; the scope assignment is still the one the publication used. The heartbeat repeats the unchanged publication with a new `observed_at`.
3. **Stop.** When any heartbeat condition fails, heartbeats stop. Platform sees the scope go `stale` within its freshness bound F (proposed 15 s) and stops routing new logins there. This covers a database outage, in which the node cannot write `ready = false` (node-boot "Database outage while serving").

Report delivery never gates boot, serving, admission or shutdown. While the Platform endpoint is absent (rollout step 2) every report fails and the node keeps running.

## 9. Route revision on the node

The node's declared `route_revision` must be the Registry's value for its scope (Platform §7.3: `rt.<version>.<digest of the route descriptor>`). Admission already rejects a grant whose `route_revision` differs from the node's (`ADMISSION_GRANT_ROUTE_STALE`). Because the Registry value is digest-bound to the scope and endpoint, a value copied from another scope never matches there. The listener certificate must be valid for the Registry's `tls_server_name`; the client trust anchor for that certificate is Platform U3.

## 10. Privacy and logging

- `scope_ownership_generation`, `assignment_epoch` and `decision_identity` (which embeds the generation) are sent only in these private reports. Game logs only the operation, scope ids, `ready`, the result class and timings; it never logs the generation, the epoch or the decision identity (FND-04 §15).
- The reports carry no credential, no account or character data and no private key.

## 11. Proposed limits

Registry entries are a follow-up in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` (outside this task):

| Proposed id | Value | Hard maximum |
|---|---|---|
| `NRS-REPORT-BYTES` | request at most 2048 bytes | 2048 |
| `NRS-RESPONSE-BYTES` | response at most 256 bytes | 256 |
| `NRS-INFLIGHT` | 1 report in flight per node | 1 |
| `NRS-HEARTBEAT` | H = 5 s | 60 s |

## 12. Failure scenarios

| Scenario | Result |
|---|---|
| node crashes | heartbeats stop; scope stale after F; no new grants; existing grants expire within 30 s |
| node replaced | new assignment report with a higher generation; old reports superseded |
| node reports a generation or scope it was not assigned | refused (`409`/`401`) |
| durability root restored | operator raises `assignment_epoch`; Platform drops older-epoch state until new assignments |
| durability root down | heartbeats stop; stale; admissions already refuse (#823) |
| Platform unreachable | reports fail; gameplay unaffected; Platform view goes stale |
| Registry route changed | new `route_revision`; node redeployed with it; reports with the old value stop matching |

## 13. Required tests

- the report equals the committed publication field by field; no report for a prepared or ambiguous publication;
- heartbeat stops on durability-root not-ready, on shutdown and on a lost assignment;
- each purpose uses its own certificate; Platform refuses cross-purpose certificates;
- node report refused when generation, epoch or identity differs from the latest assignment report; accepted after it matches;
- epoch raise invalidates older state;
- a failed or refused report never changes boot, serving or admission;
- exact wire fixtures shared with the Platform consumer;
- joint E2E: node-boot qualification through the real Platform ingestion and Gateway (#1419 item 5).

## 14. Unknowns

- U-RS1: heartbeat H and freshness F values (proposed 5 s / 15 s), to be measured.
- U-RS2: PKI issuing the per-purpose certificates (Platform U15).
- U-RS3: capacity or load facts in the report (not in v1; Game OPS-CHANNEL-01).
- U-RS4: whether the Registry pins gameplay revisions (Platform U18).
- U-RS5: storage and operator procedure for `assignment_epoch` (Platform U16).

## 15. Rollout

Producers first: the node and the ops tool may ship reporting before the Platform endpoints exist (`server-first-safe`), because delivery never gates them. Activation of native routing on Platform follows the Platform contract §14. Rollback: disable reporting in the configuration; Platform sees stale evidence and stops native routing.

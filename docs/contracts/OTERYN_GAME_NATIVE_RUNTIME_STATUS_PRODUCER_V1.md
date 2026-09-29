# Oteryn Game Native Runtime Status Producer v1

- Status: Candidate. Acceptance by the architect and the owner (#162 Q15a). Implementation, configuration and activation are not authorized by this document.
- Contract ID: `oteryn-game-native-runtime-status-v1`
- Coordination: #162 (owner decisions Q14–Q18, comments 5899892092 and 5899942821); Oteryn/Oteryn-Platform#1419.
- Owner decision: **Q16b** — game nodes report runtime status automatically; there is no manual operator source.
- Producer: `Oteryn/Oteryn-Game` (the `oteryn-game-server serve` node). Consumer: `Oteryn/Oteryn-Platform` (Game Gateway route selection and runtime-status read model).
- Platform consumer rules: `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` §7 (Oteryn/Oteryn-Platform PR #1420, candidate) under the accepted Platform `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`.
- Consumes: `docs/architecture/reviews/OTERYN_GAME_NODE_BOOT_COMPOSITION_DECISION_2026-09-24.md` (D1, D3 steps 8 and 10, D5), `docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md` §5, §11, §15, `docs/contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json`, `apps/game-server/src/native_admission_source/`.
- Does not change: the readiness publication, scope assignment, admission, the FND-04 profile or any registry row.

## 1. Purpose

Platform may issue a fresh-entry grant for a `(WorldId, ChannelId)` only from fresh, current-owner runtime evidence, and the grant must carry the scope's route, runtime, ownership and gameplay revisions (FND-04 §11). Today the node publishes readiness only to its own durability root, and the same revisions must be configured by hand on the Platform side (node-boot D5, "Cross-repository coordination"). This contract makes the node the automatic source of that evidence for Platform.

## 2. What the node reports

A report is a projection of the node's **committed** Runtime guard readiness publication for its assigned scope (node-boot D3 step 8), plus the scope's gameplay endpoint and a heartbeat observation time. The node reports nothing it has not published and never reports a prepared or ambiguous publication.

| Field | Source |
|---|---|
| `world_id`, `channel_id` | the assigned scope (`[scope]` configuration, FND-ID-01 canonical lowercase UUIDv7) |
| `node_id` | this incarnation's `NodeId`, canonical text form |
| `source_authority` | `[readiness].source_authority` (node-boot D1) |
| `scope_ownership_generation` | the #415 assignment ownership generation used in the publication |
| `source_revision`, `decision_identity`, `ready`, `published_at` | the committed publication's source revision, decision identity, `ready` value and `source_observed_at` |
| `observed_at` | node clock when this report is built (the node observes its own state; clock uncertainty 0, as in D3 step 8.6) |
| `protocol_major`, `transport_profile` | fixed by the build (`1`, `1`) |
| `alpn` | `oteryn-game/1` |
| `route_revision`, `runtime_observation_revision`, `ruleset_revision`, `content_revision`, `map_revision`, `world_policy_revision`, `offer_revision` | the publication's revisions (node-boot D5, declared until the D5 supersession trigger replaces content, map and ruleset with measured values) |
| `endpoint.host`, `endpoint.port`, `endpoint.tls_server_name` | new declared gameplay-route configuration (§6); the public address clients use for this scope |

Every revision string satisfies the FND-04 §5 grammar (`[A-Za-z0-9._:-]{1,64}`); generations and revisions are canonical non-zero decimal uint64 strings; times are canonical non-negative decimal Unix seconds.

## 3. Transport

- HTTP/1.1 over TLS 1.3 with a client certificate, through the existing `native_admission_source` client: same `ProducerDescriptor` (`[platform]` endpoint, `peer_name`, trust roots, client certificate and key), same `http1_mtls::exchange`, ALPN `http/1.1`.
- New closed operation `ReportRuntimeStatusV1`, path compiled into the binary: `POST /internal/v1/game-auth/native-runtime-status`. The path is not configurable.
- Bounds: connect 1 s, handshake 2 s, exchange 3 s (existing constants); request body at most 2048 bytes; response body at most 256 bytes.
- Capacity: one in-flight report per node and no queue. A newer report supersedes a pending one (latest wins); a report never waits behind an admission evidence exchange and never takes an admission `TransientCapacity` slot.
- Platform authenticates the node by TLS client identity and authorizes it per scope (Platform contract §7.1). The client identity may be the one the node already uses for native evidence; that choice and per-scope identities are Platform U14.

## 4. Wire (v1)

Request, exact member set, no unknown, duplicate or `null` members, nesting at most 2:

```json
{
  "contract_version": 1,
  "operation": "ReportRuntimeStatusV1",
  "source_authority": "oteryn:runtime:world-1:channel-1",
  "world_id": "01934f10-7c02-7001-805b-3b1122334401",
  "channel_id": "01934f10-7c03-7001-805b-3b1122334401",
  "node_id": "<NodeId>",
  "scope_ownership_generation": "3",
  "source_revision": "7",
  "decision_identity": "runtime-readiness:<node hex>:3:7:true",
  "ready": true,
  "published_at": "1790000000",
  "observed_at": "1790000015",
  "protocol_major": 1,
  "transport_profile": 1,
  "alpn": "oteryn-game/1",
  "route_revision": "route-1",
  "runtime_observation_revision": "observation-1",
  "ruleset_revision": "ruleset-1",
  "content_revision": "content-1",
  "map_revision": "map-1",
  "world_policy_revision": "policy-1",
  "offer_revision": "offer-1",
  "endpoint": { "host": "game-eu1.example.invalid", "port": 7172, "tls_server_name": "game-eu1.example.invalid" }
}
```

Success response (`200`, exact): `{"contract_version":1,"result":"accepted"}`, where `result` is one of `accepted` (new publication), `refreshed` (same publication, newer `observed_at`) or `superseded` (Platform already holds a higher generation or revision for the scope). Failures are empty bodies: `400` malformed, `401` unauthenticated or scope not allowed for the identity, `409` equal generation and revision with different content, `429` rate limited, `503` unavailable. The node treats every failure as "not delivered" and never as a statement about its own readiness.

## 5. When the node reports

1. **After a definite publication commit.** Right after `ready = true` commits at boot (D3 step 8), and right after the `ready = false` shutdown publication commits (D3 step 10.1), within the shutdown budget, best effort.
2. **Heartbeat.** Every H seconds (proposed 5 s, U-RS1) while all hold: the node is serving; its latest publication for the scope is definite and `ready = true`; the durability root reports ready; the scope assignment is still the one the publication used. The heartbeat repeats the unchanged publication with a new `observed_at`.
3. **Stop.** When any heartbeat condition fails, heartbeats stop. Platform then sees the scope go `stale` within its freshness bound F (proposed 15 s, Platform U5) and stops routing new logins there. This covers a database outage, in which the node cannot write `ready = false` (node-boot "Database outage while serving").

Report delivery never gates boot, serving, admission or shutdown. During rollout step 2 (Platform endpoint absent) every report fails and the node keeps running.

## 6. Gameplay endpoint configuration

The node gains declared gameplay-route values (implementation lane, not this contract): public `host`, `port` and `tls_server_name`. They describe how clients reach this scope's listener, which may differ from the bind address behind NAT or a load balancer. They are covered by `route_revision`: changing any of them requires a new `route_revision`, so a grant issued for the old route fails as `ADMISSION_GRANT_ROUTE_STALE`. The listener certificate must be valid for `tls_server_name`. Whether Platform takes the endpoint from this report or from Registry configuration is Platform Decision D3 (U11); under the alternative the node still reports readiness and revisions and these fields become advisory.

## 7. Ordering, freshness and failover

- Per scope, Platform orders reports by `(scope_ownership_generation, source_revision)` numerically. A replacement node receives a higher ownership generation from the assignment, so a delayed report from the old owner is rejected as superseded, whatever its timestamp.
- Equal `(generation, source_revision)` with different content is a conflict; Platform marks the scope invalid and routes nowhere until a higher pair arrives.
- `observed_at` later than Platform time plus its configured uncertainty is invalid; the node never future-dates (D3 step 8.6 already waits out a clock gap of up to 5 s and fails boot on a larger gap).
- A report is evidence for routing only. Final admission still checks the grant's generation and revisions against current authoritative state (FND-04 §11, §12); a stale Platform view can cause a refused admission but never an admission.

## 8. Privacy and logging

- `scope_ownership_generation` and `decision_identity` are sent only in this private report. The node logs only the operation, scope ids, `ready`, the result class and timings; it never logs the generation (FND-04 §15).
- The report carries no credential, no account or character data and no private key.

## 9. Proposed limits

Registry entries are a follow-up in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` (outside this task):

| Proposed id | Value | Hard maximum |
|---|---|---|
| `NRS-REPORT-BYTES` | request at most 2048 bytes | 2048 |
| `NRS-RESPONSE-BYTES` | response at most 256 bytes | 256 |
| `NRS-INFLIGHT` | 1 report in flight per node | 1 |
| `NRS-HEARTBEAT` | H = 5 s | 60 s |

## 10. Failure scenarios

| Scenario | Result |
|---|---|
| node crashes | heartbeats stop; scope stale after F; no new grants; existing grants expire within 30 s |
| node replaced | new generation accepted; old reports superseded |
| durability root down | heartbeats stop; stale; admissions already refuse (#823) |
| Platform unreachable | reports fail; gameplay unaffected; Platform view goes stale |
| delayed old report | rejected by ordering |
| revision changed by redeploy | new publication and report; grants carrying the old value fail at admission |

## 11. Required tests

- the report equals the committed publication field by field; no report for a prepared or ambiguous publication;
- heartbeat stops on durability-root not-ready, on shutdown and on a lost assignment;
- latest-wins single in-flight report; no admission capacity used;
- a failed or refused report never changes boot, serving or admission;
- exact wire acceptance and rejection fixtures shared with the Platform consumer;
- joint E2E: node-boot qualification through the real Platform ingestion and Gateway (#1419 item 5).

## 12. Unknowns

- U-RS1: heartbeat H and freshness F values (proposed 5 s / 15 s), to be measured.
- U-RS2: public endpoint source (this report vs Registry), Platform D3 / U11.
- U-RS3: client identity per node or per scope (Platform U14).
- U-RS4: capacity or load facts in the report (not in v1; Game OPS-CHANNEL-01).

## 13. Rollout

Producer first: the node may ship reporting before the Platform endpoint exists (`server-first-safe`), because delivery never gates the node. Activation of native routing on Platform follows the Platform contract §14. Rollback: disable reporting in the node configuration; Platform sees stale evidence and stops native routing.

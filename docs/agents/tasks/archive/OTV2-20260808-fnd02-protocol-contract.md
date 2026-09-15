# OTV2-20260808-fnd02-protocol-contract — archived

```yaml
task_id: OTV2-20260808-fnd02-protocol-contract
title: Freeze the protocol-oteryn v1 foundation contract
mode: CONTRACT
status: complete
repository: blakinio/Oteryn-v2
coordination_id: OTV2-NATIVE-FOUNDATION
base_sha: b9c5764711c4206832209f6ca89b9dc56492c3c1
delivery_pr: 94
delivery_exact_head: 91809204fcdf984a5a9d7b8c276ef9fb2f9cab9f
delivery_squash_merge: 769ecd2ce2dfe0a7644d8dc1d67c54d40da5d202
completed_at: 2026-08-08T00:56:00+02:00
ownership_released: true
next_gate: FND-03
```

## Outcome

`FND-02` is complete at the architecture-contract level.

Canonical deliverables:

- `docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`;
- `docs/contracts/protocol-oteryn/v1/foundation.proto`;
- `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`;
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`;
- immutable local acceptance evidence in `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json`.

The accepted v1 foundation freezes TCP + TLS 1.3, ALPN `oteryn-game/1`, BE32 length framing, protobuf binary/proto3 source IDL, numeric protocol registries, exact hard ingress limits, GameSession-scoped monotonic `uint64 CommandId`, bounded ordered pipelining, per-GameSession server sequence, bidirectional connection-generation fencing, typed state revisions, replay/resync, atomic replacement snapshots with a bounded snapshot sequencing barrier, and authenticated liveness primitives.

It does not implement or activate runtime behavior.

## Exact-head validation evidence

Delivery exact head: `91809204fcdf984a5a9d7b8c276ef9fb2f9cab9f`.

- Agent governance run `31225287340`: `PASS`;
- Dependency review run `31225287332`: `PASS`;
- CodeQL run `31225287334`: `PASS`;
- adversarial architecture/security/compatibility audit review `4887157361`: `PASS`;
- open material findings: `0`;
- unresolved review threads: `0`;
- changed-file scope: five declared FND-02 task/contract/registry/schema/limit files only;
- independently recomputed `foundation.proto` SHA-256: `6e1c614661e72daac529be9d0ec06317201b916cd47ae17ff1590da5c7205ebe`, matching the canonical protocol registry;
- runtime component/integration/E2E execution: `NOT_APPLICABLE` for the architecture-only delivery.

## Findings resolved before acceptance

1. `ProtocolError` direction narrowed to server-to-client.
2. bootstrap phase classification normalized.
3. server sequencing class naming normalized.
4. stale connection-generation fencing made explicit in both directions.
5. capability, partial-snapshot and liveness rollover edge cases closed.
6. stop-and-wait ambiguity replaced with bounded ordered CommandId pipelining.
7. replacement snapshot sequencing barrier added to prevent unbounded client buffering and state overtaking.

No material finding remained at merge.

## Cross-repository disposition

The older merged Platform protocol revision at `blakinio/Oteryn-Platform@c0b8703d326a04b43ae8e06f6192b0cb91c859b7` remains `RECONCILIATION_INPUT_ONLY` and is not final Oteryn-v2 protocol authority.

A later separately authorized Platform task must reconcile Gateway/World Registry/session-offer structures with accepted FND-02. No Platform write or production rollout was performed by this task.

## Next action

`FND-03` is the next ordered foundation gate. It must define the authoritative Rust runtime execution contract without redefining accepted FND-02 wire semantics.

`GAME-VISION-01` may continue in parallel where it does not alter accepted foundation boundaries.

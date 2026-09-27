# Oteryn v2 Implementation Live Allocations

> **CURRENT-STATE ROUTING ONLY.** Historical allocation ledgers are preserved in
> `docs/agents/evidence/OTV2-20260921-implementation-live-allocations-history.md`.
> Live GitHub Issue/PR/CI state and exact active task packets outrank this snapshot whenever they advance.

## Authority and reading rule

- Programme control plane: Issue #162.
- Active mutating profile: `OTV2_WORK_DELIVERY_COORDINATOR`.
- Admission protected Game main at this checkpoint: `86e25ab6c830159d9cb32aee1c3c8ff7726cdcd1`.
- This file records only current active/held routing and next gates.
- Completed allocations, old heads and superseded leases belong in task/archive/evidence, not here.

## Current programme lanes

| Lane | Live locator | Current state | Current rule |
| --- | --- | --- | --- |
| WP5 source composition | Issue #319; protected routing PR #739; active S2 PR #757 | `ACTIVE_S2_S3_COMPOSITION` | #416 routing is protected. Continue the same S2/source-composition lineages; S3-A remains separately gated. G0 is not yet proven. |
| Server Seam | Issue #247; `agent/otv2-gameplay-server-seam-01@9370b254c6ac4f6529e069c1968ae6bfa1e1750e` | `WAITING_WP5_G0` | Preserve the same branch/head. Resume only after fresh WP5 G0/source-composition readiness and overlap/custody reconciliation. |
| Content Item schema readiness | Issue #504; PR #749 | `ACTIVE_SCHEMA_READINESS` | The measured D6-M1 resource profile is accepted. Continue production typed schema/codec qualification without reimporting B1/Crystal or reminting the 38,157 identity map. |
| Agent control-plane simplification | Issue #745 | `WAITING_PHASE2_ALLOCATION` | Phase 1 is protected. Phase 2 may start only after a fresh path/ownership readback on a new canonical branch; no old #748 lease survives. |

## Shared serialization

Freshly reconcile before every writer release. The following remain one-writer-at-a-time when touched:

- root/app Cargo manifests and `Cargo.lock`;
- server/client composition roots;
- protocol/event/resource/stable-ID registries;
- shared governance/workflow surfaces;
- any path explicitly leased by another live task.

A historical lease never survives terminal merge/closeout by itself. An open Issue also does not imply an active path lease.

## Current dependency shape

```text
WP5 #319 S2/S3/source composition
  -> WP5_G0_READINESS_PROVEN
  -> resume SAME Server Seam #247
  -> downstream Client/QA -> Movement -> Combat

D6-M1 Item schema readiness #749 proceeds independently inside its current custody.
#745 Phase 2 is governance-only and requires a fresh bounded allocation.
```

## Next control-plane reactions

1. Continue the existing WP5 #319 S2/S3/composition lineages; do not create replacement workers.
2. Keep Server Seam frozen until `WP5_G0_READINESS_PROVEN`.
3. Continue #749 only inside D6-M1 schema-readiness scope; no source reimport or identity regeneration.
4. Allocate #745 Phase 2 only after fresh overlap/custody proof.
5. Refresh live state before every mutation/integration decision; this snapshot is not merge authority.

## Historical provenance

The former cumulative allocation ledger remains at
`docs/agents/evidence/OTV2-20260921-implementation-live-allocations-history.md`.
Use it only when a specific historical claim is material.

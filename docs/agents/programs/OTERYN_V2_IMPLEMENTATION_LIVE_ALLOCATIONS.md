# Oteryn v2 Implementation Live Allocations

> **CURRENT-STATE ROUTING ONLY.** Historical allocation ledgers are preserved in
> `docs/agents/evidence/OTV2-20260921-implementation-live-allocations-history.md`.
> Live GitHub Issue/PR/CI state and exact active task packets outrank this snapshot whenever they advance.

## Authority and reading rule

- Programme control plane: Issue #162.
- Active mutating profile: `OTV2_WORK_DELIVERY_COORDINATOR`.
- Protected Game main at this checkpoint: `40e9d723392b8fc1be652bdbb4b6d31b6729867b`.
- This file records only current cross-lane routing, active/held writers, shared serialization and next gates.
- Completed allocations, old heads and superseded leases belong in task/archive/evidence, not here.

## Current programme lanes

| Lane | Live locator | Current state | Current rule |
| --- | --- | --- | --- |
| WP3-A / SQLx resource closure | Issue #351; successor PR #673 | `PROTECTED_COMPLETE` | PR #673 merged as `3a384864d84560eb6ce76136afeb038576dc2976`. The old broad PR #356 is closed/unmerged evidence only; do not reactivate it as the production candidate. |
| WP4 / fresh-admission durability | Issue #329; PR #335 | `PROTECTED_COMPLETE` | PR #335 merged as `f02beb42523af6db1bb0c71d2840961e3fa5fcd0`. The terminal task is archived; no WP4 lease survives by history alone. |
| WP5 source composition | Issue #319; branches `agent/wp5-s2-native-admission-source-319`, `agent/wp5-postgres-ci-routing-416-material` | `ACTIVE_S2_AND_ROUTING` | S1 PR #735 is protected at `c59d8b25f9e3017d013d578a5a4bc9d93fa49e1d`. S2 remote head was reported at `27a2db74d40ae2577e8938a67bb7031dbedd8e23`; material #416 routing is active from protected main. G0 is not yet proven. |
| Server Seam | Issue #247; `agent/otv2-gameplay-server-seam-01@9370b254c6ac4f6529e069c1968ae6bfa1e1750e` | `WAITING_WP5_G0` | Preserve the same branch/head. Do not resume until fresh WP5 G0/source-composition readiness is proven and Work rechecks overlap/custody. |
| Content Item family-scale identity | Issue #504; PR #737; branch `agent/content-world-item-family-scale-504` | `QUALIFYING_DRAFT` | PR #737 remains open/draft at frozen head `0e643dbb5981599702b3407256e8ed03f1cad943`. Finish this canonical lineage; D6-M1 Item schema-readiness starts only after protected #737 readback. |
| Agent lifecycle hygiene | Issue #740; branch `docs/agent-hygiene-cleanup-740` | `IMPLEMENTING_PATH_DISJOINT` | Documentation/governance-only cleanup. No runtime/product authority. |

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
WP3 #673 PROTECTED_COMPLETE
  -> WP4 #335 PROTECTED_COMPLETE
  -> WP5 #319 S2 + #416 routing + later accepted composition gates
  -> WP5_G0_READINESS_PROVEN
  -> resume SAME Server Seam #247
  -> downstream Client/QA -> Movement -> Combat

Content Item #737 qualifies independently.
After #737 protects: release D6-M1 Item schema-readiness under fresh custody.
```

## Next control-plane reactions

1. Continue the existing WP5 #319 S2/routing/composition lineages; do not create replacement workers.
2. Keep Server Seam frozen until `WP5_G0_READINESS_PROVEN`.
3. Finish #737 only within its identity/resource scope; release Item schema-readiness only after protected readback.
4. Reconcile Issue #740 governance cleanup independently because its paths are documentation/governance-only.
5. Refresh live state before every mutation/integration decision; this snapshot is not merge authority.

## Historical provenance

The former cumulative allocation ledger remains at
`docs/agents/evidence/OTV2-20260921-implementation-live-allocations-history.md`.
Use it only when a specific historical claim is material.

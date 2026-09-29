# Oteryn v2 Implementation Live Allocations

> **CURRENT-STATE ROUTING ONLY.** Historical allocation ledgers are preserved in
> `docs/agents/evidence/OTV2-20260921-implementation-live-allocations-history.md`.
> Live GitHub Issue/PR/CI state and exact active task packets outrank this snapshot whenever they advance.

## Authority and reading rule

- Programme control plane: Issue #162.
- Active mutating profile: `OTV2_WORK_DELIVERY_COORDINATOR`.
- Admission protected Game main at this checkpoint: `961c74ab573d87807ed24cbd414a46d8072f72d3`.
- This file records only current active/held routing and next gates.
- Completed allocations, old heads and superseded leases belong in task/archive/evidence, not here.
- Per-task allocations are issued as #162 comments; the latest material disposition is #162 comment `5860494545`.

## Current programme lanes

| Lane | Live locator | Current state | Current rule |
| --- | --- | --- | --- |
| Door / world-object execution (CW4) | #162 comments `5860226213`, `5860394705`; Jira KAN-31 | `WAITING_CW3_MODEL` | Resumes on the same task after the CW3 local-object state model (`OTV2-20260928-cw3-local-object-state-model`) is protected. `revert_after` stays deferred until a Foundation-owned scope progression input exists. |
| Control-wire commands beyond step (use / door / item move) | `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` (only command type 1) | `WAITING_ARCHITECTURE` | Needs an owner decision in the #642 FIRST-CONTROL-WIRE-V1 pattern before any registry or seam writer. |
| Interaction "Use" orchestration | GAME-INTERACTION-01 successor (`PROPOSED / NONCANONICAL` except the chest USE slice: §5.1, §5.5, §17, §19.1, `reviews/OTERYN_GAME_D39_CHEST_USE_GAME_INTERACTION_AMENDMENT_DECISION_2026-09-29.md`); D37/D38 proposal (`CANDIDATE`) | `READY` for the chest USE slice only; `WAITING_ARCHITECTURE` for the rest | The D39 chest USE wiring child may get its own #162 allocation (CHEST-1 worker): plain `once` chests only, no keys, cooldowns, containers, weight or protocol. `Oteryn: impl interaction` stays read-only for every other interaction until the successor contract and the D37/D38 owners are accepted. |
| DUR-03 item transaction (backpack / pickup) | Issue #513; Jira KAN-12 | `WAITING_B3_ALLOCATION` | B2 is protected (#1031, #1038). B3/B4 need a fresh exact #162 allocation inside the single DUR-03 lineage; no parallel inventory writer. |
| Native client gameplay entry | `apps/client` (`PreNativeProtocol`) | `WAITING_WIRE_AND_CLIENT_QA_ALLOCATION` | Consumes only an accepted wire contract; native protocol entry belongs to the canonical Client/QA lane. |
| Native UI | `OTERYN_NATIVE_CLIENT_UI_IMPLEMENTATION_PROGRAMME_V1.md` | `UI-P0_ADMISSION_PENDING` | The L-CARGO custody cited by UI-P0 (#351, #356) is closed. P1 admission needs a fresh Cargo lease grant and an authorized P1 writer; input work waits on protected P1. |

Closed since the previous snapshot: WP5 G0 (`WP5_G0_READY`, #319 comment `5809797683`), Server Seam #247 (PR #823), ClientResume #822, Item schema readiness #749, control-plane simplification #745.

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
CW3 local-object state model -> CW4 world-object overlay (door execution)
architecture: GAME-INTERACTION-01 successor + D37/D38 acceptance -> impl interaction (other than the chest USE slice: CHEST-1 -> D39 chest USE wiring)
architecture: next control-wire owner decision -> seam command dispatch -> native client entry/click
#513 B3/B4 allocation -> pickup / inventory custody
L-CARGO release + P1 writer -> UI-P1 -> UI input
```

## Next control-plane reactions

1. Integrate the released Content/World allocations through their normal lifecycle; do not re-dispatch them.
2. Hand the architecture package (#162 comment `5860494545`, A1-A3) to the Supervising Architect; do not block other lanes on it.
3. Allocate #513 B3 only inside the DUR-03 lineage after a fresh custody readback.
4. Refresh live state before every mutation/integration decision; this snapshot is not merge authority.

## Historical provenance

The former cumulative allocation ledger remains at
`docs/agents/evidence/OTV2-20260921-implementation-live-allocations-history.md`.
Use it only when a specific historical claim is material.

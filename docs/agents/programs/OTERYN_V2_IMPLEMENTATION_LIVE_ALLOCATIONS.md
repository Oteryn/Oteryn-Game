# Oteryn v2 Implementation Live Allocations

> **CURRENT-STATE ROUTING ONLY.** The former cumulative ledger is preserved at
> `docs/agents/evidence/OTV2-20260921-implementation-live-allocations-history.md`.
> Do not append completed/historical checkpoint prose here. Live GitHub Issue/PR/CI state
> and exact active task packets outrank this snapshot whenever they advance.

## Authority and reading rule

- Programme control plane: Issue #162.
- Active mutating profile: `OTV2_WORK_DELIVERY_COORDINATOR`.
- This file records only current cross-lane routing, active/held writers, shared serialization and next gates.
- A task/PR/branch is not active merely because it appears here; fresh live readback is required before mutation.
- Completed allocations, old heads, superseded leases, repair counters and historical owner decisions belong in task/archive/evidence or immutable GitHub records, not this file.

## Current programme lanes

| Lane | Live locator | Current state | Current rule |
| --- | --- | --- | --- |
| WP3-A / SQLx resource closure | Issue #351, PR #673, branch `agent/wp3-a-upstream-first-351`, head `cacb03c3863d92c62ff5c12e58b0739e17f4fe70` | `QUALIFICATION_REVIEW_PENDING` | Owner-scoped first-slice repair is limited to `CommitOutcomeUnknown`; use upstream SQLx `close_on_drop()` / `close()` semantics and do not reopen deferred maintenance/panic hardening absent new first-slice evidence. Final exact-head review request: PR #673 comment `5761200059`. |
| Child B / WP4 durability | Issue #329, PR #335, head `834db1d7118d751e31287715d3eaac7780a0c7b9` | `HELD_UNTIL_WP3_PROTECTED` | Preserve canonical branch/history. No material release before protected WP3 terminal readback and fresh custody reconciliation. |
| WP5 source composition | Issue #319 | `HELD_UNTIL_WP3_PROTECTED` | Evidence/read-only work may continue; material source/bootstrap/composition mutation requires fresh post-WP3 allocation. |
| Server Seam | Issue #247, branch `agent/otv2-gameplay-server-seam-01` | `WAITING_DEPENDENCY` | No Server Seam material release before the accepted WP3 -> WP4/WP5 dependency gate closes. |
| Content/World native item batch | Issue #504 / programme #162, PR #719, head `1d2185dcb00abbbe73e0eb577266510533e79b3a` | `ACTIVE_DRAFT_PATH_DISJOINT` | First bounded 64-item native binding batch. No spatial, shared CW3 model, runtime, protocol, persistence, WP3, root Cargo or client scope. |
| Physical Content spatial slice | Reference evidence programme #483 / Content #504 | `BLOCKED_EXTERNAL_EVIDENCE` | First blocker remains accepted immutable-target spatial evidence for a selected real placement; do not invent/backdate target truth. |
| CONTENT-QUEST-01 evidence package | PR #709, head `6074b06786c1b7c8d57712b649ee62b5d7577a7e` | `QUALIFIED_STABLE_CANDIDATE` | Keep stable; integration follows the repository-governed Merge Queue route only. |

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
WP3-A #673
  -> protected terminal readback
  -> fresh WP4/WP5 custody reconciliation
  -> Server Seam #247

Path-disjoint Content #719 may qualify independently.
Physical spatial Content remains blocked on Reference evidence, not on #719.
```

## Next control-plane reactions

1. Consume the terminal exact-head review for PR #673; if current-gate clean, proceed through governed integration without reopening deferred hardening.
2. After protected WP3 readback, refresh #329/#335, #319 and #247 ownership before releasing any material downstream writer.
3. Allow PR #719 to qualify independently while its paths remain disjoint.
4. Do not allocate a physical spatial writer until the Reference spatial evidence gate is accepted.

## Historical provenance

The complete former 1,120-line allocation ledger is preserved verbatim at
`docs/agents/evidence/OTV2-20260921-implementation-live-allocations-history.md`.
Use it only when a specific historical claim is material.

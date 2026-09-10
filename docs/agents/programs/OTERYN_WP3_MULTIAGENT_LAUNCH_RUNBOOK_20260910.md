# Oteryn WP3 Multiagent Launch Runbook

Operator guidance for accelerating the existing WP3 SQLx-driver-accounting lineage without creating parallel mutation. This runbook is subordinate to root/nearest `AGENTS.md`, the immutable META binding, current Issue/task/allocation/PR/head/check/review state and the active Game control plane.

It grants no write, merge, production or owner-funded AI authority.

## Topology

Recommended topology for this WP3 slice is three roles:

```text
Oteryn: wp3 writer
  -> exactly one mutating writer

Oteryn: wp3 tls audit
  -> READ_ONLY, may run in parallel

Oteryn: wp3 qualification audit
  -> READ_ONLY, may run in parallel
```

The two audit roles are acceleration helpers only. They do not consume the writer slot, do not replace formal independent review and may not mutate GitHub or tracked files.

## Recommended execution configuration

These are operator recommendations, not part of repository authority. If the execution surface exposes different names, use the closest supported configuration; model/effort choice never widens scope or changes review/merge requirements.

| Role | Recommended model | Recommended effort |
| --- | --- | --- |
| `Oteryn: wp3 writer` | GPT-5.6 Sol | High |
| `Oteryn: wp3 tls audit` | GPT-5.6 Sol | High |
| `Oteryn: wp3 qualification audit` | GPT-5.6 Sol | Medium |

Use the writer at High because TLS/SQLx ownership and lifetime reasoning is the dominant correctness risk. Use the TLS auditor at High for independent allocation/lifetime falsification. Medium is sufficient for qualification reconciliation because it is primarily live-state/checklist work.

Before consuming owner-funded, personal-quota or metered AI resources, apply `docs/agents/OWNER_FUNDED_AI_POLICY.md` and any authorization already valid for the current operation.

## Launch order

1. Fresh-resolve protected `main`, Issue #351, PR #356, canonical branch `agent/sqlx-driver-budget-351`, current ownership/authority and exact head.
2. Start or resume exactly one `Oteryn: wp3 writer` on that canonical lineage.
3. Optionally launch `Oteryn: wp3 tls audit` and `Oteryn: wp3 qualification audit` in separate chats at the same time.
4. Each auditor returns its explicit exact-head packet to the writer/control plane.
5. Before consuming a packet, the writer refreshes GitHub and rejects head-bound conclusions whose `exact_pr_head_sha` no longer matches the audited candidate.
6. The writer verifies every accepted finding itself and remains the only mutating WP3 role.

## Packet relay

Relay analyst results with this compact directive:

```text
Consume the attached WP3 audit packet. Refresh live GitHub first. Reject stale exact-head conclusions, verify every accepted finding yourself, stay inside current protected WP3 authority, and continue as the single mutating writer. Do not delegate repository mutation to audit roles.
```

## Completion boundary

The writer may return `READY_FOR_FINAL_INDEPENDENT_REVIEW` only after material WP3 implementation and runtime qualification are complete on one stable exact head. The two helper audits do not satisfy the required final independent HIGH-risk review.

`WP3_PROTECTED_COMPLETE` requires the later control-plane sequence selected by current policy, including the required independent exact-head review, canonical CI, truthful Ready state, native FULL Merge Queue, real `merge_group` aggregate `game-gate`, protected-main readback and shared PostgreSQL-test custody release.

After verified `WP3_PROTECTED_COMPLETE`, retire these WP3-specific aliases in `PROMPT_LIFECYCLE.json`; a future resource-accounting task must not treat the historical #351/#356 aliases as authority.

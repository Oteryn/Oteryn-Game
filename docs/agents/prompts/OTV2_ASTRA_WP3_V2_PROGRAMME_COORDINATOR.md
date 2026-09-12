# OTV2 WP3-v2 Programme Coordinator

Short invocation:

```text
Oteryn: astra wp3-v2 programme coordinator
```

## Outcome

Coordinate the existing Game programme from current live state through WP3-v2 architecture acceptance/implementation, canonical Child B, real source/WP5 composition, fresh G0 and resume of the existing Server Seam, ending only at `SERVER_SEAM_READY_FOR_INTEGRATION` or one precise externally owned blocker.

## Authority / scope delta

- Repository: `Oteryn/Oteryn-Game`.
- Coordination locators: #162 and #364.
- This role allocates/orders work only within authority already granted by live repository state; it does not invent path leases, Platform write authority, production authority or merge authority.
- Preserve existing canonical worker lineages. Do not create replacement #356, #335 or #247 workers merely because they are blocked.
- Prefer at most 2-3 concurrent agents and one mutating owner per material lane.

## Required live reconciliation

Before every material gate, refresh protected `main`, applicable AGENTS/META binding, #162, #364, #247, #319, #329/#335, #351/#356, #588, #590, active task allocations/path overlaps and exact heads/checks relevant to the decision.

Use `docs/agents/programs/OTV2_WP3_V2_MULTI_AGENT_DELIVERY_PROGRAMME.md` as the programme DAG. Historical coordinates inside that document are evidence only.

## Current WP3 architecture state

Protected `main` now contains Revision 3 of `WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1` from merged PR #590. The artifact itself remains `CANDIDATE / NOT ACCEPTED` and grants no A4 implementation authority merely because it is present on protected main.

A1 must therefore close/validate that exact successor, not author a second B-vs-C decision. A2 supports exact-source/evidence closure. A4 starts only after live repository state proves architecture acceptance plus a fresh #162/#364 implementation allocation and path/custody reconciliation.

## Dispatch order

1. Admit A1, A2 and A3 when live authority permits. A1 closes the protected Revision-3 candidate; A2 is read-only; A3 is a Platform-owned task and this Game prompt does not grant Platform writes.
2. Hold A4 material mutation until Revision 3 is accepted through repository controls and exact implementation allocation/custody exists.
3. Keep A5 read-only on shared WP3/Cargo/Durability surfaces until protected WP3 implementation and live custody release allow canonical #335 work.
4. Allow A6 path-disjoint source-readiness work according to live #319/WP5 allocations; do not activate gated SQL/workflow/source paths early.
5. After exact WP3 and Child B protected delivery, compose real Platform/Game source work and fresh G0.
6. Only after fresh G0, resume the existing #247 Server Seam worker/branch and alias.

## Gate discipline

- Protected presence of Revision 3 is not architecture acceptance by itself.
- A1 closure is not implementation authority.
- A2 evidence is not architecture acceptance.
- Q01-Q75 terminal classification is mandatory for final WP3 implementation qualification.
- Platform local PASS is not cross-repo S3 PASS.
- G0 is not G1 and is not Server Seam terminal success.
- CI green is a checkpoint, not completion.

## Acceptance

Maintain one compact programme ledger with lane, live head, canonical owner, state, blocker, next action and evidence. Terminal success requires the criteria in the programme plan and existing repository controls. If blocked, name exactly one blocker with owner/capability, affected dependency, evidence, why it cannot be resolved under current authority and the smallest required action.

## Mandatory next-agent instruction

At the end of every completed or blocked response, tell the owner exactly which agent/session to launch next. Resolve this from fresh live programme state; do not blindly repeat a historical order.

Use this exact footer:

```text
NEXT_AGENT: <one exact alias, up to three independent aliases, or NONE>
RUN_WHEN: <gate/state that makes the launch valid>
WHY: <one concise dependency reason>
```

When several independent lanes are simultaneously legal, list at most three aliases in launch order. When the programme is terminal, use `NEXT_AGENT: NONE`. When a material blocker prevents the next worker from starting, name the blocked alias in `NEXT_AGENT`, state the precise gate in `RUN_WHEN`, and do not fabricate readiness.
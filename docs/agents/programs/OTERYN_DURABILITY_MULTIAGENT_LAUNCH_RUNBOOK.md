# Oteryn Durability Multiagent Launch Runbook

## Purpose

Provide one operator-facing procedure for running the canonical Durability multiagent pattern already registered in this repository:

- one mutating `Oteryn: sol durability lead`;
- three strict read-only Durability analyst roles;
- one uniquely active programme control plane in ChatGPT Work.

This runbook is subordinate to protected-main root/nearest `AGENTS.md`, live GitHub Issue/task/allocation/PR/head/check/review state, the current Terra/Sol scheduler, accepted architecture/contracts and the current META-owned AI review policy. Historical PR/Issue numbers in examples are provenance only.

## Source of truth before every launch

Before launching or resuming any role, resolve from live GitHub:

1. protected `main` SHA;
2. the current coordinator Issue/task and uniquely active mutating control-plane profile;
3. the current Durability Issue/task/allocation, branch, PR and exact head SHA;
4. current unresolved review findings/threads and exact-head checks;
5. current root and nearest `AGENTS.md`;
6. `docs/agents/PROMPT_LIFECYCLE.json` and the current prompt bodies;
7. this runbook and `OTERYN_V2_TERRA_SOL_EXECUTION_SCHEDULER.md`.

Never launch against a cached SHA, stale chat summary or historical task body when live GitHub has moved.

## Canonical topology

```text
ChatGPT Work
  Oteryn: work coordinator OR Oteryn: terra game coordinator
  -> exactly one mutating control plane selected by live lifecycle

Separate Chat #1
  Oteryn: sol durability lead
  -> GPT-5.6 Sol
  -> Extra High / highest available
  -> SINGLE MUTATING WRITER

Separate Chat #2
  Oteryn: sol durability authority analyst
  -> GPT-5.6 Sol
  -> High
  -> READ_ONLY

Separate Chat #3
  Oteryn: sol durability continuity analyst
  -> GPT-5.6 Sol
  -> High
  -> READ_ONLY

Separate Chat #4
  Oteryn: sol durability qualification analyst
  -> GPT-5.6 Sol
  -> High
  -> READ_ONLY
```

Do not run a second mutating Durability Lead for the same canonical task branch/PR. Do not upgrade an analyst into a writer by chat instruction, model choice or tool availability.

## Launch procedure

### Step 1 — prove the active control plane

In the already-selected ChatGPT Work control-plane session, refresh live GitHub and prove which profile is active. `Oteryn: work coordinator` and `Oteryn: terra game coordinator` are mutually exclusive for mutating control-plane work.

The control plane owns deterministic lifecycle/allocation/integration actions. It does not become the technical Durability writer.

### Step 2 — launch the single writer

Open a separate normal Chat session.

Recommended model/effort:

```text
GPT-5.6 Sol
Extra High / highest available
```

Invoke exactly:

```text
Oteryn: sol durability lead
```

The lead must independently refresh live GitHub, prove the current merged allocation and exact owned paths, preserve the valid existing task branch/PR and remain the only writer for that lane.

### Step 3 — launch the three analysts in parallel

Open three additional separate normal Chat sessions. Use GPT-5.6 Sol with High effort for each.

Chat #2:

```text
Oteryn: sol durability authority analyst
```

Chat #3:

```text
Oteryn: sol durability continuity analyst
```

Chat #4:

```text
Oteryn: sol durability qualification analyst
```

These chats may run concurrently because all three are strict read-only roles. They must independently resolve the current live Durability PR head and return only their defined advisory packets.

They MUST NOT edit files, create commits/branches, push, mutate PR/Issue/comment/review state, trigger workflows or external AI review, claim a lease, merge, close or change architecture authority.

## Analyst domains

### Authority analyst

Primary concern: Foundation/current-authority correctness, especially actual current runtime scope and final-COMMIT revalidation facts.

Expected output:

```text
AUTHORITY_ANALYSIS_PACKET
```

### Continuity analyst

Primary concern: continuity/protection shape, replacement transaction ordering, rollback and PREPARE/COMMIT invariant coherence.

Expected output:

```text
CONTINUITY_ANALYSIS_PACKET
```

### Qualification analyst

Primary concern: whole-diff consistency, regression gaps, protected-main drift, validation invalidation and final qualification plan.

Expected output:

```text
QUALIFICATION_ANALYSIS_PACKET
```

## Handoff from analysts to the writer

Separate Chat sessions do not imply hidden cross-chat state sharing. The operator/requester must relay each analyst's explicit returned packet to the Durability Lead when using separate chats.

Copy the complete packet bodies into the Durability Lead chat and use this handoff directive:

```text
Consume these analyst packets. Refresh live GitHub first. Reject any packet whose exact_pr_head_sha no longer matches the head it analyzed. Reconcile overlapping findings, verify every accepted finding yourself, reject suggestions outside the current allocation or accepted architecture, and continue as the single mutating Durability writer. Do not delegate repository mutation to analyst roles.
```

The Lead must not mechanically implement analyst recommendations. It owns technical synthesis, implementation, TDD evidence, commits/pushes, protected-main reconciliation, whole-diff self-review and final lane handoff.

## Stale-head rule

Every packet is exact-head evidence.

If the live Durability PR head changes after an analyst froze its target:

- the old packet remains historical evidence only;
- the Lead may use still-valid conceptual observations, but must not treat stale exact-head conclusions as current qualification;
- rerun only the analyst domains whose evidence was materially invalidated by the head change;
- do not rerun analysts merely because metadata or a non-risk-bearing docs line changed if their analyzed technical candidate remains representative.

The Lead must always perform its own fresh live-head verification before mutation.

## Rerun guidance after writer changes

Use analyst reruns selectively:

- material Foundation/current-authority repair -> rerun authority analyst when independent re-analysis would materially reduce risk;
- material continuity/transaction/persistence repair -> rerun continuity analyst when its previous packet is no longer representative;
- material whole-diff shape, protected-main merge-up or qualification invalidation -> rerun qualification analyst when useful;
- trivial metadata/formatting-only head movement -> do not automatically rerun all three.

Analyst fanout is acceleration, not a dependency gate. Authorized Durability work must not stall solely because an analyst session is unavailable.

## Writer completion sequence

After consuming current analyst packets, the Durability Lead proceeds autonomously inside the exact live allocation:

```text
verify live head + authority
  -> establish/retain required TDD RED evidence
  -> implement minimal semantically complete GREEN
  -> focused validation
  -> real PostgreSQL/component validation when applicable
  -> whole-diff self-review
  -> reconcile current protected main using normal non-force history-preserving integration when required
  -> rerun every validation layer invalidated by the resulting exact head
  -> apply current META-owned AI review policy to the stable material candidate
  -> repair actionable findings inside existing authority if any
  -> revalidate only what the repair invalidated
  -> return READY_FOR_INTEGRATION when all current predicates are proven
```

The Lead never merges its own lane under this profile.

## AI review boundary

The three analyst chats are internal read-only reasoning aids. They do not satisfy formal external AI review requirements.

Current root `AGENTS.md` and the organization AI review policy adopted by reference determine whether external review is selected. External AI review is advisory and never merge authority. Do not recreate obsolete local review tiers/controllers.

For material high-risk Durability candidates involving session/reconnect/fencing/durable persistence/schema risk, use the review route selected by current protected-main policy on a stable material candidate. Re-review only when a material risk-bearing repair makes the previous review no longer representative.

## Integration handoff

When the Durability Lead returns `READY_FOR_INTEGRATION`, the uniquely active control plane independently verifies:

- exact final PR head;
- current `main` relation;
- exact changed paths/leases;
- applicable checks;
- required review evidence under current policy;
- unresolved actionable findings/threads;
- no unauthorized scope expansion.

Only the control-plane/integration path may perform the terminal merge according to current repository rules. After merge, read back protected `main` before releasing a dependent lane.

## Operator copy/paste sheet

### Writer

```text
Oteryn: sol durability lead
```

Run in: separate Chat

Model: GPT-5.6 Sol

Effort: Extra High / highest available

Mode: single mutating writer

### Authority analyst

```text
Oteryn: sol durability authority analyst
```

Run in: separate Chat

Model: GPT-5.6 Sol

Effort: High

Mode: read-only

### Continuity analyst

```text
Oteryn: sol durability continuity analyst
```

Run in: separate Chat

Model: GPT-5.6 Sol

Effort: High

Mode: read-only

### Qualification analyst

```text
Oteryn: sol durability qualification analyst
```

Run in: separate Chat

Model: GPT-5.6 Sol

Effort: High

Mode: read-only

### Packet relay to writer

```text
Consume these analyst packets. Refresh live GitHub first. Reject any packet whose exact_pr_head_sha no longer matches the head it analyzed. Reconcile overlapping findings, verify every accepted finding yourself, reject suggestions outside the current allocation or accepted architecture, and continue as the single mutating Durability writer. Do not delegate repository mutation to analyst roles.
```

## Historical note

PR #270 introduced the canonical analyst aliases and single-writer fanout model. PR #252 is historical evidence of the motivating terminal-session replacement lane and must not be assumed to be the current target for future invocations. Every launch resolves the current live Durability lifecycle instead.
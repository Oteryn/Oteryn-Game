# Oteryn Durability Multiagent Launch Runbook

This runbook is operator guidance for the reusable Durability multiagent pattern. It is subordinate to root/nearest `AGENTS.md`, the immutable META policy binding, live GitHub Issue/task/allocation/PR/head/check/review state, accepted architecture/contracts and the uniquely active Game control plane.

It does not grant write, merge, production, external-repository or owner-funded AI authority.

## Purpose

Use one technical Durability writer plus optional read-only analyst fanout without creating duplicate mutation, stale-head qualification or a second control plane.

The canonical topology is:

```text
ChatGPT Work
  exactly one live-selected mutating control plane

separate chat
  Oteryn: sol durability lead
  -> SINGLE MUTATING WRITER only when an exact current allocation grants it

separate chat
  Oteryn: sol durability authority analyst
  -> READ_ONLY

separate chat
  Oteryn: sol durability continuity analyst
  -> READ_ONLY

separate chat
  Oteryn: sol durability qualification analyst
  -> READ_ONLY
```

The three analysts are optional acceleration roles. They consume no writer slot and must never become repository writers by chat instruction, model choice or tool availability.

## Resolve live authority first

Before launching or resuming any role, refresh only the live facts material to the current Durability lifecycle:

1. protected `main` SHA;
2. current root and nearest `AGENTS.md`;
3. `docs/agents/META_AGENT_POLICY_BINDING.json` and the bound policy when material;
4. current coordinator Issue/task and `active_control_plane_profile`;
5. current Durability Issue/task/allocation, branch, PR and exact head SHA;
6. current overlapping path/shared-lease ownership;
7. exact-head checks, reviews and unresolved review threads for the candidate;
8. `docs/agents/PROMPT_LIFECYCLE.json`, the selected role prompts and the current scheduler/DAG when lifecycle routing is material.

Live GitHub lifecycle state overrides stale task prose, cached SHAs, prior chat summaries and historical PR descriptions.

A reusable alias, open Issue, old branch, tool connection or previously green check is not current mutation authority.

## Execution configuration

Do not pin a model, effort tier or maximum reasoning setting in this runbook. Use the execution configuration currently exposed and authorized for the task.

Model/effort selection never changes repository authority, path ownership, review requirements or single-writer rules.

Before consuming owner-funded, personal-quota or metered AI/API resources, apply `docs/agents/OWNER_FUNDED_AI_POLICY.md` and any authorization already established for the current session.

## Launch sequence

### 1. Prove the single control plane

In ChatGPT Work, refresh the current coordinator lifecycle and prove exactly one mutating control-plane profile.

If control-plane identity is ambiguous, all control-plane mutation fails closed until the conflict is resolved. Do not use a Durability chat to choose or replace the control plane.

### 2. Prove the canonical Durability lineage

Resolve the current Durability allocation and determine whether a canonical implementation branch/PR already exists.

If it exists, preserve that lineage. Do not create a replacement branch, duplicate PR or second writer merely because the previous worker is inactive, `WAITING_EXTERNAL`, `STALLED`, rotated or no longer present in chat.

A replacement lineage is allowed only when current protected authority explicitly retires the previous one and allocates a successor.

### 3. Start or resume the single writer

Invoke:

```text
Oteryn: sol durability lead
```

The lead must independently refresh GitHub before mutation, verify the exact current allocation and owned paths, and remain the only mutating writer for the canonical Durability branch/PR.

The lead must preserve unrelated work and stop at `SHARED_LEASE_REQUIRED`, architecture, permission or dependency boundaries rather than silently widening scope.

### 4. Optionally launch the three analysts

Invoke in separate chats when their domains are useful for the current exact candidate:

```text
Oteryn: sol durability authority analyst
Oteryn: sol durability continuity analyst
Oteryn: sol durability qualification analyst
```

Each analyst freezes the exact target it inspected and stays read-only. Analysts must not:

- edit tracked files;
- create or move branches;
- commit or push;
- mutate Issues, PR metadata, comments, reviews or review threads;
- dispatch or rerun workflows;
- request or consume a write lease;
- merge, close or approve lifecycle state;
- change architecture or production authority.

### 5. Return explicit analyst packets

Each packet should identify at minimum:

```yaml
target_repository:
target_issue:
target_pr:
exact_pr_head_sha:
observed_main_sha:
analysis_domain:
findings:
evidence:
unknowns:
```

Separate chats do not imply hidden cross-chat state sharing. Only an explicit returned packet may be relayed to the writer.

### 6. Writer consumes packets only after revalidation

Relay packets to the Durability Lead with this directive:

```text
Consume these analyst packets. Refresh live GitHub first. Reject exact-head conclusions whose exact_pr_head_sha no longer matches the candidate they analyzed. Reconcile overlapping findings, verify every accepted finding yourself, reject suggestions outside the current allocation or accepted architecture, and continue as the single mutating Durability writer. Do not delegate repository mutation to analyst roles.
```

The lead owns synthesis and must not mechanically implement analyst suggestions.

## Stale-head and rerun rules

Every analyst packet is exact-head evidence.

If the candidate head changes:

- the old packet remains historical evidence;
- conceptual observations may be reused only after the lead verifies they still apply;
- rerun only analyst domains materially invalidated by the change;
- do not fan out all analysts again for metadata-only or non-risk-bearing movement;
- never treat an old green review/check as qualification of a new material head.

Useful rerun examples:

- Foundation/current-authority repair -> authority analyst when independent re-analysis materially reduces risk;
- continuity/transaction/recovery repair -> continuity analyst when the prior packet is no longer representative;
- whole-diff shape, protected-main reconciliation or validation invalidation -> qualification analyst when useful.

Analyst availability is not a dependency gate. Authorized writer work continues when it can legally progress without a new packet.

## Writer completion discipline

The lead proceeds autonomously inside the exact allocation:

```text
refresh live authority/head
  -> retain or establish required RED evidence
  -> implement the smallest semantically complete GREEN
  -> focused validation
  -> real PostgreSQL/component qualification when applicable
  -> whole-diff self-review
  -> reconcile current protected main without force/reset/replacement history
  -> rerun validation invalidated by the resulting head
  -> apply current risk-based review policy when selected
  -> repair actionable findings inside existing authority
  -> revalidate only what the repair invalidated
  -> return READY_FOR_INTEGRATION only when every current predicate is proven
```

Do not use no-op/retrigger commits, unchanged heavy reruns or test suppression as progress.

## Waiting, stalled and blocked states

`WAITING_EXTERNAL` and `STALLED` do not make a branch free for a replacement writer. Preserve the canonical lineage and follow its current resume rule.

When a dependency clears, resume the same worker lineage if current authority says so. When it does not clear, continue other authorized path-disjoint work rather than seizing the blocked paths.

Material architecture conflicts return `ARCHITECTURE_ESCALATION_REQUIRED`. Owner-only product/scope/authority choices return `OWNER_DECISION_REQUIRED`.

## Review and integration boundary

The three analyst chats are internal reasoning aids. They are not formal independent AI review and do not create a required GitHub status.

Resolve external-review selection from the current META-bound policy. External AI review is advisory and never merge authority.

When the lead returns `READY_FOR_INTEGRATION`, the uniquely active control plane independently verifies:

- exact final PR head;
- current protected-main relation;
- exact changed paths and leases;
- applicable exact-head checks;
- selected review evidence;
- unresolved actionable findings/threads;
- no scope or authority expansion.

Only the authorized integration path may perform terminal Merge Queue/merge actions. Read back protected `main` before releasing dependent ownership.

## Execution routing safety

Prefer repository-native GitHub and CI paths. Remote Desktop is exception-only and requires the exact explicit authorization required by current policy; availability alone is not permission.

Do not weaken tests, provenance, compatibility, authorization, branch protection or Merge Queue to unblock a Durability candidate.

## Historical note

PR #270 introduced the reusable one-writer plus three-read-only-analyst Durability pattern. PR #252 and older Durability branches are historical provenance only unless current live GitHub explicitly identifies one as the canonical active lineage.

Every launch or resume resolves the current lifecycle again from live GitHub.
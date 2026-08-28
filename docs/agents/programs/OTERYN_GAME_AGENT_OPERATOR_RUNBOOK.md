# Oteryn Game Agent Operator Runbook

This runbook is the owner-facing operational map for launching and supervising the reusable Oteryn Game agent architecture. It is a coordination aid subordinate to root/nearest `AGENTS.md`, `docs/agents/CODEX_REVIEW_POLICY.json`, current protected `main`, live Issues/tasks/PRs, accepted contracts and the canonical scheduler.

## Non-negotiable source order

Before telling the owner what to launch or before any agent acts on this runbook, resolve from live GitHub:

1. repository identity and protected `main` SHA;
2. current coordinator Issue/task and uniquely active control-plane profile;
3. all active task packets and their Issue/branch/PR/head bindings;
4. open PRs and recently merged PRs needed to prove dependency completion;
5. exact-head checks, reviews and unresolved review threads for active candidates;
6. `docs/agents/programs/OTERYN_V2_TERRA_SOL_EXECUTION_SCHEDULER.md`;
7. `docs/agents/CODEX_REVIEW_POLICY.json`;
8. current prompt bodies/registry and accepted architecture/contracts material to the lane.

Never treat this runbook, an alias, an old Issue number, a prior chat summary or a cached worktree as proof of current status. If live GitHub conflicts with prose here, live canonical governance wins.

## Product surface, model and effort map

| Alias / function | Where to run | Recommended model | Effort | Default mutation mode |
| --- | --- | --- | --- | --- |
| `Oteryn: work coordinator` | ChatGPT Work | Terra High when using Work's deterministic control-plane profile | High / deterministic | CONTROL_PLANE only when live lifecycle selects it |
| `Oteryn: terra game coordinator` | ChatGPT Work | Terra High | High / deterministic | CONTROL_PLANE only when live lifecycle selects it; otherwise recovery read-only |
| `Oteryn: sol supervising architect` | separate chat | GPT-5.6 Sol | Extra High / highest available | ON_DEMAND architecture decision only; no merge authority |
| `Oteryn: sol durability lead` | separate chat | GPT-5.6 Sol | Extra High / highest available | MUTATING only with exact current allocation |
| `Oteryn: sol server seam lead` | separate chat | GPT-5.6 Sol | Extra High / highest available | MUTATING only after exact prerequisite/allocation; otherwise read-only prep |
| `Oteryn: sol client qa lead` | separate chat | GPT-5.6 Sol | Extra High / highest available | MUTATING only after exact prerequisite/allocation; otherwise read-only prep |
| `Oteryn: sol movement lead` | separate chat | GPT-5.6 Sol | Extra High / highest available | MUTATING only after exact resource/dependency gate and allocation |
| `Oteryn: sol combat lead` | separate chat | GPT-5.6 Sol | Extra High / highest available | MUTATING only after exact Movement/dependency gate and allocation |
| `Oteryn: work auditor` | separate independent chat | GPT-5.6 Sol | highest available | AUDIT_READ + bounded GitHub evidence-write only |
| `Oteryn: owner execution guide` | separate chat | GPT-5.6 Sol | Extra High / highest available | READ_ONLY owner guidance only |
| `Oteryn: sol post-vsl expansion` | separate chat | GPT-5.6 Sol | Extra High / highest available | read-only-by-default decomposition |
| `Oteryn: sol world content prep` | separate chat | GPT-5.6 Sol | High; Extra High for cross-contract complexity | READ_ONLY until later exact allocation |
| `Oteryn: sol npc ai prep` | separate chat | GPT-5.6 Sol | High; Extra High for cross-contract complexity | READ_ONLY until later exact allocation |
| `Oteryn: sol systems economy prep` | separate chat | GPT-5.6 Sol | High; Extra High for durable-value/custody complexity | READ_ONLY until later exact allocation |
| `Oteryn: sol tooling ops prep` | separate chat | GPT-5.6 Sol | High; Extra High for cross-surface complexity | READ_ONLY until later exact allocation |
| native GitHub Codex review | canonical GitHub PR | repository-native Codex review capability | not owner-selected here | independent review only; policy-bounded |

Model choice never grants authority. A profile can mutate only when live canonical allocation/control-plane selection grants it.

## Work versus separate chats

Use exactly one active mutating control plane in ChatGPT Work. Do not run Work Coordinator and Terra Game Coordinator as simultaneous mutating schedulers. The inactive profile may only prepare recovery/transfer analysis.

Run technical Sol roles in separate chats so each lane has an independent context and clear ownership. A Sol chat may be open for read-only preparation before promotion, but it must not write until its exact prerequisite and allocation are canonical.

Keep `Oteryn: work auditor` in a separate non-authoring chat when its result must count as independent. Keep `Oteryn: owner execution guide` read-only; it advises the owner and never becomes a second coordinator.

## Canonical launch and promotion discipline

Do not hard-code the current active lane. Recompute the scheduler from live GitHub every time. The canonical VSL dependency shape is:

```text
Durability
  -> Server Seam
  -> Client / QA
  -> exact Movement resource/dependency gate
  -> Movement
  -> Combat
  -> VSL terminal closeout
  -> post-VSL expansion
```

Parallelize only read-only preparation or proven path-disjoint allocated mutation allowed by the scheduler. Never launch a writer merely to fill capacity.

`Oteryn: sol supervising architect` is on-demand, not a permanent lane. Launch it when a material packet returns `ARCHITECTURE_ESCALATION_REQUIRED`. Product/scope/authority choices outside accepted architecture return `OWNER_DECISION_REQUIRED`.

## Autonomous Codex technical-review loop

Every allocated mutating lane with a PR must apply `docs/agents/CODEX_REVIEW_POLICY.json` before `READY_FOR_INTEGRATION`.

When the validated route is `CODEX_REQUIRED` and native GitHub Codex review capability is proven:

```text
lane lead freezes exact candidate head
  -> lane lead requests fresh @codex review on canonical PR
  -> Codex returns exact-head evidence/findings
  -> lane lead repairs findings only inside existing allocation
  -> material head movement invalidates prior qualification
  -> lane lead revalidates exact head
  -> lane lead requests fresh @codex review
  -> exact-head PASS / no-suggestions evidence + zero blocking findings/required threads
  -> lane lead may return READY_FOR_INTEGRATION if every other gate also passes
```

The owner is not the default message relay for covered runs. Work/Terra do not request the technical review for the lane lead, do not invent risk tags, do not adjudicate technical findings and cannot waive `CODEX_REQUIRED`; they mechanically validate canonical classification and exact-head evidence.

A Codex review never replaces required author self-review or an explicitly separate governance/lifecycle audit. A Codex task/session that materially authored or modified the candidate cannot count as the independent reviewer.

Every mutating Sol lane handoff should expose:

```yaml
codex_review:
  route: CODEX_REQUIRED | CODEX_OPTIONAL | CODEX_NOT_REQUIRED_BY_THIS_POLICY
  classification_source_role:
  classification_source_ref:
  reviewed_head_sha:
  evidence_ref:
  blocking_findings: []
  required_review_threads_unresolved: 0
  status: PASS | CHANGES_REQUIRED | NOT_REQUIRED | WAITING_CAPABILITY
```

## Work Auditor loop

Any canonical Oteryn Game agent or the owner may request `Oteryn: work auditor` for a uniquely identifiable PR/Issue/task/head. The auditor independently resolves and freezes the target, audits it, and persists the required bounded GitHub evidence note.

The auditor may verify Codex routing/evidence as part of an audit. It must not become a nested Codex dispatcher, implementation worker, merge role or control plane. If it authored/materially changed the target in another role, its result cannot satisfy a genuinely independent review requirement.

## Live status classification

Classify each material lifecycle item only after fresh GitHub reconciliation:

- `DONE` — terminal merged/closed state is proven, required closeout/ownership release is canonical, and no later corrective lifecycle reopens the requirement.
- `ACTIVE` — a live Issue/task has current ownership and authorized work is in progress on its exact branch/PR/head.
- `BLOCKED` — current work cannot legally or technically advance until a named predicate becomes true; name the exact blocker and owner.
- `READY_NEXT` — all canonical prerequisites for starting/promoting the next lane are proven and exact allocation may be released or resumed.
- `DO_NOT_LAUNCH` — no current allocation, a predecessor/gate is incomplete, another control plane/lane owns the surface, or launching would create duplicated/unauthorized mutation.
- `UNKNOWN` — current evidence is insufficient or conflicting; do not upgrade to another state by inference.

A draft PR is not `DONE`. Green CI on an old SHA is not `DONE`. A historical task saying `completed` is not enough when the live Issue/PR or a newer corrective lifecycle contradicts it.

## Owner-facing execution report

When the owner asks what to run now, return this compact structure based on fresh GitHub truth:

```yaml
OTERYN_GAME_OWNER_EXECUTION_REPORT:
  protected_main_sha:
  active_control_plane:
    alias:
    where: WORK
    model:
    effort:
  run_now:
    - alias:
      where: WORK | SEPARATE_CHAT
      model:
      effort:
      mode: MUTATING | READ_ONLY | AUDIT | ON_DEMAND
      target:
      reason:
  keep_open_read_only: []
  do_not_launch: []
  done: []
  active: []
  blocked: []
  ready_next: []
  codex_reviews_required_now: []
  audits_required_now: []
  owner_decisions_required: []
  exact_next_action: <exactly one concrete action>
```

Do not list a role in `run_now` merely because its alias exists. Every mutating entry must cite/prove its live allocation and dependencies.

## Owner interruption policy

The owner should normally be interrupted only for `OWNER_DECISION_REQUIRED`, a real missing permission/capability that cannot be resolved through authorized repository-native paths, or a safety/authority conflict. Do not use the owner as a manual relay between a lane lead and Codex or between a requesting agent and Work Auditor.

## Reuse

The reusable short aliases are catalogued in `docs/agents/prompts/README.md`. The machine-readable prompt registry is `docs/agents/PROMPT_LIFECYCLE.json`. Always resolve both from protected `main` before relying on an alias or recommended model/effort.

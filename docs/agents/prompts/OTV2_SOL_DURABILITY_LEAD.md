# OTV2 Sol Durability Lead

Short invocation after canonical merge:

```text
Oteryn: sol durability lead
```

```yaml
prompt_id: OTV2_SOL_DURABILITY_LEAD
prompt_version: "1.2"
prompt_mode: SOL_LANE_LEAD
recommended_model: GPT-5.6 Sol
recommended_effort: extra-high_or_highest_available
repository: Oteryn/Oteryn-Game
lane: DURABILITY
short_invocation: "Oteryn: sol durability lead"
```

## Mission

Own deep reasoning and implementation for the currently allocated Durability lane. At this prompt's design admission, Issue #167 and draft PR #212 are live, but you MUST resolve newer GitHub truth and continue existing valid branch/PR history rather than restarting from cached identifiers.

## Mandatory startup

1. Resolve protected `main`, current Durability Issue/task, branch, PR, exact head, checks/reviews and overlapping work from GitHub.
2. Read root/nearest `AGENTS.md`, `docs/agents/BUILD_TEST_MATRIX.md`, `docs/agents/programs/OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md`, the current Durability task/allocation, `docs/agents/prompts/OTV2_IMPL_DURABILITY.md`, and all current Foundation/Durability contracts/resource rows consumed by the lane. Resolve AI-review authority from current root `AGENTS.md`; older local review-routing files are subordinate when root policy says so.
3. If an existing Durability branch/PR is valid, preserve and continue it. `UPSTREAM_ADVANCED` alone is never a reason to reset/recreate/rebase/force-push.
4. Before any write, prove exact merged allocation and exact owned paths. Without them, remain `READ_ONLY_PREPARATION` or `WAITING_ALLOCATION`.

The operator runbook supplies owner-facing placement/model/effort guidance only; it never substitutes for this lane's exact live allocation or technical authority.

## Technical authority

Within exact owned paths you may choose ordinary implementation details needed to satisfy already-accepted Durability semantics, tests and repository constraints.

You MUST NOT independently change:

- Foundation authority/fencing/admission semantics;
- accepted reconnect attempt/transport-ref semantics;
- registered resource maxima;
- shared Cargo/workspace/workflow/composition paths without an exact shared lease;
- item/value/outbox/product scope outside the active allocation;
- public schema/contract semantics beyond current authority.

Use `SHARED_LEASE_REQUIRED` for a legitimate unowned shared path. Use `ARCHITECTURE_ESCALATION_REQUIRED` for material persistence/schema/authority/contract decisions. Use `LANE_DECISION_REQUIRED` only when returning a bounded question to a separate Durability lead session is necessary; normally you are that decision owner.

## Parallel read-only analyst fanout

You are the **only mutating writer** for the canonical Durability task branch/PR. Parallel reasoning must not create multiple writers on the same lane.

When useful and available, fan out independent read-only investigation concurrently to these reusable roles:

- `Oteryn: sol durability authority analyst` — Foundation/current-authority snapshot and final-COMMIT revalidation analysis;
- `Oteryn: sol durability continuity analyst` — continuity/protection-shape and replacement transaction-ordering analysis;
- `Oteryn: sol durability qualification analyst` — whole-diff consistency, regression-gap, protected-main-drift and qualification-plan analysis.

The three analyst roles are advisory only. They have no tracked-file, branch/commit, PR/Issue/comment/review-thread, workflow, merge, lease, architecture or production mutation authority and do not satisfy formal independent-review requirements.

If the execution environment supports true subagent dispatch, run independent analyst domains in parallel and consume their returned packets directly. If the roles are run as separate chats, consume only the explicit packet supplied back to this lead/requester; never assume cross-chat memory is authoritative.

Before acting on any analyst packet:

1. refresh the live PR head and governing authority;
2. verify that the packet's exact head still matches the candidate it analyzed;
3. reconcile overlapping observations rather than applying recommendations mechanically;
4. reject suggestions outside the current allocation or accepted architecture;
5. keep all implementation, test edits, commits, pushes, main reconciliation and final qualification under this single writer.

Analyst fanout is optional acceleration, not a new dependency gate. Lack of a parallel analyst capability does not by itself block authorized Durability progress.

## Current expected outcome

Resolve live state. If the active lane still matches the 2026-08-27 transition, complete the real PostgreSQL reconnect journal/adapter including the still-required V1 COMMIT/CAS and restart/ambiguous-outcome reconciliation paths, migration/schema compatibility evidence, outage/recovery/fencing behavior and exact Foundation boundary consumption.

Do not treat this historical description as permission to widen scope if live allocation differs.

## Validation

Require, as applicable to live scope:

- focused TDD for every semantic increment;
- migration fresh/compatibility/checksum/ahead/behind/dirty/interruption evidence required by the accepted task;
- same-attempt idempotency and lost-response/restart reconciliation;
- collision/concurrency/attempt-capacity behavior;
- DB outage/recovery and fencing preservation;
- locked Rust workspace formatting/build/Clippy/tests;
- real isolated PostgreSQL E2E where the task requires it;
- genuinely independent exact-head review for persistence/fencing/schema risk when the current META-owned repository policy selects it.

Never mark real DB E2E PASS from compilation-only evidence.

## Integration handoff

Do not merge your own lane PR under this profile. Freeze a reviewed candidate and return:

```yaml
lane: DURABILITY
issue:
task_id:
admission_main_sha:
integration_main_sha:
branch:
pr:
final_head_sha:
changed_paths: []
shared_lease_used: null
state: READY_FOR_INTEGRATION | WAITING_DEPENDENCY | WAITING_ARCHITECTURE | WAITING_EXTERNAL | REVIEW_RECONCILIATION_REQUIRED
focused_validation: []
component_validation: []
e2e:
self_review:
independent_review:
codex_review:
  route: CODEX_REQUIRED | CODEX_OPTIONAL | CODEX_NOT_REQUIRED_BY_THIS_POLICY
  classification_source_role:
  classification_source_ref:
  reviewed_head_sha:
  evidence_ref:
  blocking_findings: []
  required_review_threads_unresolved: 0
  status: PASS | CHANGES_REQUIRED | NOT_REQUIRED | WAITING_CAPABILITY
architecture_escalation: null
unresolved_findings: []
recommended_control_plane_action: integrate | return_to_lane | wait | escalate
next_action: <exactly one concrete action>
```

The `codex_review` block is retained for compatibility with existing handoff consumers; current root `AGENTS.md` and the META-owned AI review policy determine whether any external AI review is selected and what evidence is advisory. Legacy local routing terminology never overrides current root policy.

The uniquely active control-plane profile, resolved from the current coordinator Issue/task, independently verifies all facts before integration. If no unique active profile is `PROVEN`, return `POLICY_CONFLICT` and do not route integration to Terra or Work by alias, model selection or reusable status.

## AI review routing — META-owned

Resolve current protected-main root `AGENTS.md` before any external AI review action. The repository adopts the current organization AI review policy by reference; conflicting older `docs/agents/**` standing-authorization/review-tier/controller prose is historical/procedural only.

- Default: no external AI review.
- Ordinary code change with clear independent-review value: prefer Codex Spark when available.
- Material high-risk/control-plane change: use one Codex deep review on a stable material candidate.
- External AI review is advisory and never GitHub merge authority; repository gates/protection/Merge Queue remain enforcement.
- Re-review only when a material risk-bearing repair makes the previous review no longer representative.
- Do not recreate local R0/R1/R2 tiers, standing review controllers or equivalent merge authority.

For Durability work involving session/reconnect/fencing/durable persistence or schema risk, treat the candidate as high-risk when current root policy still classifies those surfaces that way: stabilize the material candidate first, then use the selected deep independent review once, repair actionable findings inside existing authority, and re-review only if a material risk-bearing repair invalidates that review.

The owner is not a prompt relay merely because an older local file once described one. Any metered AI/API use outside the central policy still requires the task-specific authority applicable to that use.

## Safety

No production database/config/secrets, live data, Platform/Atlas/META/external-repository writes or Reference-parity claims. No non-covered owner-funded Codex/OpenAI/API use without exact per-invocation owner authorization.

## Remote Desktop execution routing

Before any Remote Desktop/Desktop Commander use, resolve the current Game `AGENTS.md` and the canonical META execution-routing policy at `Oteryn/Oteryn@e002fc7532188e73a0f495da3e20710541ed50e0`. Out-of-band local connector/tool registration and argument-schema inspection is capability discovery; every direct `Remote_Desktop_Commander.*` invocation is exception-only and requires a fresh valid host-exception context plus a positive per-action decision for the exact semantic host action and exact connector tool immediately before the call.

`list_devices`, `who_am_i`, `ping`, `get_config`, filesystem/search/process/session/terminal/history operations and other direct connector calls are not capability-discovery exemptions. Unknown or undeclared tools fail closed, and a prior ALLOW never authorizes a different action or tool. This prompt cannot broaden META exception reasons or use Remote Desktop as a routine fallback for repository tests, Git inspection, CI/log polling or convenience. A Remote Desktop DENY is not automatically a blocker: continue through GitHub, GitHub Actions, repository-native connectors or an isolated workspace when they can perform useful authorized work.

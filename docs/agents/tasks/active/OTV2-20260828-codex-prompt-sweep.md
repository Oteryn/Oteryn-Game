# OTV2-20260828-codex-prompt-sweep

```yaml
task_id: OTV2-20260828-codex-prompt-sweep
title: Sweep reusable prompts for canonical Codex review routing
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/codex-prompt-sweep-20260828
issue: 234
pr: 235
base_sha: 0deb460c1347213d2dbf14be845e9b0de344e792
head_sha: external_pr_evidence
final_head_sha: external_pr_evidence
head_evidence_surface: "PR #235 live head plus immutable PR comments/reviews/workflow runs"
owner: prompt-governance-sweep
created_at: 2026-08-28T07:43:45Z
updated_at: 2026-08-28T07:57:47Z
owned_paths:
  - docs/agents/prompts/OTV2_ARCHITECTURE_CONTINUATION_AGENT.md
  - docs/agents/prompts/OTV2_CLOSE_NEXT_WAVE_BLOCKERS.md
  - docs/agents/prompts/OTV2_CONTENT_FORMAT_SPIKE.md
  - docs/agents/prompts/OTV2_DOMAIN_ARCHITECTURE_DESIGN_AGENT.md
  - docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
  - docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md
  - docs/agents/prompts/OTV2_SOL_EXECUTION_ARCHITECTURE_CONTINUATION.md
  - docs/agents/prompts/OTV2_IMPL_ANALYTICS.md
  - docs/agents/prompts/OTV2_IMPL_DOMAIN_CORE.md
  - docs/agents/prompts/OTV2_IMPL_DURABILITY.md
  - docs/agents/prompts/OTV2_IMPL_FOUNDATION_RUNTIME.md
  - docs/agents/prompts/OTV2_IMPL_GAME_ABILITY.md
  - docs/agents/prompts/OTV2_IMPL_GAME_AI.md
  - docs/agents/prompts/OTV2_IMPL_GAME_CHANNEL.md
  - docs/agents/prompts/OTV2_IMPL_GAME_INTERACTION.md
  - docs/agents/prompts/OTV2_IMPL_NATIVE_CLIENT.md
  - docs/agents/prompts/OTV2_IMPL_QA_E2E.md
  - docs/agents/prompts/OTV2_IMPL_SERVER_SEAM.md
  - docs/agents/prompts/OTV2_IMPL_SIMULATION.md
  - docs/agents/prompts/OTV2_IMPL_VSL_COMBAT.md
  - docs/agents/prompts/OTV2_IMPL_VSL_CONTENT.md
  - docs/agents/prompts/OTV2_IMPL_VSL_MOVEMENT.md
  - docs/agents/prompts/OTV2_IMPL_WORKSPACE_BOOTSTRAP.md
  - docs/agents/prompts/OTV2_INDEPENDENT_PROGRAMME_ARCHITECTURE_AUDIT.md
  - docs/agents/prompts/OTV2_NEXT_WAVE_PARALLEL_PREPARATION.md
  - docs/agents/prompts/OTV2_PREP_DURABILITY_TOPOLOGY.md
  - docs/agents/prompts/OTV2_PREP_PROGRAMME_STATUS.md
  - docs/agents/prompts/OTV2_PREP_SERVER_SEAM.md
  - docs/agents/prompts/OTV2_PREP_WAVE2_RESOURCE_LIMITS.md
  - docs/agents/prompts/OTV2_TERRA_GAME_CONTROL_PLANE.md
  - docs/agents/prompts/OTV2_SOL_SUPERVISING_ARCHITECT.md
  - docs/agents/prompts/OTV2_SOL_DURABILITY_LEAD.md
  - docs/agents/prompts/OTV2_SOL_SERVER_SEAM_LEAD.md
  - docs/agents/prompts/OTV2_SOL_CLIENT_QA_LEAD.md
  - docs/agents/prompts/OTV2_SOL_MOVEMENT_LEAD.md
  - docs/agents/prompts/OTV2_SOL_COMBAT_LEAD.md
  - docs/agents/prompts/OTV2_SOL_POST_VSL_EXPANSION.md
  - docs/agents/prompts/OTV2_SOL_WORLD_CONTENT_PREP.md
  - docs/agents/prompts/OTV2_SOL_NPC_AI_PREP.md
  - docs/agents/prompts/OTV2_SOL_SYSTEMS_ECONOMY_PREP.md
  - docs/agents/prompts/OTV2_SOL_TOOLING_OPS_PREP.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/prompts/README.md
  - docs/agents/tasks/active/OTV2-20260828-codex-prompt-sweep.md
public_contracts: []
depends_on:
  - docs/agents/CODEX_REVIEW_POLICY.json@0deb460c1347213d2dbf14be845e9b0de344e792
blocks: []
external_repositories: []
```

## Outcome

All 43 reusable prompt bodies are reconciled with the canonical standing Codex review policy. Covered review operations no longer require per-run owner relay; non-covered owner-funded AI remains exact-per-invocation owner-authorized; role boundaries remain unchanged.

## Acceptance

- [x] Inspect every reusable prompt registered at admission.
- [x] Add one canonical Codex review-routing section to every reusable prompt.
- [x] Remove or repair stale broad per-run-owner clauses.
- [x] Make the five Sol lane leads explicitly own the covered `CODEX_REQUIRED` review/re-review loop under exact merged lane allocation.
- [x] Make Work/Terra/implementation control planes route missing/stale Codex evidence back to the owning lane lead without triggering on its behalf.
- [x] Preserve auditor non-nested-review and Supervising Architect separation boundaries.
- [x] Update prompt lifecycle versions, README and prompt authoring/eval standards.
- [ ] Static sweep checks and governance validation PASS on final exact head.
- [ ] Full-diff self-review PASS on final exact head.
- [ ] Fresh independent Codex exact-head review PASS with zero unresolved blocking threads.
- [ ] Expected-head squash merge and protected-main readback complete.

## Excluded scope

No runtime, Cargo/workspace, protocol/schema/resource registry, production/protected environment, secrets, live data or external-repository mutation. No change to `CODEX_REVIEW_POLICY.json` authority itself.

## Validation

Runtime/E2E: `NOT_APPLICABLE` — prompt/governance-only sweep. Required evidence is static prompt sweep validation, Agent governance, Architecture semantic audit, Merge authority audit, Merge gate, self-review and independent exact-head Codex review.

Pre-PR local candidate validation passed: 43/43 reusable prompts have one canonical routing section, stale authorization-pattern matches are zero, role-boundary assertions pass, lifecycle versions advance exactly one minor version, changed-path scope is exact, `python tools/agents/validate_governance.py` passes and `git diff --check` passes.

PR #235 exists. Its first exact-head independent Codex review found one P1 in this task packet: stale `pr: null` / checkpoint metadata. That finding is being repaired here. The reviewer also requested a literal current candidate SHA in this tracked file; canonical `DELIVERY_COMPLETENESS_AND_CLOSEOUT.md` forbids a self-referential follow-up commit merely to populate its own SHA. Therefore the current/final exact head is recorded after each freeze on the immutable PR #235 review/comment/workflow surface referenced by `head_evidence_surface`; this tracked task records the PR and evidence location without fabricating a self-referential SHA.

## Context checkpoint

```yaml
last_progress: PR #235 opened; first Codex review found one task-metadata P1 and this commit repairs the stale PR/checkpoint metadata while preserving canonical external exact-head evidence placement
status: validating
branch: docs/codex-prompt-sweep-20260828
head_sha: external_pr_evidence
pr: 235
head_evidence_surface: "PR #235 live head plus immutable PR comments/reviews/workflow runs"
blocker: null
next_action: qualify the repaired PR #235 exact head with fresh CI, whole-diff self-review and fresh independent Codex review
```

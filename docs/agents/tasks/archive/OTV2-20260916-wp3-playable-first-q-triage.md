> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #636 merged as `ee4a13212d392dd00f8adcd47b103997687b1d6c`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260916-wp3-playable-first-q-triage

```yaml
task_id: OTV2-20260916-wp3-playable-first-q-triage
title: Tighten WP3 playable-first Q and numeric-floor triage
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/wp3-playable-first-q-triage-20260916
pr: 636
base_sha: 1995bd97460774ea9fc136959d5548471b81c987
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol
created_at: 2026-09-16T15:33:00+02:00
updated_at: 2026-09-16T15:42:00+02:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/prompts/OTV2_ASTRA_WP3_V2_PROGRAMME_COORDINATOR.md
  - docs/agents/tasks/active/OTV2-20260916-wp3-playable-first-q-triage.md
public_contracts: []
depends_on:
  - PR #634 protected playable-first policy
  - PR #635 protected WP3-A authority-backed disposition
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Prevent the WP3-v2 programme coordinator from reintroducing broad #356 obligations into WP3-A by blanket inheritance of historical Q01-Q75 cells or numeric resource floors. Require current protected authority and next-playable-milestone relevance before any historical obligation becomes a WP3-A blocker, while preserving every actually accepted correctness/security/durability invariant.

## Architecture and source of truth

- `PROVEN`: protected PR #634 established `docs/repository/PLAYABLE_FIRST_ENGINEERING_POLICY.md` and the upstream-first/minimum-sufficient transition contract.
- `PROVEN`: PR #635 is protected-integrated on Game `main@0ab58cb2570b8355049659f3c95b4a5eb14a8299` and replaced blanket historical inheritance with an explicit Q01-Q75/N01-N32 authority-backed disposition.
- `PROVEN`: protected #635 retains the exact 12 MiB root/work safety requirement because current protected authority independently binds it; it does not retain every historical #356 allocation mechanism.
- `DERIVED`: the reusable coordinator prompt still needs the generic authority-first guard so future executions do not regress to blanket broad-fork-era proof scope after #635.
- `REQUIRED`: accepted protected safety/correctness/durability/compatibility/resource limits remain binding until protected authority changes them; this task does not waive any accepted invariant.

## High-risk authority/recovery qualification

Prompt/governance only. No runtime, persistence, authority-bearing session, production, credential or recovery-state mutation. Because this changes reusable coordinator behavior, normal current control-plane review policy still applies before protected integration.

## Acceptance criteria

- [x] Keep the existing alias `Oteryn: astra wp3-v2 programme coordinator`; create no duplicate coordinator.
- [x] Forbid blanket historical-Q inheritance into WP3-A.
- [x] Require each Q/property to be classified by current protected authority and release relevance.
- [x] Separate current WP3-A invariants, WP3-B representative qualification, downstream triggers, historical mechanism evidence, proven unreachable/superseded paths and unresolved authority/evidence.
- [x] Require the same current-authority proof for numeric resource floors such as the historical 12 MiB equation.
- [x] Explicitly preserve a numeric floor when current protected authority still binds it.
- [x] Prevent mechanism-named #356 tests from forcing retention of that mechanism when a smaller proof satisfies the property.
- [x] Require an explicit Q/numeric-floor disposition summary before protecting a WP3-A acceptance/allocation amendment.
- [x] Preserve owner correction comment `5698458209` as historical guidance and recognize protected #635 as the current authority-backed implementation of that correction.

## Excluded scope

- No runtime, Cargo, vendor, workflow, ruleset or production mutation.
- No change to PR #356 history or source.
- No weakening of accepted correctness/security/durability/compatibility requirements.
- No amendment of protected #635 from this branch.
- No new prompt alias/control plane.

## Implementation / findings

The reusable prompt's identity, owner, alias, status and supersession relation are unchanged; `PROMPT_LIFECYCLE.json` therefore remains untouched. The prompt delta codifies the same authority-first classification now protected by #635 so later coordinator runs cannot accidentally reconstruct the superseded blanket Q-matrix behavior.

The critical guard is property-first classification: a historical mechanism-specific test or number is not a current WP3-A requirement merely because it existed in #356. Conversely, an exact current protected requirement remains binding until protected authority explicitly changes it.

Fresh post-#635 reconciliation found one stale task-record statement on predecessor head `5d61d9f6b3b78746fa6e006fe9f0e9ffe46a51c9`: it incorrectly described #635 as still carrying blanket `ALL_OTHER_Q = CURRENT_EVIDENCE_REQUIRED`. That statement is removed here; no prompt semantics are changed by this repair.

## Validation

- prompt delta whole-diff readback against protected #634/#635: PASS; no conflict found with the protected Q/numeric-floor disposition
- predecessor #636 Architecture Semantic Audit `35103648459`: SUCCESS
- predecessor Agent Governance `35103648394` and Merge Gate `35103648150`: FAILURE because PR metadata lacked `## Validation`, not because prompt content failed validation
- task-record protected-state reconciliation: repaired on the same branch; exact successor-head readback required
- lifecycle alias/status/owner/supersession identity: unchanged
- component/E2E: `NOT_APPLICABLE`, prompt/governance only
- exact-head repository checks: required on the repaired head before review/integration

## Self-review

- predecessor head reviewed: `5d61d9f6b3b78746fa6e006fe9f0e9ffe46a51c9`
- method: complete two-file diff against protected #634 policy and protected #635 disposition/readback
- material finding: stale task-record `PROVEN` statement about #635; minimum repair applied on the same branch
- prompt findings: none requiring semantic repair
- final verdict: pending successor exact-head checks and applicable independent review

## Context checkpoint

```yaml
last_progress: protected #635 read back; #636 prompt remains aligned; stale task-record authority statement repaired in-place
status: validating
branch: governance/wp3-playable-first-q-triage-20260916
head_sha: pending readback
pr: 636
next: repair PR metadata Validation section, run exact-head gates, independent review, then use #636 only if it becomes a truthful qualified Game canary candidate
```

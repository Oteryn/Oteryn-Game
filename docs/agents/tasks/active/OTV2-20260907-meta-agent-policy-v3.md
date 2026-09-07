# OTV2-20260907-meta-agent-policy-v3

```yaml
task_id: OTV2-20260907-meta-agent-policy-v3
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agents/meta-policy-v3-game-367
issue: 367
pr: 373
base_sha: a6f69427d663539c6a8e23f166e69147b66ec078
tested_instruction_head_sha: ebe152af34bd003dd5c41b38eb550f10944be988
tested_instruction_tree_sha: 7adcaa9f1f7a4c8067bed4be1cbd4a6d3a55b687
owner: game-governance
owned_paths:
  - AGENTS.md
  - .github/workflows/agent-governance.yml
  - docs/agents/**
  - tools/agents/**
public_contracts:
  - Oteryn/Oteryn@5ed3f14400af450b5875c091e443da70f2d67ab9
```

## Outcome

Adopt central META agent policy v3 through an authenticated immutable binding, a small Game bootstrap and task-delta prompts. Retire the completed provider-copy enforcement from #237/#239 while preserving Game domain invariants and existing repository gates.

## Acceptance

- Central binding, provider overlay and every lifecycle-reusable prompt validate against the pinned protected META revision.
- Actual root/nearest instruction delivery and representative domain, scope and stale-locator behaviors remain covered.
- Existing Game governance and repository-policy checks pass.
- `Agent governance` retains its existing required status and runs the replacement consumer.
- Source/context volume is quantified without claiming token or API savings.

## Excluded scope

No runtime, Rust, protocol/schema, production, credentials, protection/ruleset, or external-repository mutation.

## Qualification

- Contract checks pass for the binding, overlay, all 47 reusable prompts, exact-pin authentication, retained Game invariants, governance and repository policy.
- Actual candidate delivery was observed from the Game root bootstrap through the immutable META binding, bound policy, nearest `apps/game-server/AGENTS.md`, routed Game domain sources and representative `OTV2_IMPL_SERVER_SEAM.md`.
- Exact tested instruction head `ebe152af34bd003dd5c41b38eb550f10944be988`, tree `7adcaa9f1f7a4c8067bed4be1cbd4a6d3a55b687`, passed Agent governance `34103196352`, Architecture semantic audit `34103196396` and Merge gate `34103196461`.
- Four fresh `gpt-5.6-sol` Medium threads ran matched baseline/candidate five-case arms and matched three-case safety repeats. All 16 case executions across five unique case types qualified. Both arms produced the exact typo repair, passed the nine positive-ID assertions, rejected the domain authority transfer, denied hostile cached authority/direct-main bypass and preserved same-session continuation without claiming background execution.
- The baseline's first identifier implementation used `isinstance`, failed the boolean assertion and was repaired before acceptance. The candidate's first identifier implementation passed all nine assertions. This bounded attempt difference is retained without a generalized efficiency or causal cost claim.
- The safety repeats actually loaded `docs/agents/prompts/OTV2_IMPL_SERVER_SEAM.md`. The candidate resolved immutable META `5ed3f14400af450b5875c091e443da70f2d67ab9` plus root, nearest and routed sources.
- Durable qualification: Issue #367 comment `5568474772`; compact comparison SHA-256 `04d1838ed8a9d885eaf5bdcbca3cb1e7053a979260bd6ad874d14102acd8fffc`.
- Limitations: finite synthetic screening only; one five-case initial arm and one three-case safety repeat per arm; no statistical proof of generalized safety or production behavior. Baseline used its historical machine pin plus the latest locally available policy, while candidate used immutable META `5ed3f144…`. Full-file volume includes partially read files. No token, cache, billing or causal cost telemetry was exposed or inferred.

## Context checkpoint

```yaml
status: validating
next_action: publish this evidence-only qualification update, then obtain fresh exact-head CI and review before normal Merge Queue integration
```

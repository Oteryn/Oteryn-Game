# OTV2-20260816-parallel-agent-prompts

```yaml
task_id: OTV2-20260816-parallel-agent-prompts
title: Prepare bounded parallel-agent prompts for current Oteryn-v2 work
mode: GOVERNANCE_DOCUMENTATION
status: implementing
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/parallel-agent-prompts-20260816
base_sha: d2af53855046df25b4e52edbd5ec14e0513a63ec
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: current coordinating agent
created_at: 2026-08-16T19:09:00+02:00
updated_at: 2026-08-16T19:09:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260816-parallel-agent-prompts.md
  - docs/agents/prompts/OTV2_GAME_ABILITY_OWNER_DECISION_AGENT.md
  - docs/agents/prompts/OTV2_DEPENDABOT_TOKIO_239_AGENT.md
  - docs/agents/prompts/OTV2_DEPENDABOT_SERDE_JSON_240_AGENT.md
  - docs/agents/prompts/OTV2_PROD_ENTITLEMENTS_115_AGENT.md
  - docs/agents/prompts/OTV2_GOVERNANCE_CHECKPOINT_CLEANUP_AUDITOR.md
public_contracts: []
depends_on:
  - live main d2af53855046df25b4e52edbd5ec14e0513a63ec
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/MULTI_AGENT_ARCHITECTURE_ORCHESTRATION.md
  - docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
blocks: []
external_repositories:
  - blakinio/Oteryn-Platform (read-only)
```

## Outcome

Create exactly five reusable, self-contained prompts for the currently identified parallel lanes while preserving serial canonicalization and existing repository authority boundaries.

## Acceptance

- five prompt files exist under `docs/agents/prompts/`;
- every prompt satisfies `PROMPTING_STANDARD.md` and `PROMPT_EVAL_STANDARD.md`;
- no prompt grants standing Codex/OpenAI/paid-review authority;
- no prompt grants production, protected-environment, runtime implementation, DDL or cross-repository write authority beyond its explicit lane;
- GAME-ABILITY remains the canonical paper-only decision lane;
- dependency PR lanes repair and validate only their own existing PRs;
- entitlement work remains bounded to Oteryn-v2 consumer/enforcement architecture unless separately authorized;
- governance cleanup lane is audit-first and must not repair without separate explicit write authorization;
- exact-head governance/merge-gate validation passes before merge.

## Excluded scope

- runtime/client/server/protocol/content implementation;
- PostgreSQL DDL/migrations;
- Platform writes;
- production/live changes;
- modifying PR #239 or #240 as part of this prompt-package task;
- making GAME-ABILITY or PROD-ENTITLEMENTS semantics accepted merely by creating prompts.

## Context checkpoint

```yaml
last_progress: task initialized from verified live main
status: implementing
branch: docs/parallel-agent-prompts-20260816
head_sha: null
pr: null
blocker: null
next_action: create the five bounded prompt files, inspect the full diff, then validate the exact final head
```

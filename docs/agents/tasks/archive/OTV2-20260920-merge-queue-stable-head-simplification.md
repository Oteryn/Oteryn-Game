# OTV2-20260920-merge-queue-stable-head-simplification — terminal archive

```yaml
task_id: OTV2-20260920-merge-queue-stable-head-simplification
title: Simplify stable-head delivery under Merge Queue
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
pr: 702
admission_base_sha: e765a314ceb2b81e4260dd1dee1f1e547f6ce920
qualified_head_sha: 07de40171b786631a8284239ba171c7340d72c50
merge_group_sha: a382e6d7f2d1ef20dfb37d13322a3d917c123c6b
protected_main_sha: a382e6d7f2d1ef20dfb37d13322a3d917c123c6b
owner: null
created_at: 2026-09-20T23:46:00+02:00
completed_at: 2026-09-21T00:25:33+02:00
execution_policy: continuous_progress
owned_paths: []
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Terminal outcome

PR #702 completed the bounded stable-head simplification and merged through the protected Game Merge Queue. Protected `main@a382e6d7f2d1ef20dfb37d13322a3d917c123c6b` now keeps an already-published candidate stable when protected `main` moves but no real source reconciliation is required. The Work coordinator and convergence protocol must classify upstream movement read-only first and let canonical `merge_group` qualification prove composition against current protected `main`.

The change deliberately did **not** weaken mutation safety. A real semantic, contract or source conflict still requires the existing isolated-workspace, custody, required-validation and normal non-force publication route. Force push, rebase/reset, direct merge, bypass, weakened checks, low-level candidate reconstruction and Remote Desktop convenience fallback remain outside this simplification.

Delivered bounded paths:

- `docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md`
- `docs/agents/PROMPT_LIFECYCLE.json`
- `tools/agents/tests/test_meta_agent_policy_adoption.py`
- `docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md`
- `docs/agents/evidence/OTV2-20260920-merge-queue-stable-head-simplification-evaluation.md`
- this task record, archived here after terminal integration

The reusable Work coordinator lifecycle metadata advanced from `1.7` to `1.8`, with the pinned META-adoption regression expectation advanced in the same candidate.

## Validation and review

- exact qualified source head: `07de40171b786631a8284239ba171c7340d72c50`;
- Agent Governance run `35540727363`: SUCCESS;
- Architecture Semantic Audit run `35540727367`: SUCCESS;
- full Merge Gate run `35540727356`: SUCCESS;
- exact-head aggregate `game-gate` job `106158928501`: SUCCESS;
- independent Codex review completed on material policy candidate `4a15acf05504e2fe387add321533f67404784ae7`;
- review found the stale lifecycle-version regression assertion as P1; final head `07de40171b786631a8284239ba171c7340d72c50` repaired it and expanded the bounded task scope accordingly;
- both review threads are resolved and no unresolved material review thread remains;
- no policy-semantic widening was added after the reviewed `4a15acf...` generation; the successor changed the deterministic regression expectation and task ownership record;
- real Merge Queue run `35541221937`: SUCCESS;
- Merge Queue aggregate `game-gate` job `106160208474`: SUCCESS;
- protected integration: PR #702 -> `main@a382e6d7f2d1ef20dfb37d13322a3d917c123c6b`.

Runtime/product E2E was not introduced by this governance-only task; the protected PR and Merge Queue gates nevertheless requalified the selected Linux, Windows, PostgreSQL, CodeQL, supply-chain and governance lanes before integration.

## Closeout

The former source branch `docs/merge-queue-stable-head-simplification-20260920` is no longer present on the remote. No active task ownership, lease or writer remains for this task.

This record is historical lifecycle evidence only. Future delivery work must resolve current protected `main`, current live GitHub authority and current task allocation; it must not reactivate this terminal packet.

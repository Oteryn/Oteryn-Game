> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #641 merged as `90d029dcd64c65e2e8ef1921efa59e8042e558b8`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260917-content-world-execution-prompts

```yaml
task_id: OTV2-20260917-content-world-execution-prompts
title: Content World delivery architecture and reusable execution prompts
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-design-dossier-20260917
pr: 641
base_sha: b44fefe08f6aaf1b2c1c23dedd92bab0de87146e
head_sha: external_pr_evidence
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
owner: owner-authorized-content-world-documentation
created_at: 2026-09-17
updated_at: 2026-09-17
execution_policy: continuous_progress
owned_paths:
  - docs/agents/programs/OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md
  - docs/agents/programs/OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md
  - docs/agents/prompts/OTV2_CONTENT_WORLD_LEAD.md
  - docs/agents/prompts/OTV2_CONTENT_WORLD_ARCHITECTURE.md
  - docs/agents/prompts/OTV2_CONTENT_WORLD_IMPORT.md
  - docs/agents/prompts/OTV2_CONTENT_WORLD_BUILD.md
  - docs/agents/prompts/OTV2_CONTENT_WORLD_RUNTIME.md
  - docs/agents/prompts/OTV2_CONTENT_WORLD_CLIENT.md
  - docs/agents/prompts/OTV2_CONTENT_WORLD_QA.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/tasks/active/OTV2-20260917-content-world-execution-prompts.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

Explicit owner request: prepare implementation architecture, a practical divided delivery plan, reusable prompts and aliases, and save them in the existing repository. Scope is documentation and prompt discovery registration only. No runtime worker, implementation lease, new control plane, production action, format acceptance or merge authority is granted by this delivery.

Preserve the full native Content/World architecture and necessary capabilities. Avoid unnecessary custom libraries, dependency forks and infrastructure; do not replace the architecture with a provisional/toy path. Earlier contradictory assistant scope reduction remains rejected as recorded in the existing owner synthesis.

Publication predecessor: `40fcb356d7a24b3e4c2b6e33c693b078072d9a5c` on the existing #641 branch. Original eight PR paths remain unchanged except that this task adds the previously unchanged prompt lifecycle registry to the PR. The seven new profiles do not supersede or modify existing prompt entries.

## Sources and scope

Root/nearest AGENTS; bound META policy 3.1.0 at `33b212e652c680bd4047be3b414c9a358b8bf26f`; its `PROMPTING_STANDARD.md` and `PROMPT_EVAL_STANDARD.md`; Game prompting extension; architecture decision discipline; existing #641 synthesis/audit; source registry #486; current #162 and existing WP3/Native UI/Reference alias conventions; BUILD_TEST_MATRIX.

Existing protected contracts remain authority. New documents are proposed execution decomposition, not new accepted gameplay contracts. The lead is subordinate to the existing #162 control plane. Exact allocations/custody, source rights, target evidence and required owner decisions remain independently verified at execution time.

## Acceptance criteria

- [x] Define component/interface ownership, delivery dependencies and meaningful parallel work without another runtime or control plane.
- [x] Define seven role-specific prompt deltas and one launch/runbook, retaining the full architecture and upstream-first implementation strategy.
- [x] Prepare seven lifecycle additions with the original 71 entries and top-level semantics unchanged.
- [x] Document pre-main PR-head bootstrap and distinguish discovery from implementation permission.
- [x] Preserve source-role/target rules, including first-class structured bulk data and OTS hypothesis boundaries.
- [x] Run focused static checks on the prepared documents and registry.
- [ ] Verify final published head/blobs, live hosted checks and any required independent review in PR evidence.

## High-risk authority/recovery qualification

Production AuthorityInvariant x ConsumerBoundary x MutationOperator testing is NOT_APPLICABLE: no controller, runtime authority consumer, transaction or recovery code changes. Prompt authority boundaries remain material and were screened explicitly. Adding discovery entries does not add grants, self-acceptance, protected integration authority or cross-repository writes. No workflow, validator, ruleset or resource registry changes are included.

## Compact prompt evaluation record

Baseline: existing #641 head above and prompt-registry blob `80a67c79925371af09a934ef2013cf26581073f9`. Baseline registry recovered from exact reads was verified byte-for-byte by Git blob digest: 44,479 bytes, 71 entries. Candidate: seven new prompts, programme/runbook and seven appended lifecycle entries. Final commit identity is recorded after publication, not self-referenced in this file.

Actual preparation environment: isolated container scratch files, no material local Git commit selected. Direct GitHub transport failed with `Could not resolve host: github.com`; publication is the explicitly authorized API-native documentation operation, not reconstruction of a selected local Git candidate. No full checkout or complete local repository governance execution is claimed.

Focused static checks executed: JSON schema/required fields; 78 unique IDs and paths; unchanged first 71 entries and top-level metadata; seven aliases and matching prompt/runbook references; nine programme/prompt Markdown files with balanced fences, final newlines and no trailing whitespace; 34 relative links checked against local new files and inspected source paths. Ten text-contract screens cover full architecture, single control plane, exact custody, source roles/target, upstream-first, no inherited 36 blockers, no toy terminal proof, pre-main bootstrap, successor/control split and truthful evaluation boundaries.

These are contract/text checks, NOT model behavior or security proof. The new task record receives separate basic formatting/path checks. No agent/model behavioral trials were run (`model_trials: 0`). Actual client instruction delivery and representative task execution are `NOT_EVALUATED`; model/effort behavior is not inferred from the alias. No before/after efficiency or token-saving claim is made. The role-specific first actual task is the behavior-validation opportunity; no automatic paid evaluation job is launched.

## Validation, review and handoff

Runtime/Rust/PostgreSQL/native-client tests: NOT_APPLICABLE to this documentation delivery and NOT_RUN. Unchanged historical 21/26/63-test models were not rerun or aggregated into a new result. Required hosted governance/game-gate and applicable policy checks remain independent and are recorded for the exact final head in the PR.

Author self-review covers the whole new package, dependency order, exact scope, no duplicate canonical writers, no fake implementation, current versus future requirements, source evidence and route bootstrap. No independent review or owner acceptance is claimed. Applicable risk-based review remains with existing policy/control plane; no new required status or review framework is created here.

Final publication evidence must list exact head, changed paths, read-back blob identities, static check limits and actual hosted outcomes. Informational #162 handoff is not worker release. Do not move a qualified final head just to replace these checkboxes with a copied CI snapshot; live PR/check evidence owns that lifecycle fact.

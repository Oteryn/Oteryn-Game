# OTV2-20260927-br-population-capture

```yaml
task_id: OTV2-20260927-br-population-capture
title: Capture every population creature page from TibiaWiki BR at the target date
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1049
jira: KAN-16
base_sha: 716e23ba0c3585dcb0d52a5aa5300d124eaac94b
head_sha: d72a5f7ac7bc8ba58d379bb90f829c2fb2cb1f0f
final_head_sha: d72a5f7ac7bc8ba58d379bb90f829c2fb2cb1f0f
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/monster-wiki-capture.yml
  - docs/agents/tasks/active/OTV2-20260927-br-population-capture.md
  - docs/agents/tasks/active/OTV2-20260927-vlarkorth-wiki-capture.md
  - docs/agents/tasks/archive/OTV2-20260927-vlarkorth-wiki-capture.md
  - tools/content-schema/monster-authoring/wiki_br_capture.py
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The Grave Danger capture of #1047 showed that TibiaWiki BR carries values that Fandom leaves as "?". For example,
BR gives Dark Knight 7,900 health, where Fandom has "?" and Canary has 1,800. The owner's source order puts BR third,
for tables and cross-checks.

This task extends the hosted-runner capture to every creature page of the 2026-09-27 wiki population sample
(1,650 titles). It keeps revision ids and non-empty infobox fields, never raw pages. It is evidence only: no value is
adopted until the owner decides how BR fills gaps left by Fandom.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: evidence tooling and a read-only capture workflow only.

## Acceptance and evidence

- `wiki_br_capture.py self-test` passes.
- The workflow uploads the population capture artifact.
- The governance and policy validators pass.

## Completion

Merged as PR #1049 (`c5f59954689426954674333412d7487597ba631b`) from final head `d72a5f7`; required checks passed on that head. The capture found
1,494 of 1,650 population titles on TibiaWiki BR. Where Fandom gives no certain value, BR fills health for 92 and
experience for 81 monsters; the owner chose to adopt only those two fields (D35).
Owner released.

# OTV2-20261001-reward-claim-followups

```yaml
task_id: OTV2-20261001-reward-claim-followups
title: Close four RewardClaim authoring review findings
mode: WORK
status: completed
repository: Oteryn/Oteryn-Game
base_branch: codex/quest-wiki-completion-20261001
branch: codex/quest-completion-followups-20261001
pr: null
base_sha: 1a5c3bbf78aae7a8bb4ac0ad035a2c178782cbba
head_sha: null
owner: codex-quest-data-parent
created_at: 2026-10-01T14:10:00Z
updated_at: 2026-10-01T14:10:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/reward-claim-authoring/
  - content/interactions/reward_claims/index.json
  - content/quests/definitions/index.json
  - .github/workflows/reward-claim-authoring.yml
  - docs/agents/tasks/archive/OTV2-20261001-reward-claim-followups.md
public_contracts: []
depends_on: [OTV2-20261001-quest-wiki-completion]
blocks: []
external_repositories: []
```

Direct owner continuation and one-writer scope: #162 comment 5933181185.
Closes the four carried LOW findings of #1364: enforce proven/default stack limits
in build and validation; retain all server-scoped legacy UID collisions in
source_checks and reject missing/stale diagnostics; mark both shebang scripts
executable; run tests/content checks in CI on every relevant tool/data change.

No counts, charges, identities, native contracts or runtime behavior are guessed.
Canonical records remain 231, with 27 ready and 204 waiting. The Canary 6117
collision retains both claim/position bindings. The Quest index is regenerated
to keep its exact RewardClaim provenance hash current.

Independent review also exposed malformed KNOWN/UNKNOWN stack envelopes; the helper
now rejects these explicitly instead of crashing or silently selecting default100.
The full KNOWN stack payload is closed and its typed stackable flag is checked
against StackCapable; missing/extra fields or a contradictory false never become ready.
Local checks PASS: 12 RewardClaim tests, 83 Quest regressions, 262 schema cases,
all semantic/deterministic content checks, Ruff 0.13.3 EXE, governance and
repository policy (59 workflows). Final PR/SHA and frozen-head CI evidence are
recorded on GitHub after guarded publication. Coordinator owns review dispatch,
integration/retargeting and MQ.

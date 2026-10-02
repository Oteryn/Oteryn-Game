# OTV2-20261001-quest-item-fact-corrections

```yaml
task_id: OTV2-20261001-quest-item-fact-corrections
title: Correct exact quest Item container facts and physical charged quantities
mode: WORK
status: completed
repository: Oteryn/Oteryn-Game
base_branch: codex/quest-source-text-completion-20261001
branch: codex/quest-item-fact-corrections-20261001
pr: null
base_sha: 7f965002dbb41a60076e1cc7d7386fadcdc392ca
head_sha: null
owner: codex-quest-data-parent
created_at: 2026-10-01T20:35:00Z
updated_at: 2026-10-01T20:50:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-migration/quest_reward_item_semantics.py
  - tools/content-migration/test_quest_reward_item_semantics.py
  - tools/content-migration/QUEST_REWARD_ITEM_ADMISSION.md
  - tools/content-schema/reward-claim-authoring/
  - content/items/definitions/items-12000-12499.json
  - content/items/definitions/items-14000-14499.json
  - content/items/definitions/items-28500-28999.json
  - content/interactions/reward_claims/
  - content/quests/definitions/index.json
  - imports/tibiawiki/quest-reward-items/
  - docs/agents/evidence/OTV2-20261001-quest-reward-item-admission.json
  - docs/agents/evidence/OTV2-20261001-quest-interaction-item-admission.json
  - docs/agents/evidence/OTV2-20261001-quest-charge-physical-quantities.json
  - docs/agents/tasks/archive/OTV2-20261001-quest-item-fact-corrections.md
public_contracts: []
depends_on: [OTV2-20261001-quest-source-text-completion]
blocks: []
external_repositories: []
```

Direct owner continuation/D277, exact root lease #162/5940044611. The source
precedence already accepted by ITEM-SEM-2 selects three guarded client/wiki facts:
235 and25302 are noncontainers;53074 has capacity22. Source XML alternatives
remain evidence. Only container semantics and computed materializable change;
901 whole definition and2854 seal remain byte-identical. Source materialization
facts do not grant runtime support above BAGS-0 capacity20.

Charged physical quantity1 is proved independently of source/default charge
comparison; six exact source roles, twelve donor binary identities and pinned
wiki defaults are retained. Three source variant quantities become KNOWN1;
CONFLICT, raw subtype1, accepted defaults and native holds remain. Real plain
counts already1 remain unchanged; their diagnostics are now explicit.

Root regenerated all derived files with their tools. Plain readiness219/12,
canonical reward-only Quest43/67 unchanged. No native vocabulary, Rust/runtime,
Item identity or frozen-parent mutation. Global project/manifest/lock and source
text capture/registry/bundle remain unchanged from the S parent.

Qualification:87 actual component tests PASS (28 Item,25 plain,17 variants,
17 Quest tree);525 admitted bindings/idempotence PASS; all three regeneration
checks plus text/bundle SOURCE_BACKED checks PASS; governance/repository-policy
and diff checks PASS. Root whole-diff self-review covers exact source scope,
known-field supersession, quantities and native holds. Independent final-head
review and new exact-head CI are recorded in PR/#162 after freeze; coordinator
owns parent-first retarget/regeneration, paid-review triggers and MQ.

High-risk authority/recovery qualification: NOT_APPLICABLE; source data/tooling,
no live production write or runtime authority decision.

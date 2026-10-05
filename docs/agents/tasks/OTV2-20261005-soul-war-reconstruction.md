# OTV2-20261005-soul-war-reconstruction

```yaml
task_id: OTV2-20261005-soul-war-reconstruction
title: Reconstruct Soul War from Crystal summer-update with video cross-check
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/soul-war-reconstruction-20261005
pr: null
base_sha: fc3db9abfb4ae05c13b0264752cfbbca13a276e6
head_sha: fc3db9abfb4ae05c13b0264752cfbbca13a276e6
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol session
created_at: 2026-10-05T22:42:41+02:00
updated_at: 2026-10-05T22:42:41+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/encounter-authoring/
  - tools/content-schema/quest-authoring/
  - content/encounters/
  - content/quests/
  - docs/agents/evidence/OTV2-20261005-soul-war-reconstruction/
  - docs/agents/tasks/OTV2-20261005-soul-war-reconstruction.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f
  - opentibiabr/canary@04b83b512114bfd888000d6e1433ed8ecaec7c5b
```

## Outcome

Reconstruct the canonical Soul War quest and its encounter data in Oteryn using the current Oteryn authoring architecture. CrystalServer `summer-update` is the primary implementation donor where it is demonstrably more complete; current Canary is the source cross-check; the six-part YouTube guide is behavioral evidence for spatial/phase mechanics. Do not copy donor Lua into Oteryn. Preserve existing accepted Oteryn decisions where they intentionally differ from either donor.

## Architecture and source of truth

- PROVEN: canonical quest identity is `oteryn:quest.soul_war_quest@quest-r1`.
- PROVEN: existing Oteryn Soul War encounter content is sourced from older Canary `47dfd51f...` and is partial.
- PROVEN: CrystalServer `summer-update` head `00ce02a57ca5a12e48f32a3476e37471167e4c3f` contains later Soul War fixes and fuller mechanics.
- PROVEN: current Canary cross-check pin is `04b83b512114bfd888000d6e1433ed8ecaec7c5b`.
- PROVEN: playlist `PL2czNtPw97ZRaU9SIaoZF42p6K5BTu-qX` was visually reviewed across all six parts; dense 1-second sampling was used around Malice, Greed, Spite, Cruelty, Hatred and Megalomania mechanics.
- CONFLICT: video timing is not accepted as an exact numeric oracle where donor source gives a precise value. Numeric conflicts stay explicit until resolved by higher-priority evidence.
- PROVEN: `docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` keeps encounter state instance-local and emits quest/reward outcomes instead of writing durable quest state directly.
- PROVEN: encounter runtime decision remains candidate/non-runtime authority; this task must not claim playability merely because authoring data is complete.

## High-risk authority/recovery qualification

NOT_APPLICABLE: content/schema/data migration only; no production mutation, credential, durable-session authority or protected-main write.

## Acceptance criteria

- [ ] Soul War gap matrix covers start/access/progression, five hunting-area access gates, taints, cooldowns, rewards, all five mini-bosses and Megalomania.
- [ ] Crystal summer-update differences that improve parity are represented without copying Lua implementation text.
- [ ] Existing Oteryn accepted decisions are preserved unless stronger evidence explicitly supersedes them.
- [ ] Video-only observations are marked behavioral evidence and never silently promoted to exact timings.
- [ ] Canonical encounter authoring contains complete representable mechanics for Malice, Greed, Spite, Cruelty, Hatred and Megalomania plus Claustrophobic Inferno and Soul War taint zones.
- [ ] Canonical Quest DATA carries the reconstructed Soul War progression/reward specification without asserting runtime activation.
- [ ] All touched authoring schemas/semantic validators and deterministic regenerators pass.
- [ ] Generated canonical content matches the authoring source exactly.
- [ ] Any mechanic outside the accepted vocabulary remains an explicit UNKNOWN/CONFLICT rather than an approximation.

## Excluded scope

- No direct merge to `main`.
- No donor Lua copied into the repository.
- No claim that Quest runtime/native lowering exists if the current server does not execute it.
- No unrelated content migration or encounter cleanup.
- No rewrite of unrelated Soul War creature/item definitions.

## Implementation / findings

Initial live-state audit found no open Soul War PR. Existing canonical encounter records cover only partial Greed, Hatred, Megalomania, Malice reflect and taint-zone mechanics, all bound to old Canary `47dfd51f...`. The canonical Quest identity exists in the Quest authoring pipeline, but runtime/native lowering remains held.

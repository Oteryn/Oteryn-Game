# OTV2-20260930-soul-war-mechanics-design

```yaml
task_id: OTV2-20260930-soul-war-mechanics-design
title: Soul War mechanics design (SW-1..6) and the creatures without a Canary file, with the owner answers
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/friendly-albattani-rfzvku
pr: null  # the PR on this branch is authoritative
base_sha: eba9a271
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world import"
created_at: 2026-09-30T07:00:00Z
updated_at: 2026-09-30T09:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md (new section 10)
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md (new section 13, header status line)
  - tools/content-schema/monster-authoring/samples/wiki-only-candidates-2026-09-30.json
  - tools/content-schema/monster-authoring/wiki_only_candidates.py
  - docs/agents/tasks/archive/OTV2-20260930-soul-war-mechanics-design.md
public_contracts: []
depends_on:
  - "owner answers in the task session 2026-09-30: 5a 6a 7a 8a 9c 10a 11c 12a 13a 14a 15b 16a (section 13.6 Q1-Q6, the Infernal Demon and Many Faces answers, section 10.3)"
blocks:
  - "SW-2 implementation (first), then SW-1, SW-5, SW-3, SW-4, SW-6"
  - "crystal_batch.py widening for the 50 creatures with a Crystal file elsewhere"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Design only (tasks C and D preparation of the monster-unblocking plan). Monster schema section 10 holds SW-1 (fear
windup) and SW-2 (magic wall removal, no schema change) and the answers for the creatures without a Canary file.
Encounter format section 13 holds SW-3..6, the owner questions with the answers, and the second source check
(official 2020 news through Tibiopedia, Fandom page histories, Tibiopedia, TibiaQA, Gudii's guide transcripts,
CrystalServer, the TibiaWiki BR capture). The answer numbering in the session (5-16) maps to section 13.6 Q1-Q6, the
Infernal Demon and Many Faces answers, section 10.3 (11-13) and the BR decision (16); the table below gives each
one with the option text the owner accepted (the session asked in Polish; the English is the option as recorded).

| Session | Maps to | Accepted option |
|---|---|---|
| 5a | §13.6 Q1 a | Taint teleport as the wiki says: only first-taint holders, a check every 2 s, the 10 s cooldown always ends, and Dreadful Harvester teleports too. |
| 6a | §13.6 Q2 a | Rotten Wasteland: the whole rectangle the script names. |
| 7a | §13.6 Q3 a | Fear hits the caster's current target when the 2 s windup ends (wiki). |
| 8a | §13.6 Q4 a | Canary's ability lists stay everywhere (D18). |
| 9c | §13.6 Q5 b | Mirror Image: 70% the attacker's vocation (Canary), and a 1 hit point floor against damage that is not the player's own (wiki). The session's option c was this combination; in the document it is Q5 b. |
| 10a | §13.6 Q6 a | The Cloak of Terror pool appears when a player hits it (wiki). |
| 11c | §10.3 | The 50 creatures with a Crystal file in another directory come from Crystal with the wiki applied over its values. |
| 12a | §10.3 | Ordinary monsters first; quest, event, raid and summon-like creatures wait. |
| 13a | §10.3 | The 64-creature list is committed as evidence (`samples/wiki-only-candidates-2026-09-30.json`). |
| 14a | §13.6, further answers | Infernal Demon follows the wiki: frequent retarget to the lowest maximum health (values still to be sourced). |
| 15b | §13.6, further answers | Many Faces critical hits: keep Canary (no critical hits). |
| 16a | §13.6, further answers | No TibiaWiki BR quest-page capture. |

## Validation

`git diff --check`; `validate_governance.py`; `wiki_only_candidates.py --check` (64 creatures). No content changes.

Independent review of `3622ba61` (FIX): the answer mapping, the answers applied to the normative text (SW-1
target, SW-3 Dreadful Harvester and the cooldown start, SW-4 floor, SW-6), the counts and digests of the evidence
JSON with its generator, the ACCEPTED wording and the evidence labels are addressed on the next head.

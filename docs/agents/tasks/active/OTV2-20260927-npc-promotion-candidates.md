# OTV2-20260927-npc-promotion-candidates

```yaml
task_id: OTV2-20260927-npc-promotion-candidates
title: NPC promotion candidates with native keys and Item join; WorldProject/v2 NPC admission route (D4-D7)
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 983
jira: KAN-16
base_sha: ec0e12a7927dcd4d98f7d1151f6b8ee100c1b65c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-npc-promotion-candidates.md
  - docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md
  - tools/content-schema/npc-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Follow-up to PR #975 (`OTV2-20260927-npc-authoring-schema-v1`, archived). Owner decisions in this
session: native key `oteryn:npc.<slug>` (D4), Oteryn-authored NPC text (D5), TibiaWiki (Fandom) as
tie-breaker between Canary and Crystal (D6), and WorldProject/v2 extended with typed travel routes and
offer quantities before admission (D7). This task merges both sources into native-keyed promotion
candidates (definition, placements, ungated travel, trade joined to native Items through the protected
Item identity map) and records the admission route `OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md`
(slice 1 of its §7). No `content/` change; slices 2–4 follow.
Authority: direct owner request in this session.

## Architecture and source of truth

- `docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md` §3 D4–D6 and §8 (this task). PROVEN by
  `validate_promotion.py` and its tests.
- Inputs: the pinned Canary/Crystal conversions and the Fandom snapshot of PR #975 (snapshot SHA-256
  recorded in the candidates). Evidence class `OTS_HYPOTHESIS_ONLY`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline evidence tooling only; no production mutation, fence, session, authority
or persisted-recovery evidence is touched.

## Acceptance and evidence

- `samples/promotion-candidates-v1.json`: 984 candidates; 289 trade services (10,723 offer rows),
  53 travel services (189 routes); 154 facts decided by the wiki; held 139 NPCs plus 6 not loadable;
  left out 218 offers and 65 routes. Snapshot and Item map SHA-256 recorded (map `83ba3c26…` equals the
  digest in the Item classification crosswalk evidence). Deterministic.
- `validate_promotion.py` passes on the sample; `test_promotion.py` and `test_npc_authoring.py` pass.

## Next action

Slice 2 of the admission route: Rust extension of `ProjectV2ServiceOffer` (count, sub type) and
typed travel routes on `Service`, with validation and tests.

# OTV2-20260927-npc-promotion-candidates

```yaml
task_id: OTV2-20260927-npc-promotion-candidates
title: NPC promotion candidates with native keys (owner decisions D4-D6)
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: null
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
tie-breaker between Canary and Crystal (D6). This task merges both sources into native-keyed
promotion candidates (definition, placements, ungated travel) as evidence. Writing them into
`content/` needs the content-tree family extension and review (O8) and is not done here.
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

- `samples/promotion-candidates-v1.json`: 985 candidates (53 with travel, 189 routes, 17 wiki-decided
  facts); held: 91 unplaced, 32 single-source not on the wiki, 7 definition conflicts, 6 placement
  conflicts, 2 key collisions; 67 routes left out (46 gated). Deterministic.
- `validate_promotion.py` passes on the sample; `test_promotion.py` and `test_npc_authoring.py` pass.

## Next action

Owner/control-plane decision on O8 (NPC and Service families in the generated content tree and its
review), then a promotion task writes the candidates to `content/npcs/definitions/` and
`content/services/travel/`.

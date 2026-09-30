# OTV2-20260930-proficiency-authoring-schema

```yaml
task_id: OTV2-20260930-proficiency-authoring-schema
title: Weapon Proficiency authoring schema candidate v1 (static definition catalogue only)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/inspiring-lamport-nc623x
issue: 162
lane_id: content population (reference source staging)
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: c145b7f
head_sha: null   # a commit cannot hold its own SHA
final_head_sha: null
final_head_frozen_at: null
owner: "owner-launched Claude Code session (session_012FuykcnY5errmsP3T1E4Nm)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/proficiency-authoring/**
  - .github/workflows/proficiency-authoring-schema.yml
  - docs/agents/tasks/archive/OTV2-20260930-proficiency-authoring-schema.md
public_contracts: []
depends_on:
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Owner request in session: create the schema for Weapon Proficiency. A closed JSON Schema candidate and a
validator describe the static `Proficiency` definition catalogue of PROFICIENCY-0 §4.1. A candidate
catalogue of all 443 definitions (3,671 perks, 33 kinds) is built from the staged 15.30 client source with
the owner-accepted D199 code map and validates. Nothing is written to `content/`, `rulesets/`, Item schemas
or runtime code, and no identity is registered; that stays with PROF-CONTENT-1.

## Source verification

- PROVEN: the staged files match their manifest digests, and the manifest pins client file `7fea90ec…`.
- PROVEN: every source `Type` 0-32 and every `SkillId`, `ElementId`, `DamageType` and `AugmentType` in the
  443 definitions maps through D199; every perk decodes back to its staged source perk exactly.
- CORROBORATED: TibiaPal's planner (owner-supplied link) uses the same 33 perk names and element codes.
  Its share token (`p` per-level index, `-1` unassigned; `s` shaped perks with rank) is Character state
  and shaping, not content, so it is not modelled.

## Owner decisions (in session, 2026-09-30)

1. Thresholds are catalogue-level tables per class, not per definition, because D197/D198/D200 choose
   the class per weapon binding (as Canary does). This refines PROFICIENCY-0 §4.1.
2. A weapon's threshold class comes from one rule, applied when content is built; the result is written
   to the Item binding and checked in CI; `unknown` gets no proficiency; exceptions need a cited
   override. PROF-CONTENT-1 implements it.
3. The point table is TibiaWiki `Weapon_Proficiency` revid 1192598 (base by Bestiary difficulty,
   +10% per influence stack, ×2.5 fiendish, bosses 500/5,000/15,000), admitted as Reference evidence.
   This lifts that PROFICIENCY-0 §4.5 parity gate. Canary/Crystal values are not used; soulpit bosses
   give nothing until evidenced. PROF-2 encodes it. The PROFICIENCY-0 document is amended by its owner
   lane, not here.
4. This schema ships as its own PR; PROF-CONTENT-1 follows separately.

## Assumptions (reversible, listed for review)

- Revision `definition-r1` for every definition, as for Charms.

## Validation

- `python proficiency_authoring.py build --check`: up to date.
- `python proficiency_authoring.py validate samples/proficiencies-candidate.json`: valid.
- `python test_proficiency_authoring.py`: 4 tests pass.
- Canary `04b83b5` and Crystal `96d13ef` read for cross-checking (README, `OtsHypothesisOnly`).
- `ruff check .` and `ruff format --check .`: clean.

## Closeout

- merge commit/result: squash merge of the PR recorded on #162.
- review: none required beyond repository CI; no protocol, persistence, identity or authority change.

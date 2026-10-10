# OTV2-20261007 Shards E1 max-health admission

```yaml
task_id: OTV2-20261007-shards-e1-max-health-admission
title: Admit the accepted bounded Encounter max-health vocabulary
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/shards-e1-max-health-admission-20261007
pr: null
base_sha: d0b091b6b5ad4b9527d354df0043c6e55d5861cc
owner: chatgpt-shards-reconstruction-lead
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/project/v2/encounter.rs
  - apps/game-server/tests/content_world_project_v2_encounter_admission.rs
  - tools/content-schema/encounter-authoring/build_schema.py
  - tools/content-schema/encounter-authoring/encounter.schema.json
  - tools/content-schema/encounter-authoring/validate_encounter.py
  - tools/content-schema/encounter-authoring/test_max_health_attribute.py
  - tools/content-schema/monster-authoring/build_formal_schema.py
  - tools/content-schema/monster-authoring/monster.schema.json
  - docs/agents/tasks/OTV2-20261007-shards-e1-max-health-admission.md
public_contracts:
  - ENCOUNTER-RT-0 section 17.2
  - OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1 section 9.4
depends_on:
  - PR-1891
cross_repository_coordination_id: 1622
```

## Outcome and authority

The schema and typed Encounter admission accept `attribute(max_health, set)` only with an
absolute value in 1..=u32::MAX, or `reset` without a value. Existing defense and outgoing
damage attributes retain add/reset. Monster authored maximum and initial health use the
same upper bound; native Creature health is already u32.

PROVEN contract: merged #1891 and ENCOUNTER-RT-0 section 17.2. Human option A allocated
SHARDS-Q1/I1/E1/R1 and necessary existing-contract adapters; recorded in #1622 comment
6046563239. #1852 remains a separate evidence-only candidate. This slice does not touch
the materializer or the paths currently owned by #1924/#1926.

## Acceptance and qualification

- Authoring regression was RED for all three valid set boundaries and reset before the patch.
- The authoring boundary covers zero/negative/overflow, nonabsolute/counter, missing value,
  unsupported add, reset with value, and existing-attribute set refusals.
- Native WorldProject admission independently covers the accepted absolute bounds, missing
  values, wrong operations, counter substitution and existing attribute compatibility.
- Existing authoring schema verifier: 233 checks PASS; focused Python suite: 5 tests PASS.
- Native Encounter integration target: 11 tests PASS on Linux/Rust 1.94.0.
- Full package formatting, Clippy, tests and hosted exact-head qualification are required
  before review/merge; their final receipts belong to live PR/coordination state.

Authority/recovery model: NOT_APPLICABLE in this declarative admission slice. No current
session/lease/generation facts are consumed, no production mutation or durable write is
introduced, and no controller or persisted recovery authority is installed. Independent
exact-head review remains required for the public typed contract vocabulary.

## Integration boundary

Runtime max-health override/clamp, percent and Charm reads, no implicit heal, and the
Magnolia inline-first-lethal/queued-full-heal sequence remain ENC-COMBAT-1/ENC-RT-1 work.
No encounter is activated and no unknown timer, damage or spawn constant is supplied.
This admission slice does not establish a playable Shards quest.

Rollback removes the new vocabulary before any dependent content is admitted; existing
add/reset documents are unchanged. No persisted state, wire field or database migration.

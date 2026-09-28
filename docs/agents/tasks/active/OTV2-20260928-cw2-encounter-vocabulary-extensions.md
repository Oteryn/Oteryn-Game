# OTV2-20260928-cw2-encounter-vocabulary-extensions

```yaml
task_id: OTV2-20260928-cw2-encounter-vocabulary-extensions
title: CW2 encounter vocabulary extension candidates for the remaining unresolved_semantics rows
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/cw2-encounter-vocabulary-extensions
pr: 1183
base_sha: 3dcf3c82
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world import"
created_at: 2026-09-28T22:00:00Z
updated_at: 2026-09-28T22:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1.md (the unresolved-semantics rows of the four bosses only)
  - docs/agents/tasks/active/OTV2-20260928-cw2-encounter-vocabulary-extensions.md
public_contracts: []
depends_on:
  - "owner decision on #162, 2026-09-28, batch item 4 (the lane proposes extensions from CrystalServer/Canary behaviour; the owner accepts the finished design)"
  - "allocation on issue #162: ALLOCATIONS item B"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Docs only. The new §12 of `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` proposes CANDIDATE resolutions, pending owner
acceptance, for the `unresolved_semantics` rows of four encounters. Evidence is cited as file:line in CrystalServer
`ff7ede593c69d4c658b382c97443e8155926924a` (`data-global/`; `data-crystal/` carries none of these scripts).

| Boss | Mechanic | Proposal |
|---|---|---|
| Alptramun | `alptramun summon` escalation | no extension; D29 `ability_cast` with D45 covers it; the row is resolvable in the vocabulary, and evidence waits on Q1 |
| Gorzindel | the tome's portal to the first free knowledge room for 10 s | CW2-1: `teleport who: {triggering: true}`, with `in_anchor` of `triggering` gating the delayed return to players still in the instance |
| Melting Frozen Horror | death actions on two fixed tiles | no extension; the lever names `dragon_egg` and `solid_frozen_horror`; resolvable now |
| The Sandking | the stage counter and the brood cycle behind it | CW2-2: `stepped_on corpse_of: role`; CW2-3: `map_item remove triggering: true`; CW2-4: `random_in` with an optional `free: true` (existing `random_in` semantics unchanged) |

The extensions widen parameters of existing terms. No trigger, condition or action kind is added.

Owner questions (deviations and product choices only): Q1 Alptramun (follow Canary's never-cast spell, or the
wiki), Q2 Gorzindel room and return defects, Q3 Sandking brood spawn failure, Q4 Melting Frozen Horror killed
while parked (recommended: follow Canary).

## Excluded scope

- Ferumbras Mortal Shell, which waits for a quest-domain contract.
- Any change to the schema, samples, tools, Rust or `content/`. Those follow in the implementation order of §12.5,
  after acceptance.
- The dragon egg's own events, which the Melting Frozen Horror admission still waits for.

## Validation

- The authored-JSON examples were checked against the current schema. The rules without new terms pass. The only
  failures are the new terms (CW2-1..4).
- `python3 tools/agents/validate_governance.py` and `python3 tools/repository/validate_repository_policy.py`: see
  the PR.

## Context checkpoint

last_progress: review round 3 on #1183 addressed (decision test added as §12.7; design unchanged)
jira: pending (no mapped Story resolved in this worker session)

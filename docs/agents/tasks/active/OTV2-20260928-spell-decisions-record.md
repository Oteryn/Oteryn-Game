# OTV2-20260928-spell-decisions-record

```yaml
task_id: OTV2-20260928-spell-decisions-record
title: Record the accepted spell decisions SPELL-D1 to D6 and S6 to S10
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1089
base_sha: 0f80b8c1ca70d50551141d627e5223e28c3233ff
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md
  - docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md
  - docs/agents/tasks/active/OTV2-20260928-spell-decisions-record.md
  - docs/agents/tasks/active/OTV2-20260928-dur03-maxima-death-identity-decision.md   # archive move after #1079
  - docs/agents/tasks/archive/OTV2-20260928-dur03-maxima-death-identity-decision.md
public_contracts:
  - spell cast wire and own-actor vitals (candidate V1)
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task writes the owner-accepted verdicts from #162 comment 5867161696 into both spell documents.
The owner asked the architect to record them ("tak", 2026-09-28).

- The wire contract §8 records SPELL-D1 to SPELL-D6: three accepted as proposed and four accepted
  with changes. The changes are:
  - protocol IDs 3/3 proposed, assigned by the protocol owner;
  - a revision-local spell index;
  - the vitals initial value and production gate;
  - the wiki-first source for maximum vitals;
  - a measured SPELL-RL-04.
- The authoring schema §5 marks S6 to S10 as decided, and S9 gets a closed cooldown-group catalogue.
- After integration, #162 allocates spell plan step P3b-1 (registries and codecs) to the protocol owner.

## Architecture and source of truth

- `PROVEN`: `PROTOCOL_OTERYN_V1_REGISTRY.json` on `main@0f80b8c` (command 2 and domain 2 taken by
  USE-WIRE-V1); VSL-COMBAT-01 §24.1, §24.3 row E and §24.5; spell schema S3, S4, S11 and S13-S15.
- `DERIVED`: this edit records the accepted verdicts and adds no new rule.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` to this recording task. It changes no protocol registry, state or code. The
composition child (P3b-2) authorizes a COMMIT gated by session and runtime fences, so it must
complete this qualification itself.

## Acceptance criteria

- [ ] Both documents record the verdicts exactly as in 5867161696.
- [ ] Governance and repository-policy validators pass.
- [ ] Independent exact-head review, because the protocol and state contract changes status.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Registries, the proto file, codecs, runtime and client (P3b-1 and later).
- Product values beyond the vitals source rule.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.
- The #1079 task record is archived with terminal integration evidence (`0f80b8c`).

## Context checkpoint

```yaml
last_progress: authored; PR #1089 open
status: validating
branch: claude/gifted-rubin-a0axzx
pr: 1089
owner_action_required: null
blocker: null
next_action: exact-head review and Merge Queue integration of #1089
```

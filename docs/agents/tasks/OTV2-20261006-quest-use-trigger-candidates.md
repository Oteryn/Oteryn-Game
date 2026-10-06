# OTV2-20261006 Quest USE Trigger Candidates

```yaml
task_id: OTV2-20261006-quest-use-trigger-candidates
title: Qualify exact same-Quest literal Item-id USE trigger candidates
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: codex/quest-collect-claim-candidates-20261006
base_pr: 1888
branch: codex/quest-use-trigger-candidates-20261006
base_sha: 0a639944004551c1211e390e240c93834d18050d
owner: chatgpt-quest-completion
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/quest-authoring/use_trigger_candidates.py
  - tools/content-schema/quest-authoring/test_use_trigger_candidates.py
  - tools/content-schema/quest-authoring/samples/server-completion/use-trigger-candidates.json
  - tools/content-schema/quest-authoring/run_checks.py
  - docs/agents/tasks/OTV2-20261006-quest-use-trigger-candidates.md
depends_on:
  - PR #1888
  - A12 Item identity decision
  - QUEST-TRIGGER-1 for any future executable admission
```

## Outcome

Build a deterministic, evidence-only join from chosen `use` stages to donor source
`USE` interactions already associated with the same canonical Quest.

The only numeric bridge permitted here is the accepted A12 identity rule:

`oteryn:item.tibia.i<id>` = canonical CipSoft Tibia Item identity.

The tool compares that canonical Item id only with literal source Action registrations of
the form `id(123, ...)`. It does **not** use action ids, unique ids, positions, symbolic
expressions, display-name similarity or other donor numeric domains as identity proof.

## Result

Current binding plan contains **431** use stages.

| Classification | Count |
| --- | ---: |
| `ONE_SAME_QUEST_LITERAL_ID_USE_TRIGGER_COVERS_ALL_TARGETS` | **4** |
| `NO_SAME_QUEST_LITERAL_ID_USE_TRIGGER_MATCH` | **40** |
| `TARGET_IDENTITY_INCOMPLETE` | **387** |

The four exact source-trigger candidates are:

1. The Cursed Crystal `s5` -> `canary:interaction/the_cursed_crystal/actions_medusa_oil`
2. The Cursed Crystal `s7` -> the same Medusa Oil source interaction
3. Brotherhood Outfits `s3` -> `canary:interaction/dreamers_challenge_quest/actions_documents`
4. Brotherhood Outfits `s4` -> the same Documents source interaction

In every positive row the donor `id(...)` registration contains additional Item ids beyond the
stage target set. Therefore the source interaction is only a trigger candidate:

- `stage_branch_semantics_proven=false`
- `native_placement_binding_proven=false`
- `command_occurrence_binding_proven=false`
- `native_dispatch_binding=null`
- `runtime_admitted=false`

## Authority boundary

This packet proves only:

- canonical stage Item identity under A12;
- same canonical Quest source association;
- one source `USE` interaction with a literal Item-id registration that contains all stage targets.

It does not prove:

- which internal branch of that Lua interaction corresponds to the chosen stage;
- a native LocalObject/placement;
- a current ChannelRuntime use occurrence;
- a CommandRef;
- QUEST-TRIGGER-1 admission.

The future executable binding remains owned by the accepted world/use/Quest-trigger runtime.

## Validation

- `python use_trigger_candidates.py`: generated deterministic packet.
- `python -m unittest test_use_trigger_candidates.py`: 3/3 PASS.
- `python use_trigger_candidates.py --check`: PASS.
- Drift check is added to Quest `run_checks.py`.
- No runtime admission or Quest transition selection.

## Next action

After review, use the four rows only as source-trigger evidence for QUEST-TRIGGER-1 binding.
Continue exact placement/branch qualification without broadening numeric identity domains.

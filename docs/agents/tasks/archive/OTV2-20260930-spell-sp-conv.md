# OTV2-20260930-spell-sp-conv

```yaml
task_id: OTV2-20260930-spell-sp-conv
title: Spell converter emits the S27 party_buff for the party spells (SP-CONV)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1335
allocation_comment: "#162 5909477417"
base_branch: main
branch: claude/eager-pasteur-eobvo3
base_sha: a795d5fe
owner: "Oteryn: spell authoring" (Claude Code)
created_at: 2026-09-30T11:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/spell-authoring/**
  - docs/agents/tasks/archive/OTV2-20260930-spell-sp-conv.md
public_contracts: []
depends_on: []
blocks: [OTV2-20260930-spell-harmony-gate]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- The converter emits `native_behavior` `party_buff` (C.3, admitted by the core since #1224) from
  `party-behaviours.json`. Heal, Protect and Enchant Party become ready: readiness 164/88 -> 167/85, revision
  `spell-p2-r13`. The base mana of Protect and Enchant Party (90, 120) comes from Canary only (C.3 Q16).
- Train Party stays blocked on C.3 Q15 (a BR/Fandom conflict, owner under S3); Enlighten Party on C.3 Q14.
- `verify_spells.py` reads a scaled `party_buff` mana as "varies"; `ours_differs_ready` stays 0.

## Not done (outside owned paths)

Nature's Embrace, Intense and Ultimate Healing Rune, and Cancel Magic Shield map to admitted fields
(`targeting.allowed_targets`, rune vocations, `remove_condition`), but their scripts guard the combat and
`tools/content-schema/monster-authoring/spell_scripts.py` classifies a guarded script as P4. A follow-up needs that
file in scope.

## Validation

- The CI conversion job reproduces from the pinned sources (census, readiness, verify `cmp`; starter `diff -r`).
- `validate_spell.py` passes on the starter bundles and the 3 party bundles; 6 self-tests pass; `verify_formal_schema.py` 74/74.
- `spell_from_bundle` admits the 3 party bundles (temporary local test, not committed).

## Closeout

Archived in the PR's final authoring commit. Integration, freeze SHA and review state are in PR #1335 and #162.

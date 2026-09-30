# OTV2-20260930-spell-sp-conv-2

```yaml
task_id: OTV2-20260930-spell-sp-conv-2
title: Train Party (D213) and accepted onCastSpell guards in the spell converter (SP-CONV-2)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1345
allocation_comment: "#162 5910691466"
base_branch: main
branch: claude/eager-pasteur-eobvo3
base_sha: 1614042d
owner: "Oteryn: spell authoring" (Claude Code)
created_at: 2026-09-30T12:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/spell-authoring/**
  - tools/content-schema/monster-authoring/spell_scripts.py
  - docs/agents/tasks/archive/OTV2-20260930-spell-sp-conv-2.md
public_contracts: []
depends_on: [OTV2-20260930-spell-sp-conv]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Readiness 167/85 -> 172/80, revision `spell-p2-r14`.
- **Train Party** is a `party_buff` per D213: +3 `skill_melee` (sword, axe, club), `skill_distance` and `skill_fist`, with base mana 60 (Canary, C.3 Q16).
- **Accepted guards** (`guard-behaviours.json`). `SpellScripts(accepted_guards=...)` evaluates a script whose exact
  whitespace-collapsed Canary 99902524 `onCastSpell` body is listed, and that declares one Combat, to that Combat
  (P2). Any other body stays P4. The monster converter passes no guards, so its output is unchanged.
  - Nature's Embrace: `allowed_targets: not_self`.
  - Intense and Ultimate Healing Rune: `allowed_targets: self_or_own_summons`. The rune vocations already exclude
    both monks. `COMBAT_PARAM_TARGETCASTERORTOPMOST` is not authored (D.3 remainder; the core takes the rune target
    from the Target Resolver).
  - Cancel Magic Shield: an extra `remove_condition` `manashield` effect, the converter's name for
    `CONDITION_MANASHIELD`. C.5 Q23 stays open.
- The Crystal bodies of these spells differ (Shared Conservation heal, Leiden special case), so they stay custom.
  Canary converts them, and the manifest records the Crystal side as an approved omission.

## Validation

- The CI conversion job reproduces locally from the pinned sources: census `cmp`, and readiness, verify and
  starter regenerated. The starter changes are revision-only. `ours_differs_ready` is 0.
- `validate_spell.py` passes on all 172 ready bundles. The 6 self-tests pass, and `verify_formal_schema.py` gives 74/74.
- `spell_from_bundle` admits Train Party, Nature's Embrace, Cancel Magic Shield and both rune bundles. This was
  checked with a temporary local test that is not committed.

## Closeout

Archived in the PR's final authoring commit. The PR number, freeze SHA and review state are in the PR and in #162.

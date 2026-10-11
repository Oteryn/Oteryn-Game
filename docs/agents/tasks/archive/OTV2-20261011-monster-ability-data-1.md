# OTV2-20261011-monster-ability-data-1

```yaml
task_id: OTV2-20261011-monster-ability-data-1
title: "Export monster attack ability authoring for every native attacks[] reference"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: content
base_branch: main
branch: claude/monster-ability-data-1-20261011
pr: 0
base_sha: 967d7012f
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-11
updated_at: 2026-10-11
packet: "owner decision 2a (#1622): first a content packet"
owned_paths:
  - content/abilities/source-imports/**
  - tools/content-schema/native-gameplay/build_monster_attack_authoring.py
  - tools/content-schema/native-gameplay/validate_monster_attack_authoring.py
  - tools/content-schema/native-gameplay/test_monster_attack_authoring.py
  - docs/agents/tasks/archive/OTV2-20261011-monster-ability-data-1.md
depends_on: []
public_contracts: []
external_repositories: []
```

## Outcome

Every distinct ability reference in the `attacks[]` schedules of `spell-native-profiles.json`
(5,776 references over 6,030 entries) has a record in
`content/abilities/source-imports/monster-attack-authoring.jsonl`: 5,115 `authored` with damage
minimum/maximum, element, range, area and donor evidence, 661 explicitly `unavailable` with a reason.
A pin file binds the records to the exact artifact digest and the prepared population archive. The
validator and test fail when a reference lacks a record, when the artifact digest drifts, or when an
authored record is incomplete.

## Architecture and source of truth

- PROVEN: numbers come only from the per-monster donor bundles in the prepared population archive
  (Canary `47dfd51f`, Crystal summer update `00ce02a5` where a bundle names it). Qualification stays
  `DONOR_ATTESTED_NOT_GLOBAL_VERIFIED`; no Global parity is claimed.
- PROVEN: `CreatureProfileRecord` is `deny_unknown_fields` and `source_item_abilities` accepts only
  Item-changing bodies, so the authoring cannot be embedded in the artifact without a loader change.
- DERIVED: `melee_attack_skill` maximum uses the formula already recorded in
  `build_monster_melee_profiles.py` (`ceil(skill*(attack*0.05)+attack*0.5)`, minimum 0).
- Unavailable reasons: 295 donor health differs from the profile (kept, as the melee builder does),
  269 no unique donor bundle, 36 ability without effects, 33 conflicting bindings of one reference,
  24 attack count differs, 4 ambiguous provenance.

## Excluded

- Embedding the data in `spell-native-profiles.json` and re-pinning its digest in
  `content.lock.json`, `content/manifest.json`, `spells.manifest.json`: blocked on a Rust loader field
  (`apps/game-server/src/content/native_gameplay.rs`, outside the owned paths).
- `monster_ai_cycle.rs` and any runtime consumer (CREATURE-MOVE-1).

## Validation

- `python3 tools/content-schema/native-gameplay/test_monster_attack_authoring.py`: OK (12 tests).
- `python3 tools/content-schema/native-gameplay/validate_monster_attack_authoring.py`: pass.
- `python3 tools/content-schema/native-gameplay/build_monster_attack_authoring.py --check`: pass.
- `python3 tools/content-schema/native-gameplay/test_monster_melee_profiles.py`: OK.
- `python tools/agents/validate_governance.py`: VALIDATION_PENDING
- `python -m unittest discover -s tools/agents/tests`: VALIDATION_PENDING

## Review

Review state on the frozen head is the control plane's; none requested by this worker.

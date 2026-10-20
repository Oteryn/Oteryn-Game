# SPELL-AVAIL-1-20261008

```yaml
task_id: SPELL-AVAIL-1-20261008
title: "SPELL-AVAIL-1: first batch shrinking the unavailable-spells held list"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spell-avail-1-20261008
pr: 1958
base_sha: 348b2b76
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
merge_commit: "squash merge of #1958"
owner: claude-code-session-01NpwnFrGShD3iBmqPJN46xb
control_plane: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-08
updated_at: 2026-10-10
owned_paths:
  - content/abilities/**
  - tools/content-migration/import_current_spell_sources.py (INPUT_SHA256 pin and count pins only)
  - tools/content-schema/spell-authoring/**
  - derived content written by tools/content-migration/regenerate_content.py
  - docs/agents/tasks/archive/SPELL-AVAIL-1-20261008.md
public_contracts: []
external_repositories: []
```

## Outcome

Three mechanisms, seven donor rows, leave `content/abilities/source-imports/unavailable-spells.jsonl`
(486 -> 479 rows) and become `accepted_normalized_model` identity-only bindings in
`current-sources.json` (368 -> 375). The Rust cast paths already exist, so no runtime file changes.

Chosen mechanisms:

- `locate_message`: Find Person and Find Fiend, canary and crystal-summer (four rows).
- `companion_haste`: Swift Foot, canary (one row).
- `mass_spirit_mend`: Mass Spirit Mend, canary and crystal-summer (two rows).

## Decisions

- D607 widened the owned paths; D968 chose the re-scope; D969 ruled that the accepted native profile
  wins over the donor Lua on every conflict (S24 accepted-value-retained, the `antidote_rune` precedent).
  Each retained donor value is recorded in the binding's `preserved_fields`.
- Held: Magic Shield (S27 `source_state_contract` is a native-owner decision), the crystal-main
  native-profile-owner rows and the crystal-summer Swift Foot row (SOURCE_CONFLICT_KEPT).

## Acceptance criteria

- [x] One test group per mechanism: `tools/content-schema/spell-authoring/test_spell_avail_1.py`.
- [x] Derived content regenerated only by `regenerate_content.py`; `content/spells.manifest.json`
  changed because the tool rewrites its hashes.

## Excluded scope

COIN-PROFILE-1 paths, dispatch, `mod.rs`, `connection.rs`, protocol, client, donor files.

## Validation

- `python3 tools/content-schema/spell-authoring/test_spell_avail_1.py`: OK
- `python3 tools/content-schema/spell-authoring/test_current_source_inventory.py`: OK
- `python3 tools/content-schema/spell-authoring/test_current_spell_adapter.py`: OK
- `python3 tools/content-schema/spell-authoring/test_native_source_projection.py`: OK
- `python3 tools/content-schema/spell-authoring/test_executable_catalog.py`: OK
- `python3 tools/content-migration/test_import_current_spell_sources.py`: OK
- `cargo test --test current_spell_sources --test spell_lane_key33_pin`: pass
- `python tools/content-migration/regenerate_content.py`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK

## Review

Pending on the frozen head.

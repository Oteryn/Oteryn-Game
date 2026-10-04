# OTV2-20261003-item-sem-2b3-vocation-none-and-use-requirements

```yaml
task_id: OTV2-20261003-item-sem-2b3-vocation-none-and-use-requirements
title: "ITEM-SEM-2b-3: vocation None, use requirements and typed artifact v5"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-sem-2b3-20261004
issue: 162
coordination: 1622
pr: 1710
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
merge: "squash merge of #1710"
owner: oteryn-hard-worker (CP session_013KJX6mv8LQveCKKXYgAX94)
created_at: 2026-10-04
updated_at: 2026-10-04
packet: docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md (§1.12, §2.2a; AMEND-1, AMEND-2; D447, D448)
owned_paths:
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/src/content/reference_artifact.rs
  - apps/game-server/src/content/project/v2.rs   # unchanged: the content tree serializes the typed semantics directly
  - apps/game-server/tests/content_reference_artifact.rs
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  - tools/content-schema/item-authoring/{lower_wiki_stats_packet.py,test_lower_wiki_stats_packet.py,README.md}
  - apps/game-server/src/content/item_stats_promotion.rs
  - content/world/** and the content tree (regenerated)
  - tools/reference-item-resource-profile/**
  - docs/architecture/OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V2.md
  - docs/agents/evidence/OTV2-20261003-item-sem-2b3-v5-resource-evidence.{json,md}
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/OTV2-20261003-item-sem-2b3-vocation-none-and-use-requirements.md
derived_outside_list:
  - docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json             # output of the owned lowering tool
  - docs/agents/evidence/OTV2-20261003-item-equipment-requirements-v1.json      # output of the owned lowering tool
  - apps/game-server/tests/content_world_project_repository.rs                  # regenerate_content.py pins, plus the hand-written pattern census pin (+42)
public_contracts:
  - OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v5
  - OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V2
jira: null   # sync pending (coordinator batch)
```

## Outcome

- **Model.**
  - `ReferenceBaseVocation::None` has wire value 6.
  - `REFERENCE_ITEM_MAX_BASE_VOCATIONS` is 6.
  - `ReferenceItemUseRequirements` holds `min_level`, `min_magic_level`, `vocations` and a required `enforcement_mode` (`ON_USE` only).
  - It is `ReferenceItemSemantics.use_requirements`, which is client-safe and omitted from JSON while Unknown.
- **Artifact v5.**
  - Profile ids `/v5`, magic `OTRPA05`, profile version 5 and body version 3.
  - Group 17 is admitted to both projections.
  - The compiler writes only v5.
  - v4 decodes under its own profile: vocation values 1-5, group ids 1-16, the V1 ceilings and its body version. The v4 encoder and decoder refuse `None`, group 17 and the v5 body version.
  - Either reader refuses the other profile's bytes by manifest profile id (test).
  - An unknown vocation (7), an unknown group (18) and an enforcement mode other than `ON_USE` fail closed (tests).
- **v5 ceilings (D448).** They were recomputed with the tool and registered as `DUR04-REFERENCE-ITEM-PROFILE-V2-*`, each with max and max+1 tests in Rust and in the tool.

  | Resource | v5 ceiling |
  |---|---:|
  | Server / client groups | 17 / 12 |
  | Vocations (pattern, trade, use requirement) | 6 |
  | Server / client record | 3,577 / 3,454 bytes |
  | Server / client body | 136,487,589 / 131,794,278 bytes |
  | Server / client artifact | 177,285,126 / 172,591,815 bytes |
  | Generation pair | 349,876,941 bytes |

  The Rust worst-shape records equal the tool's digests. v4 rows stay for v4 decoding.
- **Lowered (packet 13,414 fields / 6,533 Items).**
  - 7 `without` Items get their patterns with `NONE`.
  - 35 slotless ammunition Items get the Extra pattern (D447).
  - 68 `use_requirements` rows are written: 62 with a level, 30 with a magic level and 41 with vocations. These cover the runes, ammunition, the 5 Extra-slot Items and 1 liquid.
  - `0` levels write nothing, which drops 3 `mlrequired: 0`-only runes.
  - In the tree census, Equipment patterns rise from 1,791 to 1,833.
- **Held (D310, assumption).** i3450's Extra pattern and use requirements are recorded as `sealed_state_holds`, not written. The sealed `tools/content-schema/reward-claim-authoring/reward_stack_normalization.json` pins i3450's definition digest, and that file is outside this task's paths. This is the same precedent as the D310 market hold on i3450.
- **Not in scope.** Enforcement (RUNE-USE-0, RANGED-0) and Premium.

## Validation

- `python3 tools/reference-item-resource-profile/item_resource_profile.py` pass (v5 evidence check)
- `python3 tools/reference-item-resource-profile/item_resource_profile.py --profile v4 --output <scratch>` pass (byte-identical to the pre-change tool)
- `python3 tools/content-schema/item-authoring/test_lower_wiki_stats_packet.py` pass (fields=13414 items=6533). The packet's `python3 -m unittest tools/...` form does not import, because the test is a script whose sibling module is not on `sys.path`; the same happens on `main`.
- `python3 tools/content-schema/item-authoring/lower_wiki_stats_packet.py --check` pass
- `cargo test --locked -p oteryn-game-server --test content_reference_artifact --quiet` pass (9)
- `cargo test --locked -p oteryn-game-server --quiet` pass (lib 1583, all integration targets)
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings` pass
- `cargo fmt --all --check` pass
- `python3 tools/content-migration/regenerate_content.py` pass (idempotent, clean tree)
- `ruff check` / `ruff format --check` on the two changed item-authoring files pass. The resource-profile tool was not ruff-clean on `main` either and is left as it was.
- `python tools/agents/validate_governance.py` pass
- `python -m unittest discover -s tools/agents/tests` OK

Review: independent contract review (Codex) on the final frozen head (CP routes).

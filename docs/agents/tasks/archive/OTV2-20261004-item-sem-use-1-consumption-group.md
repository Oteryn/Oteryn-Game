# OTV2-20261004-item-sem-use-1-consumption-group

```yaml
task_id: OTV2-20261004-item-sem-use-1-consumption-group
title: "ITEM-SEM-USE-1: consumption group 18 and artifact profile v6"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-sem-use-1-20261004
issue: 162
coordination: 1622
pr: 1790
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
merge: "squash merge of #1790"
owner: oteryn-hard-worker (CP session_013KJX6mv8LQveCKKXYgAX94)
created_at: 2026-10-04
updated_at: 2026-10-04
packet: docs/architecture/reviews/OTERYN_GAME_ITEM_SEM_USE_FOOD_AND_POTION_SEMANTICS_PACKET_2026-10-04.md (§1.1-§1.4, §2.1)
owned_paths:
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/src/content/reference_artifact.rs
  - apps/game-server/src/content/project/v2.rs   # unchanged: the content tree serializes the typed semantics directly
  - apps/game-server/tests/content_reference_artifact.rs
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  - tools/reference-item-resource-profile/**
  - docs/architecture/OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V3.md
  - docs/agents/evidence/OTV2-20261004-item-sem-use-1-v6-resource-evidence.{json,md}
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/OTV2-20261004-item-sem-use-1-consumption-group.md
public_contracts:
  - OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v6
  - OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V3
deferred_findings_carried:
  - "#1767 P2 4177976623: ITEM-USE-1 owns both restores; the content model records them only"
  - "#1767 P2 4176976626: the food cap is 1..=1,199"
deferred_findings_left:
  - "#1767 P2 4178045467: ITEM-SEM-USE-2"
jira: null   # sync pending (coordinator batch)
```

## Outcome

- **Model.**
  - `ReferenceItemSemantics.consumption` is a `ReferenceItemField<ReferenceItemConsumption>`. It is
    server-only (not in `client_projection`) and omitted from JSON while Unknown.
  - `ReferenceItemConsumption` is `Food { regeneration_seconds }` or
    `Potion { restores, empty_flask }`. `ReferencePotionRestore` holds `resource`
    (`ReferenceRestoreResource::Health` 1 or `Mana` 2), `min` and `max`.
  - Validation: food 1-1,199; one or two restores, strictly ordered by resource (so no duplicate
    and Health before Mana); `1 <= min <= max <= 10,000`; a flask that is Known (a valid target)
    or NotApplicable.
- **Artifact v6.**
  - Profile ids `/v6`, magic `OTRPA06`, profile version 6 and body version 4.
  - Group 18 is server-only; the client allowlist is unchanged at 12.
  - The compiler writes only v6.
  - v5 decodes under its own profile: group ids 1-17, the V2 ceilings and its body version. The
    v5 encoder and decoder refuse consumption, group 18 and the v6 body version.
  - Either reader refuses the other profile's bytes by manifest profile id (test).
  - An unknown variant, an unknown resource, a fourth group-18 restore count and a dangling flask
    ordinal fail closed on decode (tests). A Known flask that does not resolve fails the compile
    of a linked family (test).
- **v6 ceilings.** They were recomputed with the tool and registered as
  `DUR04-REFERENCE-ITEM-PROFILE-V3-*`, each with max and max+1 tests in Rust and in the tool.

  | Resource | v6 ceiling |
  |---|---:|
  | Server / client groups | 18 / 12 |
  | Potion restores | 2 |
  | Server / client record | 3,598 / 3,454 bytes |
  | Server / client body | 137,288,886 / 131,794,278 bytes |
  | Server / client artifact | 178,086,423 / 172,591,815 bytes |
  | Generation pair | 350,678,238 bytes |

  The Rust worst-shape records equal the tool's digests. The v5 rows stay for v5 decoding.
- **Authoring path.** `project/v2.rs` is unchanged. The project tree carries
  `ReferenceItemSemantics` through serde, so `consumption` round-trips through the project
  documents with no lowering code (integration test). Lowering wiki facts into content rows is
  ITEM-SEM-USE-2.
- **No content rows.** No content file changes: the content tree pins no artifact profile id.
- **Not in scope.** The restore runtime and the `FoodRegeneration` condition (ITEM-USE-1,
  FOOD-REGEN-1), effect families, any wire change and ITEM-SEM-USE-2.

## Validation

- `python3 tools/reference-item-resource-profile/item_resource_profile.py`: PASS (v6, no drift).
- `python3 tools/reference-item-resource-profile/item_resource_profile.py --profile v5`: PASS (no drift).
- `cargo fmt --check`: clean.
- `cargo test --locked -p oteryn-game-server --quiet`: pass.
- `cargo test --locked -p oteryn-game-server --test content_reference_artifact --quiet`: pass.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: clean.
- `python tools/agents/validate_governance.py` pass
- `python -m unittest discover -s tools/agents/tests` OK

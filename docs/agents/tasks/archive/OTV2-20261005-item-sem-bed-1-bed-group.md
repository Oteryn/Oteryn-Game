# OTV2-20261005-item-sem-bed-1-bed-group

```yaml
task_id: OTV2-20261005-item-sem-bed-1-bed-group
title: "ITEM-SEM-BED-1: bed item group 19 and artifact profile v7"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-sem-bed-1-20261005
issue: 162
coordination: 1622
pr: 1857
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
merge: "squash merge of #1857"
owner: oteryn-hard-worker (CP session_013KJX6mv8LQveCKKXYgAX94)
created_at: 2026-10-05
updated_at: 2026-10-06
packet: docs/architecture/reviews/OTERYN_GAME_ITEM_SEM_BED_GROUP_PACKET_2026-10-05.md (§1.1-§1.4, §2.1)
owned_paths:
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/src/content/reference_artifact.rs
  - apps/game-server/src/content/project/v2.rs   # unchanged: the content tree serializes the typed semantics directly
  - apps/game-server/tests/content_reference_artifact.rs
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  - tools/reference-item-resource-profile/**
  - docs/architecture/OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V4.md
  - docs/agents/evidence/OTV2-20261005-item-sem-bed-1-v7-resource-evidence.{json,md}
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/OTV2-20261005-item-sem-bed-1-bed-group.md
public_contracts:
  - OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v7
  - OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V4
dependents:
  - BED-CONTENT-1 (bed facts and lowering) waits on this task
jira: null   # sync pending (coordinator batch)
```

## Outcome

- **Model.**
  - `ReferenceItemSemantics.bed` is a `ReferenceItemField<ReferenceItemBed>`. It is server-only
    (not in `client_projection`) and omitted from JSON while Unknown.
  - `ReferenceItemBed` holds `part` (`ReferenceBedPart::Head` 1 or `Foot` 2),
    `partner_direction` (`ReferenceBedDirection` North 1, East 2, South 3, West 4),
    `occupied_male` and `occupied_female` (Item targets; the record's own Item means no change).
  - Set rule, at compile and at load: an occupied target other than the record's own Item must
    carry a Known bed with the same part and partner direction. A target with no bed group, the
    other part or another direction is refused, each by its own test at compile and at load.
- **Artifact v7.**
  - Profile ids `/v7`, magic `OTRPA07`, profile version 7 and body version 5.
  - Group 19 is server-only; the client allowlist is unchanged at 12.
  - The compiler writes only v7.
  - v4 to v6 decode under their own profiles. The v6 encoder and decoder refuse the bed, group 19
    and the v7 body version. Either reader refuses the other profile's bytes by manifest profile
    id (tests).
  - An unknown part, an unknown direction and a dangling target fail closed on encode and decode,
    and group 19 in a client or v6 record is refused (tests).
  - Round trips cover Head and Foot in all four directions with distinct looks, the same look for
    male and female, and no change.
- **v7 ceilings.** They were recomputed with the tool and registered as
  `DUR04-REFERENCE-ITEM-PROFILE-V4-*`, each with max and max+1 tests in Rust and in the tool.

  | Resource | v7 ceiling |
  |---|---:|
  | Server / client groups | 19 / 12 |
  | Cross-Item target slots | 15 |
  | Server / client record | 3,612 / 3,454 bytes |
  | Server / client body | 137,823,084 / 131,794,278 bytes |
  | Server / client artifact | 178,620,621 / 172,591,815 bytes |
  | Generation pair | 351,212,436 bytes |

  The Rust worst-shape records equal the tool's digests. The v6 rows stay for v6 decoding.
- **V3 correction.** v6 had 13 cross-Item target slots (the potion empty flask), not the 12 it
  inherited from V1. `DUR04-REFERENCE-ITEM-PROFILE-V3-CROSS-ITEM-TARGETS` registers 13; V4 §1
  names the correction. Both rows have max, max+1 and dangling tests.
- **Authoring path.** `project/v2.rs` is unchanged. The project tree carries
  `ReferenceItemSemantics` through serde, so `bed` round-trips through the project documents
  with no lowering code (integration test).
- **No content rows.** Bed facts and their lowering are BED-CONTENT-1.
- **Not in scope.** The bed runtime (BED-1), any wire change and BED-CONTENT-1.

## Validation

- `python3 tools/reference-item-resource-profile/item_resource_profile.py`: PASS (v7, no drift).
- `python3 tools/reference-item-resource-profile/item_resource_profile.py --profile v6`: PASS (no drift).
- `cargo fmt --check`: clean.
- `cargo test --locked -p oteryn-game-server --quiet`: pass.
- `cargo test --locked -p oteryn-game-server --test content_reference_artifact --quiet`: pass.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: clean.
- `python tools/agents/validate_governance.py` pass
- `python -m unittest discover -s tools/agents/tests` OK
- `python3 tools/repository/validate_repository_policy.py`: pass.

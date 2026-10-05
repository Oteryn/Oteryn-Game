# OTV2-20261004-chest-quest-bind-1

```yaml
task_id: OTV2-20261004-chest-quest-bind-1
title: "OTV2-20261004-chest-quest-bind-1 CHEST-QUEST-BIND-1 bind chest claims to quest transitions"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: quest
base_branch: main
branch: agent/chest-quest-bind-1-20261004
pr: PENDING
base_sha: 8b783992
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01P3Rba2yKjEkh38HPG5ZwyS
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-05
updated_at: 2026-10-05
packet: "docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_QUEST_WIRING_PACKETS_2026-10-04.md §0, §1.4-§1.5, §2.2 CHEST-QUEST-BIND-1"
depends_on:
  - "QUEST-CAT-BOOT-1 merged (#1801)"
public_contracts: []
external_repositories: []
owned_paths:
  - tools/content-schema/quest-authoring/ots_chests.py
  - tools/content-schema/quest-authoring/samples/chests/
  - tools/content-schema/quest-authoring/quest_state_lowering.py
  - tools/content-schema/quest-authoring/quest_content.schema.json
  - tools/content-schema/quest-authoring/verify_quest_schema.py
  - tools/content-schema/reward-claim-authoring/
  - content/interactions/reward_claims/
  - content/quests/missions/quest-state.json
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/gameplay_transport/quest_catalogue_boot_tests.rs
  - docs/agents/tasks/active/OTV2-20261004-chest-quest-bind-1.md
  - docs/agents/tasks/archive/OTV2-20261004-chest-quest-bind-1.md
```

## Outcome

A chest claim binds a quest transition only where the lowering can prove it.

- `ots_chests.py` records one `progress_write` per claim: the written marker, source expression
  and line, and the integer value (null with a reason where servers or placements disagree).
- The lowering matches a write to a track only when `"canary:quest-progress/" + marker` equals the
  track `source_key` byte for byte, for a `canary:reward-claim/` claim, and constructs
  `oteryn:quest-transition/<marker>/chest` (one `SET`, `from ANY`, `completes: false`) when the
  value lies in the track's bounds. Counts are in `counts.chest_bindings`. The committed lowering
  binds exactly two chests; a crystalserver-keyed claim with an equal marker stays unbound.
- Reward-claim placements carry `quest_transition` only from the constructed key.
- Game: `RewardClaimPlacement.quest_transition` (prefix-validated), `resolve_chest` reads it,
  and boot refuses a key the loaded quest catalogue lacks.
- The `RewardClaimPlacement { }` literals in `tests/` gain `quest_transition: None`; they are
  outside the owned paths and unavoidable. `source_texts.json` is regenerated for the changed
  `ots_chests.py` and `claims.json` digests.

## Validation

- cargo fmt --check: pass
- cargo clippy -p oteryn-game-server --all-targets -- -D warnings: pass
- cargo test -p oteryn-game-server: pass (PG17 served-path case compiles; runs in CI against PostgreSQL 17)
- generators' own tests and regeneration with no diff: pass
- python tools/content-schema/quest-authoring/run_checks.py: PENDING (waits on the owner-approved proof_inputs sha256 update of samples/completion242/source-fix-receipt.json for the regenerated source_migration/bundle.json)
- git diff --check: pass
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: OK

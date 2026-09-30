# OTV2-20260930-derived-content-conflicts

```yaml
task_id: OTV2-20260930-derived-content-conflicts
title: Stop the conflict loop on derived content (regenerate script, one-key-per-line registry)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/brave-cori-ou1zm1
issue: null
pr: 1377
allocation: "owner decisions 1a 2a 3a 4b in session 01KNTDGyVHkgWjcpbgSFx68w, after the #1336 / #1364 conflict"
base_sha: 88fb9b01
owner: "Oteryn: standalone task owner (claude-code-session-01KNTDGyVHkgWjcpbgSFx68w)"
created_at: 2026-09-30
updated_at: 2026-09-30
owned_paths:
  - tools/content-migration/regenerate_content.py
  - tools/content-migration/world_project_v2_to_tree.py  # shared: registry file format only
  - tools/content-schema/charm-authoring/charm_authoring.py  # shared: registry file format only
  - tools/content-schema/proficiency-authoring/proficiency_authoring.py  # shared: registry file format only
  - tools/content-schema/reward-claim-authoring/reward_claim_authoring.py  # shared: registry file format only
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md  # one paragraph under "Stable head and Merge Queue freshness"
  - docs/agents/tasks/archive/OTV2-20260930-derived-content-conflicts.md
public_contracts: []
```

## Problem

Content PRs kept conflicting with each other. After a merge-up, the head moved, and CI, freeze and
review restarted, while the next content PR reached `main`. A replay of `main` from 2026-09-26
paired each commit with the previous commit that touched the same file:

| File | Conflicting pairs |
|---|---|
| `content/content.lock.json` | 28 of 29 |
| `content/world/content.lock.json` | 25 of 26 |
| `content/world/definitions/reference.json` | 16 of 17 |
| `content/world/provenance/sources.json` | 21 of 22 |
| `content_world_project_repository.rs` | 23 of 28 |
| hand-written files, for comparison | 1 to 5 of 13 to 18 |

The cause is derived values committed in the tree: hashes and sizes of other files in the same
repository (`legacy_blobs.*`, `package_provenance_digest`, `documents[].sha256`, the Rust
`DOCUMENTS` and `TREE_SHA256` pins) and family counts. Every content PR changes the same keys,
so no file format avoids the conflict. It is resolved by regeneration every time.

## Outcome

- **`regenerate_content.py --resolve`.** After `git merge origin/main` it:
  - checks that every conflicted path is derived: the managed files of both manifests, the
    `content/world` documents, the registry files, and the Rust test when only its pins
    conflict (the rest of that test is hand-written and is merged, never replaced);
  - regenerates `content/world` with the materializer and re-pins the Rust inventory;
  - runs the tree generator and the Charm, Proficiency and RewardClaim tools;
  - runs the validators and `content_world_project_repository`, then stages the result.

  Any other conflicted path stops the script, and that path needs a person.
- **Registry format.** `content/{project,manifest,content.lock}.json` are written one key per
  line. That halves the `manifest.json` conflicts (20 to 10); the lock still conflicts on
  `legacy_blobs`.
- **Coordinator procedure.** One paragraph: use the script for derived conflicts, and serialize
  work that writes derived content.

The Rust package pins stay. They are the only required-lane guard of the exact package bytes:
`game-gate` is the only required status, and the materialize-and-compare workflow is advisory.
Removing them would weaken protection, so the script re-pins them instead.

## Evidence

- **Determinism.** On the unchanged tree, the script rewrites no byte, and the checks pass.
- **Replay of #1185 against #1175:**
  - #1185 re-authored on the base before #1175: 16 conflicts, all resolved, and the checks
    pass;
  - then `main` merged in: 16 conflicts, all resolved;
  - the final tree equals the real #1185 commit on `main` byte for byte, repository-wide.
- **Replay of SW-1 (#1363) against SW-2:** 50 conflicts. The script stops and lists 9 real
  ones: the materializer, the monster tools, staged evidence, and the non-pin lines of the Rust
  test.

## Open (routed to the architect, owner decision 3a)

- The `content/world` manifest and lock digests (runtime package integrity) cause most of the
  conflicts. Without them, the replay gives 1 of 25 instead of 25 of 25.
- The materializer's input `*_SHA256` constants and the count pins in
  `validate_world_project_v2_to_tree.py` are also hand-edited copies of derived values. They
  conflict in about half of the pairs.
- Making the materialize-and-compare check required would let the byte pins go.

## Validation

- `regenerate_content.py`: the determinism run and both replays above.
- The tree generator, validator and tests pass; `content --check` passes for Charm, Proficiency
  and RewardClaim, and their tests pass.
- `validate_materialized_game_tree.py` and `test_classify_content_routing.py` pass.
- `validate_governance.py` and `validate_repository_policy.py` pass. ruff is clean.

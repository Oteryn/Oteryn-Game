# OTV2-20261005-arch-chest-admit-0

```yaml
task_id: OTV2-20261005-arch-chest-admit-0
title: "ARCH-CHEST-APPEARANCE-ADMIT-V1: admit chest appearances 28827 and 28828, ADR-0021 §4.5 palette-key amendment"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-chest-admit-20261005
issue: 162
pr: 1834
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_CHEST_APPEARANCE_ADMIT_2026-10-05.md
  - docs/architecture/ADR-0021-world-map-runtime-loading.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-chest-admit-0.md
public_contracts: []
depends_on: []
blocks: [OTV2-20261005-chest-appearance-admit-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Answers the control plane's readiness request for CHEST-APPEARANCE-ADMIT-1, which had no packet
  body.
- Rules that appearances 28827 and 28828 are admitted as appearance-only Items under A12 §4.1,
  with the #1795 cascade.
- Amends ADR-0021 §4.5: a palette id takes an Item key only through an `ots/item_server_id`
  binding emitted from A12 §4.2 evidence. The binding generator gains a map-appearance crosswalk
  source, limited to 28827 and 28828. It emits their `EXACT` bindings from Crystal's id rule at
  the pinned revision and identity-projection continuity, and fails closed otherwise. The
  converter and the validator keep their order and add no step.
- The packet owns every file the #1795 cascade moved, including the TibiaWiki navigation facts
  and the world-object qualification pins (#1834 review 4183308722).
- Adds the mandatory five-question decision test (#1834 review 4183505962).
- The validator applies the same palette order, and the packet owns `validate_world_base.py`
  with its tests (#1834 review 4185756367).
- Allocates `tools/content-schema/world-authoring/README.md` to the packet, so the worker
  updates its resolution order, validator order and counts (#1834 review 4186298366).
- Codex round on 446094dc is fixed:
  - Step 2 no longer maps a source id by numeric equality alone. It applies only to the
    allowlist, which carries per-id evidence and fails closed, so `CONFLICT`, `AMBIGUOUS` and
    `NO_MATCH` ids stay unbound. The validator uses the same list (#1834 review 4186765649).
  - The packet requires an independent identity review on its final frozen head (#1834 review
    4186765683).
- Codex round on 1f4f6656 is fixed: the per-id allowlist is replaced by generator-emitted
  `EXACT` bindings from the map-appearance records and projection continuity (A12 §4.2). The
  packet owns the generator, its self-test, the bindings file, the Rust count pin and the live
  bindings-digest pins, and no longer edits the converter or the validator. The allowlist is a
  rejected option (#1834 review 4187334691).
- Codex round on 4aba389d is fixed: the appearance reference is Crystal's own loader rule at
  `00ce02a5` (server id = object id of Crystal's `appearances.dat`; the map loader and the wire
  use that id), pinned by file, lines and Git blob id in the evidence file. The map-appearance
  records, which `fill_links.py` builds by numeric indexing, are no longer read or cited as
  evidence (#1834 thread r4187840952).
- Packet CHEST-APPEARANCE-ADMIT-1 (impl worker), after #1830 and #1805 merge, with one writer on
  `content/world/pins/`.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

task_id: OTV2-20260924-source-family-classification
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
branch: agent/full-content-g3-source-family-classification-20260924
base_sha: ea8a563026e10b5464ece55c7ee0c790e716a786
pr: 828
head_sha: 20e8e5d99b7b42b6fd008e67029623ea7a4a8e3a
final_head_sha: 20e8e5d99b7b42b6fd008e67029623ea7a4a8e3a
merge_commit_sha: ac3d69f1c20e09ce006e3efb0777d4e4f32275d1
merge_group_run_id: 35987455034
merge_group_game_gate: SUCCESS
g3_substantive_completion: false
ownership_released: true
owned_paths:
  - tools/content-census/source_family_classification.py
  - tools/content-census/source_family_classification_self_test.py
  - .github/workflows/source-family-classification.yml
  - docs/agents/evidence/OTV2-20260924-source-family-classification.json
  - docs/agents/tasks/active/OTV2-20260924-source-family-classification.md

# OTV2-20260924-source-family-classification

> Lifecycle: TERMINAL / COMPLETED TASK, PARTIAL G3 COHORT ONLY. PR #828 merged at `ac3d69f1c20e09ce006e3efb0777d4e4f32275d1` after qualification of exact PR head `20e8e5d99b7b42b6fd008e67029623ea7a4a8e3a`. Merge-group run [35987455034](https://github.com/Oteryn/Oteryn-Game/actions/runs/35987455034) completed with aggregate `game-gate` SUCCESS. This task's ownership is released. The delivered cohort does not substantively complete G3 family classification.

## Objective

Build the first deterministic G3 source-family cohort over every exact page ID in the pinned G2 global source universe. A qualifying direct signature identifies only a primary source-definition family. Pages outside the signatures remain explicitly `UNKNOWN`; preserve the source shape and G1 candidate-family observations on every page.

This is partial G3 progress, not family-classification closure. G1 candidate-family relations remain evidence-only and unresolved. No Item target dispositions, canonical target identities, gameplay semantics, runtime changes, or new family definitions are admitted.

## Pinned input and lineage

- Protected main admission: `ea8a563026e10b5464ece55c7ee0c790e716a786`; terminal G2 archive #826 has successful merge-group game-gate run `35984587432`.
- G2 output artifact `10800249169`, run `35982151518`, head `9c333fbba17f60cbf8eb107ecb623c5cda679d6c`.
- G2 ZIP SHA-256: `8b5a78e6daa72b5120a2218d395e498c8c2aa236bf7152f730f67ea99693af50`.
- Embedded universe schema `OTERYN_GLOBAL_SOURCE_ID_UNIVERSE/v1`, SHA-256 `59b6c85e18a6e55b65491f53fa701b9596009228e6e06ea7a3b0b6d493d47d6c`, 15,787 exact page IDs and 504 cross-lane overlaps.
- Reuse the protected G2 downloader's verified artifact/run/ZIP/exact-member functions; do not edit that downloader. Keep the full classified corpus in a GitHub Actions artifact with 14-day retention. Commit only compact manifest and task/evidence records.

## Direct signatures

Protected Item qualifies only when the protected provenance reports `INFOBOX_ITEM`.

G1 non-Item signatures require `STRUCTURED_PRIMARY`, `redirect == false`, the exact root, source surface, and base infobox template:

| Family | Root | Surface | Base template | Additional exact category |
|---|---|---|---|---|
| Creature | `creatures` | `Stworzenia` | `Predefinição:Infobox Criatura` | `Categoria:Criaturas` |
| NPC | `npcs` | `NPC-e` | `Predefinição:Infobox NPC` | — |
| Achievement | `achievements` | `Osiągnięcia` | `Predefinição:Infobox Achievement` | — |
| Mount | `mounts` | `Mocowania` | `Predefinição:Infobox Mount` | `Categoria:Montarias` |
| Outfit | `outfits` | `Stroje` | `Predefinição:Infobox Outfit` | — |
| Quest | `quests` | `Zadania` | `Predefinição:Infobox Quest` | — |
| Ability | `magical-archive` | `Magiczne Archiwum` | `Predefinição:Infobox Spell` | — |

Winterlight Solstice page ID `45642` remains `UNKNOWN`: it is a World Quest page with a Mount list template and no `Categoria:Montarias`. Do not mint Rune or Bestiary family definitions. Preserve the G0 WorldProject/v2 vocabulary and definitions/relations/placements/runtime split. Hard exclusions are `Kalkulatory`, `Narzędzie do nasycania`, and `Dostawca`.

## Acceptance and validation

- Observed direct counts must be derived from the pinned artifact and equal Item 5,475; Creature 2,149; NPC 1,253; Achievement 569; Mount 252; Outfit 134; Quest 272; Ability 171. These are 10,275 disjoint direct assignments. The other 5,512 pages remain `UNKNOWN` with source shape preserved.
- Focused counterexamples cover Mount-list World Quest, infobox without root/surface, root without base infobox, same-ID dual provenance, same-title distinct IDs, parse-error/redirect/no-infobox unknown, and corrupt input digest.
- Run local focused tests and exact-input classification reproduction. The hosted exact-head workflow verifies the G2 artifact metadata, run/head, ZIP bytes, exact archive members, universe digest, schemas and counts, then uploads the full classified output and compact manifest as a 14-day artifact.
- Preserve input artifact and deterministic classified-universe digests in the evidence record. Exact-head Agent Governance, Architecture Semantic Audit, Merge Gate/game-gate, and the G3 workflow must pass before returning the draft PR as `READY_FOR_INTEGRATION`.

First hosted classification run `35986595291` passed both jobs at candidate `67aaa41f505c2da588315cafc833f2914ba16676`. It uploaded artifact `10801758693` (`source-family-classification-67aaa41f505c2da588315cafc833f2914ba16676`), ZIP SHA-256 `cb39d2404317328b7efee71e489ea93413d9ac3eefb0c92f6ce4b2c143d082d7`, 14-day retention. The task packet/evidence metadata is being corrected; final exact-head qualification must run after that write.

Coordinator owns independent review, admission through governed Merge Queue, and terminal merge-group/protected-main readback.


## Final hosted classification and terminal closeout

- PROVEN: final exact-head G3 workflow run [35986883931](https://github.com/Oteryn/Oteryn-Game/actions/runs/35986883931) completed successfully at PR head `20e8e5d99b7b42b6fd008e67029623ea7a4a8e3a`.
- PROVEN: final workflow artifact `10801778929`, `source-family-classification-20e8e5d99b7b42b6fd008e67029623ea7a4a8e3a`, ZIP SHA-256 `a63635e4cdfcd237cf69b5e2fe0471c30723e45fe3ab2ccc9095730a9337d2e7`. The pinned G2 input manifest SHA-256 is `e3c2de4d1b5a5db80d59ee36465a0ea76a8f9d300bf015d1646d8321ea71a4d1`.
- PROVEN: the final manifest covers **15,787** exact page IDs: **10,275** direct assignments (Item 5,475; Creature 2,149; NPC 1,253; Achievement 569; Mount 252; Outfit 134; Quest 272; Ability 171) and **5,512 UNKNOWN**.
- PROVEN: final manifest status is `PARTIAL_SOURCE_DEFINITION_CLASSIFICATION_NO_IDENTITY_OR_SEMANTIC_PROMOTION`; G3 is **not substantively complete**. The direct-signature cohort does not resolve remaining candidate relations or promote identities or semantics.
- Historical first hosted run 35986595291 / artifact 10801758693 and its recorded ZIP digest above remain preserved as a separate initial run.
- PROVEN: PR #828 merged at protected `main` merge commit `ac3d69f1c20e09ce006e3efb0777d4e4f32275d1`; merge-group run 35987455034 aggregate `game-gate` succeeded.
- TERMINAL: this bounded task is completed and its authoring ownership is released. Further G3 work requires a new live allocation.

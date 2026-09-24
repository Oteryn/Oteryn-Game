task_id: OTV2-20260924-source-family-classification
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
branch: agent/full-content-g3-source-family-classification-20260924
base_sha: ea8a563026e10b5464ece55c7ee0c790e716a786
pr: pending
head_sha: pending
final_head_sha: pending
owned_paths:
  - tools/content-census/source_family_classification.py
  - tools/content-census/source_family_classification_self_test.py
  - .github/workflows/source-family-classification.yml
  - docs/agents/evidence/OTV2-20260924-source-family-classification.json
  - docs/agents/tasks/active/OTV2-20260924-source-family-classification.md

# OTV2-20260924-source-family-classification

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

Coordinator owns independent review, admission through governed Merge Queue, and terminal merge-group/protected-main readback.

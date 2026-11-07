# Full content census tooling

This directory is the canonical destination for **new** tooling created by the
full Tibia content census/crosswalk programme.

It does not replace or relocate the protected predecessor tooling under
`tools/reference-world-corridor-census/`. Existing scripts remain at their
published paths unless a later measured maintenance problem justifies a bounded
migration.

## Organization

Create family directories only when the first real tool for that family is
implemented. Do not commit empty directory trees.

Expected family-oriented lanes include, as needed:

- `items/`
- `terrain/`
- `world-objects/`
- `transitions/`
- `areas/`
- `houses/`
- `creatures/`
- `npcs/`
- `abilities/`
- `effects/`
- `quests/`
- `encounters/`
- `documents/`
- `achievements/`
- `outfits/`
- `mounts/`
- `charms/`
- `presentations/`
- `assets/`
- `placements/`
- `relationships/`

A `common/` package is created only after at least two concrete consumers prove
the same bounded collection/parsing/provenance logic should be shared.

## Stage convention

A substantial family lane should make its stage explicit:

`discovery -> classification -> crosswalk -> verification -> evidence/tests`.

The exact file names and module boundaries are implementation choices. The
important invariant is that source discovery, identity matching and semantic
promotion do not collapse into one opaque script.

## Programme invariants

- Source-universe-first discovery.
- Deterministic ordering and stable digests.
- Bounded requests, continuation and response bytes.
- Exact source revisions and provenance.
- Explicit source-shape/family/crosswalk dispositions.
- No name-only `EXACT_MATCH`.
- No silent identity minting or semantic promotion.
- Definitions, relationships and placements remain distinct.
- Large reproducible raw outputs stay in bounded scratch/CI artifacts unless a
  demonstrated repository need requires retention; Git retains compact
  manifests, counts, digests and provenance.
- Do not bulk-copy long-form TibiaWiki prose or unlicensed assets.

The three intentionally excluded source areas are not collected or represented:
`Kalkulatory`, `Narzędzie do nasycania`, and `Dostawca`.

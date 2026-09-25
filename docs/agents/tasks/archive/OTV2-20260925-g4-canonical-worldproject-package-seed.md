---
task_id: OTV2-20260925-g4-canonical-worldproject-package-seed
mode: IMPLEMENT
status: implementing
issue: 162
issue_comment: 5826953735
pr: 874
repository: Oteryn/Oteryn-Game
base_commit: 1680eb5dc6145aa3e271ac8665f33a50ed837b76
branch: agent/otv2-g4-canonical-worldproject-package-seed
---

# G4 canonical WorldProject/v2 package seed

Materialize the protected CW2-B1 38,157-Item family as the first tracked repository
`WorldProject/v2` package at `content/world/`. The package is written only by the
canonical v2 writer and is a durable authoring/provenance input, not a server runtime
activation path.

## Boundaries

- Canonical Item identity remains `(Item, ProductionKey, definition revision)`.
  Crystal/OTS numeric values stay inside the protected import evidence and never
  become canonical, client, runtime, or wire identities.
- The 23 Items and 69 known semantic atoms are the already-protected CW2-B1 semantic
  promotion. This task does not infer or promote any Wiki semantics.
- `provenance/sources.json` contains zero G4 source records and zero
  `ProjectV2SourceIdentityBinding` rows. Mass G4 population remains gated on separately
  admitted exact source-identity evidence.
- Definition, Presentation, Asset, and runtime/wire identity layers remain separate.
  The seed adds no Presentation/Asset binding and no runtime activation consumer.
- `licensing_metadata=PENDING`, protected `OTS_HYPOTHESIS_ONLY` donor evidence, and the
  B1 reimport state are preserved exactly; none is upgraded to production authority.

## Allocated paths

The allocation owns exactly the 16 paths recorded in the evidence file: eleven
`content/world` documents, one canonical materializer example, one repository contract
test, one dedicated workflow, this task record, and its evidence record.

## Acceptance

- Run the Rust 1.94 canonical writer twice into fresh roots and require byte-for-byte
  equality, exactly eleven regular files, and no symlinks.
- Capture and parse the tracked package with the production filesystem adapter, rewrite
  it canonically, and require exact bytes, sizes, SHA-256 digests, and tree digest.
- Require exactly 38,157 unique Item definitions, 38,157 protected import candidates,
  38,157 reimport states, and the pre-existing 23-Item/69-field protected semantics.
- Require empty declarations, worlds, placements, Presentation bindings, Assets,
  sources, source-identity bindings, and editor records; reject runtime/wire/client-ID
  fields in the tracked JSON.
- Run the focused WorldProject suites, formatting, clippy, governance, repository
  policy, architecture audit, and the repository's full hosted merge gate at one frozen
  exact PR head.

## Non-claims

- No Wiki binding or Wiki-derived semantic promotion.
- No new canonical family outside Item.
- No runtime loader, server activation, Presentation/Asset identity, or client/wire ID.
- No readiness, queue, merge, or production authority.

---
task_id: OTV2-20260925-g4-mount-252-population
mode: IMPLEMENT
status: implementing
issue: 162
issue_comment: 5831631091
pr: null
repository: Oteryn/Oteryn-Game
base_commit: 5d2db4ecf845b0a5332b96a75f0b76aae553fb09
branch: agent/full-content-mount-g4-252-population-20260925
---

# G4 Mount canonical population

Create 252 non-executable Mount declarations, exact source-identity bindings,
and editor catalogue entries in the existing WorldProject/v2 package. The
selected input is the 252-row Mount partition of G4 crosswalk artifact
`10848111721`, matched by page ID, page revision, timestamp, and raw page
digest to the original G4 source capture artifact `10831362943`.

The source key `oteryn:source.tibiawiki` is reused at a distinct source
revision. `tibiawiki-nonitem-g1-snapshot:0b7caf98940305a91c5384dfb828c0afcf016f087f71572ec71d7c936c6387df`
is an **Oteryn snapshot revision token derived from the recorded G1 snapshot
digest**, not a MediaWiki field. The source artifact SHA-256 is the original
`non-item-source-capture.json` digest
`f47dbe5832e7b1accd652852303d638a4f19367bab3951f260cc39a0d93b7713`.
The derived crosswalk SHA-256 `38d827ba66bb7a04a3f5a94bbb873ca4485d4b7c873de957812a9c1be20a67c5`
is selection evidence only.

Each Mount has a unique namespaced canonical key under
`oteryn:content.mount.*`, `definition-r1`, an exact Wiki page-ID binding,
and one editor display name with the namespaced `oteryn:editor.mount` tag.
Typed `speed_bonus`, `premium`, Presentation/Asset, taming item, acquisition
relationships, world placement, and runtime activation are left absent.
The 38,157 existing Item reference definitions and four unaffected package
documents remain byte-identical.

The exact eleven owned paths are the generator and repository package test;
this task packet and its selected evidence file; and seven changed canonical
package documents: `definitions/declarations.json`, `editor/author.json`,
`provenance/imports.json`, `provenance/sources.json`, `manifest.json`,
`content.lock.json`, and `project.json`. The canonical v2 writer alone
materializes the package. Qualify the frozen remote head through the existing
G4 workflow's two-pass materialization, package equality, focused tests,
Clippy and format checks, the full game-gate and independent review.

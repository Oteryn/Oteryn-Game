> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #685 is merged on protected main as `0ac1093817064eee4e50a54be3bf0ddfc90c7caf`. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260919-content-world-cw3-project-snapshot-504

```yaml
task_id: OTV2-20260919-content-world-cw3-project-snapshot-504
title: CW3 canonical project snapshot foundation
mode: BUILD
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw3-project-snapshot-504
issue: 162
pr: null
base_sha: f059194f6372736d254ae3050cc91b8e8846b480
head_sha: null
owner: Oteryn: content world build
created_at: 2026-09-19T22:03:04Z
updated_at: 2026-09-19T22:03:04Z
owned_paths:
  - apps/game-server/src/content/project.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/tests/content_world_project.rs
  - docs/agents/tasks/active/OTV2-20260919-content-world-cw3-project-snapshot-504.md
public_contracts:
  - OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v1 snapshot boundary
production_authority: NONE
```

## Outcome

Implement the smallest canonical editable-project snapshot admitted by the
protected source-profile decision. The reader accepts immutable caller-owned
logical-locator/owned-byte pairs, verifies a coherent root, manifest and exact
Content Lock, strictly decodes typed JSON, and lowers the selected project-owned
records into the existing `ReferencePlayableContentSource` and
`link_reference_playable` boundary.

The canonical writer emits a complete caller-owned document set. It does not
open, traverse or publish filesystem paths. Filesystem capture, no-follow and
alias admission, staging, journalling, crash recovery and atomic publication
remain a required successor child.

## Supported subset

The executable selected subset consists only of project-owned typed Reference
Item, Presentation, Behavior and Creature records already supported by the
existing linker. Unsupported record families and shapes fail closed. Creature
references remain exact typed key/revision references and are resolved by the
existing linker; this project layer supplies no second gameplay graph or
validation authority.

The protected CW2-B4 Ability/Effect/Formula batch is retained as typed import
candidate data with exact source artifact and mapper provenance, normalized
baseline fields, explicit candidate/blocked dispositions and deterministic
three-way reimport states. It has zero native bindings and cannot enter the
selected Reference closure through this schema. This change therefore does not
claim an imported family is playable.

Author metadata has a separate schema and inventory role. Lowering never reads
display names, descriptions, categories or notes.

## Coherence and determinism

- Duplicate JSON members, floating-point values, unknown fields, unknown
  required/optional features, unsupported schemas and trailing values fail.
- Canonical logical locators are validated before document decode. Control
  documents cannot appear in the managed inventory and every admitted document
  must appear exactly once.
- The manifest binds every managed document's role, schema, length and SHA-256.
  Its exact bytes construct the existing `PackageManifestBinding`; the lock
  binds that package provenance; the root binds both exact control documents
  and one project revision.
- Canonical output sorts set-like records by stable typed identity and preserves
  only semantic sequences. Input enumeration and record permutation do not
  affect bytes. Multiple documents of one role are combined by stable identity,
  so regrouping does not mint or delete semantic records.
- Reimport stores baseline, upstream and local typed candidate values. Unchanged,
  upstream-only, local-only, converged, divergent conflict and deletion cases
  have explicit deterministic outcomes; a stored inconsistent decision fails.

## Evidence limits

The implementation takes a non-zero finite `ProjectEvidenceLimits` value from
the caller. It does not register product/full-world maxima. The exact canonical
focused corpus contains 6 documents, 4 selected Reference records, 2 protected
B4 candidates and 1 reimport state. Its measured dimensions are:

| Dimension | Exact corpus maximum |
| --- | ---: |
| Largest document | 2,260 bytes |
| Aggregate document bytes | 5,314 bytes |
| Longest locator | 23 bytes |
| Most locator segments | 2 |
| Deepest JSON document | 9 levels |
| Most decoded JSON values in one document | 73 |
| Most decoded JSON key/value string bytes in one document | 1,866 bytes |

The broader values in the test helper are mutation-test headroom, not accepted
corpus measurements or product limits. Boundary tests derive and assert the
exact emitted byte dimensions and find the minimum passing JSON budgets, prove
equality is accepted, then prove max+1 input fails for every used limit
dimension. Document count is charged before map insertion; JSON depth/value and
string budgets are charged by Serde seeds and visitors before collection growth
or recursive admission; canonical writes use a bounded `Write` adapter. All
aggregate arithmetic is checked and overflow fails closed. These measurements
remain evidence-only and cannot be promoted to production admission or D3 World
Bundle limits.

## Acceptance criteria

- [x] Only the four allocated paths are changed.
- [x] Snapshot input and canonical document-set output perform no filesystem I/O.
- [x] Strict JSON duplicate/unknown/float/trailing-data negatives fail.
- [x] Locator traversal, absolute/drive/UNC, separator, case, device and alias
  spellings fail; duplicate locator bytes fail.
- [x] Inventory, byte length, digest, root, manifest, lock and project revision
  coherence fail closed.
- [x] The v1 Content Lock contains exactly one non-floating, non-dependency root
  entry; extra or ignored entries fail.
- [x] Canonical rewrite and input permutation are byte-identical; regrouped
  records lower to the same selected definitions.
- [x] Real protected B4 source/mapper digests and identities survive while native
  promotion remains impossible.
- [x] Three-way unchanged/upstream/local/converged/conflict/delete outcomes and
  inconsistent stored-state rejection are covered.
- [x] Metadata changes cannot select or redirect gameplay.
- [x] Existing typed-reference/linker failures remain authoritative and a minimal
  project-owned closure links through `link_reference_playable`.
- [ ] Published immutable head passes hosted exact-head Rust and repository gates.

## Validation

Passed locally on the allocated checkout:

- `python -B tools/agents/validate_governance.py`;
- `python -B tools/repository/validate_repository_policy.py`;
- `git diff --check`;
- exact four-path custody inspection.

Local Rust is absent. The API-published immutable head must pass the repository
merge gate's Rust 1.94 route: formatter check, locked all-target workspace build,
strict workspace Clippy, workspace tests and aggregate game-gate. The focused
`content_world_project` integration target must pass within that route.

## Deferred claims

This snapshot is not secure filesystem ingestion or crash-safe publication. It
is not a Reference compiler or artifact carrier. It does not complete the first
real imported-family project-to-compiled-artifact journey, establish native B4
identities, promote parity, or authorize B5/B6. World Bundle format and
production resource maxima remain D3-owned.

## Handoff

The parent control plane owns independent review, integration and the successor
filesystem child. This worker does not merge or trigger external review.

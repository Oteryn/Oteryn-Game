---
task_id: OTV2-20260924-g4-direct-nonitem-family-crosswalk
mode: IMPLEMENT
status: implementing
issue: 162
issue_comment: 5821824145
pr: null
repository: Oteryn/Oteryn-Game
base_commit: 6185045c20631b8614b1d4ad1dacc5d918f73b0a
branch: agent/otv2-g4-direct-nonitem-family-crosswalk
---

# G4 direct non-Item family crosswalk inventory

Join the protected G3 direct-family classifications to the merged #857 source-provenance capture using only exact MediaWiki page IDs. Preserve exact current page namespace, decimal external ID, namespaced page key, revision ID and timestamp, and the digest of the exact revision. Do not use page titles as join keys or canonical identity evidence.

## Scope

- Creature: 2,149; NPC: 1,253; Achievement: 569; Quest: 272; Ability: 171; Outfit: 134; Mount: 252.
- G3 input: artifact 10801778929, run 35986883931, exact archive digest recorded in the evidence manifest.
- G4 input: merged #857 artifact 10831362943, run 36053194532, exact archive digest recorded in the evidence manifest.
- The G3 signature establishes source-family classification only; its candidate relationships and target identity remain unresolved.
- Record `MISSING_CANONICAL_IDENTITY` only when a deterministic protected-base inventory shows no production canonical target corpus. Otherwise keep the row as an evidence-only target-inventory blocker.
- Any G3/G4 revision or timestamp mismatch remains `SOURCE_REVISION_REVALIDATION_REQUIRED`; specifically, Creature page 63947 must not be promoted across its revision drift. Apply the same fail-closed rule to any other drift discovered.
- Crosswalk output and manifests are workflow-artifact-only. Do not emit canonical bindings, definitions, gameplay semantics, aliases, Presentation/Asset IDs, runtime IDs or population.

## Validation

- Verify both exact artifact runs, IDs, names, sizes, archive SHA-256 digests, member lists, canonical payload digests and expected G3 family counts.
- Require one exact source page-ID row in G4 for each G3 direct-definition row; fail closed on missing/duplicate IDs, malformed source identity or stale revision/timestamp.
- Run focused synthetic tests for exact-ID-only join behavior, shared titles, revision drift, malformed IDs, source-family signature mismatch, missing G4 IDs, and target-inventory blockers.
- Produce one bounded source-only workflow artifact; commit only the five allocation paths in one Git Data commit. No PR or integration action is authorized by this task.

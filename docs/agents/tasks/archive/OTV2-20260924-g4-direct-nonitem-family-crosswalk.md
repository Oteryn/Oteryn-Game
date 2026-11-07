---
task_id: OTV2-20260924-g4-direct-nonitem-family-crosswalk
mode: IMPLEMENT
status: archived
issue: 162
issue_comment: 5821824145
closeout_allocation_comment: 5822708237
closeout_r2_allocation_comment: 5822789966
closeout_r3_allocation_comment: 5822884089
pr: 861
repository: Oteryn/Oteryn-Game
base_commit: 6185045c20631b8614b1d4ad1dacc5d918f73b0a
branch: agent/otv2-g4-direct-nonitem-family-crosswalk
archive_branch: agent/otv2-g4-direct-nonitem-family-crosswalk-archive-r3
superseded_archive_branch: agent/otv2-g4-direct-nonitem-family-crosswalk-archive-r2
superseded_archive_head: 5259d782ac278df70140a21baaaf0dd72acf8d32
archive_r2_base_commit: ead12491b10bec15cc2d9f53f07dbd320edd9d57
archive_r3_base_commit: c516182255d3ea1724e91671c8d2187eb622e3df
terminal_main_commit: 339b6ba6b6081868d5361040b5ea40c7c118ef2f
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


## Closeout

- Status: merged and archived. PR #861 integrated exact head `62a234d5785d93f62c5dd84c66ff1493f1cf0796` to protected `main@339b6ba6b6081868d5361040b5ea40c7c118ef2f`.
- Exact-head hosted validation: run 36058305835 succeeded; artifact 10832769419; archive SHA-256 `c4a290d19b0a2db2842003d532f79894e7fa05137efbcf7362be661d01665aa1`; output SHA-256 `1d3c8944bf68c63814942578ac07fe232eff900d6d261476d46bb742e64c5396`.
- Independent review passed (comment 5822590223); governed executor run 36062132750 and real `merge_group` run 36062184948 succeeded. Terminal integration record: Issue #162 comment 5822707953.
- Result: 4,800 source-only rows, comprising 4,798 `MISSING_CANONICAL_IDENTITY` and 2 `SOURCE_REVISION_REVALIDATION_REQUIRED` (Creature 63947; Quest 46925).
- No source binding, definition, population, gameplay semantic or promotion was emitted. Implementation allocation paths are released.
- This archival record does not modify implementation, workflow, source, binding, definition or semantic content.


## Archive R2 lifecycle qualification

- R2 allocation: Issue #162 comment 5822789966; branch `agent/otv2-g4-direct-nonitem-family-crosswalk-archive-r2`, based on protected `main@ead12491b10bec15cc2d9f53f07dbd320edd9d57`.
- This closeout supersedes unqualified archive head `6738c4d09db9c34654eb7064aefb7d34f7e8efaa`; no qualification or validation is reused from that head.
- The workflow selects archive mode from the archived task path in the PR diff. Archive mode requires the active path absent, archived task status and `MERGED_ARCHIVED` evidence, exact prior integration facts and released implementation paths, and an exact archive-only changed-path set. It does not run the implementation-parent or artifact-generation gates.
- Implementation mode retains the original exact candidate/base checks, Python compile, synthetic tests, pinned artifact validation, bounded-output assertions, and artifact upload.
- This R2 allocation changes only the workflow lifecycle routing and archive records. No collector, self-test, source, binding, definition, population or promotion implementation changed.


## Archive R3 parent-fenced validation

- R3 allocation: Issue #162 comment 5822884089; branch `agent/otv2-g4-direct-nonitem-family-crosswalk-archive-r3`, based on protected `main@c516182255d3ea1724e91671c8d2187eb622e3df`.
- This closeout supersedes unqualified R2 head `5259d782ac278df70140a21baaaf0dd72acf8d32`; no qualification or validation is reused from R2.
- Archive lifecycle mode derives the exact candidate parent from `head^1`, requires a sole parent equal to the R3 evidence/allocation base, and validates the changed paths over `parent..head`. It does not depend on the pull-request event's moving base SHA.
- R2 lifecycle checks and implementation-mode gates remain in place; archive mode still validates the archived task, terminal evidence, released implementation paths, and absence of implementation tool changes without re-running artifact generation.
- No collector, self-test, source, binding, definition, population or promotion implementation changed.

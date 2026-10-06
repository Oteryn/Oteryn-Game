# ITEM-SOURCE-RECOVERY-20261005

Owner: Root, single writer. Latest user instruction: work solo, check existing repository data, finish the work and create a PR. No subworkers were used for this publication.

Owned paths: `imports/ots-native-admission/solo-source-item-recovery-20261005/`, the two `tools/content-migration/*solo_item_source_recovery.py` files and this task record. No active server or content-authoring path is changed.

The latest main read was `f8c5e621c15cb3909ed6158c02feebf0b9689644`. Relevant Item model/data paths were unchanged from the audited `98ca4aeaf7d579af64d43269e9b3b9bd65d49915`. Main's 127 Known charge entries and 118 spell item profiles were reused as existing data.

Results: 317 losslessly archived local files, including all 149 uncommitted files; 33,975 targets and 45,721 own rows in each new charge/leveldoor and ability-declaration batch; 55,012 observations in the definition successor with exact inverse preservation of the prior 9,291 rows. The full package verifier passed: 20,735 declared ability events across 13,724 own rows, zero Native promotions. This is source coverage, not completed classification/runtime coverage.

Validation: complete package digest/recovery/cohort/schema verification passed; charge producer 10 tests, ability producer 3 tests and recovery boundary 2 tests passed; changed runnable Python files passed Ruff. Full local server check did not pass: storage exhaustion followed by cgroup memory SIGKILL. Ad hoc Rust test builds also failed dependency resolution. Those failures are recorded honestly; no current Native qualification is claimed.

Disposition: publish a draft recovery/source-data PR. Active Native import is blocked. The local reconciled candidate is kept outside the PR; its data reconciliation has zero held conflicts, but its code/whole-parent gates/metadata/readback are not qualified. Preserve accepted V6 consumption group 18 when continuing; archived experimental media group 18 is not safe to apply as-is.

Remaining: qualify model/schema/registered consumer successors, both Native views and exact inverses; update owning metadata; pass affected server/content gates and required independent review; complete Canary/Crystal source parity; then external wiki enrichment. Classification and runtime import remain unfinished. The PR is not merged by this task.

## PR #1826 drive (PR1826-DRIVE-1)

origin/main (88c5464c) merged into the branch with a clean merge commit; no conflicts and no content change. Validation after the merge: `validate_governance.py` passed (22 policy documents, 9 lanes); `tools/agents/tests` 54 tests OK; `verify_solo_item_source_recovery.py` passed (45,721 charge rows, 0 Native promotions); recovery boundary tests 2 OK; charge producer 10 tests OK; Ruff on both recovery scripts clean. The ability producer test needs its capture-time filesystem paths and is not portable (see package README); it is not a repository CI check. The path-filtered `content-tree-migration` and `item-authoring-schema` workflows do not select this PR's paths.

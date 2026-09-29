# Archived tasks

Terminal task records are moved here only after required merge/closeout and ownership release. Preserve exact PR, head, validation, audit and E2E evidence.

A task that ends with one PR moves its record here in that PR's final authoring commit, before FREEZE_SHA, with the closeout filled as far as it is known before freeze: `status: completed`, the PR number, validation results and review state. A commit cannot contain its own SHA: the exact frozen head is the one in the FREEZE_SHA entry, and `merge commit/result` is recorded as `squash merge of #N` (resolve it with `git log --grep "(#N)"`). The move is part of the task's owned paths. The record reaches `main` only if the PR merges; if the PR is closed unmerged, nothing is archived. A task that spans several PRs stays in `tasks/active/` until its last PR, which archives it the same way. Separate archive-only PRs are for records that could not close in their own PR.

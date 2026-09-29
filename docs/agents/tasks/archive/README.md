# Archived tasks

Terminal task records are moved here only after required merge/closeout and ownership release. Preserve exact PR, head, validation, audit and E2E evidence.

A task that ends with one PR moves its record here in that PR's final authoring commit, before FREEZE_SHA, with the closeout filled: `status: completed`, the PR number and the validated head. Record `merge commit/result` as `squash merge of #N` (resolve it with `git log --grep "(#N)"`), and ownership is released on merge. The record reaches `main` only if the PR merges; if the PR is closed unmerged, nothing is archived. A task that spans several PRs stays in `tasks/active/` until its last PR, which archives it the same way. Separate archive-only PRs are for records that could not close in their own PR.

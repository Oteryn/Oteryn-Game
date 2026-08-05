# Task closeout, audit and E2E

Before a task becomes `completed`:

## Diff and ownership

- inspect every changed file and full diff;
- confirm paths/contracts match declared ownership;
- remove accidental/generated/temporary content;
- reconcile overlapping or superseded PRs.

## Acceptance map

Map each acceptance criterion to exact evidence. Mark missing evidence explicitly; do not substitute adjacent checks.

## Independent audit

Audit exact final head for architecture, security, concurrency, failure paths, omissions, unsupported claims, temporary validation residue and cross-repository drift. Record zero open material findings for pass.

## E2E

Run the named real scenario where applicable. For governance/docs-only tasks use `NOT_APPLICABLE` with reason and run governance/link validation instead.

## Exact-head gate

Verify required checks on final unchanged SHA. Re-check PR head, reviews, mergeability and ownership immediately before merge.

## Lifecycle closeout

After merge record merge commit/resulting state, archive task, release ownership and update programme/Issue. Do not leave duplicate active task records.

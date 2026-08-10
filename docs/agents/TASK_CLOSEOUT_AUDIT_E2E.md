# Task closeout, audit and E2E

Before a task becomes `completed`:

## Diff and ownership

- inspect every changed file and full diff;
- confirm paths/contracts match declared ownership;
- remove accidental/generated/temporary content;
- reconcile overlapping or superseded PRs.

## Acceptance map

Map each acceptance criterion to exact evidence. Mark missing evidence explicitly; do not substitute adjacent checks.

## Mandatory self-review

Every task requires a full-diff self-review on the exact final head. Challenge architecture, security, concurrency, failure paths, omissions, unsupported claims, temporary validation residue, stale evidence and cross-repository drift. Record zero open material findings for readiness.

The implementing agent may perform this review, but must not call it independent when it implemented or materially authored the same change.

## Risk-based independent review

Independent review/audit is additionally required when:

- an explicit owner instruction, accepted contract or trusted-base task policy requires it;
- the change affects authentication/authorization, admission/session/lease/reconnect/fencing, protocol/wire/transport trust, authoritative persistence/migrations, item/currency/economy conservation, secrets/updater/artifact trust, production/protected environments, multichannel/world-shared authority/failover, or comparable high-risk behavior;
- governance reduces a safety gate, expands authority or weakens required evidence;
- material uncertainty, complexity or blast radius makes common-mode self-review risk unacceptable.

Low-risk documentation/navigation, typo and stale-task bookkeeping changes do not require an independent reviewer solely because they are PRs when no rule above applies.

An independent reviewer may be a qualified human, a separate non-authoring agent/session, Codex or a dedicated audit workflow. **Codex is optional and may be used only when independent review is actually required and Codex is the necessary or appropriate available independent mechanism for that requirement.** Do not invoke it merely for extra assurance on a task that does not require independent review.

If independent review is required but unavailable, record the blocker. Never relabel self-review as independent.

## E2E

Run the named real scenario where applicable. For governance/docs-only tasks use `NOT_APPLICABLE` with reason and run governance/link validation instead.

## Exact-head gate

Verify required checks on final unchanged SHA. Re-check PR head, required review state, mergeability and ownership immediately before merge. A material repair invalidates prior exact-head review/audit evidence and requires the applicable review/validation again.

## Lifecycle closeout

After merge record merge commit/resulting state, archive task, release ownership and update programme/Issue. Do not leave duplicate active task records.

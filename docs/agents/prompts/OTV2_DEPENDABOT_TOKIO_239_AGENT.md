# Oteryn-v2 Dependabot Tokio PR #239 Repair Agent

Alias: `OTV2-TOKIO-239`

## 1. Role and mode

```text
ROLE: DEPENDENCY REPAIR / VALIDATION AGENT
MODE: REPAIR
TARGET_PR: #239
MERGE_AUTHORITY: NONE_UNLESS_OWNER_EXPLICITLY_AUTHORIZES_PR_239
```

Repair and validate the existing Tokio dependency-update PR #239. Do not create a replacement dependency PR merely to regenerate CI.

## 2. Authority

Writable repository: `blakinio/Oteryn-v2` only, limited to the existing PR #239 branch/metadata and any exact task bookkeeping required by current governance.

External repositories are read-only.

Do not change runtime behavior beyond the dependency update itself, broaden features, refactor unrelated code, change governance, touch production, invoke Codex/OpenAI/paid review, or merge without exact authority.

## 3. Mandatory startup

Read root `AGENTS.md`, `AGENTS.override.md`, `docs/agents/AGENTS.md`, `BUILD_TEST_MATRIX.md`, `ANTI_STALL_AND_EXECUTION_BUDGET.md`, `DELIVERY_COMPLETENESS_AND_CLOSEOUT.md` and applicable repository policy.

Then inspect live:

- exact `main` SHA;
- PR #239 exact head/base/mergeability;
- full PR diff and changed filenames;
- current Tokio pin on live main;
- review submissions/threads;
- exact-head workflow/check state and failure annotations;
- other open dependency PRs for overlap;
- current branch drift using compare API.

Known historical baseline to verify: PR #239 proposed Tokio `1.51.4 -> 1.53.1`, was based on an older main, had large drift, and its governance/aggregate gate failed while Rust build/test sub-gates had passed on the old head. Historical checks are not final evidence after any head move.

## 4. Outcome

If the dependency update is still relevant and not superseded:

1. preserve the narrow two-file dependency intent unless live resolution requires a justified lockfile change;
2. update/rebase the existing Dependabot PR onto current `main` using a safe supported mechanism;
3. repair PR metadata to satisfy current repository requirements while preserving useful Dependabot release-note provenance;
4. freeze the new exact head only after content and metadata are coherent;
5. perform full-diff self-review;
6. obtain fresh exact-head required CI;
7. leave PR #239 merge-ready if all gates pass, but do not merge unless exact owner authority for #239 exists.

If the update is already present, duplicated, incompatible, or materially superseded, classify with exact evidence and stop; do not close unless the current owner instruction explicitly grants close authority.

## 5. Safety and dependency review

Verify at minimum:

- exact version and lockfile resolution;
- no unexpected feature activation;
- no dependency-source change outside crates.io lock resolution;
- current Rust/MSRV compatibility;
- advisories/license/ban/source gate;
- Linux workspace build, strict Clippy, tests and synthetic harness when required;
- Windows production-client build/Clippy/smoke/harness when required;
- any behavior-sensitive Tokio changes relevant to enabled features (`io-util`, `net`, `rt-multi-thread`, `sync`, `time`).

Do not infer safety solely from upstream release notes or a Dependabot compatibility badge.

## 6. PR metadata repair

Current governance requires a compliant title and PR body sections including `## Summary`, `## Scope` and `## Validation`.

Repair metadata without deleting useful original dependency provenance. Metadata repair does not justify a content-only/no-op commit.

## 7. Validation ladder

On the final unchanged head:

- inspect changed paths/full diff;
- run/observe `Merge gate / validate` and all applicable Rust/supply-chain sub-gates;
- perform deliberate full-diff self-review;
- verify zero requested changes and unresolved threads;
- compare final head against live `main` and require no unhandled strict-update drift;
- record exact run IDs/results.

A pass from the superseded pre-rebase head proves nothing about the final head.

Independent review is not automatically required for a narrow routine dependency bump unless the final diff/risk policy triggers it. Never invoke owner-funded AI without exact permission.

## 8. Anti-stall

Follow repository budgets. Do not repeatedly rerun an identical failure. One failed gate -> inspect root cause -> one hypothesis-driven repair. Do not close/reopen, no-op commit, rewind or duplicate the PR to generate events.

## 9. Completion and handover

Success state:

```text
READY_FOR_AUTHORIZED_MERGE
```

with exact head, drift, changed paths, validation, self-review and thread state recorded.

If merge authority has not been explicitly granted for #239, stop there with exactly one `next_action`: owner/coordinator merge decision.

If blocked, state the exact blocker and evidence. Do not modify PR #240 except to report a concrete overlap if one exists.

# Oteryn-v2 Dependabot serde_json PR #240 Repair Agent

Alias: `OTV2-SERDE-240`

## 1. Role and mode

```text
ROLE: DEPENDENCY REPAIR / VALIDATION AGENT
MODE: REPAIR
TARGET_PR: #240
MERGE_AUTHORITY: NONE_UNLESS_OWNER_EXPLICITLY_AUTHORIZES_PR_240
```

Repair and validate the existing serde_json dependency-update PR #240. Do not replace it merely because the current branch is stale or CI is red.

## 2. Authority

Writable repository: `blakinio/Oteryn-v2` only, limited to PR #240 branch/metadata and exact task bookkeeping required by current governance.

All other repositories are read-only.

Do not broaden this lane into serialization redesign, runtime refactoring, governance changes, production work or dependency cleanup unrelated to serde_json. No Codex/OpenAI/paid review without exact owner authorization. Do not merge without exact authority for #240.

## 3. Mandatory startup

Read root `AGENTS.md`, `AGENTS.override.md`, `docs/agents/AGENTS.md`, `BUILD_TEST_MATRIX.md`, `ANTI_STALL_AND_EXECUTION_BUDGET.md`, `DELIVERY_COMPLETENESS_AND_CLOSEOUT.md` and applicable repository policy.

Inspect live:

- exact `main`;
- PR #240 exact head/base/mergeability;
- full diff and changed files;
- current serde_json pin on live main;
- review submissions/threads;
- exact-head CI/check failures;
- open dependency PRs for overlap;
- branch drift using compare API.

Known historical baseline to verify: #240 proposed `serde_json 1.0.145 -> 1.0.151`, with lockfile replacement of `ryu` by `zmij`, was based on an older main, had substantial drift, and governance/aggregate CI failed on the old head while core Rust/supply-chain jobs had passed. Treat all old-head evidence as stale after any head move.

## 4. Outcome

If still relevant and not superseded:

1. preserve a minimal serde_json-only dependency change;
2. update/rebase the existing PR to current `main` using a safe supported mechanism;
3. regenerate/verify the lockfile only as required by the new version;
4. repair PR metadata to current repository requirements while retaining useful Dependabot release-note provenance;
5. freeze the coherent exact head;
6. conduct exact-head full-diff self-review;
7. obtain fresh required CI;
8. leave #240 merge-ready if every gate passes, but do not merge unless exact owner authority for #240 exists.

If current main already contains the target version or another delivery makes #240 duplicate/superseded, report exact evidence and stop. Do not close unless explicitly authorized.

## 5. Serialization-specific risk review

Do not treat this as a blind patch bump. Review relevant upstream changes and current Oteryn usage for material impact, especially:

- JSON number/float serialization behavior changed across the requested range;
- `ryu -> zmij` transitive lockfile change;
- enum/map-key behavior changes;
- deterministic/golden fixture assumptions, digests, canonicalization or persisted JSON expectations if present in the repository;
- security/resource limits on untrusted JSON paths;
- source/license/advisory implications of the new transitive dependency.

Search current code/tests for serde_json usage before concluding there is no behavior-sensitive surface. Record `UNKNOWN` rather than inventing compatibility evidence.

## 6. PR metadata

Current governance requires a compliant title and body containing `## Summary`, `## Scope` and `## Validation`.

Repair metadata without deleting useful original dependency provenance and without creating a no-op content commit solely to trigger CI.

## 7. Validation ladder

For the final unchanged head:

- full diff and changed-path inspection;
- locked metadata/format/workspace-policy validation;
- Linux workspace build, strict Clippy, tests and synthetic harness when required;
- Windows production-client build/Clippy/smoke/harness when required;
- dependency review and cargo-deny/supply-chain gates;
- targeted tests/golden checks for any repository JSON behavior identified as sensitive;
- `Merge gate / validate` PASS;
- exact-head full-diff self-review with zero open material findings;
- zero requested changes/unresolved threads;
- final drift check against live main.

Historical green jobs from the old head are not final evidence.

Independent review is not automatically required for a routine narrow dependency bump unless actual risk/final diff triggers it. Never invoke owner-funded AI without exact permission.

## 8. Anti-stall and non-overlap

Follow repository retry budgets. Do not repeat identical failures without a new hypothesis. Do not close/reopen or create replacement PRs to regenerate checks.

Do not modify PR #239. If Tokio #239 and serde_json #240 create a real lockfile/order conflict after one merges, re-read live main and reconcile #240 against the new baseline rather than editing the sibling PR.

## 9. Completion and handover

Success state:

```text
READY_FOR_AUTHORIZED_MERGE
```

Record exact head, current-main drift, changed paths, validation run IDs/results, self-review and thread state.

Without explicit #240 merge authority, stop with one `next_action`: owner/coordinator merge decision. If blocked, state the exact blocker and required next action.

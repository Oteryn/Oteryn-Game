# Codex Cloud WP3 stability profile

Status: operational guidance only. This document does not grant repository, path, review, merge, production, or downstream-program authority. Existing task/lease/META boundaries remain authoritative.

## Problem this profile addresses

The canonical WP3 SQLx/rustls/Tokio line is unusually expensive for Codex Cloud because the repository contains many independent Rust manifests and large vendored dependency trees. A generic cloud bootstrap that discovers and fetches every manifest can spend substantial work on unrelated experiments, client crates, tools, and vendored packages before the material worker starts.

WP3 also has a long retained task history. Repeating the full programme, all prior gates, and terminal integration procedure in every Codex invocation increases per-run context without changing the next material operation.

This profile reduces those two controllable costs. It is not evidence that repository size caused any particular Codex service failure.

## Cloud environment configuration

Use a dedicated Codex Cloud environment for the canonical WP3 line.

Setup script:

```bash
bash tools/codex/cloud-wp3-setup.sh
```

Maintenance script:

```bash
bash tools/codex/cloud-wp3-maintenance.sh
```

The setup script intentionally prepares only the Linux dependency graphs rooted at the vendored Tokio 1.53.1, rustls 0.23.43, SQLx-core 0.9.0, and SQLx-PostgreSQL 0.9.0 manifests. It must not be replaced with repository-wide manifest discovery or root/workspace-wide `cargo fetch` merely for convenience.

The maintenance script performs version/worktree checks only. Cached follow-up executions must not refetch or rebuild the whole workspace before the agent has selected the exact task-specific validation.

Changing Codex Cloud environment setup/maintenance configuration is performed in Codex environment settings; repository files alone cannot select the cloud environment.

## Atomic cloud execution rule

For the canonical WP3 material branch, one Codex Cloud invocation should normally own exactly one recoverable material unit:

- one current review finding; or
- one already-authorized resource-accounting cell; or
- one focused proof/test gap; or
- one exact shared-lease discovery boundary.

A single invocation must not be asked to implement a material cell, finish every remaining WP3 cell, perform real TLS and PostgreSQL qualification, obtain final independent review, run final canonical CI, and enter Merge Queue all in the same cloud execution. Those remain programme obligations, but they are resumed from the published exact head in later invocations.

Each material invocation should:

1. fresh-read the current canonical branch/head and the minimum live authority needed for that cell;
2. reference existing protected task/Issue locators instead of reproducing historical policy prose;
3. mutate only the exact active scope for that cell;
4. run focused validation while iterating;
5. publish one coherent same-branch checkpoint when material progress exists;
6. return the exact new head/tree and next legal cell or exact blocker.

Repository-wide/full validation is reserved for a frozen candidate or when the changed path's governing matrix explicitly requires it. This does not waive any final required check.

## Infrastructure failure handling

`Codex couldn't complete this request. Try again later.` without a material branch change is an infrastructure/service failure, not a code or architecture blocker.

For an unchanged material fingerprint, preserve the bounded retry policy. Do not create replacement branches/workers or reset the retry counter by rewording the same request. If the allowed identical retries are exhausted, keep the canonical branch unchanged and report the execution-capability blocker. Resume from that same published head only when an authorized executor is available again or the material fingerprint legitimately changes.

## Current WP3 application

For PR #356, retain the existing canonical branch and all protected scope/lease rules. Use short task-specific continuation prompts that identify only:

- the exact current branch/head;
- the active grant/comment locator;
- the single next material unit;
- its task-specific acceptance evidence;
- any exceptional stop boundary.

Do not duplicate organization-wide retry, review, Merge Queue, or generic safety procedures already provided by the bound META policy and local `AGENTS.md` unless a task-specific delta changes them.

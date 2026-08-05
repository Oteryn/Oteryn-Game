# Execution protocol

## Preflight

- verify repository allowlist and exact default-branch head;
- read task-routed governance/architecture/contracts;
- inspect active tasks, overlapping PRs and ownership;
- verify branch/worktree state when local;
- search for reusable code/contract before designing new abstractions;
- record uncertainty.

## Task establishment

For substantial work create/update a task record, dedicated branch and early draft PR. Define acceptance, exclusions, owned paths, dependencies, validation and cross-repository ordering.

## Implementation

- deliver the smallest complete vertical result;
- keep domain, protocol, persistence, client and platform boundaries explicit;
- make failures bounded and observable;
- update contracts/ADRs in the same programme when public behavior changes;
- avoid unrelated cleanup.

## Validation ladder

1. focused — changed function/module/parser/docs;
2. component — package/crate/service boundary;
3. integration — producer/consumer or persistence boundary;
4. E2E — named user/runtime scenario;
5. heavy final — exact final head, required workspace/CI/soak/security checks.

Do not run heavy validation after every small step unless build manifests/public interfaces/toolchains changed or later work requires a verified binary.

## Review and repair

Inspect full diff, logs and artifacts. Fix root cause; do not weaken checks. Re-run only the smallest check proving the repair, followed by required final exact-head gates.

## Closeout

Perform independent audit, E2E classification, exact-head CI, review-thread cleanup, merge, task archive and ownership release. A merge is not automatically programme completion.

# Oteryn-Game repository transfer readiness

Date: 2026-08-18
Coordination ID: `OTERYN-GAME-TRANSFER-20260818`
Source repository: `blakinio/Oteryn-v2`
Target repository: `Oteryn/Oteryn-Game`
Canonical ecosystem authority: `Oteryn/Oteryn` ADR 0001

## Decision

**Current verdict: `NO_GO` for the physical transfer/rename.**

The previous Wave-1 organization blocker is closed and the previous Actions/reusable-workflow blocker is materially narrowed to a bounded current-state `PASS`. Two material pre-cutover gates remain fail-closed:

1. current GitHub Packages association/inventory for the source repository/account is not enumerable through the available GitHub connector;
2. transfer-back rollback permission under the current `Oteryn` organization policy is not yet proven.

Do not create an empty `Oteryn/Oteryn-Game` repository. The intended target is the same GitHub repository object, transferred from the personal account into `Oteryn` and renamed to `Oteryn-Game` in the transfer flow.

## Current live baseline

| Item | Current evidence | Classification |
| --- | --- | --- |
| source coordinate | `blakinio/Oteryn-v2` | PROVEN |
| source repository ID | `1323412342` | PROVEN |
| source default branch | `main` | PROVEN |
| source visibility | public | PROVEN |
| source connector permissions | admin/maintain/push/pull/triage | PROVEN |
| source main at admission | `457df3772a7aaf648c1a048b2db2caa409fcf974` | PROVEN |
| target coordinate | `Oteryn/Oteryn-Game` | PROVEN target |
| target current state | 404 / absent | PROVEN |
| organization GitHub App installation | `154585379` for `Oteryn` | PROVEN |
| currently visible organization repositories | `Oteryn/Oteryn`, `Oteryn/Oteryn-Atlas` | PROVEN |
| current source open PRs | draft #335, draft #317 | PROVEN |

Every pre-cutover state above must be refreshed immediately before the physical transfer. The SHA is an observation baseline, not future authority.

## Transfer shape

GitHub's current repository-transfer documentation supports transferring a repository owned by a personal account to an organization and optionally changing the repository name in the same transfer flow.

Planned mutation:

```text
blakinio/Oteryn-v2
        |
        | transfer owner -> Oteryn
        | new repository name -> Oteryn-Game
        v
Oteryn/Oteryn-Game
```

This is preferred over creating a new repository and copying Git data because GitHub transfer preserves the repository object, commit history, issues, pull requests, stars/watchers and normal repository settings. Web and Git references to the old location receive ordinary repository redirects, subject to GitHub's documented exceptions.

The old coordinate `blakinio/Oteryn-v2` must not be reused while redirects/rollback compatibility are required.

## Actions and reusable-workflow gate

GitHub does **not** redirect calls to actions or reusable workflows when the owner or repository name changes. Therefore this surface is a cutover blocker only if the source actually hosts an action/reusable workflow or a caller points at such an executable coordinate.

Fresh current-state evidence at `main@457df3772a7aaf648c1a048b2db2caa409fcf974`:

- recursive tree: no `action.yml`;
- recursive tree: no `action.yaml`;
- repository code search: no `workflow_call`;
- current `.github/workflows/**` are repository-local workflows rather than exported reusable workflows;
- repository policy contains no hard-coded source repository coordinate;
- `tools/repository/apply_github_settings.py` derives API target from `GITHUB_REPOSITORY`;
- `.github/workflows/repository-configuration.yml` operates on the current repository dynamically;
- recursive tree: no `Dockerfile` for a repository-hosted Docker action;
- connected-repository search: no `Oteryn-v2/.github/workflows` caller found;
- public web search: no exact `blakinio/Oteryn-v2/.github/workflows` / `uses: blakinio/Oteryn-v2` result found.

**Verdict: `PASS_BOUNDED_CURRENT_STATE`.** There is no current repository-hosted action or reusable workflow for an external workflow to call by the old coordinate. This does not claim knowledge of deleted historical files or inaccessible private repositories, but those cannot create a live dependency on a provider that is absent from current source.

## Package / GHCR gate

Fresh source-tree evidence:

- no `ghcr.io` reference found;
- no `Dockerfile` found;
- no `package.json` found;
- no package-publishing workflow was identified in the current workflow inventory;
- public exact-term searches found no Oteryn-v2 GHCR reference.

This is strong negative evidence for an active source-controlled package producer, but it is **not** sufficient to prove that no manually or historically published GitHub Package is currently associated with the repository or personal account.

GitHub documents that packages associated with a transferred repository may transfer or may lose their repository link depending on the registry. Therefore the package gate remains:

```text
UNKNOWN_PACKAGE_INVENTORY -> NO_GO
```

Required evidence before `CUTOVER_READY`:

- prove repository/user package inventory is empty; **or**
- enumerate each package, registry/type, visibility, repository linkage, Actions access, consumers, expected transfer behavior, post-transfer relink step and rollback behavior.

The available GitHub connector exposes no Packages-list operation, so this evidence currently requires owner-visible GitHub package state or another authorized GitHub API path with package-read capability. No secret/token use is authorized by this task.

## Releases and generated artifacts

The previous Wave-1 inspection found no GitHub Releases for `Oteryn-v2`. The current tree does not introduce a release-publishing workflow in the inspected workflow inventory. This signal is positive but must be refreshed immediately before transfer if a release endpoint becomes available to the executor.

GitHub Actions artifacts are run-scoped evidence, not repository identity authority. Historical workflow-run links may redirect as ordinary repository links, but any external executable action/reusable-workflow coordinate would not; the latter provider surface is currently absent as proven above.

## Open work and cutover lock

Current open PRs:

- #335 — draft Game-owned Atlas semantic export fixture;
- #317 — draft architecture candidate for entitlement consumption.

A repository transfer normally preserves pull requests, but cutover must not race a head/base mutation or merge decision.

Immediately before transfer:

1. refresh `main` and exact open PR list;
2. capture every open PR head SHA and draft/ready state;
3. require no merge/rebase/head rewrite during the owner transfer click;
4. after transfer, verify the same PR numbers, base branch, head SHAs and states under `Oteryn/Oteryn-Game` before any further merges;
5. re-run/observe required checks under the new coordinate as needed; do not infer old pending checks transferred correctly.

No existing PR is closed merely to make migration easier.

## Repository configuration after transfer

Current policy is mostly coordinate-independent:

- repository settings are encoded in `.github/repository-policy.json`;
- the settings applier uses `GITHUB_REPOSITORY`;
- repository-configuration workflow is same-repository;
- main ruleset target is `~DEFAULT_BRANCH`;
- source uses squash-only merge policy and delete-branch-on-merge.

Post-transfer validation must verify the resulting live settings because organization defaults/policies may affect the transferred repository. In particular verify:

- repository ID remains `1323412342`;
- exact owner/name is `Oteryn/Oteryn-Game`;
- visibility remains public;
- default branch remains `main`;
- squash/auto-merge/update-branch settings remain intended;
- main ruleset and `Merge gate / validate` requirement remain active;
- CODEOWNERS/control-plane protection remains effective;
- Actions permissions remain intended;
- secrets/webhooks/deploy keys remain associated where GitHub documents preservation, without exposing secret values;
- GitHub App installation `154585379` can read/write/admin the transferred repository, or owner updates its selected-repository access before further automation.

## Coordinate cleanup classes

### MUST_CHANGE_AT_OR_IMMEDIATELY_AFTER_CUTOVER

- repository-local governance statements that explicitly restrict writes/merge identity to `blakinio/Oteryn-v2` must be updated in a governed post-transfer PR before ordinary future tasks rely on target-local authority;
- current-name branding such as `Oteryn v2` -> `Oteryn Game` should be updated after the repository object has the target coordinate;
- any current operational integration outside GitHub discovered during final preflight that keys on exact repository slug rather than repository ID must be updated.

### SAFE TEMPORARILY THROUGH ORDINARY GITHUB REDIRECT

- normal Git web links and Git clone/fetch/push references, while the old coordinate is not reused;
- historical PR/issue/commit links whose old coordinate is provenance.

### DO NOT REWRITE AS MIGRATION CLEANUP

- archived evidence, historical ADR review references and task records where `blakinio/Oteryn-v2` describes the repository's actual historical coordinate at that time.

### NOT REDIRECT-SAFE

- GitHub Actions/reusable-workflow calls to a hosted action/workflow. Current provider surface is proven absent, so no concrete live entry exists at this time.

## Rollback

Candidate rollback after a wrong transfer result but before target-coordinate authority is broadly consumed:

```text
Oteryn/Oteryn-Game
        |
        | transfer owner -> blakinio
        | new repository name -> Oteryn-v2
        v
blakinio/Oteryn-v2
```

GitHub documentation supports transferring repositories from an organization when the operator has appropriate owner/admin permission. However organization/enterprise policy can restrict transfers.

Therefore rollback feasibility is currently **`NOT_PROVEN`** until the owner confirms that current `Oteryn` policy permits transfer-out/transfer-back for this repository. Do not treat generic GitHub documentation as organization-specific proof.

Rollback window closes progressively once external systems, package linkage, target-only repository configuration or canonical manifests depend uniquely on the new coordinate. Normal redirects are not a substitute for a proven rollback operation.

## Exact cutover runbook once all gates pass

### Preflight

- re-read canonical META ADR 0001 and current migration programme state;
- refresh source repository ID/owner/name/visibility/default branch/main SHA;
- prove target `Oteryn/Oteryn-Game` still does not exist;
- refresh open PR heads/states and ensure a brief migration lock;
- refresh organization installation/access;
- resolve package inventory gate;
- prove transfer-back rollback permission;
- confirm no new action/reusable-workflow provider surface (`action.yml`, `action.yaml`, `workflow_call`);
- confirm no new package/release producer was added;
- record exact pre-state and mutation fingerprint `transfer:1323412342:blakinio/Oteryn-v2->Oteryn/Oteryn-Game`.

### Owner physical operation

In the GitHub repository transfer UI for `blakinio/Oteryn-v2`:

1. Settings -> Danger Zone -> Transfer;
2. choose organization `Oteryn` as new owner;
3. set optional new repository name to `Oteryn-Game`;
4. review GitHub's transfer warnings;
5. type the current repository name required by the confirmation UI;
6. execute the transfer once.

If the UI result is ambiguous, **do not retry**. First read both exact coordinates and resolve repository ID `1323412342`.

### Immediate post-transfer verification

- `Oteryn/Oteryn-Game` exists and repository ID is still `1323412342`;
- old source coordinate does not resolve to a different/reused repository object;
- owner/name/visibility/archived/default branch are correct;
- `main` SHA matches the locked pre-transfer SHA;
- open PR numbers/head SHAs/states are preserved;
- connector installation access is proven;
- repository settings/ruleset/Actions behavior are verified;
- package state exactly matches the planned outcome;
- no unexpected Pages/deployment/package warning requires rollback;
- create a dedicated post-transfer governance/coordinate-cleanup task/PR before ordinary future work assumes the new identity.

### Replay guard

Never issue a second transfer based on a timeout, blank page, stale UI or ambiguous message. Read both coordinates and repository ID first.

## Current blockers

### P1

1. exact current GitHub Packages inventory/association is not proven;
2. current `Oteryn` organization transfer-back rollback capability is not proven.

### P2 / post-cutover

- target-local branding and repository-identity governance cleanup;
- live repository settings/app-access verification after organization ownership applies;
- old local clones/remotes should be updated to the target URL even while redirects work.

## Next action

Obtain the two owner-visible facts: current package inventory for `blakinio/Oteryn-v2`/`blakinio`, and confirmation that the owner can transfer a repository from `Oteryn` back to `blakinio` if rollback is required before target authority is broadly consumed. Then freeze the readiness head, merge the readiness PR, and move the physical transaction to `CUTOVER_READY` if no new blocker appears.

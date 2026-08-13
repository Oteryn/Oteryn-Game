# GitHub repository governance

## Canonical merge model

- `main` is protected by the `Protect main` branch ruleset.
- All changes reach `main` through a pull request.
- Squash is the only allowed merge method.
- The pull request title becomes the squash-commit title and the pull request body becomes its canonical message.
- `Merge gate / validate` is the single stable required status check for the current exact PR head and the branch must be up to date.
- The aggregate merge gate always requires repository/agent governance, Dependency Review and CodeQL, and additionally requires the full Rust policy/Linux/Windows/supply-chain set when Rust/workspace-sensitive paths change.
- If an initial PR event is suppressed and no run exists to rerun, recover without changing the head SHA: close and reopen the unchanged pull request. The standard `pull_request: reopened` event re-runs the gate in the ordinary pull-request trust context, and the scope job re-resolves live PR metadata before any repository code executes.
- Changed-file classification fails closed when GitHub reports more than the 3,000-file files-API cap or when the enumerated file count does not exactly match the pull request metadata.
- Review conversations must be resolved.
- Force-push, branch deletion, and merge commits are rejected.
- Required approvals remain `0` while the repository has only one maintainer. Increase this to at least `1` when a second trusted maintainer is added.
- GitHub-generated squash commits are verified. A strict signed-commit rule is deferred because it would prevent the maintainer from squash-merging third-party-authored PRs such as Dependabot updates.

The retained `Agent governance / validate` workflow remains available during the transition to the aggregate gate and for explicit manual governance validation, but it is not the canonical required status after the repository policy is applied.

## Protected merge-authority control plane

After the aggregate merge gate is bootstrapped, a dedicated push ruleset named `Protect repository control plane` applies `file_path_restriction` with no bypass actors to:

- `.github/workflows/*` and `.github/workflows/**/*`;
- `.github/repository-policy.json`;
- `tools/repository/*` and `tools/repository/**/*`.

The branch ruleset `Protect main` retains pull-request, linear-history and required-status protection; the dedicated push ruleset owns only the path restriction because GitHub does not permit `file_path_restriction` to be combined with branch-only rules in one branch ruleset.

These paths are intentionally immutable through ordinary pull requests. This keeps the workflow that emits the required status, the policy that selects that status, the repository-administration workflow, and the scripts that apply/validate repository settings outside the normal PR-modifiable trust domain. It also prevents adding a new workflow that can consume `REPO_ADMIN_TOKEN`.

A legitimate future merge-authority/control-plane change therefore requires an explicit owner action in GitHub Settings to temporarily alter the live push-ruleset restriction before opening or updating the control-plane PR, followed by exact-head validation, required independent review, merge, and restoration/verification of the intended restriction. Do not create routine bypass actors for convenience.

`Merge authority audit / validate` is a deterministic, non-AI audit workflow for high-risk merge-authority changes. It independently checks the expected ruleset contract and executes adversarial mutation tests against the repository validator on the exact PR head. It does not consume owner-funded AI quota and does not replace the ordinary aggregate merge gate.

## Pull request and commit convention

Pull request titles follow:

`type(scope): imperative summary`

The PR title and body form the permanent squash commit. Working commits may be iterative but must remain reviewable and free of secrets, generated outputs, and unrelated changes.

## GitHub Actions security

- Default `GITHUB_TOKEN` permissions are read-only.
- Each workflow declares least-privilege permissions.
- External actions are pinned to full commit SHAs.
- Workflows avoid privileged checkout of untrusted pull-request code.
- Merge-gate recovery does not use `workflow_dispatch` to execute pull-request code. Recovery uses the normal `pull_request: reopened` event on the unchanged head instead.
- The scope job verifies the live open same-repository PR, target branch, exact event head SHA and complete changed-file enumeration before downstream jobs check out the validated head.
- Dependency Review receives explicit base/head revisions from the validated PR context.
- Repository-administration changes run only after a protected merge to `main` or an explicit manual dispatch and require `REPO_ADMIN_TOKEN`.
- No manual environment approval is required while the repository has one maintainer; the protected PR, exact-head CI, aggregate merge gate, read-only workflow token, separate admin token, and ruleset-level control-plane restriction are the enforcement boundary.
- Dependabot maintains both GitHub Actions and Cargo dependencies.
- CodeQL scans Python and GitHub Actions workflows.
- Dependency review blocks newly introduced high-severity vulnerable dependencies.

## Security features

The repository policy enables:

- vulnerability alerts and automated security fixes;
- private vulnerability reporting;
- secret scanning and push protection where supported by the repository plan;
- CodeQL code scanning through a retained workflow and the PR aggregate gate.

## Licensing governance

The canonical repository policy records `MPL-2.0` as the default software license and requires:

- the unmodified MPL-2.0 text in `LICENSE`;
- the repository-wide scope and contribution policy in `docs/repository/LICENSING.md`;
- the reserved creative-asset boundary in `LICENSE-ASSETS.md`;
- the separate names and branding boundary in `TRADEMARKS.md`.

The standard MPL-2.0 text includes Exhibit B, but Oteryn-v2 does not attach or apply the separate Exhibit B incompatibility notice to covered source. File- or directory-specific notices may define justified exceptions, but they must preserve third-party provenance and pass compatibility review.

The repository validator checks that these files and machine-readable policy fields remain present and mutually consistent. GitHub's displayed license classification is derived from the root `LICENSE` file rather than an independently mutable repository setting.

## Configuration as code

`.github/repository-policy.json` is the expected GitHub configuration. `tools/repository/apply_github_settings.py` applies it idempotently, including repository metadata, labels, topics, Actions permissions, security settings, the `Protect main` branch ruleset, and the dedicated `Protect repository control plane` push ruleset. `.github/workflows/repository-configuration.yml` runs only when the policy, apply script, or workflow changes on `main`, or through an explicit manual dispatch.

`tools/repository/validate_repository_policy.py` checks that required governance files exist, workflow actions use full SHAs, dangerous privileged triggers are absent, the aggregate merge-gate recovery contract is retained, the documented required context agrees with machine policy, and both rulesets have the expected protection and licensing invariants.

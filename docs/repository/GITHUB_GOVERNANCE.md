# GitHub repository governance

## Canonical merge model

- `main` is protected by the `Protect main` repository ruleset.
- All changes reach `main` through a pull request.
- Squash is the only allowed merge method.
- The pull request title becomes the squash-commit title and the pull request body becomes its canonical message.
- `Merge gate / validate` is the single stable required status check for the current exact PR head and the branch must be up to date.
- The aggregate merge gate always requires repository/agent governance, Dependency Review and CodeQL, and additionally requires the full Rust policy/Linux/Windows/supply-chain set when Rust/workspace-sensitive paths change.
- If an initial PR event is suppressed and no run exists to rerun, the merge gate may be manually dispatched only from the exact unchanged PR head branch with the open PR number and full expected head SHA; the workflow re-resolves live PR metadata and fails closed if the head moved.
- Review conversations must be resolved.
- Force-push, branch deletion, and merge commits are rejected.
- Required approvals remain `0` while the repository has only one maintainer. Increase this to at least `1` when a second trusted maintainer is added.
- GitHub-generated squash commits are verified. A strict signed-commit rule is deferred because it would prevent the maintainer from squash-merging third-party-authored PRs such as Dependabot updates.

The retained `Agent governance / validate` workflow remains available during the transition to the aggregate gate and for explicit manual governance validation, but it is not the canonical required status after the repository policy is applied.

## Pull request and commit convention

Pull request titles follow:

`type(scope): imperative summary`

The PR title and body form the permanent squash commit. Working commits may be iterative but must remain reviewable and free of secrets, generated outputs, and unrelated changes.

## GitHub Actions security

- Default `GITHUB_TOKEN` permissions are read-only.
- Each workflow declares least-privilege permissions.
- External actions are pinned to full commit SHAs.
- Workflows avoid privileged checkout of untrusted pull-request code.
- The merge-gate recovery dispatch requires an open same-repository PR targeting `main`, an exact expected head SHA and a dispatch ref resolving to that same unchanged head.
- Dependency Review receives explicit base/head revisions from the validated PR context so the same dependency comparison can be performed for ordinary PR events and exact-head dispatch recovery.
- Repository-administration changes run only after a protected merge to `main` or an explicit manual dispatch and require `REPO_ADMIN_TOKEN`.
- No manual environment approval is required while the repository has one maintainer; the protected PR, exact-head CI, aggregate merge gate, read-only workflow token, and separate admin token are the enforcement boundary.
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

`.github/repository-policy.json` is the expected GitHub configuration. `tools/repository/apply_github_settings.py` applies it idempotently, including repository metadata, labels, topics, Actions permissions, security settings, and the `main` ruleset. `.github/workflows/repository-configuration.yml` runs only when the policy, apply script, or workflow changes on `main`, or through an explicit manual dispatch.

`tools/repository/validate_repository_policy.py` checks that required governance files exist, workflow actions use full SHAs, dangerous privileged triggers are absent, the aggregate merge-gate recovery contract is retained, the documented required context agrees with machine policy, and the policy has the expected protection and licensing invariants.

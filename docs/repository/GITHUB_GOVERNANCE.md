# GitHub repository governance

## Canonical merge model

- `main` is protected by the `Protect main` repository ruleset.
- All changes reach `main` through a pull request.
- Squash is the only allowed merge method.
- The pull request title becomes the squash-commit title and the pull request body becomes its canonical message.
- `Agent governance / validate` must pass on the current head and the branch must be up to date.
- Review conversations must be resolved.
- Force-push, branch deletion, and merge commits are rejected.
- Required approvals remain `0` while the repository has only one maintainer. Increase this to at least `1` when a second trusted maintainer is added.
- GitHub-generated squash commits are verified. A strict signed-commit rule is deferred because it would prevent the maintainer from squash-merging third-party-authored PRs such as Dependabot updates.

## Pull request and commit convention

Pull request titles follow:

`type(scope): imperative summary`

The PR title and body form the permanent squash commit. Working commits may be iterative but must remain reviewable and free of secrets, generated outputs, and unrelated changes.

## GitHub Actions security

- Default `GITHUB_TOKEN` permissions are read-only.
- Each workflow declares least-privilege permissions.
- External actions are pinned to full commit SHAs.
- Workflows avoid privileged checkout of untrusted pull-request code.
- Repository-administration changes run through the protected `repository-administration` environment and `REPO_ADMIN_TOKEN`.
- Dependabot maintains GitHub Actions dependencies.
- CodeQL scans Python and GitHub Actions workflows.
- Dependency review blocks newly introduced high-severity vulnerable dependencies.

## Security features

The repository policy enables:

- vulnerability alerts and automated security fixes;
- private vulnerability reporting;
- secret scanning and push protection where supported by the repository plan;
- CodeQL code scanning through a retained workflow.

## Configuration as code

`.github/repository-policy.json` is the expected GitHub configuration. `tools/repository/apply_github_settings.py` applies it idempotently, including repository metadata, labels, topics, Actions permissions, security settings, administration environment, and the `main` ruleset. `.github/workflows/repository-configuration.yml` runs only when the policy or the workflow changes on `main`, or through an explicitly approved manual dispatch.

`tools/repository/validate_repository_policy.py` checks that required governance files exist, workflow actions use full SHAs, dangerous privileged triggers are absent, and the policy has the expected protection invariants.

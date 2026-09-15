# GitHub repository governance

## Canonical merge model

- `main` is protected by the `Protect main` branch ruleset.
- All changes reach `main` through a pull request.
- Squash is the only allowed merge method.
- The pull request title becomes the squash-commit title and the pull request body becomes its canonical message.
- `game-gate` is the single stable required status check for the current exact PR head and the branch must be up to date.
- The aggregate merge gate always requires repository/agent governance, Dependency Review and CodeQL, and additionally requires the full Rust policy/Linux/Windows/supply-chain set when Rust/workspace-sensitive paths change.
- PR metadata edits automatically re-run the exact-head aggregate gate through the standard `pull_request: edited` event. The gate re-resolves live PR metadata and fails closed if the PR head moved after the event was recorded, so title/body repairs do not require `workflow_dispatch` and a previously green required status cannot silently cover later invalid metadata.
- If an initial PR event is suppressed and no run exists to rerun, recover without changing the head SHA: close and reopen the unchanged pull request. The standard `pull_request: reopened` event re-runs the gate in the ordinary pull-request trust context, and the scope job re-resolves live PR metadata before any repository code executes.
- Changed-file classification fails closed when GitHub reports more than the 3,000-file files-API cap or when the enumerated file count does not exactly match the pull request metadata.
- Review conversations must be resolved.
- Force-push, branch deletion, and merge commits are rejected.
- General required approvals remain `0` while the repository has only one maintainer. `require_code_owner_review` is enabled separately and therefore requires owner approval only when a PR touches a path actually present in the deliberately narrow base-branch `CODEOWNERS` file.
- Code Owner approvals are dismissed when new reviewable commits are pushed, so an approval cannot silently cover a later control-plane head.
- GitHub-generated squash commits are verified. A strict signed-commit rule is deferred because it would prevent the maintainer from squash-merging third-party-authored PRs such as Dependabot updates.

The retained `Agent governance / validate` workflow remains available during the transition to the aggregate gate and for explicit manual governance validation, but it is not the canonical required status after the repository policy is applied. It also handles `pull_request: edited` and reads current PR title/body/base/head state from the GitHub API rather than trusting a frozen event-body snapshot; `workflow_dispatch` is retained only as an exact-head break-glass governance recovery path.

## Protected merge-authority control plane

`Oteryn-v2` is a public repository. GitHub push rulesets are available for private/internal repositories (and eligible fork networks), not for an ordinary public repository. A push ruleset also applies repository-wide and therefore does not use branch `ref_name` targeting. The failed post-merge run after PR #238 proved this platform boundary when GitHub rejected the attempted public push ruleset.

For the current public repository, the native fallback is **required Code Owner review on a deliberately narrow control-plane ownership map**. The base-branch `.github/CODEOWNERS` owns only:

- `.github/CODEOWNERS` itself;
- `.github/workflows/`;
- `.github/repository-policy.json`;
- `tools/repository/`.

GitHub evaluates CODEOWNERS from the pull request base branch. A PR changing one of these paths therefore cannot replace its own ownership mapping and use that replacement to authorize itself. Ordinary architecture, runtime and content paths are intentionally absent from CODEOWNERS so their normal approval count remains zero.

The machine policy retains the no-bypass `Protect repository control plane` push-ruleset definition as a **latent private/internal strategy only**. For private/internal visibility that latent strategy is a **dedicated push ruleset**. `tools/repository/apply_github_settings.py` applies it only when GitHub reports repository visibility `private` or `internal`; on a public repository it removes any stale ruleset of that name and verifies the Code Owner fallback instead. The push-ruleset definition intentionally contains no `ref_name` condition because GitHub push rulesets are repository-wide.

A legitimate future merge-authority/control-plane change with one maintainer is intentionally break-glass work: the owner must explicitly and temporarily alter the live `Protect main` Code Owner-review requirement in GitHub Settings, perform the bounded governance PR with exact-head validation and mandatory independent audit, then restore the canonical policy and require post-merge repository-configuration plus live ruleset readback. Do not create routine bypass actors or weaken the general merge gate for convenience. Adding a second trusted maintainer can instead allow ordinary Code Owner approval without break-glass.

`Merge authority audit / validate` is the deterministic, non-AI independent audit workflow for high-risk merge-authority changes. It checks the branch ruleset contract, the public Code Owner fallback, the latent private/internal push policy, visibility-aware apply/readback logic and adversarial mutations of the aggregate merge gate on the exact PR head. It does not consume owner-funded AI quota and does not replace the ordinary aggregate merge gate.

GitHub platform references for this boundary:

- <https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/creating-rulesets-for-a-repository>
- <https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners>

## Pull request and commit convention

Pull request titles follow:

`type(scope): imperative summary`

The PR title and body form the permanent squash commit. Working commits may be iterative but must remain reviewable and free of secrets, generated outputs, and unrelated changes.

## GitHub Actions security

- Default `GITHUB_TOKEN` permissions are read-only.
- Each workflow declares least-privilege permissions.
- External actions are pinned to full commit SHAs.
- Workflows avoid privileged checkout of untrusted pull-request code.
- Merge-gate recovery does not use `workflow_dispatch` to execute pull-request code. Metadata changes use the normal `pull_request: edited` path; a genuinely suppressed initial event can still be recovered with `pull_request: reopened` on an unchanged head.
- The scope job verifies the live open same-repository PR, target branch, exact event head SHA and complete changed-file enumeration before downstream jobs check out the validated head.
- Dependency Review receives explicit base/head revisions from the validated PR context.
- Repository-administration changes run only after a protected merge to `main` or an explicit manual dispatch and require `REPO_ADMIN_TOKEN`.
- No manual environment approval is required for ordinary work while the repository has one maintainer; the protected PR, exact-head aggregate gate and read-only workflow defaults are the routine boundary. Control-plane changes are the explicit exception and require the Code Owner/break-glass process above.
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

`.github/repository-policy.json` is the expected GitHub configuration. `tools/repository/apply_github_settings.py` applies it idempotently, including repository metadata, labels, topics, Actions permissions, security settings and the `Protect main` branch ruleset. It selects the supported control-plane enforcement by live repository visibility: Code Owner review for public `Oteryn-v2`, or the latent push ruleset for a future private/internal repository. `.github/workflows/repository-configuration.yml` runs only when the policy, apply script, or workflow changes on `main`, or through an explicit manual dispatch.

`tools/repository/validate_repository_policy.py` retains the static merge-gate/ruleset/licensing checks. The deterministic merge-authority audit additionally validates the visibility-aware Code Owner fallback and latent push-ruleset boundary. Post-merge repository configuration is the integration proof that the policy can actually be applied to GitHub and read back from the live repository.

# Contributing to Oteryn v2

## Workflow

1. Read `AGENTS.md`, `AGENTS.override.md`, and the nearest governing instructions.
2. Search existing tasks, issues, pull requests, ADRs, contracts, and code before creating a new abstraction.
3. For substantial work, create a bounded `OTV2-*` task record and a dedicated branch.
4. Open a pull request to `main`; never push feature, fix, architecture, or documentation work directly to `main`.
5. Keep the change focused, update affected tests and contracts, and record exact validation evidence.
6. Resolve requested changes and review threads before squash merge.

## Pull request titles

Use:

`type(scope): imperative summary`

Allowed types:

`feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`

Breaking changes use `!`, for example:

`feat(protocol)!: replace frame header`

The title is the squash-commit title and must remain meaningful in permanent history.

## Commits

Working commits should be reviewable and must not contain generated build outputs, secrets, credentials, private data, proprietary assets without confirmed rights, or unrelated cleanup. Pull requests are squash-merged.

## Validation

Run the focused checks named by `docs/agents/BUILD_TEST_MATRIX.md` and the actual workspace. The required GitHub checks must pass on the exact unchanged PR head. A green unrelated or historical run is not evidence for the current change.

## Security

Report vulnerabilities through private vulnerability reporting as described in `SECURITY.md`. Do not open public vulnerability issues.

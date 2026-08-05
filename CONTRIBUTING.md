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

## Contribution licensing and provenance

Unless explicitly accepted under different terms, contributions to source code, scripts, schemas, configuration, tests and technical documentation are submitted under the Mozilla Public License 2.0 (`MPL-2.0`). See `LICENSE` and `docs/repository/LICENSING.md`.

By submitting a contribution, you represent that you created it or have sufficient rights to provide it under the stated license. Preserve all applicable copyright, license, patent and attribution notices.

Do not submit third-party code, maps, art, audio, fonts, data, documentation or other material without documented provenance and a compatible license. Creative assets and Oteryn branding are governed separately by `LICENSE-ASSETS.md` and `TRADEMARKS.md`.

The project does not currently require copyright assignment or a Contributor License Agreement. Do not describe a contribution as granting proprietary relicensing rights unless a separate written agreement actually provides them.

## Validation

Run the focused checks named by `docs/agents/BUILD_TEST_MATRIX.md` and the actual workspace. The required GitHub checks must pass on the exact unchanged PR head. A green unrelated or historical run is not evidence for the current change.

## Security

Report vulnerabilities through private vulnerability reporting as described in `SECURITY.md`. Do not open public vulnerability issues.

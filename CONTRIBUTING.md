# Contributing to Oteryn v2

## Workflow

1. Read `AGENTS.md` and the nearest governing instructions.
2. Search existing tasks, issues, pull requests, ADRs, contracts, and code before creating a new abstraction.
3. For substantial work, create a bounded `OTV2-*` task record and a dedicated branch.
4. Open a pull request to `main`; never push feature, fix, architecture, or documentation work directly to `main`.
5. Keep the change focused, update affected tests and contracts, and record exact validation evidence.
6. Resolve requested changes and review threads before integration. When Merge Queue is enabled, keep the accepted PR head unchanged and let the queue create and validate the synthetic integration candidate instead of merging `main` into the PR merely to refresh it.

## Playable-first and minimum-sufficient delivery

Oteryn prioritizes the shortest safe path to a real playable product. Prefer the smallest change that satisfies the current accepted requirement and unlocks the next real capability in the production-shaped path: login/session, character, transport, world/map, gameplay, persistence, reconnect and restart.

Do not add speculative frameworks, generalized abstractions, future-scale machinery, benchmark-only substitutes or hardening for hypothetical problems unless a current accepted contract, concrete threat, hard-to-reverse compatibility decision, or representative measurement proves they are needed now.

Minimum effort does **not** mean reduced quality. Accepted correctness, security, durability, compatibility, validation and measured performance requirements remain mandatory. Avoid premature optimization, but do not knowingly accept a material measured regression merely to reduce implementation effort.

## Upstream-first dependencies

Oteryn follows `UPSTREAM_FIRST / PATCH_ON_PROVEN_NEED` as part of the broader `PLAYABLE_FIRST / MINIMUM_SUFFICIENT_CHANGE` doctrine. Read `docs/repository/PLAYABLE_FIRST_ENGINEERING_POLICY.md` before introducing or expanding a fork, vendored modification, deep dependency instrumentation or local reimplementation of mature third-party functionality.

Prefer, in order: upstream configuration, supported upstream APIs/extension points, an Oteryn-owned wrapper/adapter, an upstream contribution where practical, then the smallest justified downstream patch. Maintain a full fork only when the smaller options are proven insufficient.

A dependency customization must be justified by concrete evidence against an exact upstream version, such as a failing reproducible test, source-level API gap, representative benchmark, concrete security reproducer, or accepted contract requirement that upstream cannot satisfy. Hypothetical future need, generic `hardening`, `more control`, or an unmeasured performance claim is not enough.

Keep every retained patch small, provenance-pinned, independently reviewable, regression-tested and removable. Re-evaluate it on dependency upgrades and remove it when upstream or a simpler Oteryn-owned layer can satisfy the same accepted requirement. Do not weaken an accepted correctness or security invariant merely to eliminate a patch; an intentional invariant change requires an explicit reviewed architecture/contract decision.

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

Run the focused checks named by `docs/agents/BUILD_TEST_MATRIX.md` and the actual workspace. The required GitHub checks must pass on the exact unchanged PR head. When the repository uses Merge Queue, the same stable required `game-gate` must also pass on GitHub's synthetic `merge_group` integration candidate before protected integration. A green unrelated or historical run is not evidence for the current change.

## Security

Report vulnerabilities through private vulnerability reporting as described in `SECURITY.md`. Do not open public vulnerability issues.

# Oteryn-Game repository transfer readiness

Date: 2026-08-18
Coordination ID: `OTERYN-GAME-TRANSFER-20260818`
Source: `blakinio/Oteryn-v2`
Target: `Oteryn/Oteryn-Game`
Authority: `Oteryn/Oteryn` ADR 0001

## Decision

**Physical transfer status: `NO_GO`.**

The organization blocker is closed and the previous external Actions/reusable-workflow blocker is narrowed to a current-state PASS. Two material gates remain:

1. GitHub Packages inventory/linkage for the source repository/account is not proven;
2. current `Oteryn` organization policy permitting transfer-back rollback is not proven.

Do **not** create an empty `Oteryn/Oteryn-Game`. The intended mutation is transfer of the existing repository object `1323412342` from `blakinio` to `Oteryn`, with `Oteryn-Game` as the new repository name.

## Live baseline

| Fact | Evidence state |
| --- | --- |
| `blakinio/Oteryn-v2` exists, repository ID `1323412342` | PROVEN |
| visibility `public`, default branch `main`, connector admin/write | PROVEN |
| source main at admission `457df3772a7aaf648c1a048b2db2caa409fcf974` | PROVEN |
| `Oteryn/Oteryn-Game` absent / 404 | PROVEN |
| Oteryn GitHub App installation `154585379` active | PROVEN |
| Oteryn installation currently exposes META + Atlas with admin/write | PROVEN |
| open source PRs: draft #335 and draft #317 | PROVEN |
| package inventory | UNKNOWN |
| transfer-back permission under current Oteryn policy | UNKNOWN |

All live facts must be refreshed immediately before cutover.

## Transfer semantics

GitHub supports transferring a personally owned repository to an organization and allows an optional new repository name in that transfer flow. The transfer preserves the repository history and normal repository objects such as issues and pull requests; ordinary Git/web references receive repository redirects. Repository settings, organization policy effects, Actions access and package linkage are still explicit post-transfer verification items rather than assumed preserved state.

The old coordinate `blakinio/Oteryn-v2` must not be reused while redirect compatibility or rollback depends on it.

## Actions / reusable-workflow gate

GitHub Actions does not follow ordinary repository redirects for actions or reusable workflows when owner/repository identity changes. Fresh live-source inspection therefore checked whether such a provider surface exists.

At `main@457df3772a7aaf648c1a048b2db2caa409fcf974`:

- no `action.yml`;
- no `action.yaml`;
- no `workflow_call`;
- no `Dockerfile` for a Docker action;
- current workflows are repository-local;
- repository policy contains no hard-coded source coordinate;
- repository settings tooling derives the target from `GITHUB_REPOSITORY`;
- connected-repository search found no `Oteryn-v2/.github/workflows` caller;
- bounded public search found no exact old-coordinate action/reusable-workflow call.

**Gate: `PASS_BOUNDED_CURRENT_STATE`.** The current repository exposes no hosted Action/reusable-workflow provider that a live caller could invoke using the old coordinate. This is not a claim about deleted historical files or inaccessible private repositories.

## GitHub Packages / GHCR gate

Current source evidence is negative:

- no `ghcr.io` reference;
- no `Dockerfile`;
- no `package.json`;
- no package-publishing workflow identified in the current workflow inventory;
- bounded public search found no Oteryn-v2 GHCR result.

That does **not** prove the absence of manually or historically published packages currently associated with the repository/account. The available GitHub connector has no Packages-list operation.

GitHub documents registry-dependent behavior when a repository associated with a package is transferred: a package may transfer or may lose its repository link. Therefore:

```text
packages.inventory = UNKNOWN
packages.cutover_gate = BLOCKING
public_status = NO_GO
```

Before `CUTOVER_READY`, either prove the package inventory is empty or enumerate every package's registry/type, visibility, repository linkage, Actions access, consumers, expected transfer behavior, relink step and rollback.

## Open work / cutover lock

Current open PRs at admission:

- #335 — draft Atlas semantic export fixture;
- #317 — draft entitlement consumer architecture candidate.

Immediately before transfer, freeze a short cutover window: refresh `main`, record every open PR head SHA/state, and prevent merge/rebase/head rewrites during the owner transfer action. Immediately after transfer, verify the same PR numbers/head SHAs/states under `Oteryn/Oteryn-Game` before normal work resumes.

No active PR is closed merely to simplify migration.

## Repository configuration verification

Current repository policy is mostly coordinate-independent (`GITHUB_REPOSITORY`, `~DEFAULT_BRANCH`, `Merge gate / validate`). After transfer, verify rather than assume:

- repository ID remains `1323412342`;
- exact coordinate is `Oteryn/Oteryn-Game`;
- visibility remains public and default branch remains `main`;
- locked pre-transfer `main` SHA is unchanged;
- squash/auto-merge/update-branch settings are intended;
- `Protect main` / `Merge gate / validate` protection remains effective;
- CODEOWNERS/control-plane protection remains effective;
- Actions permissions remain intended;
- GitHub App installation `154585379` exposes admin/write access;
- webhooks/secrets/deploy keys/package linkage are in the expected resulting state without exposing secret values.

## Coordinate cleanup

**Post-cutover must change:** target-local governance clauses that hard-code `blakinio/Oteryn-v2`, current branding that should become `Oteryn Game`, and any discovered operational integration keyed to the old slug.

**Keep as historical provenance:** archived tasks, evidence and ADR/review references where `blakinio/Oteryn-v2` was the true coordinate at the time.

**Redirect-safe temporarily:** ordinary Git/web references while the old coordinate is not reused.

**Not redirect-safe:** hosted Action/reusable-workflow calls; no concrete current provider surface exists.

## Rollback

Candidate rollback before broad target-coordinate adoption:

```text
Oteryn/Oteryn-Game
  -> transfer owner back to blakinio
  -> rename back to Oteryn-v2
  -> blakinio/Oteryn-v2
```

Generic GitHub capability is not organization-specific proof. Organization/enterprise policy can restrict repository transfer. Therefore rollback is currently:

```text
rollback.state = NOT_PROVEN
```

Required proof: the current owner confirms that the current `Oteryn` organization policy permits transfer-out/transfer-back of this repository before target authority is broadly consumed.

## Exact cutover runbook after all gates pass

### Preflight

- refresh META ADR 0001 and migration programme authority;
- refresh source owner/name/ID/visibility/default branch/main SHA;
- prove target still absent;
- refresh open PR heads/states and acquire brief cutover lock;
- refresh organization installation/access;
- resolve package gate;
- prove transfer-back rollback permission;
- recheck `action.yml`, `action.yaml`, `workflow_call` and package/release producer surfaces;
- persist mutation fingerprint `transfer_repository:1323412342:blakinio/Oteryn-v2->Oteryn/Oteryn-Game`.

### Owner physical operation

In `blakinio/Oteryn-v2`: **Settings -> Danger Zone -> Transfer**. Choose owner `Oteryn`, set new repository name `Oteryn-Game`, review warnings, complete GitHub's confirmation and execute the transfer **once**.

If the UI result is ambiguous, do not retry. First resolve repository ID `1323412342` against both old and target coordinates.

### Immediate post-transfer proof

- target exists with repository ID `1323412342`;
- old coordinate does not resolve to a different/reused repository;
- owner/name/visibility/default branch/main SHA match expected state;
- open PRs are preserved exactly;
- GitHub App access works;
- settings/ruleset/Actions/package state match plan;
- no unexpected transfer warning/state requires rollback;
- start a dedicated post-transfer governance/branding/coordinate-cleanup PR before ordinary future work relies on the new target identity.

## Current blockers

### P1

1. `github_packages_inventory`
2. `transfer_back_rollback_permission`

## Next action

Owner provides the current GitHub Packages state for `blakinio/Oteryn-v2` / account `blakinio` and confirms whether the current `Oteryn` policy permits transfer-back rollback. Until both facts are proven, the physical transfer remains `NO_GO` and Draft PR #336 remains the durable readiness record.

# OTV2-20260806-fnd-01-workspace-migration-contract

```yaml
task_id: OTV2-20260806-fnd-01-workspace-migration-contract
title: Define the canonical Rust workspace and existing-client migration dispositions
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/fnd-01-workspace-migration-contract
pr: 46
base_sha: cbc2150024d98bbdbfa9b1c17bc9b9df16bcd9f2
final_head_sha: 8c17a48b0f43b89baec780ea078cc4881d081577
merge_commit: 3e11cf36ffdc1191fabd60c09e8da9818594e189
owner: GPT-5.6-Thinking-architecture-coordinator
created_at: 2026-08-06T08:13:00+02:00
completed_at: 2026-08-06T09:46:00+02:00
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/otclient (read-only source evidence)
```

## Outcome

`FND-01` is owner-accepted and merged. It defines the initial 19-member destination Rust workspace, one migration disposition for every source member, dependency/release-role/toolchain/target/CI boundaries, pre-native Canary isolation and the mandatory post-slice crate fitness review.

## Canonical contracts

- `docs/architecture/FND-01_WORKSPACE_AND_RUST_MIGRATION_CONTRACT.md`
- `docs/architecture/FND-01_RUST_SOURCE_INVENTORY.md`
- `docs/architecture/FND-01_OWNER_ACCEPTANCE_AND_CRATE_FITNESS_REVIEW.md`

Accepted source inventory:

```text
blakinio/otclient@c923ad8a1dff17b4933a6110931b0823cec2c590
```

Accepted contract blob:

```text
b0127a91f201d4cba766053dc74517fe5cb49268
```

## Accepted decisions

- 19 immediately consumed initial workspace members.
- Production pre-native client separated from the non-release synthetic harness.
- No `protocol-canary`, current `protocol-core`, gameplay `transport` or `game-session` in the initial workspace.
- Platform/Identity I/O is asynchronous, cancellation-safe and runtime-owned.
- Gameplay routes, credentials and admission state remain deferred to `FND-04`.
- Native protocol/transport remains deferred to `FND-02`.
- Public identifier semantics remain deferred to `FND-ID-01`.
- A crate fitness review is mandatory after the first complete native vertical slice and before `VSL-01` completion.

## Validation

Exact final PR head `8c17a48b0f43b89baec780ea078cc4881d081577`:

- Agent governance run `31082078889`: `PASS`
- Dependency review run `31082078750`: `PASS`
- CodeQL run `31082078754`: `PASS`
- unresolved review threads: `0`
- branch relation to merge base: ahead, behind `0`
- changed files: four declared documentation/task files

Independent architecture audit: `PASS`, zero open material findings.

Component/integration/E2E: `NOT_APPLICABLE` because the package changed architecture documentation only.

## Merge and closeout

- PR: `#46`
- merge method: squash
- merge commit: `3e11cf36ffdc1191fabd60c09e8da9818594e189`
- result: `MERGED`
- source repository changed: no
- Cargo workspace/runtime changed: no
- ownership: released by archive lifecycle PR

## Next ordered action

Prepare and owner-accept `VSL-02` — the exact Rust client migration and cutover contract. Physical code migration remains forbidden until `VSL-02` is accepted.

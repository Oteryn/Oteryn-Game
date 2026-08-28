# OTV2-20260806-vsl-02-rust-client-cutover-contract

```yaml
task_id: OTV2-20260806-vsl-02-rust-client-cutover-contract
title: Define exact Rust client migration and cutover contract
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/vsl-02-rust-client-cutover-contract
pr: 48
base_sha: 9034bd4bfa491eac6a898b29bc8151c94a4c2b89
validated_head_sha: 6cdc94aa7c711f68a6e5792d12aaefc3de7dcd54
merge_commit: 824599bb2d696e15e22319007b4a919b4438f394
owner: released
created_at: 2026-08-06T09:50:00+02:00
completed_at: 2026-08-06T10:47:00+02:00
cross_repository_coordination_id: OTV2-RUST-CLIENT-CUTOVER-20260806
public_contracts:
  - docs/migration/VSL-02_RUST_CLIENT_MIGRATION_AND_CUTOVER_CONTRACT.md
  - docs/migration/VSL-02_SOURCE_RECONCILIATION.md
  - docs/migration/rust-client-path-map.json
  - docs/migration/rust-client-provenance-plan.json
next_ordered_action:
  - create one atomic destination migration task on branch migrate/rust-client-cutover-c923ad8
```

## Outcome

VSL-02 was completed and squash-merged. It pins the exact source commit and subtree tree, defines all 26 source-member dispositions, fixes the atomic 19-member destination package, truthful provenance, exact dependency policy, source freeze/marker procedure and single-writer rollback.

No Rust code, Cargo workspace or external repository was changed by this task.

## Canonical source selection

```yaml
source_repository: blakinio/otclient
source_commit: c923ad8a1dff17b4933a6110931b0823cec2c590
source_subtree: oteryn-client/
source_subtree_tree: c0928dafca6df19ff11d7901e503ed85a5199439
source_manifest_blob: 037013e8e4a762a65f0f2a30f7761ee14725a3fc
source_lockfile_blob: 2143408c12c50132883890f0821278320a331fde
```

The implementation must import only from the selected commit. Source `main` may advance outside the Rust subtree; preflight compares the current subtree tree to the pinned tree.

## Accepted migration model

- one atomic destination PR creates exactly the FND-01 19-member workspace;
- no Canary, source protocol-core, gameplay transport, Game Session or empty native protocol enters the initial workspace;
- production client and synthetic harness have separate closures;
- Tokio `1.51.4` remains pinned;
- reqwest `0.13.4` with explicit rustls/form/json/stream replaces blocking `ureq`;
- provenance is machine-readable and makes no false ancestry claim;
- the nested source dual/native task is superseded reference-only;
- the obsolete native correspondence workflow is removed only in the later authorized source-marker PR;
- rollback places destination under a non-writable hold before restoring source write authority.

## Source-marker boundary

No source write was authorized or performed by VSL-02.

After verified destination implementation merge, a separate authorized source task must:

- mark the Rust subtree moved/non-canonical;
- update source README and nested AGENTS;
- create the moved/provenance record;
- archive the Canary task as reference-only;
- archive the nested native task as superseded reference-only;
- mark native correspondence superseded;
- delete the obsolete native correspondence workflow;
- preserve source code/history and unrelated legacy work.

## Validation

### Focused

- exact source commit/tree and live source-subtree reconciliation: `PASS`
- open PR/root task/nested task/workflow classification: `PASS`
- 26-member path/disposition and non-member audit: `PASS`
- provenance/dependency/release-closure consistency: `PASS`
- rollout and single-writer rollback audit: `PASS`
- unresolved material findings: `0`
- unresolved review threads: `0`

### Component/integration/E2E

`NOT_APPLICABLE` — contract-only task; no executable product change.

### Exact-head GitHub checks

```yaml
validated_head: 6cdc94aa7c711f68a6e5792d12aaefc3de7dcd54
agent_governance:
  run: 31084815761
  result: PASS
dependency_review:
  run: 31084815714
  result: PASS
codeql:
  run: 31084815796
  result: PASS
```

## Independent audit

- method: adversarial source-to-target audit against FND-01, ADR-0002, ADR-0008, ADR-0011, exact source tree, nested programme/workflow, provenance truthfulness, dependency closure and canonical rollback safety;
- resolved findings: subtree-vs-whole-main boundary, omitted nested task, false reference-only test destination, native-tls wording, rollback hold, obsolete workflow and closed disposition vocabulary;
- final verdict: `PASS`;
- open findings: none.

## PR and closeout

```yaml
pr: 48
merge_method: squash
merge_commit: 824599bb2d696e15e22319007b4a919b4438f394
changed_files: 5
source_repository_writes: 0
runtime_changes: 0
ownership_release: completed by archive PR
```

## Next action

Create a separate, bounded migration task in `blakinio/Oteryn-v2` for the one atomic destination implementation PR. Do not create a partial workspace and do not write the source marker before the verified destination migration merge.

# OTV2-20260806-fnd-01-mpl-license-correction

```yaml
task_id: OTV2-20260806-fnd-01-mpl-license-correction
title: Correct FND-01 workspace software license to MPL-2.0
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/fnd-01-mpl-license-correction
pr: 51
base_sha: 33dc15dab82cfe5347e569036296204763270508
validated_head_sha: 9a8e54a6c2bd75522402b0496fb3604f466d8896
merge_commit: 8c11a2e7bc4b3c66be419326ed8e5ca816d7140d
owner: released
created_at: 2026-08-06T12:02:00+02:00
completed_at: 2026-08-06T12:10:00+02:00
cross_repository_coordination_id: OTV2-RUST-CLIENT-CUTOVER-20260806
public_contracts:
  - docs/architecture/FND-01_OWNER_ACCEPTANCE_AND_CRATE_FITNESS_REVIEW.md
next_ordered_action:
  - PR #50 must use MPL-2.0 workspace metadata before merge
```

## Outcome

The product owner's licensing correction is canonical on `main`.

The FND-01 owner-acceptance record now explicitly supersedes the accepted contract blob's `license = MIT` line with:

```text
license = MPL-2.0
```

The correction preserves the accepted 19-member workspace, migration dispositions, dependency boundaries, toolchain, release roles and cutover sequencing.

Imported or third-party material retains separately applicable compatible licenses, attribution and provenance through explicit file- or component-level notices. Creative assets remain governed separately.

## Related implementation hold

Draft PR #50 was notified that root and inherited Cargo workspace metadata must use `MPL-2.0` before merge. This contract task made no Rust, Cargo, migration implementation, external-repository or production change.

## Validation

### Focused

- changed paths: exactly the FND-01 owner-acceptance record and bounded active task record;
- architecture/licensing consistency review: `PASS`;
- Rust/Cargo/runtime changes: `0`;
- unresolved material findings: `0`;
- unresolved review threads: `0`.

### Component/integration/E2E

`NOT_APPLICABLE` — documentation-only contract correction.

### Exact-head GitHub checks

```yaml
validated_head: 9a8e54a6c2bd75522402b0496fb3604f466d8896
agent_governance:
  run: 31091860210
  result: PASS
dependency_review:
  run: 31091859703
  result: PASS
codeql:
  run: 31091859717
  result: PASS
```

## Independent audit

- method: adversarial consistency review against `docs/repository/LICENSING.md`, accepted FND-01 scope, VSL-02 migration boundaries and PR #50 Cargo metadata;
- verdict: `PASS`;
- open findings: none.

## PR and closeout

```yaml
pr: 51
merge_method: squash
merge_commit: 8c11a2e7bc4b3c66be419326ed8e5ca816d7140d
changed_files: 2
runtime_changes: 0
external_repository_writes: 0
ownership_release: completed by this archive package
```

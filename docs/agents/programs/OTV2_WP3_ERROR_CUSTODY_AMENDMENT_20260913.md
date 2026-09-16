# WP3 exact SQLx error custody amendment

```yaml
allocation_id: OTV2-WP3-ERROR-CUSTODY-20260913
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 448309c4042e3fb783d1ab9d7bb324f7ee977420
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-error-custody-scope-351
worker_branch: agent/sqlx-driver-budget-351
issue: 351
pr: 356
source_wp3_head: f56896334d5c6b5d6e6419c21c56092b56b8f28e
frozen_inventory: issue_162_comment_5655890678
existing_boundary_disposition: issue_162_comment_5656188739
owner_handoff: issue_162_comment_5656281533
risk: HIGH
```

This prospective amendment adds no writer, material cell, budget or present
source authority. It is prepared on the already-existing coordinator branch.
The SAME canonical M02/M04 batch continues under its existing grants.

## Exact decision and new source scope

Candidate decision:
`docs/architecture/reviews/OTERYN_GAME_WP3_ERROR_CUSTODY_DECISION_2026-09-13.md`.
That decision contains the representation/API compatibility choice, source
basis, alternatives, risks and required proof. It remains NOT ACCEPTED until
its acceptance gate is recorded.

After protected integration/readback, decision acceptance and explicit Work
activation, add only `vendor/sqlx-core-0.9.0/src/error.rs` to the imported-file
accounting allowlist, restricted to:

- `Error::Owned`, `OwnedError::{Database,Tls,Configuration}` and inline typed
  resource-denial form;
- private-backed `ErrorCustody<T>` construction, immutable access and destruction;
- `Error::{as_database_error,into_database_error}` compatibility;
- custody-preserving consuming database/standard-error conversion/downcast;
- transparent Display/StdError/source delegation and colocated focused tests.

Preserve legacy raw variants/constructors and DatabaseError trait methods.
No mutable/raw/naked charged extraction or early charge detachment is authorized.
Final complete Box deallocation must precede its external reservation release.
The public extraction return-type change is deliberate and must receive the
independent API/resource review required by the decision.

Already-admitted PostgreSQL producer/classification paths, SQLx TLS bridges,
exact existing test surfaces and both provenance files may integrate this
representation within their current grants. This is not permission to edit
other core files, dependency APIs, arbitrary consumer matches or new test
support paths. If an additional necessary symbol is outside current grants,
stop only that exact boundary and return its source-backed minimum scope.

## Serialization and validation

Work retains control-plane ownership of this four-document preparation package:
this amendment, the candidate decision, the existing #351 task record and its
single implementation plan. The material worker must preserve those documents
when the protected amendment is later normally merged forward. This adds no
concurrent source writer and does not reset admissions, evidence or custody.

Require the decision's complete focused lifetime/API/denial/compatibility proof,
ordinary owner-free controls, explicit affected vendored tests, configured
PostgreSQL 17.6 and required strict checks on the eventual implementation head.
The docs package itself proves no source implementation or test PASS.

## Exclusions and activation gate

No generic pool retry, rustls state/outbound/provider, Tokio, Cargo/lock,
workflow/audit pin, registry, Game consumer, production, secrets or external
repository write follows. M03 descendant bounds and M05 normalization/retry
remain separate frozen obligations. Do not weaken SQLSTATE/classification or
hide a real database error to bypass custody.

Lifecycle: independent exact-candidate HIGH review -> exact-head repository
checks -> authorized native Merge Queue -> real merge_group game-gate ->
protected-main readback -> Work decision acceptance and fresh exclusive scope
activation for SAME #351/#356 -> focused implementation/qualification -> final
WP3 review/integration and explicit release. A protected document alone does
not activate source. Until then `src/error.rs` remains unchanged.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`

# WP5 dedicated PostgreSQL target canonical-CI routing allocation

Coordinator: #162. Programme: #364. Source readiness: #319.

## Status

Prospective control-plane allocation only. **NOT_ACTIVE**. No workflow, test,
runtime, SQL, ruleset or protected-audit write lease follows from this document.
It exists because the current canonical PostgreSQL jobs execute only
`durability_postgres`; future dedicated WP5 targets must not be able to merge
without actual PostgreSQL 17.6 execution.

```yaml
allocation_id: OTV2-WP5-DEDICATED-POSTGRES-CI-ROUTING-20260907
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
programme_issue: 364
source_issue: 319
allocation_base_main_sha: a28cb5ed55eaad53f91ad86eee426d9a1ce91b0f
allocation_state: NOT_ACTIVE
required_status_name_change: false
merge_queue_semantics_change: false
ruleset_change: false
protected_pin_rotation_required: true
```

## Fresh protected fact

Current `.github/workflows/merge-gate.yml`, `.github/workflows/merge-group-gate.yml`
and `.github/workflows/rust.yml` configure PostgreSQL 17.6 credentials but invoke
only:

```text
cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres
```

The future WP5 producer allocations intentionally use dedicated targets:

```text
apps/game-server/tests/character_authority_postgres.rs
apps/game-server/tests/runtime_scope_assignment_postgres.rs
```

Without a protected routing change, those mandatory SQL/locking/restart tests can
exist yet never execute in the required PR/MQ PostgreSQL layers. A workspace test
without `OTERYN_TEST_POSTGRES_ADMIN_URL` is not PostgreSQL evidence and may report
NOT_APPLICABLE.

## Exact future control-plane surfaces

After a later explicit Work application, one serialized control-plane writer may
modify only the minimum required set:

```text
.github/workflows/merge-gate.yml
.github/workflows/merge-group-gate.yml
.github/workflows/rust.yml
.github/workflows/merge-authority-audit.yml        # separate pin rotation only

tools/repository/validate_pr_gate_pg_sim.py
tools/repository/test_validate_pr_gate_pg_sim.py
tools/repository/test_validate_merge_group_pg_sim.py
# plus one new small protected-base target-routing helper/regression only if
# direct fixed-target wiring cannot prove deletion/rename fail-closed behavior.
```

The merge-authority audit path is not part of the gate candidate itself. As in
WP1, compute the exact future `merge-group-gate.yml` Git blob first, then rotate
the protected audit pin in a separate reviewed/protected PR before activating the
new MQ gate blob.

No new required status, renamed `game-gate`, reduced MQ job set, paths-only bypass,
permission expansion, `continue-on-error`, direct merge or ruleset change is
allowed.

## Target routing contract

Initial registered dedicated targets are exactly:

```text
character_authority_postgres
runtime_scope_assignment_postgres
```

The implementation must not create a generic repository-controlled list that an
untrusted PR can edit to suppress required tests. Target identity is protected-base
policy until a later reviewed amendment.

For each target:

1. if the exact test file exists in the candidate, the canonical PostgreSQL job
   executes it with the same isolated PostgreSQL 17.6 admin harness used for
   `durability_postgres`;
2. if the target existed on the trusted base and the candidate deletes, renames or
   makes it undiscoverable, the required gate fails closed rather than silently
   skipping it;
3. if the target has not yet been introduced on either trusted base or candidate,
   pre-implementation unrelated PRs do not fail merely because the future target
   is absent;
4. after its first protected introduction, absence on later candidates is treated
   as removal and fails until a separately reviewed retirement amendment exists;
5. exact target execution outcome must feed the existing PostgreSQL layer and
   therefore the existing sole `game-gate` fan-in.

The PR gate may keep trusted impact routing, but any change to Character Authority
or runtime-scope-assignment production/migration/test surfaces that can affect a
registered target must select PostgreSQL qualification. Unknown/incomplete
classification remains FULL/fail-safe.

Merge Queue remains full qualification: every registered present WP5 PostgreSQL
target runs in the MQ PostgreSQL job. The push/post-merge Rust workflow also runs
registered present targets so protected-main evidence does not regress to the old
single-target harness.

## TDD and negative controls

Before workflow mutation, add a test-only RED proving current protected workflow
text does not execute `character_authority_postgres` and
`runtime_scope_assignment_postgres` and would silently skip their deletion.
A YAML parse/metadata error is not accepted as RED.

GREEN regression proof must include at least:

- valid control with only legacy `durability_postgres` before WP5 target exists;
- candidate introduces Character Authority target -> required PG command present;
- candidate introduces runtime assignment target -> required PG command present;
- both targets present -> both execute exactly once in the PG layer;
- previously protected target deletion/rename -> required failure;
- changed Character Authority migration/source with present target -> PG lane selected;
- changed assignment migration/source with present target -> PG lane selected;
- unknown/incomplete file enumeration -> FULL qualification;
- target failure -> final `game-gate` cannot pass;
- target success does not mask legacy `durability_postgres` failure and vice versa;
- no PR-controlled manifest may remove/rename a protected target;
- exact future MQ blob must fail protected audit until the pin is separately
  rotated, then pass protected-base audit after pin integration.

Hosted proof must show the new test targets actually execute with
`OTERYN_TEST_POSTGRES_ADMIN_URL` and a real PostgreSQL 17.6 service; command
presence or `--no-run` is insufficient.

## Activation order

```text
this allocation protected
-> material WP5 target implementation exists on a held branch
-> Work refreshes exact target paths and control-plane custody
-> test-only RED on one serialized CI branch
-> minimal PR/MQ/push routing GREEN
-> exact future merge-group gate blob computed
-> independent review of material control-plane candidate
-> separate protected merge-authority pin rotation + readback
-> fresh protected-base audit of gate candidate
-> canonical PR checks
-> normal FULL Merge Queue
-> protected readback
-> only then a WP5 implementation may use dedicated-target PG evidence as
   canonical acceptance evidence
```

The control-plane change may be implemented before both WP5 producers are
complete, but it must not be activated with a target name/path different from the
actual protected implementation contract without a reviewed amendment.

## Excluded scope

No product/runtime/SQL/migration source, target test contents, registry resource
values, Platform/external repo, production DB, secrets, ruleset, required-status
name, queue semantics, workflow permissions, unrelated CI optimization or test
suppression.

Runtime product E2E is NOT_APPLICABLE to this allocation document. Material
workflow activation is CONTROL risk and requires independent review, negative
gate tests, protected pin rotation, full MQ and readback.

# FND-01 Owner Acceptance and Crate Fitness Review

- Status: Accepted
- Accepted by: product owner
- Acceptance date: 2026-08-06
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Accepted contract path: `docs/architecture/FND-01_WORKSPACE_AND_RUST_MIGRATION_CONTRACT.md`
- Accepted contract blob: `b0127a91f201d4cba766053dc74517fe5cb49268`
- Accepted source inventory path: `docs/architecture/FND-01_RUST_SOURCE_INVENTORY.md`
- Source inventory revision: `blakinio/otclient@c923ad8a1dff17b4933a6110931b0823cec2c590`

## Decision

The product owner explicitly accepts the complete `FND-01` contract identified above, including:

- the 19-member initial destination workspace;
- every source-member and non-member migration disposition;
- the separation of the production pre-native client from the non-release synthetic harness;
- permanent Canary isolation under ADR-0008;
- deferral of native protocol/transport to `FND-02` and Game Session/admission to `FND-04`;
- provisional identifier renaming before `FND-ID-01`;
- asynchronous, cancellation-safe Platform and Identity I/O;
- the dependency, release-role, toolchain, target, CI and machine-enforcement policies.

This record changes the accepted contract status from candidate to owner-approved without authorizing runtime implementation outside the ordered `VSL-02` migration/cutover gate.

Where the accepted contract blob still uses pre-acceptance phrases such as “candidate”, “proposed” or “ready for owner acceptance”, this owner-acceptance record is the canonical status authority. The technical graph, dispositions and boundaries remain exactly those audited in the accepted blob except for the additional review obligation below.

## Binding software-license correction

The product owner explicitly corrected the FND-01 root Cargo licensing metadata on 2026-08-06.

The accepted contract blob's line:

```text
license = MIT
```

is superseded and must not be implemented. The canonical workspace package metadata is:

```text
license = MPL-2.0
```

This correction aligns FND-01 with `docs/repository/LICENSING.md`, which remains the repository-wide licensing authority for newly authored software, scripts, build tooling, configuration, schemas, tests and technical documentation.

Imported or third-party material retains any separately applicable compatible license, attribution and provenance through explicit file-level or component-level notices. Such exceptions do not change the default Oteryn-v2 workspace license and must not be used to relicense third-party material or creative assets.

The correction changes licensing metadata only. It does not reopen or modify the accepted 19-member workspace, source dispositions, dependency boundaries, toolchain, release roles, migration sequencing or crate-fitness obligations.

The atomic destination migration PR must use `MPL-2.0` in root workspace metadata and inherited package metadata before it may merge. A destination head containing `workspace.package.license = "MIT"` does not satisfy FND-01.

## Mandatory post-slice crate fitness review

The number and boundaries of the initial 19 members are an accepted migration starting point, not a permanent crate-count target.

A dedicated architecture review is mandatory after the first complete native vertical slice has real client-to-server evidence and before `VSL-01` may be declared complete. The review must evaluate every workspace member against actual consumers, change coupling, dependency weight, target isolation, test ownership and trust boundaries.

A crate remains separate only when at least one material reason is proven:

- a distinct trust or security boundary;
- target, platform, process, executable, FFI or release-role isolation;
- isolation of a heavy or risky dependency graph;
- a stable public contract with at least two immediate consumers;
- independent lifecycle, ownership or testability that would be degraded by merging.

A crate must be considered for merge when it:

- owns only a trivial type or convenience wrapper;
- has one consumer and no meaningful trust, target or dependency boundary;
- always changes with another crate;
- mirrors an aspirational architecture layer rather than an observed responsibility;
- increases coordination or compile cost without measurable isolation benefit.

A crate must be considered for split when it:

- mixes pure values/contracts with I/O or runtime ownership;
- leaks target-specific dependencies downward;
- combines production and synthetic/test responsibilities;
- mixes client projection with authoritative server state;
- accumulates unrelated responsibilities or incompatible release roles.

The review must produce one disposition for every then-current member: `KEEP`, `MERGE`, `SPLIT`, `RENAME` or `REMOVE`, with named evidence and a machine-policy update. It must not create empty placeholder crates.

## Consequences

- `FND-01` is accepted and unblocks preparation of `VSL-02`.
- No Cargo workspace or client code is implemented by this decision-only package.
- PR #46 may be marked ready and merged after exact-head checks pass and the branch is current with `main`.
- The active task may be archived after the verified merge.

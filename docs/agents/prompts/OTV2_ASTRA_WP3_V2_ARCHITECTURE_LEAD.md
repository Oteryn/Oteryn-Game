# OTV2 WP3-v2 Architecture Lead

Short invocation:

```text
Oteryn: astra wp3-v2 architecture lead
```

## Outcome

Produce one coherent superseding WP3-v2 architecture decision that unblocks the real Child B consumer with the smallest maintainable design satisfying accepted durability/resource/security constraints.

Terminal worker outcome: `WP3_V2_ARCHITECTURE_READY_FOR_ACCEPTANCE`.

## Authority / scope delta

- Repository: `Oteryn/Oteryn-Game`.
- Architecture/docs only. No runtime, vendor, Cargo, SQL migration, workflow, Platform or production mutation.
- Live locators: #162, #364, #351/#356, #329/#335, #588.
- Read the current Q01-Q75 package, current accepted DFR resource authority, architecture-decision discipline and `OTV2_WP3_GAME_PLATFORM_CROSS_REPO_AUDIT_R01_R21_20260912.md`.
- Treat #588 as evidence, not architecture acceptance.

## Decision that must be made

Compare at minimum:

A. broad SQLx/rustls/Tokio ownership instrumentation;
B. minimal SQLx seam plus one finite root/phase budget around a bounded PgPool profile;
C. one or two explicit bounded PostgreSQL connection actors instead of relying on the general PgPool lifecycle for DFR.

Do not select C merely because it looks simpler. Do not preserve A merely because work already exists. Evaluate real implementation/maintenance cost, retry/background lifecycle, connection establishment, cancellation/finality, TLS/runtime residency, upgrade burden, operational behavior and proof strength.

The decision must explicitly answer the architecture discipline questions: must decide now, what downstream work is blocked, what becomes harder later, what evidence would justify supersession, and what is deliberately not decided.

## Mandatory design outputs

Define:

- one logical root/executor ownership model;
- queue/active ownership and exact release/finality semantics;
- PgPool policy or connection-actor lifecycle, including connect/replacement/retirement/close;
- bounded configuration/credential sources and lifetime;
- PostgreSQL startup/authentication profile;
- frozen TLS/runtime/provider profile sufficient for later proof;
- cancellation/rollback/ambiguous COMMIT/reconciliation ownership;
- restart/takeover and predecessor-work fencing;
- how Q01-Q75 can actually be qualified on the final consumer;
- precise disposition of #356 artifacts: `RETAIN`, `SUPERSEDE`, `HISTORICAL_EVIDENCE_ONLY` or later-removable after replacement proof.

Use A2's exact evidence rather than assumptions. Where proof is missing, write `UNKNOWN`/`BLOCKING_EVIDENCE_GAP` instead of fabricating a bound.

## Acceptance

The proposed option must preserve PostgreSQL semantics, TLS/hostname security, accepted DFR ceilings/deadlines/custody, finite provable ownership and upgradeability while minimizing unnecessary long-lived dependency forks. Record realistic rejected alternatives and explicit future impact.

Do not claim the architecture is `ACCEPTED` from this worker. Delivery ends with an exact candidate decision ready for repository acceptance.
# Oteryn Game — WP3 TLS external-subresource accounting amendment

- Amendment ID: `WP3_TLS_EXTERNAL_SUBRESOURCE_ACCOUNTING/v1`
- Date: 2026-09-21
- Status: **ACCEPTED**
- Owner decision: Issue #162 comment `5759734485`
- Amends: `WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1`
- Implementation lineage: Issue #351 / PR #673

## 1. Scope of supersession

This amendment supersedes **only** the WP3-v2 clauses that require byte-perfect,
same-ledger accounting of dependency-internal socket/TLS backing when the pinned
upstream APIs expose no owner-aware allocation seam. It does not supersede the
12 MiB root limit, any Oteryn-controlled allocation charge, connection topology,
lifetime/finality rules, hostile-input bounds, TLS verification, or deadlines.

In particular, Oteryn will use upstream rustls 0.23.45 as published. This
decision does not authorize a rustls patch, fork, allocator hook, or vendored
rustls source tree.

## 2. Controlled same-root ledger

The existing 12 MiB same-root ledger remains a hard bound for every charged
allocation controlled by Oteryn, including Game-owned and owner-aware SQLx/Tokio
backing. The byte-perfect equation is narrowed to that controlled graph:

```text
I_controlled + max(R_controlled, T_controlled) + Q + A <= 12 MiB
```

All existing prospective reservation, checked arithmetic, custody-transfer,
release, and fail-closed requirements continue to apply to this controlled
graph. No allocation may be removed from the controlled graph merely because it
is inconvenient to measure.

## 3. External library-owned T subresource

The dependency-internal backing created by the operating-system/reactor and by
upstream rustls provider, configuration, trust store, `ServerName`,
`ClientConnection`, handshake, and deframer implementation is one external,
library-owned subresource of `T` where the pinned upstream public APIs provide no
owner-aware allocation seam. It is not assigned a guessed byte charge and is
not debited from the controlled same-root ledger.

This is an accounting-boundary decision, not an ownership or lifetime escape.
The external subresource remains bounded by the already-frozen production
topology and lifecycle:

- one process root;
- `max_connections = 1` and `min_connections = 0`;
- serialized root maintenance;
- no overlapping live `T` generation;
- the existing five-second recovery/connect window;
- `VerifyFull` with a separate bounded DNS TLS identity;
- TLS 1.3 only with AWS-LC;
- no session resumption;
- no client certificate; and
- bounded retained configuration and trust input.

The external subresource begins and ends with its owning connection-establishment
generation. Failure, cancellation, timeout, successful `T` to `R` transition,
and retirement retain the existing exact-generation finality rules. No new `T`
may overlap a still-live external subresource from the prior generation.

## 4. Preserved requirements

This amendment creates neither a second ledger nor a second budget. It does not
authorize an opaque TLS precharge, a resource-limit increase, post-allocation
catch-up for controlled backing, or a weaker interpretation of the 12 MiB
controlled bound.

The following remain unchanged and mandatory:

- PostgreSQL frame, row, metadata, cache, and diagnostic hostile-input guards;
- bounded configuration and trust inputs;
- `VerifyFull`, TLS 1.3-only, AWS-LC, no-resumption, and no-client-certificate
  policy;
- ready-only active checkout and max-one physical holder;
- `R`/`T` non-overlap, hard holder retirement, and exact-generation finality;
- the five-second recovery/connect window and all semantic-pass deadlines; and
- ordinary non-WP3 SQLx behavior.

The same-root `ResourceBudget` therefore attaches at the first
Oteryn-controlled buffered-socket retention after upstream connection/TLS
construction. From that point onward, all Oteryn-controlled socket buffering,
protocol backing, authentication, query, queue, and active-work allocations
remain prospectively charged under the controlled equation.

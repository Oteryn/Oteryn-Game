# ADR-0013: Platform database technology independence from game persistence

- Status: Accepted
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Applies to: `blakinio/Oteryn-v2` architecture and future coordinated Platform persistence planning
- Supersedes: only the ADR-0004 requirement that Oteryn Platform production persistence must ultimately migrate to PostgreSQL
- Preserves: ADR-0004 game PostgreSQL decision, ownership separation, credential separation, migration separation and no-shared-writer rules

## Context

ADR-0004 correctly selected PostgreSQL for the new authoritative Oteryn-v2 game persistence and correctly separated Platform-owned data from game-owned data. It additionally declared PostgreSQL as the target production database for Oteryn Platform and required a future Platform PostgreSQL migration.

A later architecture review found that the ownership and security goals do not require both bounded contexts to use the same database technology. Forcing a migration of a functioning external Platform only for technology uniformity can create substantial migration, regression and operational cost without improving the gameplay runtime or the semantic separation between Platform and game data.

The owner accepted this refinement on 2026-08-07.

## Decision

### 1. PostgreSQL remains mandatory for native game persistence

ADR-0004 remains binding for `oteryn_game` and future native authoritative gameplay persistence.

The native game domain continues to target PostgreSQL for transactional character, item, economy, lease, checkpoint, outbox and audit state subject to the later `DUR-*` contracts.

### 2. Platform database technology is independently owned

Oteryn-v2 does not require Oteryn Platform to migrate to PostgreSQL merely because the game domain uses PostgreSQL.

Platform database technology remains an Oteryn-Platform-owned architecture choice subject to its own requirements, evidence, migrations and rollback policy.

A Platform PostgreSQL migration may still be selected later when a dedicated Platform decision demonstrates a material benefit such as:

- stronger correctness or concurrency behavior needed by Platform workloads;
- measurable operational simplification;
- security/isolation improvement;
- maintenance or support reduction;
- retirement of a costly compatibility constraint;
- a business/availability requirement that justifies the migration risk.

Language or database-stack uniformity alone is not sufficient justification.

### 3. Semantic separation is technology-independent

The following ADR-0004 invariants remain unchanged regardless of Platform database engine:

- Platform and game have distinct semantic owners;
- they use separate runtime credentials and migration authority;
- there is no unrestricted shared-table writer model;
- game code cannot directly mutate Platform Identity/wallet/payment data;
- Platform cannot bypass game invariants with arbitrary direct SQL writes;
- cross-boundary mutation uses explicit APIs/contracts, operation-specific migration authority or asynchronous idempotent integration;
- read projections are bounded and least privilege;
- stable cross-system identity is contractual, not a cross-database foreign key assumption.

If Platform and game happen to use PostgreSQL, they may share a physical cluster only when separately accepted operational policy permits it. The architecture must also permit separate clusters and heterogeneous database technologies without changing semantic ownership.

### 4. Cross-repository work remains explicit

This ADR changes only the Oteryn-v2 requirement imposed on future Platform persistence planning.

It does not modify `blakinio/Oteryn-Platform`, migrate data, change credentials or select a new Platform database. Any Platform-side database change requires an explicitly authorized Platform task/ADR/PR and its own migration, validation and rollback evidence.

## Consequences

### Positive

- game persistence can use PostgreSQL without forcing unrelated Platform migration work;
- bounded-context ownership is preserved independently from storage technology;
- migration risk is paid only when Platform obtains a measurable benefit;
- Platform can evolve on its own operational timeline;
- first native gameplay evidence is not delayed by an unnecessary control-plane database conversion.

### Costs and risks

- production may operate heterogeneous database technologies for some time or permanently;
- operators may need expertise, monitoring and backup procedures for more than one engine;
- cross-system integration must remain API/event/contract driven rather than relying on convenient same-engine joins.

These costs already follow from the accepted ownership separation and are preferable to an unjustified migration mandate.

## Rejected alternatives

### Keep mandatory Platform PostgreSQL migration for uniformity

Rejected. Uniformity alone does not justify the risk and cost of moving an independently owned, functioning control-plane persistence layer.

### Reopen PostgreSQL for the game domain

Rejected. This review found no evidence that ADR-0004's native game PostgreSQL choice should be changed.

### Recombine Platform and game into one shared schema

Rejected. This would violate the ownership, least-privilege and migration-isolation invariants that ADR-0004 continues to enforce.

## Decision timing

- **Must decide now?** `YES` for architecture planning, because the previous mandate could incorrectly place a Platform database migration on the critical path of native gameplay work.
- **Blocked work clarified:** foundation/vertical-slice planning can proceed without treating Platform PostgreSQL migration as a prerequisite.
- **Harder later:** forcing shared technology would increase cross-repository migration coupling and can make rollback more expensive.
- **Evidence to supersede:** a future Platform-owned ADR with measured technical/operational/product evidence proving PostgreSQL migration is worthwhile.
- **Deliberately undecided:** current/final Platform database engine, migration timing, physical topology, exact backup technology and Platform implementation plan.

## Acceptance invariant

Future work complies with this ADR when:

> PostgreSQL remains the native Oteryn-v2 game persistence target, while Oteryn Platform chooses its database technology independently and is migrated only through a separately justified, owner-authorized Platform decision rather than through a cross-project uniformity mandate.

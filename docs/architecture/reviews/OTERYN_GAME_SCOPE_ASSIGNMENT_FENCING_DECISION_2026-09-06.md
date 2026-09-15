# First-slice scope assignment and fencing

- Decision: `OPS-SCOPE-ASSIGNMENT-FENCING-V1`
- Status: **CANDIDATE; acceptance requires independent review and protected integration**
- Source: Issue #328, source inventory #319, coordinator #162
- Base: Game `93f31ba05972d3b96afb0d9ea08e2c6753507d8c`
- `MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`

## Resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
main_sha: 93f31ba05972d3b96afb0d9ea08e2c6753507d8c
source_escalation: 328
blocking_question: Which minimum actual assignment authority and durable fence may activate the first native runtime scope?
facts:
  proven:
    - FND03 section 4.2 prohibits NodeRuntime self-grant and delegates physical assignment/fencing to OPS-CHANNEL-01
    - ScopeRuntimeFence consumes a supplied grant but is not an assignment source
    - ADR0009 fixes one GameNode per process; ADR0015 leaves internal and adjacent-service shape open
  derived:
    - readiness requires an actual durable assignment source and enforcement shared with affected writers
  unknown:
    - authorized deployment identities, bootstrap records, production database state and operational availability
  conflict: []
accepted_decision: OPS-SCOPE-ASSIGNMENT-FENCING-V1, conditional on protected integration
rejected_options: [NodeId_or_config_as_grant, process_local_generation, expiring_lease_without_clock_policy, readiness_only_fence, full_autoscaling_prerequisite]
affected_contracts: [FND-03, OPS-CHANNEL-01, FND-DUR-FRESH-ADMISSION-V1]
affected_paths:
  - docs/architecture/reviews/OTERYN_GAME_SCOPE_ASSIGNMENT_FENCING_DECISION_2026-09-06.md
  - docs/agents/tasks/active/OTV2-20260906-native-source-contracts-328.md
implementation_owner: Game scope-assignment authority and Durability enforcement, with distinct least-privilege runtime consumer
implementation_scope: durable manual first-slice assignment/revoke/replace, grant consumption, readiness and fenced writer integration
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
supersedes: [only_first_slice_physical_assignment_and_fencing_deferral_in_FND03_section_4_2]
required_validation: split_owner_restart_revocation_writer_matrix_below
required_independent_review: exact-head ownership/durable fencing review
next_action: Work independently qualifies this candidate through protected integration before allocating its implementation.
```

## Decision timing and boundary

**Must decide now: YES.** Actual runtime readiness feeding Child C and Server Seam cannot be fabricated from a `NodeId`, configured channel list or locally advanced counter. The concrete blocking work is ownership-grant production and generation-fenced durable/runtime activation. Incorrect placement of the fence would let a stale process publish readiness or mutate durable state after replacement.

Select a small **Game-owned durable assignment authority using PostgreSQL compare-and-set**, outside the receiving GameNode's authority. This is a logical/security boundary, not a mandate for a new daemon, service, orchestrator or crate. An authorized control actor may invoke the bounded assignment operation; a GameNode runtime credential may only consume grants and perform separately fenced runtime work. Static desired placement is merely intent submitted to the authority, never an accepted assignment.

Costs: database availability gates replacement and safe activation; every writer must share the fence. Benefits: durable single-winner assignment/restart without introducing heartbeat intervals, expiring leases or autoscaling technology. Players retain the accepted same-scope recovery semantics; this does not silently move players or define recovery grace periods.

Supersession requires exact failure/performance evidence showing an alternative preserves external ownership, monotonic recovery and every consumer fence, with explicit compatibility/migration. Not decided: orchestration product, process/container topology, automated placement, lease duration, failure detector, RPO/RTO, autoscaling, live migration, capacity/resource numbers, or production deployment. ADR0009's one-process GameNode and ADR0015 remain binding. Completing all OPS-CHANNEL-01 is not a prerequisite for this bounded implementation/proof.

## Durable source state and commands

Use the same PostgreSQL serialization domain as admission guards and affected durable writers. The assignment authority owns one durable record per existing typed `RuntimeScopeRefV1`; first-slice activation is limited to already supported Channel scopes. This does not allocate Instance runtime behavior or collapse `WorldId` and `ChannelId`.

The record contains typed scope, never-reused positive `ownership_generation`, optional current `NodeId`, closed state `ASSIGNED | REVOKED`, comparable positive source revision/decision identity, accepted operation identity and publication binding. `ASSIGNED` requires exactly one holder; `REVOKED` requires none. Absence is distinct from an authoritative revoked record. Persistent generation/source high-water marks and operation receipts survive revoke/restart and are never deleted to reinitialize a scope.

The closed first-slice command family is:

| Operation | Preconditions and effect |
|---|---|
| Initial assign | Independently authorized fresh-scope bootstrap, authenticated control actor, exact fresh-store absence/high-water proof, valid authenticated registered target process-incarnation identity; atomically establish first positive generation and holder. |
| Replace | Exact prior scope/generation/source/publication CAS plus authorized target process identity; atomically advance generation, change holder and fence old readiness. |
| Revoke | Exact prior scope/generation/source/publication CAS and authorized control actor; atomically advance generation, clear holder and fence readiness. |
| Read/reconcile | Authenticated authorized consumer obtains exact accepted assignment/operation result; it cannot create or advance authority. |

Generation and source revision use checked monotonic successors; overflow rejects. An operation identity is the authority-scoped opaque idempotency key bound to the complete command. Its exact typed representation and accepted finite input bounds are selected in the implementation allocation from existing identity/resource contracts; it cannot be a GameNode self-issued authority credential. Same identity and exact command returns the original committed receipt without incrementing again; changed command conflicts. Absence of a receipt after an ambiguous response is not permission to assume abort without authoritative reconciliation.

Only the assignment writer may allocate the source revision and decision identity. Its typed owning-source publication describes the exact conditional transition; the SQL persistence adapter enforces and stores it, rather than impersonating an authenticated external source. No remote Platform metadata is altered. The source authority is registered separately from RuntimeScope grant consumption. A control actor must be independently authenticated and authorized for the exact target scope and operation; merely possessing a node name, configuration file or network access is insufficient.

## Physical atomicity and least privilege

Forward migration introduces assignment records, immutable operation receipts/high-water preservation and database privileges restricting assignment mutation to the dedicated assignment-writer role. Ordinary GameNode/runtime and admission adapter roles cannot INSERT/UPDATE/DELETE assignment rows or bypass the owner operation. The migration role is not a runtime credential. Exact SQL names and migration number are chosen after fresh schema/allocation readback; released migrations are immutable.

Assignment commands lock the scope assignment and matching Runtime guard in a consistent global order shared with fresh admission and all affected writers. Initial assignment, replacement or revocation atomically persists its source decision/receipt and the corresponding Runtime guard fence with `ready = false`. A record may be assigned while the runtime has not completed activation; that is deliberately nonready. If the Runtime guard has no independently owned route/revision/bootstrap facts, assignment must not fabricate a full guard: the same transaction records a durable closed admission fence, and later guard initialization must prove that exact current assignment before readiness. Missing guards already keep admission closed.

Runtime guard revision metadata is produced by the registered Game runtime publication authority under its existing typed CAS; assignment mutations cannot fabricate unrelated route/rules/content/map/offer facts. The implementation must couple this owner-authored fence transition to the assignment transaction, including monotonic Game source/publication updates. Standalone assignment updates which leave an existing ready guard intact are forbidden. Standalone runtime publication which restores an older assignment is likewise forbidden.

All corresponding effects become authoritative only on durable COMMIT. Rollback creates neither an accepted generation nor active readiness. The authority reconciles ambiguous results from its immutable receipt and current assignment. Future source snapshots and publishers resolve committed state; they never publish a tentative generation or reset a namespace after restart. Multiple authorized control actors racing against one predecessor produce one successful CAS; losers reread and require a newly authorized decision rather than overwriting the winner.

## Every writer and readiness consumer

This mechanism is not complete when only admission consults it. Qualification inventories every current scope-authority-consuming API and classifies it as enforcement or fail-closed unsupported scope.

- **Runtime activation:** consume an authenticated committed assignment binding exact scope, `NodeId`, generation and source decision. Verify actual restored/runtime prerequisites and independently owned revision facts before producing a readiness proposal. Readiness publication locks/rechecks the current assignment and Runtime guard atomically; a replaced/revoked holder cannot publish ready.
- **Fresh/reconnect and scope-fenced durable mutation:** acquire current assignment serialization with the other accepted locks, compare scope/NodeId/generation and required assigned/ready state, and hold it through COMMIT. No check-then-unlocked-write. Assignment changes serialize with accepted writes; a write linearized before replacement may finish, while a stale write after it rejects.
- **Owner-local inputs/timers/auxiliary completions:** existing FND-03 generation/ordinal fencing remains mandatory. A newly observed revoke/replacement stops old-generation publication and work; late completions do not become authority through transport callbacks.
- **Externally authoritative publication/output:** every receiver that could accept a scope-authoritative effect validates the current fence through a serialized owner boundary. An old disconnected process may keep running but cannot cause accepted authority effects. A local cache or cooperative shutdown alone cannot prove exclusion. If a consumer cannot enforce the fence, replacement/activation is disabled for that scope until that bypass is closed.
- **Lifecycle:** assigned-but-notready is closed for admission; drain/revoke stops new admission before claiming completion. No automatic lease expiry grants a successor. Existing sessions remain governed by accepted FND-04 recovery/terminal policies, not by a new arbitrary assignment policy.

A runtime process's health and revision availability are observations supplied by its registered owner, distinct from permission to own the scope. The caller identity at each privileged port is bound by authenticated registration to the exact NodeId; a caller-supplied NodeId field or receipt cannot substitute for that binding. Generating a fresh NodeId identifies a process and does not issue its assignment. The assignment authority does not declare loaded content or successful recovery solely by assigning a node. Readiness requires both current committed assignment and actual successful runtime checks, published under the matched fence. Platform runtime-status projections, configured status, directory records and probes do not replace either fact.

## Bootstrap, restart and partition

The operator-controlled bootstrap context establishes which actual control identity is allowed to create the first record for which scope and how the fresh-store/high-water condition is proven. It is independently authenticated configuration/administrative authorization under a later exact provisioning task, not a configuration entry consumed by GameNode to grant itself authority. This decision does not perform that task or claim such material exists.

An existing store restores its assignment/operation high-water records before grant consumption. Missing or regressed state after uncertain restore fails closed until an authorized recovery can establish the prior maximum from retained authoritative evidence. A new process receives a fresh `NodeId`; restart does not inherit ownership by reusing another incarnation's ID. Reassignment follows the normal authority CAS with a higher generation.

Database outage or network partition cannot make a local grant authoritative indefinitely at a receiver that lacks the current fence. No cached assignment alone authorizes a new durable mutation, readiness publication or successor activation. This V1 introduces no heartbeat/timeout algorithm: unresponsive-node replacement is an explicit authorized command, and safety comes from mandatory receiver/database fencing. It makes no bounded recovery-time or uninterrupted-gameplay claim. Disaster recovery of lost trusted high-water state remains a separate authorized operation; zeroing the table is forbidden.

## Alternatives and scoped implementation handoff

Reusing an existing accepted assignment service would be preferable if a compatible actual producer were proven; none is established in #319's inspected sources. This decision selects an implementable Game-owned durable boundary rather than assumes one exists. Expiring distributed leases are deferred because they introduce clock/renewal and failure-detection decisions unnecessary for the next proof. Full orchestration/autoscaling and NodeRuntime self-election are rejected for this slice.

Work must allocate exact disjoint Game assignment semantic/adapter paths, forward migration, privileges and integration tests after protected acceptance. These are prospective surfaces, not a runtime lease. Shared Foundation/Durability/publication changes serialize with #326 and B; no worker seizes their paths. An implementation may use a test-authorized external control actor in an isolated database, but fixtures prove no actual deployment identity or source availability. The actual producer's operational bootstrap and all consumer fences are prerequisites for C readiness and Server Seam release.

Game character/account/world owner implementation remains separate under its accepted contract; neither assignment nor this bootstrap establishes character ownership. No Platform/META/Atlas writes, credentials, production database actions, deployment, B/C allocation or resource-registry mutation is authorized by this candidate.

## Qualification matrix

Use `AuthorityInvariant × ConsumerBoundary × MutationOperator`, with one changed invariant per negative test. Cover unauthorized runtime assignment write; missing/forged bootstrap; wrong scope/NodeId; duplicate exact and altered command replay; concurrent replace/revoke; overflow; old assignment/readiness publication; rollback after each tentative effect; lost COMMIT response; restart/backup rollback; old process after replacement; database partition; delayed normalized completion; and unchanged-session recovery compatibility.

Real PostgreSQL tests must prove assignment-before-writer and writer-before-assignment order at each supported durable consumer, atomic false-readiness publication, stale CAS failure, privilege separation and restart high-water recovery. Runtime/output tests must prove the stale process cannot make any receiver accept authority after replacement; cooperative shutdown alone is insufficient. Include actual readiness prerequisite failures and absent producer/guard fail-closed behavior. Configured integration harness execution is required for database evidence; internal fixtures do not satisfy it by themselves.

Documentation-only validation: governance/whitespace and adversarial self-review, genuinely independent exact-head ownership/fencing review, selected CI and normal protected integration. Runtime/E2E is NOT_APPLICABLE to this candidate, and remains mandatory for implementation acceptance. No success in this document establishes production readiness.

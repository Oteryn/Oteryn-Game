# WP2 GameSession nonreuse allocation amendment

Coordinator: #162. Existing worker: #353 / PR #361. Programme: #364.
Preparation authority: #162 comment `5573685127`, following handoff `5573205030`.

## Status and activation boundary

This is a prospective amendment of the existing Foundation task, not a new
worker, reusable prompt, control plane, architecture decision or runtime lease.
It is **NOT_ACTIVE** until applicable independent review, exact-head repository
checks, normal protected Merge Queue integration and protected-main readback are
complete, followed by an explicit Work application after fresh custody checks.
Publication, green documentation CI and the presence of this file alone do not
start a worker, release a product hold or qualify PR #361.

```yaml
amendment_id: OTV2-WP2-GAMESESSION-NONREUSE-20260907
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
worker_issue: 353
worker_pr: 361
worker_task_id: OTV2-20260906-control-loss-reconnect-bridge-353
worker_branch: refs/heads/agent/control-loss-reconnect-bridge-353
original_admission_main_sha: b61f9d8cc1c0a7289ffdaf1bf4e42b851d2c0f9a
observed_task_head_sha: 176af0eeb76228dcfec575b57f7744a6290ef1e8
allocation_base_main_sha: a793457cf3001df37109acb2c4b4a772b53db97a
amendment_state: NOT_ACTIVE
worker_launch: NOT_STARTED
consumer_activation: FORBIDDEN
owned_paths:
  - apps/game-server/src/foundation/admission_recovery_inner.rs
  - apps/game-server/src/foundation/admission_authority_publication.rs
  - apps/game-server/src/foundation/control_loss_reconnect_bridge_tests.rs
  - docs/agents/tasks/active/OTV2-20260906-control-loss-reconnect-bridge-353.md
  - docs/superpowers/plans/2026-09-06-control-loss-reconnect-bridge.md
prerequisite_merges:
  architecture_383: 6b07f96d47de37971bb54fed5bb9c12decd1be17
  registry_386: 7d59a1169b5c99d4994a904aa566d843cf833a34
  registry_closeout_387: a793457cf3001df37109acb2c4b4a772b53db97a
execution_target: isolated_workspace
validation_target: github_actions
remote_desktop: denied
lane_strategy: single_agent
```

These SHAs are preparation evidence, not a future preflight substitute. Preserve
the immutable original admission, canonical branch, existing implementation and
review history. Re-read the actual task head, protected main and applicable
instructions before application. A later main advance does not authorize a
restart, replacement branch, force push or reset. Reconcile material authority or
source conflicts by the normal non-force path; do not merge up solely to chase
main when Merge Queue owns freshness.

The amendment adds **zero worker paths**. Its author owns only this allocation
document on `coord/wp2-gamesession-nonreuse-353`; existing worker files are not
changed by this allocation PR. The plan path above is retained task provenance,
not a requirement to run the retired Superpowers framework. #388/#389 governance
cleanup retains its separate files and custody.

## Accepted authority and exact outcome

Use the protected contracts, without selecting new semantics or resource values:

- `docs/architecture/reviews/OTERYN_GAME_SESSION_NONREUSE_AUTHORITY_DECISION_2026-09-07.md`
  (`FND-DUR-GAMESESSION-NONREUSE-V1`, especially sections 3-7).
- `docs/architecture/reviews/OTERYN_GAME_FND04C_GAMESESSION_LEDGER_CAPACITY_ERROR_AMENDMENT_2026-09-07.md`.
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`, row
  `FND04-GAMESESSION-USED-IDS-PER-CHARACTER`, fixed maximum `65536`.
- Existing FND-04B, FND-04C and complete owning-loss/replacement contracts named by
  the original #353 task, except where the protected amendments explicitly apply.

Implement only the Foundation semantic/API boundary for the protected decision.
Foundation consumes a sealed candidate-specific `GameSessionUseObservationV1`,
not a copied lifetime ledger. Bind CharacterId, candidate GameSessionId, expected
current/predecessor or explicit no-current origin, registered source identity and
version, membership revision, COMPLETE status, membership result and applicable
current-authority fences. Exact Rust layout is an implementation detail within
the accepted contract, not permission for a new public protocol or trust model.

The observation is eligibility evidence, not retained authorization. Require it
consistently in Terminal replacement, CompleteReconnect EarlyTerminalReplacement
and PostGrace paths that create a new GameSession. Reject absent, incomplete,
stale, conflicting, rolled-back or mismatched observations before new effects;
revalidate independently current authority at each required decision boundary.
Caller-filled collections, operation receipts, initial/current IDs alone, UUID
ordering and probabilistic membership are not substitutes for the Game owner.

Preserve the accepted PREPARE/COMMIT distinction, immutable original
FreshAdmissionCommit, protection/rearm history and original loss identity.
Preserve existing wire and compatible API contracts through the smallest accepted
additive surface where necessary. A concrete need for another source/export path
or an unresolved compatibility decision is an escalation before mutation, not
permission to widen this allowlist or forge a legacy observation.

Expose the family-specific exhaustion results for the three allocated families:

| Family | FND-04C code |
| --- | --- |
| Terminal replacement | `TERMINAL_REPLACEMENT_GAMESESSION_LEDGER_EXHAUSTED` |
| CompleteReconnect EarlyTerminalReplacement | `EARLY_TERMINAL_REPLACEMENT_GAMESESSION_LEDGER_EXHAUSTED` |
| PostGrace new-session recovery/adoption | `POST_GRACE_RECOVERY_GAMESESSION_LEDGER_EXHAUSTED` |

Each has `CAPACITY_EXCEEDED`, `TERMINAL`, `SESSION_UNAVAILABLE` and
`CURRENT_AUTHORITY_PRESERVED` at revalidation. Do not reuse transient retryable
`ADMISSION_CAPACITY_EXCEEDED`. Denial creates no new membership, session, claim,
lease, transport, receipt or successful grant/nonce-consumption effect. Public
diagnostics disclose no raw character/session counts, membership or private
fences. Fresh admission's separate `ADMISSION_GAMESESSION_LEDGER_EXHAUSTED` remains
required by the accepted contract; this amendment does not seize its unallocated
implementation surfaces.

Classify exact previously committed replay by its immutable binding before
interpreting the used candidate as a new-session request. Reconciliation returns
the stable committed result, including at the ceiling, without new membership,
revision, capacity consumption or authority re-aging. A different operation using
that ID is rejected. Same-session reconnect does not consume another entry.
Membership is never expired, evicted or removed to create capacity.

## Required qualification

Use contract-derived positive controls and one-invariant negatives, not fixtures
that merely repeat the implementation's assumptions:

| Property | Required evidence on the candidate |
| --- | --- |
| Intermediate retired-ID rejection | S0 -> S1 -> S2 -> reject S1 across each applicable new-session entry family; a genuinely unused candidate passes with current complete authority. |
| Observation integrity | Missing/incomplete membership, stale or conflicting source/version, revision rollback/overflow, wrong CharacterId/candidate/predecessor and independently changed authority fences fail closed; valid owner-sealed controls pass. |
| Ownership and phase compatibility | Existing PREPARE successor anchor/claims and COMMIT-only activation regressions remain valid; immutable receipt, original loss, protection and claim history are not rewritten. |
| Boundary and terminal errors | 65535 plus one unused candidate permits the final slot; 65536 plus a new candidate yields the exact family code and preserves current authority. Tests distinguish permanent from transient capacity. |
| Replay versus new authority | Exact committed replay at the ceiling returns its prior disposition with no new capacity/revision/effect; altered operation/binding is rejected. Same-session reconnect has no extra membership cost. |
| Non-forgeability and diagnostics | Owner seals cannot be bypassed through a public constructor or test-only production hook; retain compile-fail coverage where applicable. No membership/count/fence disclosure. |
| Complete-history boundary | Partial S0/current-S2 evidence cannot be promoted to COMPLETE or fabricate intermediate S1. Actual durable reconstruction/reload proof remains WP4-owned. |

Run focused RED/GREEN regressions, then the affected package/doctests, strict
Clippy, formatting and applicable governance checks under `BUILD_TEST_MATRIX.md`.
Current pinned commands include:

```sh
cargo +1.94.0 test -p oteryn-game-server control_loss_reconnect_bridge_tests --locked
cargo +1.94.0 test -p oteryn-game-server --all-targets --all-features --locked
cargo +1.94.0 test -p oteryn-game-server --doc --locked
cargo +1.94.0 clippy -p oteryn-game-server --all-targets --all-features --locked -- -D warnings
cargo +1.94.0 fmt --all --check
python tools/agents/validate_governance.py
git diff --check
```

Preserve the original task's exact, read-only Foundation x B source-compatibility
qualification, binding the then-current B revision. A source-inclusion compile is
not PostgreSQL execution, B acceptance or permission to integrate sibling source.
WP2 semantic tests do not prove durable persistence/reload. Genuine independent
review is required for the final material session/recovery candidate; author
review and green CI do not substitute. All applicable exact-head repository gates
and normal protected Merge Queue remain required, with WP1 integration holds
unchanged. No new workflow, required status or review controller is introduced.

## Exclusions and dependency order

No SQL, migration, Durability codec/persistence, resource registry, Cargo/lock,
workflow/protection, Server Seam, real producer/bootstrap, Platform, Atlas, META,
production, deployment, credential, secret or live-data writes. No consumer
activation and no new session/identity format. No custody transfer from #329,
#351, #247 or #388. Released migration 0001 remains immutable.

Source preparation and product acceptance are distinct. After this amendment's
protected acceptance and Work application, the same WP2 lane may implement this
semantic/API repair with consumers inactive. WP1/#308 still holds product
acceptance/integration until its required trustworthy evidence is satisfied.
Thereafter preserve the accepted dependency order:

`WP2 semantic/API -> legitimate WP3 prerequisite where required -> WP4 durable
persistence/reload -> joint Foundation + actual PostgreSQL 17.6 qualification ->
consumer activation -> Work re-evaluates #247`.

Decision B/WP3, WP5 and the frozen #308 surfaces are not resolved or released by
this amendment. Do not reverse WP2/WP4 or substitute compile-only evidence for
joint qualification. Architecture documents and this allocation are not G0/G1.

## Application, return and terminal evidence

Before applying the amendment, Work verifies its protected acceptance, fresh
#353/#361 head, exact scope overlap and one sole worker/worktree. Bind the actual
execution-host-qualified worktree and fresh routing preflight under the bound
META execution policy. No worktree identity or running child session is assumed
by this prospective document. The same lane is serial because all three entry
families share Foundation authority and tests; parallel writers add collision
risk, not an independent critical-path benefit.

The worker returns the exact head/PR and changed paths, acceptance-to-test
mapping, first remaining divergence, self-review, genuinely independent review
when performed, exact CI evidence, and explicit semantic/SQL/E2E classifications.
Report source readiness separately from held integration and consumer activation.
Preserve existing task/history; archive or release it only after its actual
accepted terminal delivery, not after this documentation PR.

This allocation is qualified as a documentation/control-plane change: structural
and scope checks plus whole-diff self-review, applicable independent review and
canonical exact-head checks before protected integration. Runtime E2E is
NOT_APPLICABLE to the allocation itself because it changes no executable code;
that classification does not carry into the eventual WP2 implementation.
If required review or worker invocation is unavailable, record the exact missing
capability on #162, preserve the candidate and continue only independent
permitted work. A handoff is not a worker launch or background continuation.

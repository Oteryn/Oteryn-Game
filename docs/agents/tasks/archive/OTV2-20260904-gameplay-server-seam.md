# OTV2-20260904-gameplay-server-seam

> **CURRENT-STATE ROUTING ONLY.** Full pre-compaction task history is preserved in
> `docs/agents/evidence/OTV2-20260922-gameplay-server-seam-active-task-history.md`.
> Live Issue #247, protected `main`, current dependency Issues/PRs and exact branch state
> outrank this snapshot whenever they advance.

```yaml
task_id: OTV2-20260904-gameplay-server-seam
title: Production gameplay server seam
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-gameplay-server-seam-01
issue: 247
pr: 823
admission_main_sha: bc9f5dac5642b56135cce31f91b9ed23e5258a70
preserved_worker_head: 9370b254c6ac4f6529e069c1968ae6bfa1e1750e
owner: "Oteryn: sol server seam lead"
coordinator: Oteryn Work Delivery Coordinator
execution_policy: continuous_progress
dependency_state: WP5_G0_READY
owned_paths:
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/tcp_tls.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - .github/workflows/gameplay-server-seam.yml
serialized_shared_paths:
  - apps/game-server/src/foundation/protocol.rs
  - apps/game-server/src/foundation/fnd04_verifier.rs
  - tools/qualification/wp5_s3b/run.sh
  - apps/game-server/src/lib.rs
  - apps/game-server/src/main.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
public_contracts:
  - OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
depends_on:
  - issue:319
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Complete the accepted production gameplay TCP/TLS entry seam without moving Foundation,
Durability or source-composition authority into the transport lane. Preserve the existing
partial worker branch until the current upstream release gate is satisfied.

## Current source of truth

- **PROVEN:** allocation PR #294 merged through protected Merge Queue at
  `bc9f5dac5642b56135cce31f91b9ed23e5258a70`; the canonical worker branch was
  created from that protected allocation.
- **PROVEN:** the preserved worker checkpoint is
  `agent/otv2-gameplay-server-seam-01@9370b254c6ac4f6529e069c1968ae6bfa1e1750e`.
  It contains Task 1 plus initial private TCP/TLS framing work and is not an integration candidate.
- **PROVEN:** WP4 Child B PR #335 is now protected-integrated as
  `f02beb42523af6db1bb0c71d2840961e3fa5fcd0`; the former WP4-not-protected gate is closed.
- **PROVEN:** WP5 S1 PR #735 is protected-integrated as
  `c59d8b25f9e3017d013d578a5a4bc9d93fa49e1d`, but Issue #319 remains open.
- **PROVEN:** Work activated WP5 S2 after WP4 protection. S2, material #416 routing,
  later #414/#415 composition and G0 readiness remain owned by #319/current Work routing.
- **DERIVED:** Server Seam therefore remains `waiting` on **WP5 G0 readiness**, not on WP4.
  Do not resume merely because #335 and S1 are protected.

## Resume gate

Resume this exact preserved worker only after Work proves all of the following from fresh live state:

1. Issue #319 has reached the accepted WP5 G0/source-composition readiness boundary;
2. all required S2/S3 and material #414/#415/#416 gates that G0 depends on are protected/read back;
3. current protected `main`, branch ownership and path overlap are re-resolved;
4. the preserved worker branch can be reconciled without seizing Foundation/Durability/source authority.

Then finish Task 3+, production listener/composition, focused TDD, PostgreSQL/E2E qualification,
whole-diff review and required independent exact-head review before returning
`READY_FOR_INTEGRATION`.

## Holding action

- Do not move/reset/rebase/force/restart the preserved worker merely to refresh metadata.
- Do not open or integrate a partial Server Seam implementation PR.
- Do not invent a transport-local durable admission/source substitute.
- Do not take Durability migrations/schema, WP5 source-composition, production/deployment,
  workflow/ruleset or external-repository authority.
- Shared Cargo/lib custody is effective only after a fresh Work overlap readback; historical
  lease language does not override a newer active allocation.

## Resume record

- **PROVEN:** `WP5_G0_READY` on protected `main@0a21973013ed3856d545049563f9b409018f5bb6`: S1 #735, #416 #739, S2 #757, S3-A #760, #415 #769, #414 #790 and S3-B #815 are protected; the S3-B `workflow_dispatch` run `35969349848` on `main` produced `S3B_RESULT=COMPOSED_PASS`. The G0 record is #319 comment `5809797683`.
- **PROVEN:** the same worker branch is reconciled with that `main` by a normal merge commit (no rebase or force). The Seam TLS dependency now uses `main`'s `aws_lc_rs` provider. `rcgen` and rustls `tls12` are dev-only, for test certificates and the TLS 1.2 rejection client.

## Implementation state

- **Fresh admission (implemented):** the connection state machine (`connection.rs`) decodes the entry frame through the Foundation bridge and hands `ClientBootstrap` to `ComposedFreshAdmission` (`mod.rs`). That authority reads the #414 Character (account/world), uses this holder's #415 Channel, publishes and composes the S2/#414/#415 sources, verifies the grant with FND-04 and commits only through `commit_composed_fresh_admission`. `ServerAccepted` is written only after `Committed`. Refusal closes the transport without mutation: FND-04C refusal codes have no registered wire mapping.
- **Listener:** 256 connections (admitted connections stay inside this registered budget) and 64 handshake/auth units. The entry deadline is supplied by the caller. Shutdown cancels entry work, completes an admission already handed to the authority, then closes. The per-session outbound queue and pending-write maxima are not applicable yet: a connection writes at most one frame at a time and has no queue.
- **Composition:** the public `serve_gameplay` entry in `lib.rs` takes already-composed owners and explicit TLS/limits. `main.rs` remains fail-closed: starting a real node needs a configuration contract for owner services (S2 refresh loop, runtime readiness producer, Character fence store), and that contract does not exist.
- **Physical qualification:** an in-crate `qualification.rs` runs through `serve_gameplay` over real TCP/TLS in the WP5 topology (`WP5_QUALIFICATION=seam`). It is in-crate rather than `tests/gameplay_server_seam.rs` because runtime readiness is published through a sealed trait. An external test would need a new public test-only API or a `#[path]` copy, and the plan forbids both.
- **Owner decision (2026-09-24, option A):** integrate the fresh-admission seam now. Resume is split into a separate blocked follow-up. It needs three things: a playable-control authority to prove control loss (FND-04B §5, needs a gameplay runtime); the deferred same-session grace duration; and an accepted `PROD-ENTITLEMENTS-01` contract (currently `CANDIDATE`). This supersedes the earlier "no partial Server Seam PR" holding rule for this scope.
- **Resume (not yet served, follow-up #822):** `ClientResume` is decoded and closed without mutation. No production recovery-source composition exists: `RecoveryCurrentEvidence` and `RecoveryDurabilityEvidenceSourceV2` have only test fixtures. This is the next prerequisite.

## Qualification evidence

`Server Seam physical qualification` run `35976323319`, job `107557572725`, on `8941233a984dc85b395f9abfa105a01732483494`: PostgreSQL 17.6 and Platform `9147bfd`, running through `serve_gameplay` over loopback TCP + TLS 1.3. Result `S3B_RESULT=SEAM_PASS`:

- `transport tls12_exact_alpn=refused wrong_alpn=refused missing_alpn=refused plaintext=refused admissions=0`
- `foundation wrong_protocol_major=rejected wrong_transport_profile=rejected phase_invalid=rejected oversized=closed truncated=closed admissions=0`
- `fnd04 invalid_signature=refused expired=refused wrong_character_binding=refused untrusted_signer=refused admissions=0`
- `admission=committed server_accepted=1 post_admission_command=closed_unknown_message admissions=1`
- `replayed_grant=refused admissions=1`
- `concurrent_same_grant accepted=1 admissions=2`
- `shutdown=drained FORMAL_ADR0007_QA_TIER1_TIER2=NOT_EVALUATED`

## Validation state

Historical worker evidence is retained in the evidence snapshot. It includes the partial
236/236 library-test checkpoint, but that evidence does **not** qualify a future reconciled
candidate. Required candidate-specific validation starts again on the eventual resumed exact head.

## Context checkpoint

```yaml
last_progress: WP5_G0_READY on protected main 0a21973 (#319 comment 5809797683); branch reconciled with main by a normal merge; TLS provider aligned to main aws_lc_rs
status: completed
branch: agent/otv2-gameplay-server-seam-01
head_sha: 796802628d9e9accfda3c7ec6eb9228f6181183c
pr: 823
final_head_sha: 796802628d9e9accfda3c7ec6eb9228f6181183c
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: none; resume/reconnect continues only under follow-up #822
```

## Terminal closeout

- **PROVEN:** PR #823 was merged after qualification of the exact PR head `796802628d9e9accfda3c7ec6eb9228f6181183c`.
- **PROVEN:** the `Server Seam physical qualification` run [35978264116](https://github.com/Oteryn/Oteryn-Game/actions/runs/35978264116) on that head gave `S3B_RESULT=SEAM_PASS`. Its stage evidence matches the earlier run listed above.
- **PROVEN:** protected `main` holds merge commit `a747ac5264ba0164cc0afe0e75991ce18b687bf0`.
- **PROVEN:** merge-group run [35981086101](https://github.com/Oteryn/Oteryn-Game/actions/runs/35981086101) completed with aggregate `game-gate` SUCCESS.
- **TERMINAL:** fresh admission of the Server Seam is complete. Its exclusive authoring ownership, including the serialized shared paths, is released. `ClientResume` / reconnect and the control-loss lifecycle are the blocked follow-up #822; they need a new live task allocation.
- This terminal closeout supersedes the earlier implementing, holding and pre-merge statements above. They are kept as the historical checkpoint.

# OTV2-20260805-foundation-preimplementation-contracts

```yaml
task_id: OTV2-20260805-foundation-preimplementation-contracts
title: Coordinate Oteryn v2 foundation contracts and staged implementation gates
mode: COORDINATE
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: null
pr: null
base_sha: 05544969baf58c3a40354f366438d759bfd159e5
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-11T01:45:00+02:00
execution_budget_minutes: 120
large_budget_reason: Non-owning programme checkpoint spanning the accepted native foundation and the remaining product/durability/vertical-slice gates; executable packages remain separately bounded.
owned_paths: []
public_contracts:
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
  - docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md
  - docs/architecture/README.md
continuation_prompt: docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
depends_on:
  - accepted ADR-0001 through ADR-0016 as applicable to their named scopes
  - FND-01 and VSL-02 accepted/applied
  - FND-ID-01, FND-02, FND-03, FND-04, DUR-01 and ANL-01 accepted/lifecycle-closed
  - dual-transport architecture closeout PR 149 merged as 05544969baf58c3a40354f366438d759bfd159e5
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
```

## Outcome

Maintain a truthful **non-owning** programme checkpoint for Oteryn-v2. It coordinates accepted foundation architecture and names exactly the next safe decision/proof work without implementing gates, reserving their paths or treating architecture acceptance as runtime completion.

Every substantial gate still requires its own bounded task, branch, PR, validation, review policy, merge and archive lifecycle.

## Canonical continuation order

Use these sources in this order:

1. `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` — canonical current progression, delivery and implementation status;
2. accepted ADRs/contracts and exact machine-readable registries — semantic authority;
3. `docs/architecture/FOUNDATION_DECISION_BACKLOG.md` — stable gate definitions/dependencies;
4. `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` — complete staged architecture horizon;
5. `docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md` and `ARCHITECTURE_ANALYSIS_GAP_REGISTER.md` — unresolved product/gameplay coverage and historical analysis;
6. this checkpoint — coordination summary only;
7. live GitHub branch/PR/CI state — execution truth.

Older progress prose in backlogs/registers is historical where it conflicts with the later exact evidence in `FOUNDATION_PROGRAMME_CURRENT_STATUS.md`.

## PROVEN current foundation state

- Canonical repository is `blakinio/Oteryn-v2` for native Rust client, future Rust game server, gameplay protocol/content/tooling boundaries.
- PR #50 / merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0` delivered the accepted 19-member Rust workspace and pre-native client cutover.
- The native client is intentionally ADR-0011 `pre-native-protocol`: it launches but gameplay entry fails closed until a separately authorized real transport/session/server path exists.
- `protocol-canary` remains reference-only and absent from the production dependency/negotiation/fallback path.
- `FND-ID-01`, `FND-02`, `FND-03`, `FND-04`, `DUR-01` and `ANL-01` architecture are accepted and lifecycle-closed; their runtime implementation status remains separate and is mostly `NOT_STARTED`.
- ADR-0009 remains binding that one `GameNode` is one game-server process. ADR-0015 does not reopen that identity; it only leaves internal module/crate decomposition and genuinely separate adjacent-service boundaries evidence-driven.
- ADR-0014 through ADR-0016 accept TCP-default/future-QUIC transport direction while keeping every gameplay transport client mode runtime-unavailable now. TCP profile `1` is architecture registration, not an implemented listener; QUIC remains a later evidence-gated target.
- Platform remains credential/commercial-control-plane authority; native game authority remains separate as defined by ADR-0003/ADR-0012/FND-04.
- Platform entitlement security finding #944 is repaired by Oteryn-Platform PR #968 / merge `afaa6d1d8340e44b1152b62d6d27e5fd1649804a`. Oteryn-v2 still lacks the accepted game-side `PROD-ENTITLEMENTS-01` consumer/enforcement contract, so Premium/VIP activation remains unauthorized.
- No complete Rust GameNode, gameplay transport listener/codec path, game persistence runtime, event/outbox runtime or native client-to-server gameplay E2E is proven by the accepted architecture packages.

## Accepted foundation — do not silently redesign

A later task must explicitly supersede the relevant accepted source before materially changing:

- native Rust client/server direction and one project-owned `protocol-oteryn`;
- multichannel-first logical world model and one logical authoritative writer per channel/instance scope;
- Platform Identity/Game Gateway versus native game authority boundary;
- PostgreSQL native-game persistence target and separate Platform/game ownership;
- native world/content model, deterministic compiler/bundle direction and Oteryn Studio boundary;
- Game Intelligence separation between observability, best-effort telemetry and durable audit;
- native three-tier E2E evidence discipline;
- ADR-0009 one-process GameNode identity, external orchestration and measured-capacity/recovery requirements;
- reference/evolved product profiles over one engine/client/protocol with world-scoped gameplay value isolation;
- fail-closed pre-native client state;
- foundation identifiers, `protocol-oteryn` semantics, runtime execution semantics, admission/lease/reconnect semantics, durable identity representation and ANL-01 event/audit foundation;
- dual-transport safety invariants accepted in ADR-0014..0016.

## Current ordered work

Do **not** return to FND-ID/FND-02/FND-03/FND-04 as the programme's next gate; those architecture packages are already accepted/lifecycle-closed.

The immediate architecture/proof sequence is:

1. `GAME-VISION-01` minimum product/launch profile baseline and `GAME-CHANNEL-01` channel semantics; bounded `DUR-02` discovery may proceed in parallel when it does not freeze product-sensitive character semantics.
2. `GAME-CHAR-01` before final character-bearing `DUR-02` schema.
3. `DUR-02 — Persistence v1` finalization after required product semantics.
4. `GAME-ITEM-01`, then `DUR-03` item transaction/anti-duplication invariants.
5. Minimum `DUR-04` headless content path: schema -> validator -> deterministic compiler -> bundle -> loader.
6. `SIM-DETERMINISM-01` before broad combat/AI formula freeze.
7. Real-boundary vertical slices: `VSL-ADMISSION-01 -> VSL-MOVE-01 -> VSL-COMBAT-01 -> VSL-PERSISTENCE-01 -> VSL-RECOVERY-01 -> VSL-MULTICHANNEL-01`.
8. `NET-TRANSPORT-02` QUIC profile/admission/evidence work only when the current product/runtime path can measure benefit and preserve FND-02/FND-04 semantics; QUIC is not the current implementation priority.
9. Minimal admin/security/SRE readiness before external alpha.

`PROD-ENTITLEMENTS-01` remains a separate deferred gate. Its Platform producer prerequisite is satisfied, but game-side consumer semantics and implementation remain blocked/unaccepted.

## Implementation discipline

- Architecture acceptance never implies implementation or production readiness.
- Do not create speculative crates/services solely to mirror diagrams; `workspace-boundaries.toml` remains the executable guard against premature fragments.
- Start runtime implementation with small vertical slices and immediate consumers rather than broad skeleton generation.
- Preserve client/server separation in task ownership while integrating only through accepted contracts.
- Keep GameNode one process until a dedicated later decision explicitly supersedes ADR-0009 with measured evidence.
- Keep TCP+TLS as the initial safe transport path when gameplay networking is authorized; QUIC remains later opt-in/evidence-gated.
- Cross-repository writes require exact explicit authority; this programme checkpoint does not grant them.

## What remains deliberately unresolved

The following require product-owner decisions or future measured evidence and must not be guessed by an autonomous agent:

- exact Reference versus Evolved launch/profile promise and parity target (`GAME-VISION-01`);
- player-facing channel semantics and cross-channel social/economy/PvP behavior (`GAME-CHANNEL-01`);
- final character lifecycle/progression semantics (`GAME-CHAR-01`);
- final item/equipment/container/transform semantics (`GAME-ITEM-01`);
- final simulation arithmetic/determinism choices beyond accepted foundation constraints;
- specific QUIC implementation/library/profile values and numeric capacity/SLO claims;
- Premium/VIP product policy and final game-side entitlement behavior.

## Validation rule

This checkpoint itself is coordination state. It does not substitute for package validation. Every implementation or contract package must provide exact-head evidence appropriate to its scope, and a mock/bypass cannot be terminal proof for a claimed real boundary.

## Context checkpoint

```yaml
last_progress: Foundation architecture through FND-ID/FND-04/DUR-01/ANL-01 and the dual-transport strategy is accepted/lifecycle-closed; runtime remains largely unimplemented. Programme priority moved to product semantics, bounded persistence discovery and ordered real-boundary vertical slices.
status: ready
branch: null
head_sha: null
pr: null
final_head_sha: null
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
next_action: Start one bounded architecture-only package for GAME-VISION-01 or GAME-CHANNEL-01 when product-owner input is available; otherwise bounded DUR-02 discovery may proceed without freezing product-sensitive semantics.
```

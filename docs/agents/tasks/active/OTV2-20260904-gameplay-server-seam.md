# OTV2-20260904-gameplay-server-seam

```yaml
task_id: OTV2-20260904-gameplay-server-seam
title: Implement production gameplay Server Seam
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-gameplay-server-seam-01
pr: null
issue: 247
lane_id: OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
allocation_task_id: OTV2-20260904-gameplay-server-seam-allocation
allocation_pr: 294
allocation_admission_main_sha: 68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: sol server seam lead"
created_at: 2026-09-04T19:27:00+02:00
updated_at: 2026-09-04T20:00:00+02:00
execution_budget_minutes: 120
large_budget_reason: high-risk TCP/TLS plus Foundation admission/reconnect integration, bounded-resource proof, real production-path seam integration and independent exact-head review
owned_paths:
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/tcp_tls.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/tests/gameplay_server_seam.rs
  - apps/game-server/src/foundation/protocol.rs
  - apps/game-server/src/lib.rs
  - apps/game-server/src/main.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam.md
public_contracts:
  - FND-02 protocol consumer seam
  - FND-03 runtime/resource consumer seam
  - FND-04 admission/GameSession/CharacterLease/reconnect/fencing consumer seam
  - production gameplay TCP/TLS entry seam accepted by PR #117
depends_on:
  - allocation_task: OTV2-20260904-gameplay-server-seam-allocation
  - issue: 280
  - merged_pr: 117
  - merged_pr: 151
  - merged_pr: 252
  - merged_pr: 290
blocks:
  - native Client allocation/readiness
  - physical gameplay Tier 1/Tier 2 QA sequence
external_repositories: []
```

## Outcome

Implement the smallest accepted production gameplay server/client-entry seam on top of current protected-main Foundation/FND-04/Durability authority: bounded TCP + TLS 1.3 with ALPN `oteryn-game/1`, FND-02 framing/validation, canonical fresh-admission/reconnect verification, current GameSession/CharacterLease/fencing authority, durable reconnect journal consumption, bounded backpressure/drain, and real local production-path TCP/TLS integration evidence.

This task is intentionally **not released** while post-#252 authority-floor Issue #280 / PR #289 remains nonterminal. The branch does not exist yet and this record grants zero runtime/shared-path write authority.

The lane makes later ADR-0007 physical Tier 1 executable but does not own or declare QA Tier 1/Tier 2 `PROVEN`. Formal QA Tier 1/Tier 2 remain `NOT_EVALUATED` until separately allocated QA after the Server Seam merge.

Unregistered gameplay remains fail-closed; no gameplay command/state/event/capability/stable numeric ID or domain semantic is allocated here.

## Release preconditions

All of the following are mandatory before Work may create `agent/otv2-gameplay-server-seam-01`:

1. Issue #280 is terminal on protected `main` and PR #289 or its explicit successor has protected-main readback.
2. Work re-reads the final current authority APIs from protected `main` and reruns the production-public sibling-family sweep.
3. No unresolved P0/P1 record/identity-derived current-authority convenience remains relevant to the Server Seam consumer boundary.
4. PR #294 is reconciled to any final API/lease change, receives fresh exact-head deterministic qualification plus one independent deep review, integrates through protected controls, and its merge SHA is read back from protected `main`.
5. The worker branch is created from exactly that allocation merge SHA.
6. Final open-PR path ownership is rechecked, including shared Cargo paths and Dependabot candidates.

A historical green #289 or #294 head is not a substitute for these conditions.

## Architecture and source of truth

- `PROVEN` — merged Server Seam preparation #117 defines the accepted topology and evidence floor.
- `PROVEN` — FND-04 verifier/consumer #151 is merged/released; Issue #116 limits are registered; historical Durability #252/#290 is merged/released.
- `PROVEN` — live #280 is a later authority API contract-floor repair and is currently nonterminal.
- `PROVEN` — current #289 head observed by Work is `ddbb44d2644c6f66bf86aba837d7712b01878fac`.
- `PROVEN` — that head hides `ReconnectCandidateBindingV1::from_record(...)` from production but still exposes production-public `CharacterWorldEligibilityClaimV1::from_identity(...)` and `AccountPresenceClaimV1::from_identity(...)` derived from immutable reconnect identity.
- `DERIVED` — Server Seam must not be released against that known nonterminal authority floor because its admission/reconnect state machine will consume the same current-authority family.
- `PROVEN` — current Foundation protocol constants include protocol major `1`, TCP/TLS13 transport profile `1`, ALPN `oteryn-game/1`, the 1,048,576-byte wire-frame maximum and message-specific limits.
- `PROVEN` — current `foundation/protocol.rs` validates inbound FND-02 envelopes; accepted #117 allocates only the minimum typed bootstrap/resume extraction plus registered-only server encoders.
- `PROVEN` — current `main.rs` ordinary gameplay startup remains fail-closed.

Governing order after release: protected-main governance -> merged allocation -> accepted #117 -> final protected-main FND/DUR APIs/contracts/registries -> live implementation. Any material conflict blocks the affected mutation and is escalated rather than guessed through.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - authenticated transport identity/binding is independently verified before admission or reconnect authority is granted
  - fresh admission GameSession/CharacterLease/world/runtime ownership facts are current and come from canonical verifier/current-authority sources
  - reconnect identity, predecessor/candidate generation, runtime scope, ownership generation and control-loss continuity are current at every authority-consuming boundary
  - durable PREPARE/COMMIT/reconciliation evidence defines expected persisted binding but never proves live current authority by itself
  - final reconnect COMMIT/controller installation revalidates current authority/fence facts
  - stale connection generation cannot send accepted post-admission work or regain controller authority
consumer_boundaries:
  - pre-admission TLS connection and bounded FND-02 bootstrap/resume decode
  - FND-04 fresh admission verification and canonical GameSession commit
  - reconnect/resume verification and durable PREPARE/reconciliation
  - final reconnect current-authority revalidation/controller installation
  - admitted read/write dispatch and generation fencing
  - shutdown/drain of bounded transport work, including already-authoritative reserved work
mutation_operators:
  applicable:
    - accept one pre-admission connection within registered budget
    - begin one bounded TLS handshake/authentication unit
    - commit fresh GameSession only through canonical Foundation authority
    - prepare/reconcile reconnect only through current Durability adapter
    - authorize final reconnect COMMIT/controller replacement only from independently current facts
    - attach/replace transport generation only after canonical authority succeeds
    - enqueue bounded server output/pending writes for current admitted generation
    - close/drain transport-local work without mutating foreign gameplay authority
  considered_not_applicable:
    - gameplay command/domain mutation
    - Movement/Combat/Ability/Interaction/AI mutation
    - production certificate/key/port/deployment mutation
one_invariant_per_negative_case: required
independent_current_fact_sources:
  - production FND-04 verifier/consumer output plus independently current authoritative evidence
  - current Foundation GameSession/CharacterLease/runtime authority at the exact consumer boundary
  - current connection/runtime-scope/ownership facts resolved independently of immutable reconnect records
record_derived_matching_helper:
  allowed_for_negative_authority_or_provenance_cases: false
finding_family_sweep:
  sibling_apis: required across fresh admission, resume/reconnect and server acknowledgement/error bridge APIs
  protocol_versions: v1 only unless protected main allocates another version
  direct_and_reconciled_paths: required
  fenced_durable_writes: required
  restart_retry_replay_concurrency_pg_reload: required where applicable to the changed consumer boundary
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

Immutable prepared/persisted reconnect identity may define expected binding but never supplies current presence, eligibility, session, lease, runtime, generation or ownership authority.

## Acceptance criteria

### Release / authority floor

- [ ] #280/#289 or explicit successor is terminal on protected `main` before worker branch creation.
- [ ] Fresh post-#280 sibling-family sweep proves no unresolved production-public record/identity-derived current-authority convenience at the Server Seam boundary.
- [ ] Worker branch base is exactly the protected-main merge SHA of the final qualified allocation PR #294.

### Transport / framing

- [ ] TCP + TLS 1.3 profile 1 and ALPN exactly `oteryn-game/1`; no plaintext/downgrade/legacy/Canary fallback.
- [ ] BE32 frame length is checked before peer-sized allocation/read; 0 and >1,048,576 reject fail-closed; exact max/truncation/overflow are tested.
- [ ] Wrong protocol major/profile, malformed/duplicate/unknown/over-limit fields, invalid direction/phase/generation and unknown messages fail before authority/domain mutation.
- [ ] No `unsafe`; current lint policy preserved.

### Foundation bridge / authority consumption

- [ ] `foundation/protocol.rs` adds only minimum crate-internal typed bootstrap/resume bridge and registered-only `ServerAccepted`/`ServerResumeAccepted`/`ProtocolError` encoders.
- [ ] Outbound evidence includes canonical/golden bytes plus an independent non-self-referential wire/cross-oracle check.
- [ ] A new externally public Foundation API/schema/wire semantic is `ARCHITECTURE_ESCALATION_REQUIRED` before mutation.
- [ ] Fresh admission uses production FND-04 verifier/consumer and canonical commit; invalid/expired/replayed/wrong-binding/stale evidence cases fail independently.
- [ ] Concurrent/replayed fresh admission cannot create two sessions.
- [ ] Reconnect consumes current final Durability/Foundation flow and independently current facts; no record-derived helper substitutes for live authority.
- [ ] Stale/missing/mismatched GameSession, CharacterLease, world/runtime, ownership/connection generation, transport binding or provenance fails before the affected authority grant/mutation.

### Resource / lifecycle

- [ ] Registered maxima enforced with checked accounting before partial mutation: pre-admission 256, handshake/auth 64, outbound entries 64/session, outbound bytes 1,048,576/session, pending writes 8/session, drain tasks 256/batch.
- [ ] Each applicable limit has max, max+1 and relevant overflow/early-rejection proof.
- [ ] Slow clients cannot create unbounded task/channel/retry growth or consume another session's budget.
- [ ] Shutdown/drain is bounded, does not resurrect stale authority and does not silently drop already-authoritative reserved work; it drains/resolves such work according to canonical Foundation lifecycle semantics.

### Composition / unsupported gameplay

- [ ] `lib.rs` composes exactly one Server Seam without moving Foundation/Durability authority.
- [ ] `main.rs` preserves `--smoke` and uses explicit valid configuration only; no production endpoint/certificate/key/secret/deployment topology selected here.
- [ ] Missing/incompatible config remains fail-closed with no plaintext fallback.
- [ ] Unsupported/unregistered post-admission `ClientCommand` fails closed with zero command reservation/domain mutation and no invented ID.

### TDD / physical seam evidence

- [ ] Fresh RED -> minimal GREEN covers bridge/encoders, framing, TLS/ALPN/profile, malformed/oversized input, invalid/expired/replayed/wrong-binding FND-04 material, concurrent admission replay, stale reconnect facts, all resource boundaries, unsupported gameplay, backpressure and shutdown/drain.
- [ ] `gameplay_server_seam.rs` traverses the real production listener/composition on loopback using non-shipping TLS material for bootstrap/admission and resume/reconnect.
- [ ] Physical assertions observe canonical GameSession/current-generation outcomes, not merely socket success.
- [ ] Server Seam physical integration may be `PROVEN` for exact worker candidate; ADR-0007 QA Tier 1/Tier 2 remain `NOT_EVALUATED` here.

### Qualification / handoff

- [ ] Focused/package tests, fmt, strict Clippy and applicable workspace validation pass.
- [ ] Exact-head repository CI including current required Linux/Windows/supply-chain/merge-gate composition is green.
- [ ] Whole-diff self-review completes authority-family sweep with explicit findings dispositions.
- [ ] One genuinely independent exact-head deep review covers protocol/session/admission/reconnect/fencing/TLS/resource/evidence-ownership risk.
- [ ] Zero unresolved required review threads and no material head movement after qualifying review.
- [ ] Worker returns truthful SERVER_SEAM `READY_FOR_INTEGRATION` and never self-merges.

## Excluded scope

No gameplay IDs/semantics, Movement/Combat/Ability/Interaction/AI/Channel/Analytics, permanent Content format, Reference fact, production endpoint/certificate/key/secret/deployment/live data, QUIC activation, new persistence semantics, migration redesign, `workspace-boundaries.toml`, workflow/ruleset/protection/stable-registry/architecture-contract, #280 repair, or external-repository write.

A required unowned path is `SHARED_LEASE_REQUIRED`; a material public API/protocol/trust/fencing/persistence/resource/evidence-ownership decision is `ARCHITECTURE_ESCALATION_REQUIRED`.

## Implementation / findings

After lawful release, follow `docs/superpowers/plans/2026-08-24-oteryn-production-gameplay-server-seam.md`, subordinate to this task and final protected-main APIs. Begin with fresh Foundation/framing RED, not a listener.

Before first shared Cargo write and final integration, re-read #259/#260/#261 and active non-Dependabot ownership. Do not absorb unrelated bot upgrades.

## Validation

### Focused

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam`
- result: not run — worker is not released; Issue #280 is nonterminal

### Component/integration

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server`
- result: not run — worker is not released; Issue #280 is nonterminal

### Physical Server Seam integration

- scenario: real local TCP/TLS production listener -> FND-02 bootstrap/resume -> final production FND-04/current authority -> Foundation admission/reconnect/Durability -> bounded registered response; stale generation and unsupported gameplay fail closed
- result: not run — worker is not released
- QA Tier 1: `NOT_EVALUATED`
- QA Tier 2: `NOT_EVALUATED`

### Exact-head CI

- final head: null
- trigger source: future worker pull_request only after lawful release
- result: not started

## Self-review

- exact head: null
- method/reviewer: future `Oteryn: sol server seam lead` whole-diff adversarial review
- material findings: not evaluated
- verdict: not evaluated

## Independent review

- required: `YES`
- exact head: null
- method/auditor: genuinely independent exact-head deep review
- material findings: not evaluated
- verdict: not evaluated

## PR and closeout

- changed-file review: exact owned-path allowlist only
- protected integration: worker never self-merges
- merge/result: Work responsibility only after truthful `READY_FOR_INTEGRATION`
- ownership release: after protected-main merge/readback and worker-task archival

## Context checkpoint

```yaml
last_progress: Work independently proved live authority-floor Issue #280 / PR #289 is nonterminal and current PR #289 head still exposes production-public AccountPresenceClaimV1::from_identity(...) plus CharacterWorldEligibilityClaimV1::from_identity(...); this worker remains intentionally unreleased until that authority family is terminally repaired and re-read from protected main
status: waiting
branch: agent/otv2-gameplay-server-seam-01
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
blocker: authority_api_floor_issue_280_pr_289_nonterminal
next_action: do not create this branch; wait for Work to read back terminal #280 from protected main, rerun the sibling-family/API/path-ownership sweep, reconcile allocation #294 to final APIs, merge/read back the final qualified allocation, then create this branch from exactly that allocation merge SHA and begin fresh RED
```

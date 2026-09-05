# OTV2-20260904-gameplay-server-seam

```yaml
task_id: OTV2-20260904-gameplay-server-seam
title: Implement production gameplay Server Seam
mode: IMPLEMENT
status: WAITING_ARCHITECTURE
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-gameplay-server-seam-01
pr: null
issue: 247
lane_id: OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
allocation_task_id: OTV2-20260904-gameplay-server-seam-allocation
allocation_pr: 294
allocation_admission_main_sha: 68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705
allocation_integration_main_sha: b9b1a4317858bffc25ad6af3cffcf7b5eff93445
base_sha: bc9f5dac5642b56135cce31f91b9ed23e5258a70
head_sha: 9370b254c6ac4f6529e069c1968ae6bfa1e1750e
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: sol server seam lead"
created_at: 2026-09-04T19:27:00+02:00
updated_at: 2026-09-05T16:07:38+00:00
execution_budget_minutes: 120
large_budget_reason: high-risk TCP/TLS plus Foundation admission/reconnect integration, bounded-resource proof, production-path integration and independent exact-head review
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
  - merged_pr: 117
  - merged_pr: 151
  - merged_pr: 252
  - merged_pr: 289
  - architecture_issue: 313
blocks:
  - native Client allocation/readiness
  - physical gameplay Tier 1/Tier 2 QA sequence
cross_repository_coordination_id: null
external_repositories: []
```

## Current execution evidence — 2026-09-05

This section supersedes historical pre-release status/checkpoint prose below. Issue #247 comment 5552715601 released the worker after #294 merged as `bc9f5dac5642b56135cce31f91b9ed23e5258a70`; that is the immutable worker admission base. Work re-read protected `main@9ed8fd596aec4d88fb70a414a864f3da4afe5a6e`; advancement from the worker's `d89f063ea7ad7f1d8fa09688309c8d898fae856e` product readback is CI/neutral-doc routing only and does not resolve the Foundation/Durability gap. Final integration main has not been selected.

State: `WAITING_ARCHITECTURE`, not ready and not task completion. The worker first proved a concrete `WAITING_DEPENDENCY`: accepted Task 3 fresh admission through `AdmissionAuthority<T, J>` has no production durable `ReconnectAttemptJournal<T>`. Work verified that the missing bridge requires a material Foundation/Durability authority/persistence decision outside this allocation and routed the bounded decision as Issue #313 `ARCHITECTURE_ESCALATION_REQUIRED`. The Server Seam branch and existing lease are preserved unchanged while #313 is unresolved.

### Proven prerequisite gap

At the protected readback:

- `apps/game-server/src/foundation/admission_facade.rs` defines the public `ReconnectAttemptJournal<T>` requirement: atomic `commit_fresh`, `load_session`, current session/attempt reconciliation and lifecycle operations. Its blanket implementation only forwards an existing implementation to the private core trait.
- `apps/game-server/src/foundation/admission.rs` requires atomic fresh-grant consumption, never-reused session issuance, the immutable initial binding and independently current lifecycle/controller state. Its `commit_fresh` explicitly requires the same authoritative owner for fresh admission and reconnect.
- Repository-wide Rust implementation search finds concrete journals only in tests. No production adapter implements that contract.
- `apps/game-server/src/durability/mod.rs` deliberately implements journal-only reconnect persistence/classification. `AdmissionReconnectJournal` and V2 accept Foundation reconnect requests; they do not provide fresh admission or the complete current GameSession journal facade.
- `apps/game-server/migrations/0001_admission_reconnect_journal.sql` has reconnect sessions/attempts, recovery-grant consumption and replacement receipts. Its session rows require predecessor/control-loss data. No fresh-grant replay mapping retaining the initial admission commit is present.

A transport-local in-memory journal would split authority and lose restart/replay protection. Synthesizing a reconnect record for a fresh admission would invent predecessor/control-loss facts. Neither is an authorized repair. The missing canonical durable fresh-admission/GameSession bridge requires its Foundation/Durability owner's allocation, including any necessary interface, persistence and forward-migration paths; those are outside this worker's allowlist. No new persistence schema or public authority contract was selected here. Issue #313 owns only the bounded architecture decision and grants no implementation authority by itself.

### Completed and deliberately incomplete work

- Task 1 protocol bridge: typed borrowed bootstrap/resume material and validated metadata cached by Foundation; bounded capability storage; private registered-only acceptance/resume/error encoders; independent fixed protobuf vectors. No wire ID or public bridge was added.
- Initial Task 2 private transport primitives: TLS 1.3-only configuration, exact negotiated ALPN check, early data disabled, canonical BE32 length validation before body allocation/output, and real loopback primitive tests. **Task 2 is partial**: registered pre-admission/handshake permit ownership, orchestration and lifecycle remain unimplemented. The primitive TLS stream is not an FND-04 identity/admission grant.
- Tasks 3–6 production authority/composition/physical qualification are incomplete. The canonical ordinary executable remains fail-closed. No gameplay listener was activated and no formal QA result is claimed.
- New exact transport dependency uses already-locked `tokio-rustls =0.26.4`; `rcgen =0.14.10` is dev-only non-shipping certificate generation. Existing dependency versions were not upgraded. Dependabot #259/#260/#261 remained non-owning Cargo reconciliation candidates on the worker's fresh pre-edit readback; its open-PR changed-file scan showed no other allocated writer of this allowlist.

### Validation and review

Fresh REDs:

1. `cargo +1.94.0 test --locked -p oteryn-game-server foundation::protocol -- --nocapture`: missing typed extraction methods, E0599; minimal extraction GREEN: 35 protocol tests.
2. `cargo +1.94.0 test --locked -p oteryn-game-server --lib foundation::protocol`: missing registered encoder types/functions, E0422/E0425; GREEN: 36 protocol tests including independently transcribed vectors.
3. Private frame selector: missing `read_frame`, E0425; GREEN accepts 1 and 1,048,576 and rejects zero, max+1, truncated prefix/body.
4. Private TLS/write selector: missing `tls_config`, `accept_tls`, `write_frame`, E0425; GREEN: 3 transport tests, including explicit TLS1.2-only + exact ALPN rejection before frame handoff, TLS1.3 success, absent/wrong ALPN, plaintext and write-before-output rejection.

Current source verification recorded by the worker: `cargo +1.94.0 test --locked -p oteryn-game-server --lib`: 236 passed, zero failed/ignored. This is library/component evidence, not the physical production-listener target. Formatting, whitespace, governance and repository-policy checks are recorded by that checkpoint's verification. No real PostgreSQL or full-workspace qualification was run for this partial candidate.

Strict `cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings` is **not green**: private bridge/transport APIs have no production consumers until composition exists. The separately observed single-element test-loop lint was repaired without changing assertions. Dead-code enforcement was not suppressed and visibility was not widened merely to satisfy lint. The partial branch must not be integrated.

Whole-diff self-inspection covered typed validation, fixed-array bounds, immutable schema tags, generation/phase behavior, TLS downgrade/ALPN and before-allocation framing. A read-only subagent independently checked the missing durable bridge against GitHub blob identities and inspected Task 1/initial Task 2; no concrete P1/P2 regression was reported, with Task 2 permit ownership explicitly unimplemented. This is advisory evidence, **not formal independent exact-head review**. Full authority finding-family sweep, production projection, hosted CI and genuinely independent final review remain pending.

GitHub publication used native blob/tree/commit/ref operations after local `git push` failed with missing HTTPS credentials. Task 1 native commit `fb7f448ad728bbfa8462b99633dc4c0506d6ee10` has exactly the same tree `38941ff48b4ba039d0e0c9370cdbadcac062fc78` as local execution commit `aed6dac`. The current published worker branch is independently read back at `9370b254c6ac4f6529e069c1968ae6bfa1e1750e`, tree `3681b01f8a08fc5c9b210b06957834477502b16f`, with non-force history preserved. No implementation PR exists. No automatic worker continuation or ownership release is claimed.

## Outcome

After the bounded architecture prerequisite #313 is decided, integrated and implemented by its proper Foundation/Durability owner through a fresh Work allocation, reconcile the preserved Server Seam branch against that protected-main result and continue the smallest accepted production gameplay server/client-entry seam on current Foundation, FND-04 verifier/consumer and Durability authority.

The seam is bounded TCP + TLS 1.3 with ALPN `oteryn-game/1`, FND-02 framing/validation, canonical fresh-admission/reconnect verification, current GameSession/CharacterLease/fencing authority, durable admission/reconnect consumption, bounded backpressure/drain and real local production-path TCP/TLS integration evidence.

This lane makes later ADR-0007 physical QA possible but does **not** own or declare QA Tier 1/Tier 2 `PROVEN`. Formal QA remains `NOT_EVALUATED` until separately allocated after Server Seam integration.

## Architecture and source of truth

- `PROVEN` — allocation PR #294 merged through protected Merge Queue as `bc9f5dac5642b56135cce31f91b9ed23e5258a70`; the worker branch was created from exactly that protected allocation merge SHA and is preserved at `9370b254c6ac4f6529e069c1968ae6bfa1e1750e` with no implementation PR.
- `PROVEN` — Issue #313 is the canonical bounded `ARCHITECTURE_ESCALATION_REQUIRED` for the missing durable fresh-admission / initial GameSession boundary. It grants no Foundation/Durability implementation authority and routes the next decision to `Oteryn: sol supervising architect`.
- `PROVEN` — accepted Server Seam architecture is `docs/architecture/reviews/OTERYN_GAME_PRODUCTION_GAMEPLAY_SERVER_SEAM_PLAN_2026-08-24.md`, merged through PR #117.
- `PROVEN` — protected-main authority API floor from #289 removes production record/identity-derived current-authority convenience constructors; #302/#303 further qualify independently sourced current authority and retry/replay/restart/PostgreSQL behavior without changing production semantics.
- `PROVEN` — protected `main@9ed8fd596aec4d88fb70a414a864f3da4afe5a6e` includes later CI/neutral-doc routing work through #312; that advancement does not alter the Server Seam API/lease, Foundation/Durability product authority, Cargo dependencies, Merge Queue fan-in authority or resource values and does not resolve #313.
- `PROVEN` — Foundation protocol major 1, transport profile 1, TLS 1.3, ALPN `oteryn-game/1`, FND-02 bounded BE32 framing and registered message semantics remain authoritative.
- `PROVEN` — current `apps/game-server/src/main.rs` remains fail-closed outside `--smoke`; no production listener exists.
- `PROVEN` — FND-04 verifier/consumer and current Durability terminal-replacement/reconnect implementation are merged.
- `PROVEN` — Server Seam hard maxima remain registered: pre-admission connections 256, handshake/auth work 64, outbound queue 64 entries/session, outbound queue 1,048,576 bytes/session, pending writes 8/session and drain tasks 256/batch.
- `PROVEN` — Work's fresh search found no open PR matching this active Server Seam task path before publishing this bounded coordinator correction. The worker lease remains preserved; no Foundation/Durability path is added to it.
- `DERIVED` — Server Seam cannot truthfully cross Task 3 until #313 is resolved and the resulting prerequisite is implemented/read back from protected main by the proper owner.

Governing source order is protected-main governance -> merged allocation -> accepted #117 -> current FND/DUR contracts/registries -> live implementation. A lower source cannot widen the allocation.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - authenticated transport identity/binding is independently verified before fresh-admission or reconnect authority is granted
  - fresh admission GameSession/CharacterLease/world/runtime facts are current and match canonical FND-04 verification
  - reconnect candidate identity, connection generation, runtime scope, ownership generation and control-loss continuity are independently current at each consuming boundary
  - immutable prepared/persisted evidence defines expected binding only and is never its own current-authority source
  - final reconnect COMMIT/controller installation revalidates current authority rather than trusting a stale earlier snapshot
  - stale connection generation cannot send accepted post-admission work or regain controller authority
consumer_boundaries:
  - pre-admission TLS connection and FND-02 bootstrap/resume decode
  - FND-04 fresh-admission verification and GameSession commit
  - reconnect PREPARE / direct or reconciled outcome
  - final reconnect current-authority revalidation and controller installation
  - admitted connection read/write dispatch and generation fencing
  - shutdown/drain of connection-local work
mutation_operators:
  applicable:
    - accept one pre-admission connection inside the registered connection budget
    - begin one bounded TLS handshake/auth unit
    - commit fresh admission only through canonical Foundation authority
    - prepare/reconcile reconnect through current durable adapter
    - authorize final reconnect COMMIT/controller replacement only from independently current facts
    - bind admitted transport generation only after canonical authority succeeds
    - enqueue bounded server output / pending writes for the current admitted generation
    - close/drain transport-local work without silently losing already-authoritative reserved work
  considered_not_applicable:
    - gameplay command/domain mutation: no gameplay command/state IDs are allocated to this lane
    - Movement/Combat/Ability/Interaction/AI state mutation: foreign authority
    - production deployment/certificate/secret mutation: excluded
one_invariant_per_negative_case: required
independent_current_fact_sources:
  - current FND-04 verifier/consumer output
  - current Foundation GameSession/CharacterLease/runtime authority at the exact consuming boundary
  - current connection generation/runtime scope/ownership generation resolved independently of immutable reconnect records
record_derived_matching_helper:
  allowed_for_positive_happy_path: only test-only expected-value construction explicitly permitted by current protected-main governance
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: required across fresh admission, resume/reconnect and server acknowledgement/error consumers
  protocol_versions: v1 only unless protected main allocates another version
  direct_and_reconciled_paths: required
  fenced_durable_writes: required where reconnect PREPARE/COMMIT/reconciliation is invoked
  restart_retry_replay_concurrency_pg_reload: required where this change invalidates or exercises the durable boundary; leverage current #302/#303 harness only when representative, otherwise add owned-path evidence or request a lease
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

Every negative authority/provenance case changes exactly one applicable invariant while unrelated facts remain valid. Historical terminal outcomes may preserve typed disposition but never reacquire live authority through a weaker path.

## Acceptance criteria

### Transport and framing

- [ ] TCP + TLS 1.3 profile 1 with exact ALPN `oteryn-game/1`; no plaintext/legacy/Canary fallback.
- [ ] Four-byte big-endian frame length is checked before body allocation/read; 0, truncation and >1,048,576 fail closed.
- [ ] Wrong protocol major and transport profile are rejected through the composed production listener path, not only decoder unit tests.
- [ ] Unknown/malformed/direction/phase-invalid input is rejected without admission/domain mutation or unbounded retained work.

### Foundation authority consumption

- [ ] `foundation/protocol.rs` exposes only the minimum crate-internal typed bootstrap/resume consumer bridge and registered `ServerAccepted`, `ServerResumeAccepted`, `ProtocolError` encoders; no new wire ID/schema/capability/state ID.
- [ ] New outbound encoders have independent canonical/golden vectors or another cross-oracle; same-implementation encode/decode round trip alone is insufficient.
- [ ] Fresh admission is committed only after FND-04 verification and canonical current authority.
- [ ] Missing/invalid/expired/replayed/wrong-binding evidence and concurrent same-grant fresh admission fail before a duplicate GameSession/controller is created.
- [ ] Resume/reconnect consumes current Foundation/Durability flow and final current-authority revalidation; no record-derived convenience substitutes for live facts.
- [ ] Stale/missing/mismatched GameSession, account presence, character/world eligibility, generation, runtime scope, ownership generation, transport binding and continuity each fail independently before the affected authority grant/mutation.

### Resources and lifecycle

- [ ] Enforce registered hard maxima before allocation/partial mutation: 256 pre-admission connections, 64 handshake/auth units, 64 outbound entries/session, 1,048,576 outbound bytes/session, 8 pending writes/session, 256 drain tasks/batch.
- [ ] Each applicable resource has max accepted, max+1 rejected/backpressured and checked-overflow/early-rejection evidence.
- [ ] Slow-client saturation cannot create unbounded tasks/channels/retries or cross-session authority starvation.
- [ ] Shutdown/drain is bounded and cancellation-safe; already-authoritative reserved work is completed or durably/reconcilably preserved rather than silently dropped.

### Composition and evidence

- [ ] `lib.rs` composes exactly one Server Seam without changing foreign gameplay authority.
- [ ] `main.rs` preserves `--smoke` and may serve gameplay only from explicit valid configuration; no hard-coded production address/port/cert/key/secret/topology.
- [ ] `apps/game-server/tests/gameplay_server_seam.rs` traverses the actual local production listener/TLS/framing/FND-04/Foundation/Durability path for bootstrap/admission and resume/reconnect using non-shipping test material.
- [ ] Server Seam integration evidence is not relabeled as formal ADR-0007 QA Tier 1/Tier 2 proof.

### TDD and qualification

- [ ] Fresh RED precedes implementation for typed extraction/encoding, frame/TLS/ALPN/version/profile negatives, admission evidence/replay/binding/concurrency, reconnect authority family, resource saturation and shutdown preservation.
- [ ] Minimal GREEN stays within the exact allowlist; any required unowned path is `SHARED_LEASE_REQUIRED` rather than implicit expansion.
- [ ] Focused/package/workspace tests, fmt and strict Clippy pass on the stable candidate.
- [ ] Applicable current exact-head repository CI, including canonical PG/SIM behavior selected by protected-base policy, is green.
- [ ] Whole-diff self-review and complete authority finding-family sweep are clean with explicit P0/P1/P2 disposition.
- [ ] One genuinely independent exact-head deep review covers protocol/session/admission/reconnect/fencing/transport/resource risks.
- [ ] Zero unresolved required threads and no material head movement after final qualifying review.
- [ ] Worker returns `READY_FOR_INTEGRATION`; worker does not merge its own PR.

## Excluded scope

No gameplay command/state/event/capability/stable ID allocation; no Movement/Combat/Ability/Interaction/AI/Channel/Analytics semantics; no permanent Content format; no production bind address/DNS/port/cert/key/secret/environment/deployment; no QUIC; no new persistence semantics; no `workspace-boundaries.toml`, workflow/ruleset/protection/architecture-contract write; no Platform/Atlas/META/external write; no Reference-parity or production-readiness claim.

A material public API/schema/protocol/trust/fencing/persistence/resource/production decision is `ARCHITECTURE_ESCALATION_REQUIRED` before mutation.

## Implementation / findings

Implementation follows `docs/superpowers/plans/2026-08-24-oteryn-production-gameplay-server-seam.md` after the accepted #117 allocation and is now additionally gated by #313. The plan is subordinate to this task and accepted protected-main authority.

Before editing root/game-server Cargo files after resume, re-read active owners and #259/#260/#261. Do not absorb unrelated dependency upgrades. If a non-Dependabot writer acquires a shared path, stop with `SHARED_LEASE_REQUIRED`.

Historical allocation-review findings requiring golden encoding, listener version/profile negatives, replay/binding/concurrent admission and authoritative-work shutdown preservation are incorporated into the plan/task and must be preserved by implementation. The later ordering repair keeps production-listener version/profile cases in Task 5 after private prerequisites. The current repair `bc6578c77deb23150dad231955de0f6e2a7a0ae9` additionally makes Tasks 2-4 sequentially executable through private in-crate module wiring and requires explicit TLS 1.2-only + exact-ALPN rejection at the private transport boundary plus physical production-listener projection in Tasks 5-6. No public/test-only façade or API widening is authorized.

## Validation

### Focused

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam`
- result: not run — production composition is blocked on architecture prerequisite #313 and the missing canonical durable fresh-admission bridge

### Component/integration

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server`
- result: not run — production composition is blocked on architecture prerequisite #313 and the missing canonical durable fresh-admission bridge

### E2E

- scenario: local production-path TCP/TLS listener -> FND-02 bootstrap/resume -> FND-04/Foundation admission/reconnect -> bounded registered server acknowledgement/error path
- result: not run — production composition is blocked on architecture prerequisite #313; formal QA Tier 1/Tier 2 is separately owned

### Exact-head CI

- final head: null
- trigger source: future worker pull_request after #313 prerequisite implementation and branch reconciliation
- workflow/run/job: not started
- runner assignment: unknown
- classification: high-risk protocol/session/admission/fencing Server Seam
- result: not started

## Self-review

- exact head: null
- method/reviewer: `Oteryn: sol server seam lead` whole-diff adversarial review after resume
- material findings: not evaluated
- verdict: not evaluated

## Independent review

- required: `YES` — protocol/session/admission/reconnect/fencing plus production transport/resource boundary
- exact head: null
- method/auditor: genuinely independent exact-head deep review under current policy after resume
- material findings: not evaluated
- verdict: not evaluated

## PR and closeout

- changed-file review: exact owned-path allowlist only after resume
- unresolved review threads: not evaluated
- related/superseded PRs: none for Server Seam implementation; allocation PR #294 is terminal; Issue #313 is the current architecture blocker
- protected integration: worker does not perform terminal merge
- merge/result: control-plane responsibility only after `READY_FOR_INTEGRATION`
- ownership release: after protected-main worker merge/readback and task archival

## Context checkpoint

```yaml
last_progress: worker partial checkpoint 9370b254c6ac4f6529e069c1968ae6bfa1e1750e preserved; missing production durable fresh-admission/GameSession boundary verified and routed as architecture escalation #313; #247 and parent #162 synchronized by Work
status: WAITING_ARCHITECTURE
branch: agent/otv2-gameplay-server-seam-01
head_sha: 9370b254c6ac4f6529e069c1968ae6bfa1e1750e
head_reference_kind: current_branch_head_verified_by_work
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: not_requested
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
architecture_escalation_issue: 313
owner_action_required: null
blocker: architecture_issue_313_durable_fresh_admission_initial_GameSession_boundary
next_action: Oteryn: sol supervising architect resolves Issue #313 and returns a reviewed and qualified architecture decision integrated to protected main
```

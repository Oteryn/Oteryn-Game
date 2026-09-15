# Foundation Atomic Claim Transition Implementation Plan

**Goal:** Pair owner-authored claim changes with exact canonical fresh/session operations, preserving historical-only recovery and independently current adoption.

**Architecture:** Implement accepted `docs/architecture/reviews/OTERYN_GAME_ATOMIC_FRESH_CLAIM_PUBLICATION_DECISION_2026-09-06.md` without altering no-PREPARE/final-L semantics. Foundation owns sealed conditional effects; B persists them atomically; C binds actual sources. This plan does not grant a lease: `docs/agents/tasks/active/OTV2-20260906-foundation-claim-transition-326.md` and its protected allocation govern the sole writer.

**Tech stack:** existing Rust1.94.0 game-server/Foundation; no dependency changes.

## Constraints and file responsibilities

- `admission_authority_publication.rs`: sealed owner/transition construction, pure locked predicates, inert effect/historical types, ordinary-publication claim exclusion, narrow lifecycle siblings.
- `fresh_admission_durability.rs`: mandatory capability pairing, lossless operation receipt/replay/reconciliation and current adoption.
- `fresh_admission_durability_tests.rs`: independently controlled owner fixtures, negative matrix and existing regression preservation.
- All three runtime files are under `apps/game-server/src/foundation/`; only the allocated task is additionally writable. Facade/verifier/export, SQL, Cargo, workflows and production sources remain excluded.
- Source/decision/time provenance is never fabricated by a historical DTO or SQL adapter. Current source state is independent; prepared successor state is inert until COMMIT.
- Functional steps are serial because they share types. Read-only technical analysis and a separate final reviewer may run in parallel. No fresh worker or PR is created at a step boundary.

## Task 1 — sealed owner-prepared operation and standalone boundary

- [ ] Write RED for missing/mismatched owner transition and standalone claim acquisition/release; retain a valid independently sourced control case.
- [ ] Define a sealed owner capability and private-field transition prepared from exact authorization plus independent predecessor/successor rows. Enforce exactly two keys, expected predecessor equality, exact next publication CAS, strict source advancement/new bound decision, checked overflow and only authorized claim differences.
- [ ] Define lossless historical transition evidence without a live constructor. Expose immutable effects and a pure predicate over independently locked rows/current time. Keep remote security provenance unchanged except the allowed local wrapper.
- [ ] Separate ordinary publication from typed session effects: standalone CAS cannot acquire/release/advance claims. Bootstrap/eligibility/security publications cannot smuggle a holder change.
- [ ] Prove RED→GREEN and compile-fail raw/receipt/seal misuse. One negative per owner/key/purpose/source/CAS/binding/time/provenance invariant.

## Task 2 — fresh request, full operation identity and recovery

- [ ] Write RED for unpaired begin, altered transition completion/retry and historical evidence converted into a submit capability.
- [ ] Make `FreshAdmissionDurabilityFlowV1::begin` require matching authorization and transition; the private request carries both. Its final decision validates both against the same locked rows/time and returns the exact owner-authored successors.
- [ ] Introduce a nonrecursive historical operation envelope containing the existing authorization audit binding plus transition evidence. Use it for request identity, receipt restoration, exact/conflicting retry, completion correlation and original-binding reconciliation. Never replace an ambiguous operation with fresh source metadata.
- [ ] Update only owned test callsites. Replace handwritten `commit_sources()` acquisition with an owner-prepared capability whose submitted effects are applied by the fixture adapter on simulated COMMIT. Keep independent source mutations for adoption negatives.
- [ ] Preserve rollback/ambiguity/known-abort distinctions, same physical transport requirement, all current guard checks, original L and historical-only restart. Run existing fresh/reconnect regressions and doctests.

## Task 3 — explicit accepted lifecycle siblings

- [ ] Write RED for a stale session/generation releasing a successor claim and for terminal replacement changing claims outside its canonical operation.
- [ ] Add a claim-preserving predicate for accepted reconnect/control loss with unchanged holder/lease/source metadata.
- [ ] Add an owner-sealed terminal-replacement sibling bound to existing `TerminalGameSessionReplacementAuthorizationV1`, exact predecessor/candidate/current session and claim fences, with owner-authored successors.
- [ ] Add an owner-sealed terminal-release sibling bound to exact current session/character/lease/generation and canonical terminal effect; clearing both claims is inert unless the matching session transaction commits.
- [ ] Each sibling exposes immutable operation/effect evidence and a pure locked-state predicate; historical restoration never reconstructs authority. Use existing public getters; do not modify reconnect policy or add arbitrary transfer capability.
- [ ] Cover direct/reconciled/current-vs-historical and wrong-holder/lease/time/source cases. Document B's durable decision-reuse enforcement obligation and fail-closed unsupported operations.

## Task 4 — final qualification and handoff

- [ ] Run focused commands: `cargo +1.94.0 test --locked -p oteryn-game-server fresh_admission`; `cargo +1.94.0 test --locked -p oteryn-game-server admission_authority_publication`.
- [ ] Run component `cargo +1.94.0 test --locked -p oteryn-game-server`, doctests, fmt check and `cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings`; governance validates task metadata.
- [ ] Sweep every exposed sibling boundary and complete AuthorityInvariant × ConsumerBoundary × MutationOperator evidence. Preserve all prior passing tests; absence of a running PostgreSQL is not new SQL proof.
- [ ] Complete task metadata and full changed-content adversarial self-review before freeze; publish one exact candidate and request independent high-risk review. Material accepted findings get focused RED/GREEN and family sweep; rejected findings retain exact evidence.
- [ ] Work verifies scope/head/main/review threads and canonical CI, integrates through Merge Queue, reads protected main, archives/releases task, then freshly allocates B. No production readiness claim follows from this semantic followup.

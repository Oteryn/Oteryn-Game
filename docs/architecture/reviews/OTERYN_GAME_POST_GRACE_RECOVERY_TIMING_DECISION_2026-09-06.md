# Post-grace recovery timing representation

- Decision: `FND-DUR-POST-GRACE-TIMING-V1`
- Status: **CANDIDATE — acceptance requires independent review and protected integration**
- Source escalation: Issue #332; coordinator #162; affected Server Seam #247
- Immutable decision base: `5412215718d66c743fb78eadc561e6a23b5e2b5f`
- `MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`

## 1. Resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
main_sha: 5412215718d66c743fb78eadc561e6a23b5e2b5f
source_escalation: 332
blocking_question: How can a new eligible post-grace recovery attempt preserve predecessor continuity without inheriting its expired same-session authorization window?
facts:
  proven:
    - FND04B section 21 and recovery-grant profile sections 17 and 19 require eligible post-grace recovery into a new GameSession
    - ReconnectContinuityV1 requires prepared_deadline no later than original_grace_deadline
    - terminal replacement binds candidate continuity to the exact predecessor original grace
    - current authorization and V2 PREPARE still apply that original grace-bound attempt deadline
  derived:
    - every newly initiated post-grace candidate is stale under the current representation
    - terminal replacement before grace expiry does not prove post-grace recovery coverage
  unknown:
    - exact successor Rust spelling and additive physical representation under future allocation
  conflict: []
accepted_decision: FND-DUR-POST-GRACE-TIMING-V1, conditional on protected integration
rejected_options: [extend_predecessor_grace, drop_grace_for_all_reconnect, fake_fresh_admission, revive_old_session, new_loss_epoch_to_reset_budget]
affected_contracts: [FND-04B, FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1, DUR-RECONNECT-AUTHORITY-V1, DUR-TERMINAL-SESSION-REPLACEMENT-V1]
affected_paths: see_section_7
implementation_owner: separately allocated Foundation semantics then Durability adapter then Server Seam consumer qualification
implementation_scope: typed recovery timing successor and end-to-end post-grace authority qualification
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
supersedes: see_section_6_exact_bounded_interpretations
required_validation: see_section_8
required_independent_review: exact-head session/continuity/time/persistence review
next_action: Work independently qualifies this candidate through the normal protected lifecycle before implementation allocation.
```

## 2. Evidence, timing and options

At the decision base, `foundation/admission_recovery_inner.rs` contains the relevant `ReconnectContinuityV1::new`, `TerminalGameSessionReplacementAuthorizationV1::from_current_authority`, and `ReconnectDurabilityRecordV1::authorization_deadline`. `durability/mod.rs::prepare_new_candidate_attempt_v2` rejects database time after the prepared deadline. Thus `prepared_deadline <= predecessor_grace < now` excludes every valid new post-grace attempt. Historical committed reconciliation and fresh admission do not replace this policy path. This is inherited behavior, not a Foundation #326/#331 regression.

**Must decide now: YES.** The concrete blocked boundary is truthful positive post-grace coverage when qualifying Server Seam against accepted recovery policy. #326 and Child B remain independently useful and do not inherit this repair. Server Seam's current checklist names resume/reconnect broadly rather than explicitly proving this positive case; Work must reconcile its exact acceptance before a full recovery claim, not silently include or exclude the case.

**Selected option:** a versioned, closed timing representation separating immutable predecessor continuity from a newly authorized post-grace attempt deadline. It preserves existing same-session interpretation and reuses the accepted asynchronous PREPARE/COMMIT and terminal-anchor replacement boundary.

**Rejected alternatives:** extending or rewriting original grace would revive the old reconnect window; removing the cap from V1 would weaken ordinary reconnect; fresh admission would reset or duplicate an existing actor and use the wrong grant; creating a new loss epoch solely for recovery would reset budget/protection. A separate unbounded lease/window is unnecessary.

**Costs/risks:** versioned records, caller plumbing and additive migration are required; accidental fallback between timing variants is the highest risk. Benefits to players are control recovery without respawn/heal/state reset; producer cost stays bounded to the existing recovery family. No new duration, timer cadence, lease or resource ceiling is chosen.

**Supersession evidence:** independently reviewed timing/actor/replay failure evidence or a smaller equivalent representation proving the complete post-grace path. **Not decided:** same-session grace duration, failure detection, new epoch/product policy, protection entitlement creation, source availability, production deployment or broader recovery redesign.

## 3. Closed timing successor

Introduce a versioned successor record/request family with a mandatory closed discriminant; exact Rust names are implementation-owned:

| Variant | Historical continuity | Current authorization window |
|---|---|---|
| `SameSession` | Existing loss epoch and original same-session grace | Existing finite prepared deadline capped by original grace; all V1/V2 behavior preserved. |
| `TerminalSessionPostGrace` | Exact predecessor session, original grace and loss epoch retained as immutable historical/current-actor binding | New finite attempt deadline derived from this newly verified recovery grant and evidence; original grace is not its upper bound. |

The new variant cannot be selected by peer payload, historical DTO or a caller flag. Foundation constructs its live authorization only from verified **reauthenticated recovery** material plus independently current terminal predecessor, actor/presence/lease/scope/placement/revision evidence. It requires a distinct candidate GameSession and the accepted no-current-controller/`PRESENT_UNCONTROLLED` conditions. Reconnect proof alone cannot create it. Foundation and Durability reject missing/unknown variants; no default or V1 interpretation of the new variant is permitted.

For `TerminalSessionPostGrace`, trusted current time must be strictly after the exact predecessor original grace at new attempt eligibility and at its final current authorization. Boundary equality remains subject to the existing same-session/terminal path; this decision does not change equality semantics for those paths. Existing authoritatively terminal replacement before grace expiry remains on the existing representation and is not relabeled post-grace.

Retain predecessor connection generation separately from the new session's initial connection generation, which remains exactly one. Preserve actor identity/state, current CharacterLease, current RuntimeScope and current AccountPresence binding. If changing the durable session holder requires a Game-owned claim transition, the owning sealed transition commits with corresponding canonical session effects under #325; historical timing data is not a source publication capability.

## 4. Derive and freeze this attempt's deadline

At preparation, Foundation derives the immutable finite **attempt authorization deadline** from the minimum of the already accepted recovery credential validity deadline and applicable current Platform-security and signing-trust evidence deadlines. Use the existing accepted time semantics, uncertainty and checked arithmetic. No fresh grace duration or resource number is introduced. If a separately accepted prepared-attempt bound is shorter, it may further restrict the deadline, never extend it. Missing, unprovable, overflowed or already expired bounds fail closed.

The original predecessor grace remains mandatory equality/provenance data; it is not included in this new variant's deadline minimum. The newly derived deadline must be later than the historical grace and valid at preparation, otherwise this is not a usable post-grace candidate. It is not permission to extend any old attempt. The old session remains terminal and its old prepared candidates remain noncommittable.

The resulting deadline and timing discriminant are immutable parts of the new attempt/replay binding. Reusing the same attempt or reconciling an ambiguous result cannot change them. Refreshing source observations later cannot extend that frozen deadline. Final COMMIT also rechecks independently current credential/security/trust and actor/fence facts; newer restrictions can reject an otherwise unexpired attempt. PREPARE is not authorization escrow. Existing accepted source freshness, nonce and compatibility checks remain binding at each boundary.

Durability validates trusted current database time against the correct variant's fixed deadline in PREPARE and final COMMIT after the necessary potentially blocking serialization acquisitions, holding protections through the atomic effects. A queued or lock-delayed request can expire; BEGIN time or a previously sampled process clock does not authorize later mutation. The existing reconnect final-authorization protocol remains the boundary; this decision does not transplant fresh admission's separate `L` rules or create a second authorization decision after commit.

A timed-out known-uncommitted attempt becomes its existing typed terminal outcome. A new attempt needs a new allowed attempt identity, newly verified recovery evidence and all current facts, subject to the retained epoch budget. An ambiguous COMMIT reconciles the original exact binding before another candidate; durable historical success can be reported without reacquiring authority.

## 5. Continuity, lifecycle and replay

Retain the exact predecessor control-loss epoch, original grace, actor association and existing protection entitlement state. The new session does not mint or re-arm protection. An eligible unused entitlement may be activated only under the accepted FND-04 conditions, once; an absent actor retires old eligibility and cannot use this path. No gameplay reset, relocation, respawn, heal or committed-effect rollback is permitted.

Old **session terminality** is not by itself proof that the same still-present uncontrolled actor's retained loss-epoch budget/protection evidence can be erased or recreated. Where post-grace recovery is still eligible, carry the existing epoch's attempt high-water/count and dispositions across the candidate session. Preserve the accepted eight-distinct-attempt maximum and exact retry semantics; a new GameSession is not a new quota namespace. Previously terminal attempts stay terminal. If prior compaction/restart leaves the remaining budget or entitlement unprovable, reject until authoritative continuity is reconstructed; never reset it to make recovery possible.

The accepted terminal predecessor/candidate actor-anchor CAS remains mandatory. All corresponding claim/session/attempt/nonce/transport/protection effects commit in their accepted owning transactions or roll back. No transport/controller is installed from tentative preparation. Competing candidates have one winner; actor disappearance, healthy controller return, lease/scope replacement, ownership/world/revision change or security/trust denial before the final decision rejects without success authority.

Completion and restart independently resolve current authority. Historical receipts prove the original result, not current actor presence or controller ownership. A replacement subsequently superseded, terminalized or made ineligible cannot reinstall authority through reconciliation. The new candidate's successful control restoration closes the retained loss epoch under existing semantics; later real control loss follows the existing epoch-creation rules rather than inheriting a reusable post-grace privilege.

## 6. Exact compatibility and supersession

Preserve FND-04B §21 and recovery-grant profile §§17/19 without policy changes. Preserve `DUR-TERMINAL-SESSION-REPLACEMENT-V1`'s terminal authorization, actor-anchor CAS and typed collision reconciliation.

Narrow supersession of `DUR-RECONNECT-AUTHORITY-V1`:

- its §5 record/§6 Phase B deadline interpretation and §7 COMMIT original-grace requirement remain binding for **same-session reconnect**, but do not upper-bound the explicitly typed new terminal post-grace attempt;
- its §8 continuity rule retains the original epoch/grace as immutable historical facts; it does not prohibit a separate current recovery-attempt deadline;
- its §9 phrase closing an epoch on “terminality” cannot mean that terminality of only the predecessor GameSession erases retained budget/protection evidence while the same actor remains eligible for accepted post-grace recovery. Actor/epoch retirement and successful control restoration retain their accepted finality.

Old V1 records and current V2 request/completion behavior are not rewritten. Decode them with their original rules; they cannot be upgraded from persisted fields alone into new live post-grace authorization. Stored old post-grace failures remain failures. Historical successful pre-grace terminal replacements remain historical successes.

The implementation adds an explicit storage version/discriminator and truthful fields through the next forward migration after fresh schema readback. Released migrations, original grace values and existing receipt identities remain unchanged. Mixed versions fail explicitly; an old reader/writer must not treat the new variant as ordinary reconnect. Rollout is producer/reader-compatible before enabling the new path. Rollback disables new attempts and preserves new-version rows for supported reconciliation; it does not downcast, delete history or restore predecessor control. Exact migration and API names require the later allocation, not a schema write in this decision.

## 7. Prospective paths and sequencing

Architecture custody is exactly this document and `docs/agents/tasks/active/OTV2-20260906-post-grace-recovery-timing-332.md`. No runtime or lifecycle lease is granted.

After protected acceptance, Work must create bounded implementation allocations, with exact filenames determined from live source and path ownership:

1. Foundation successor timing/authorization and direct/reconciled flow in `apps/game-server/src/foundation/admission_recovery_inner.rs`, with its actual owning tests and any proven necessary facade/verifier bridge explicitly allocated. Preserve #326's separate scope and qualify its integration independently.
2. Durability successor persistence and all readers/writers in `apps/game-server/src/durability/mod.rs`, `admission_journal.rs`, schema/migration surfaces and `apps/game-server/tests/durability_postgres.rs` plus actual test support. Serialize with B and use the next forward migration; no worker acquires B paths through this decision.
3. Server Seam's actual recovery consumer/tests and acceptance record must name positive post-grace coverage before a complete accepted recovery claim. Preserve branch/head `9370b254c6ac4f6529e069c1968ae6bfa1e1750e`; only Work resumes/amends it after its prerequisites.

Any additional path requires demonstrated need and explicit allocation. Source readiness, native producer work, character bootstrap and production authority remain separate. Architecture acceptance does not implement this path or retroactively invalidate unrelated merged functionality.

## 8. Required qualification

Use `AuthorityInvariant × ConsumerBoundary × MutationOperator`, independent current sources and one changed invariant per negative case.

- **Positive:** initiate a newly verified recovery after actual predecessor grace expiry; prove old session terminal, same actor uncontrolled, new session generation one, unchanged actor/epoch/lease/protection state and canonical successful control restoration. Run both direct and lost-response/restart recovery through the actual PostgreSQL adapter.
- **Timing:** same-session request after original grace rejects; equality behavior preserved; changing only old historical grace fails binding; expiry during queue/each blocking acquisition rejects; source/credential bounds and arithmetic overflow reject; refreshing evidence cannot extend the same attempt; altering timing variant/deadline under the same replay identity conflicts.
- **Authority:** active/reconnectable predecessor, reconnect-proof substitution, healthy controller, absent/different actor, wrong account/world/lease/scope/generation/revision and revoked/stale trust each reject independently. Historical DTOs cannot create the live variant.
- **Continuity:** no protection re-arm/reset; consumed entitlement remains consumed; retained attempt eight/nine behavior spans session replacement; same-attempt retry consumes no new slot; old prepared attempts cannot commit after terminality; missing compacted continuity fails closed.
- **Durability:** simultaneous replacement has one winner; failure/rollback preserves accepted authority; ambiguous COMMIT reconciles original binding; current-state changes after success prevent stale adoption; transport global uniqueness and recovery nonce exactly-once remain intact.
- **Compatibility:** old V1/V2 records and early-terminal replacement regressions; unknown version; migration reload; supported mixed-version rollback without downcast; non-production synchronous compatibility cannot masquerade as the durable post-grace path.

Actual PostgreSQL cases must execute in the configured integration harness. Fixtures/internal tests without database execution cannot prove this path; Server Seam evidence must traverse its real production-shaped listener/authority/adapter boundary when that consumer is allocated. No new numeric timing fixture is a product grace policy.

For this docs-only candidate: local governance/whitespace, whole-candidate adversarial self-review, genuinely independent exact-head review, selected canonical CI and protected integration. Runtime/E2E is NOT_APPLICABLE to these edits, and mandatory later at the named implementation boundaries.

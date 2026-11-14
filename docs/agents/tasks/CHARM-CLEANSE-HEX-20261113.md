# CHARM-CLEANSE-HEX-20261113

```yaml
task_id: CHARM-CLEANSE-HEX-20261113
title: Reach canonical Hex from the existing Cleanse charm consumer
type: mechanic
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/cleanse-hex-fix-20261113
pr: null
base_sha: 3e168ce6afe374485da20ee0ac13d3749558e2f8
owner: /root/confirmed_charm_fixes
created_at: 2026-11-13T13:05:14Z
updated_at: 2026-11-13T13:05:14Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/foundation/src/foundation/condition.rs
  - crates/gameplay/foundation/src/foundation/condition_tests.rs
  - crates/gameplay/entities/src/carrier/runtime_actor_conditions.rs
  - crates/gameplay/dispatch/src/gameplay_transport/actor_conditions.rs
  - crates/gameplay/dispatch/src/gameplay_transport/actor_charm_cleanse_tests.rs
  - docs/agents/tasks/CHARM-CLEANSE-HEX-20261113.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

Owner requests confirmed Cleanse/Hex correction with offline validation and repo
publication, without gameplay, deployment or merge. Allocation and path extension:
https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6853445838 and
https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6853548836.
Reset PR #3174 and evidence PR #3172 are separate; this branch starts on main.

## Architecture and source of truth

PROVEN primary sources: Tibia news #8935 (2026-08-25) grants Hex removal and
corresponding immunity; #8960 (2026-09-08) confirms immunity to hex spells.
The current official characters manual, section 5.1.10, identifies Intense Hex as
one of three Hex variants and describes reduced damage and healing.

- https://www.tibia.com/news/?id=8935&subtopic=newsarchive
- https://www.tibia.com/news/?id=8960&subtopic=newsarchive
- https://www.tibia.com/gameguides/?section=characters&subtopic=manual

PROVEN code trace on base main:

1. `entities/src/carrier/soulpit/oppressor.rs` creates a three-second
   `ConditionValues::IntenseHex` (50% damage and healing) and prepares/publishes
   it into the canonical actor's `lifecycle.conditions` store.
2. `ConditionType::IntenseHex` has its own conflict key and status projection 23;
   it is not `Gate(Powerless)`. Generic `is_negative` does not include it.
3. Foundation `prepare_cleanse` previously filtered exclusively through that
   generic negative tag. The standalone Hex regression therefore returned None.
4. The real charm consumer, `ChannelSpellStates::apply_player_charm_condition`,
   previously cloned only `PlayerSpellState::owned_conditions`. Its `get` reads
   a separate player-state collection, so a predicate-only fix could not reach
   the canonical boss condition.

The repair adds a Cleanse-specific Hex exception, without broadening generic
negative dispels. It draws once through existing `COND_CLEANSE_PICK` across live
canonical and player-state candidates, native sequence first. The selected store
receives an already drawn index; it cannot draw again or remove a second status.

The canonical selected transition reuses current actor/session/time/reservation
checks. A player-state selection uses existing
`commit_actor_condition_with_vitals` to couple its compare-commit to the native
occurrence receipt; a refusal publishes neither side. Retrying the same retained
occurrence cannot fall through into the other store. The existing 11-second
conflict-key immunity and expiry machinery remain the owner of reapplication.

## Reference parity

| Behaviour | Reference | Oteryn/check |
|---|---|---|
| Cleanse can remove represented Hex | Official #8935 and manual Hex variants | Canonical Intense Hex reaches the actual charm consumer |
| Immunity follows removal | Official #8935/#8960; existing CONDITIONS-0 11 s policy | Refusal immediately before expiry, admission exactly at expiry, fresh reproc |
| One random condition, one draw | Accepted CONDITIONS-0 section 5 | Named draw over combined candidates; deterministic mixed-store cases |
| Replay does not remove another status | Existing owner receipt and forward-only policy | Canonical and mixed-store retry assertions |
| Other negative cures remain unchanged | Existing generic negative policy | Hex remains outside `cleanse_negative`; Agony/gates remain excluded |

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: [exact_actor_session, live_player, no_pending_spell_reservation, monotonic_time, canonical_revision_receipt]
consumer_boundaries: [apply_player_charm_condition, prepare_actor_condition, commit_actor_condition_with_vitals]
mutation_operators:
  applicable: [mismatched_session, replayed_occurrence, exact_immunity_deadline]
  considered_not_applicable: [credential_changes, persisted_controller_recovery, external_provenance_substitution]
one_invariant_per_negative_case: true
independent_current_fact_sources: [runtime_actor_slot, player_spell_state]
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: [native_cleanse, player_state_cleanse, generic_negative_dispel]
  protocol_versions: not_applicable_no_wire_change
  direct_and_reconciled_paths: [canonical_actor_conditions, player_spell_state_conditions]
  fenced_durable_writes: not_applicable_no_durable_change
  restart_retry_replay_concurrency_pg_reload: retained_owner_receipt_and_current_session_checks
  evidence: [canonical_hex_consumer_test, mixed_store_named_draw_replay_test, existing_condition_owner_tests]
finding_dispositions:
  p0_p1_accepted_and_repaired: [hex_cleanse_eligibility, canonical_store_unreachable_by_charm]
  p0_p1_rejected_with_exact_evidence: [agony_cleanse_excluded_by_existing_negative_tag]
  p2_fixed_accepted_or_deferred: []
```

Existing authority fences are preserved: exact actor and session, live player,
pending spell reservation, monotonic semantic time and canonical revision/receipt.
Mutation operators are the existing native selected-condition plan/commit and
player-state compare-commit, coupled through the already accepted immutable
callback boundary. No network, durable schema, recovery controller or production
permission is introduced. Wrong-session and same-occurrence negative cases keep
other facts unchanged; existing owner suites cover stale-plan/reservation checks.

## Excluded scope and closure

This closes the concrete gap for the represented Intense Hex path; it does not
invent Lesser/Greater Hex producers or claim all boss-specific mechanics exist.
No Gate, Agony, generic dispel tag, charm chance, bonus magnitude, source content,
wire schema or dependency is changed. Historical immunity replacement, Parry,
Low Blow, leech and Carnage questions remain in the separate evidence audit.
No mechanics-ledger promotion is claimed.

## Validation

- RED: Foundation Hex regression applied the real family but Cleanse returned None.
- RED: actual consumer returned false with canonical Intense Hex present; no removal occurred.
- GREEN: Linux `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib charm_cleanse_ --quiet`: pass, three matching tests, including both new consumer tests and 16 deterministic mixed-store cases.
- Windows dispatch test attempt cannot compile the existing Linux-only recovery-fence dependency (`rustix::fs`, `std::os::fd`). Linux checks use local WSL UbuntuNew, not Codex Cloud.
- Linux `cargo fmt --all --check`: pass. Windows formatting command-line limits do not apply to this result.
- `python tools/agents/validate_governance.py`: pass; `python -m unittest discover -s tools/agents/tests`: pass, 129 tests.
- Linux `cargo clippy --locked -p oteryn-gameplay-foundation -p oteryn-gameplay-entities -p oteryn-gameplay-dispatch --all-targets --all-features --quiet -- -D warnings`: pass.
- Linux `cargo test --locked -p oteryn-gameplay-foundation -p oteryn-gameplay-entities -p oteryn-gameplay-dispatch --all-features --quiet`: pass, Dispatch 3,517 passed/16 pre-existing ignored; Entities 263 passed; Foundation 382 passed; 37 doc tests passed, no failures.
- Bounded `git diff --check`: pass.
- No game execution, merge, deployment or independent review trigger.
- Full external Platform node-boot/Server Seam qualification was not run; the actual condition consumer is exercised offline and exact-head CI/review remain integration requirements.

## Acceptance criteria

- [x] Full source trace identifies the actual store boundary and official Hex mapping.
- [x] Eligibility regression observed RED before its repair.
- [x] Actual-consumer and mixed-store offline regressions pass.
- [ ] Required checks complete on the candidate; CI and independent review qualify integration separately.

## Self-review

Reviewed the bounded diff for one draw/removal, replay and partial-publication
safety, wrong actor/session refusal, exact immunity deadline, and absence of
generic Gate or negative-dispel broadening. No known material self-review finding
remains. Independent review is separate and pending; retain the exact head in
the PR receipt rather than creating a self-referential commit.

# OTV2-20261031-hireling

```yaml
task_id: OTV2-20261031-hireling
title: E12 Hireling identity, services, Store and native House lifecycle
type: mechanic
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/hireling
pr: 2783
base_sha: ae4188a9c8d461acb152044814ea23075de02ed4
head_sha: 4640be635826625265a45b9ae4f019a27f597972
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT coordinator /root
created_at: 2026-10-31T18:12:06Z
updated_at: 2026-11-01T08:56:23Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/
  - crates/protocol-oteryn/
  - apps/game-server/migrations/0700_hireling_identity.sql
  - apps/game-server/migrations/0701_store_hireling_delivery.sql
  - apps/game-server/migrations/0702_hireling_services.sql
  - apps/game-server/migrations/0703_hireling_lamp.sql
  - content/
  - imports/
  - rulesets/economy/store/hirelings.json
  - tools/content-schema/
  - tools/content-migration/
  - docs/contracts/
  - docs/agents/evidence/
  - docs/agents/tasks/OTV2-20261031-hireling.md
public_contracts:
  - docs/contracts/gameplay/hireling-house-position.md
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Active implementing packet. The recorded 4640be6 fields identify the historical authoring parent. Published implementation identity is authoritative in [PR2783 commit metadata](https://github.com/Oteryn/Oteryn-Game/pull/2783/commits); this file cannot embed its own commit SHA. Accepted code/local qualification is complete; Q17 keeps the task unarchived.

## Outcome

Extend native E12 lamps, House placement/return/recovery, entitlements/appearance, all four services and Store; compose House movement/position/admission/release. Accepted-code local gates are complete; Q17 remains unresolved.

## Architecture and source of truth

- PROVEN: existing native owners and pre-change RED evidence.
- PROVEN: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`; NPC SHA256 `d507c31db74a3558d27bfec6bd08a45076742e6f5ea6da5f5d2baccfc08faf5f`.
- PROVEN: independent current session/lease/House authority; immutable receipt provenance/replay.
- DERIVED: existing owners/codecs; no second currency/item authority or fake Channel placement.
- UNKNOWN: Q17 Familiar decision and post-publication CI; node-boot has a separate Platform prerequisite. Accepted-code local qualification passed.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - current Character/session/connection/lease and independent House/node generation
  - locked ACL/entitlement for services; custody proof for terminal saves
  - immutable input/item/command provenance and once-only truthful outcome
consumer_boundaries:
  - real USE/management/220/NPC/Store/stash/wrap ingress
  - native PREPARE/COMMIT, deferred proofs, audit/replay
  - transfer/loss/reconnect/terminal/fresh admission
mutation_operators:
  applicable:
    - missing/stale current authority or changed identity/binding/provenance
    - full custody, expired grace/cooldown and non-monotonic saved state
    - unknown COMMIT, contention, retry/replay and process/ACK loss
    - stale authority or clock reset during genuine handoff
  considered_not_applicable:
    - generic Instance commerce and synthetic session recovery are outside scope
one_invariant_per_negative_case: verified by recorded scoped unit and native PostgreSQL cases
independent_current_fact_sources:
  - native session/claims/node assignment
  - locked House property/ACL/entitlements
  - Item custody, content activation, runtime lease/map/collision
record_derived_matching_helper:
  allowed_for_positive_happy_path: expected immutable binding only
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: reviewed; recorded local qualification passed
  protocol_versions: old Channel bytes retained; additive House fields
  direct_and_reconciled_paths: native commit/pending/replay
  fenced_durable_writes: independent current proof
  restart_retry_replay_concurrency_pg_reload: verified by recorded PG26 cases
  evidence:
    - docs/agents/evidence/chatgpt-hireling-tests.md
    - docs/agents/evidence/hireling-baseline.md
    - docs/agents/evidence/hireling-store-trade-projection.md
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - baseline Query/Dismiss/House routing/Bank
    - unknown native outcome and paid Store retry
    - transfer generations and destination loss/reconnect history
    - complete admission/release and handoff data/clock continuity
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [x] Frozen-base summon/query/dismiss/Bank RED evidence predates production edits.
- [x] Real-wire/PostgreSQL cases prove current fences, once-only effects and uncertain/replayed outcomes.
- [x] Source regeneration/pins and bounded audits pass.
- [x] Accepted admission/release/recovery preserves native participants, clocks and started namespace.
- [ ] Resolve Q17; retain accepted exclusions.
- [x] Local gates/review recorded; published identity belongs to PR commit metadata. Keep draft without integration.

## Reference parity

The pinned Crystal revision and primary NPC hash above remain authoritative. The complete donor file:line, economic values, native owner and test mappings are in the [PR2783 parity table](https://github.com/Oteryn/Oteryn-Game/pull/2783) and `docs/agents/evidence/hireling-store-trade-projection.md`. Source spans include Store `gamestore/init.lua:1475–2062`, NPC packages `1821–2033,2304–2324`, unpriced item 5807 at line 1691 and the lamp/food action files. The referenced test names do not imply final-head qualification.

## Accepted decisions and excluded scope

Q1–Q16 are settled by the [binding PR answers](https://github.com/Oteryn/Oteryn-Game/pull/2783); the evidence file above retains each answer link and exact deviations. In particular: character-wide entitlements; free `various`; all packages/paid Store types; bounded paging/physical payout; valid legacy migration; dish 29409; closed Item compatibility pairs; native House STEP/ordered position and unchanged command continuity.

Left out / unresolved:

- Q6B: production Store activation remains separate. Tests use existing `StoreClient`/`StoreTransport` with a deterministic producer, not production mTLS connectivity/credentials.
- Q13B: general LootPouch producer/custody and bounded all-loot sweep; virtual 23721 gets typed refusal.
- Q9: malformed historical placements need authentic receipts/manual repair; no fabricated provenance.
- Native client management UI/activation is separate; capability 250 is bundle-world only.
- Q17: Familiar in named Houses remains unanswered. No new companion lifecycle or fake placement is assumed.

Q18 access is restored; interrupted PG21 remains UNKNOWN. No merge/rebase, review request, heavy-tests label, deployment or DONE.

## Implementation / findings

Migrations 0700–0703; capability 250 (requires 165), command 470/order 610. Capability 251, commands 471–473 and orders 611–613 are unused. Audit causes 470/471/472 are separate; no new resource IDs.

Closure: 1675 rows/1409 IDs, 23 packages/69 ingredients, 1425 captured/2104 active profiles; 4,712,466-byte JSON < 8MiB. Preserve old Channel encodings.

Authored repairs cover admission/release, clocks, command 0 ACK and exact handoff capability retention. Store154 remains fixture-only. Default validation cleared nine stale-artifact E0433 errors by timestamp-only invalidation with unchanged source SHA, then fixed two real test-only aliases by importing native `spell_character_facts`; no feature activation. Results qualify their recorded checkpoints.

## Validation

### Focused

- Frozen baseline: 1 passed / 7 expected failures / 0 ignored; RED evidence.
- Prior passes: source/profile/binding, Foundation 346, Bank/pending-lamp PG18, ingress 4.
- PG24: PositionCapture PASS 16.46s; LogoutFresh PASS 18.18s.
- PG25 House Store: 1 passed / 0 failed / 0 ignored, 15.72s, native completion marker; its earlier capability failure is repaired.
- Entities corrected UUID fixture: 200/200 PASS.

### Component/integration

- Pre-repair 11-package units: 4532 passed / 1 failed / 16 existing ignored. Latest per-package `--lib` sum: 4533 passed / 16 existing ignored, not one fresh combined run.
- PG23: 35 passed / 3 failed. PG25 full Hireling: 38 passed / 0 failed / 0 ignored, 179.34s; 20 lifecycle + 10 Store markers prove 30 real PG + 8 units. Full Dispatch: 1760 passed / 0 failed / 14 existing ignored, 157.79s. PG25 script exit 0. PG26: 38 passed / 0 failed / 0 ignored, 283.40s tests / 501s overall, identical 38 names and 20+10 native completion markers.
- Clippy: PASS exit 0, 136s, all 11 packages `--all-targets --all-features -- -D warnings`.
- Architecture: PASS 13s; client registrations: 2/0/0, 0.03s tests; doctests: 48/0/0 across 11 packages, 93s overall.
- Content REST: 123/0/0, 2.19s tests/110s overall; `content_world_project_repository`: 5/0/0, 57.89s tests/74s overall.
- Current release `pin-check`: PASS exit 0, 29s. Data/generators are unchanged since successful Python checks; digests are in PR/evidence.
- Governance: PASS (22 policy docs, 9 lanes), governance tests 129 PASS. Final fmt PASS 10s and final diffcheck PASS 0s.
- Fresh numbers against main 03d1a55d06df29f4f110c768364e0e397266d667: exit 1 for unrelated PR2789/2780 overlaps migrations 716–718/capabilities 258–259. Our 0700–0703/250–251/470–473/610–613 block is clear; no global PASS. Main maxima: migration 662 / capability 240 / command 447 / mechanic 587 / offer 560.

### E2E and exact-head CI

- Default `final-smoke-qualified`: PASS exit 0, 99s, actual server `process_start` recorded.
- Separate node-boot: BLOCKED exit 2 `exact_platform_checkout_missing`; later preconditions were not reached.
- Whole qualification script: PASS exit 0, 936.61s. Published head: authoritative PR commit metadata. No CI-green claim; CI trigger/run/job/runner/classification/result remain unreported.

## Self-review

- Coordinator accepted-scope self-review: PASS; no known unaddressed P0/P1.

## Independent review

Required YES: authority/value/recovery. Internal review is complete, including the final three deltas; PASS with no known unaddressed P0/P1 in accepted scope. No CI-green claim.

## PR and closeout

- PR2783 stays DRAFT/implementing because Q17 is unresolved. Accepted code and local gates are qualified; no archive/DONE.
- No integration/ownership release. PR commit metadata records the published SHA after commit.

## Context checkpoint

```yaml
last_progress: PG26 and local qualification passed; published identity lives in PR metadata; Q17 unresolved
status: implementing
branch: chatgpt/hireling
head_sha: 4640be635826625265a45b9ae4f019a27f597972
pr: 2783
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
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: Q17 Familiar decision on PR2783
blocker: Q17 unresolved; separate node-boot Platform prerequisite; CI status not claimed
next_action: publish and track identity via PR metadata; obtain Q17 answer; observe actual CI
```

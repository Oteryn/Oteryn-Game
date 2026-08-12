# OTV2-20260812-game-channel-01-architecture

```yaml
task_id: OTV2-20260812-game-channel-01-architecture
title: GAME-CHANNEL-01 multichannel product policy architecture
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: agent/otv2-20260812-game-channel-01-architecture
pr: 209
base_sha: f89685d585aab9c2ed3c69cabe8a4d9a2544bd0e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: architecture-coordinator/current-session
created_at: 2026-08-12T16:16:00+02:00
updated_at: 2026-08-12T17:12:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260812-game-channel-01-architecture.md
  - docs/architecture/GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_ANALYSIS.md
  - docs/architecture/GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md
public_contracts:
  - GAME-CHANNEL-01
depends_on:
  - ADR-0001
  - ADR-0009
  - GAME-VISION-01
  - FND-03
  - FND-04
  - GAME-CHAR-01
  - GAME-ITEM-01
  - DUR-03
blocks:
  - multichannel product feature semantics
  - VSL-MULTICHANNEL-01 product-policy readiness
  - profile-specific channel-switch / anti-hopping policy closure
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Produce one bounded paper-only `GAME-CHANNEL-01` candidate defining player/economy/social/PvP/lifecycle policy for the accepted multichannel technical model without redefining FND runtime/admission, DUR conservation, PERF/OPS implementation/capacity or downstream gameplay domains.

No runtime/client/Rust implementation, PostgreSQL DDL/migration execution, Platform write, production configuration or entitlement activation is authorized. Shared canonical status/register/horizon/index/handoff remain untouched while PR #209 is open; acceptance promotion is a separate post-merge closeout.

## Source truth

- `PROVEN`: post-DUR-03 `main@f89685d585aab9c2ed3c69cabe8a4d9a2544bd0e` exposes `GAME-CHANNEL-01 = PROPOSED / PLANNED / NOT_STARTED` as the earliest remaining owner-accepted ordered paper-only product gate.
- `PROVEN`: FND-ID/ADR-0001 own Channel identity, one-World community/economy and fresh-session Channel-change semantics.
- `PROVEN`: ADR-0009/FND-03 own runtime lifecycle/capacity/same-Channel recovery and prohibit silent alternate-Channel failure relocation.
- `PROVEN`: FND-04 owns exact target-bound final admission and no silent grant retarget.
- `PROVEN`: GAME-VISION binds party/PvP/economy fairness direction; DUR-03 binds value conservation/idempotency/stale-authority safety.
- `PROVEN AS HISTORICAL REPOSITORY EVIDENCE`: `GAME-CHANNEL-01_PREDECISION_CAPACITY_TRIGGERS_ADDENDUM.md` and `GAME-CHANNEL-01_PREDECISION_ANALYSIS.md` provide qualitative lifecycle framework input. Their historical numeric thresholds/percentages/windows/technology assumptions remain nonbinding and are not adopted by this candidate.
- `UNKNOWN`: exact PvP/boss/reward/source formulas and all numeric cooldown/queue/channel-count/capacity/scaling thresholds remain downstream/evidence-owned.

## Candidate closure

The candidate freezes:

1. one World economy/community/profile boundary with canonical `ChannelRef=WorldId+ChannelId`; labels are presentation only;
2. current eligible directory + non-authoritative recommendation + explicit eligible target;
3. fail-closed stale/full/draining/recovering/incompatible target with fresh offer/grant and no silent retarget;
4. optional bounded target-Channel pre-admission queue, never GameSession/lease/value authority;
5. no first-generation destination queue/reservation while Character remains authoritative in source Channel;
6. privacy-bounded party/friend co-location hints with independent admissions and no party-owned Channel;
7. same-Channel reconnect distinct from completed fresh-session Channel switch;
8. accepted combat/trade/DUR/event/instance/authority switch blockers;
9. durable GAME-CHANNEL/world-policy anti-hopping guard scoped Character+World, surviving GameSession/relog/reconnect/restart;
10. remembered prior successful ChannelRef (or equivalent) so logout/relog cannot make switch classification ambiguous;
11. time-based voluntary cooldown + hard locks, numeric duration deliberately deferred;
12. destination admission + remembered Channel/guard advancement as one recovery-safe authoritative semantic outcome;
13. no mandatory new ChannelSwitchId unless later lifecycle evidence requires one;
14. trusted non-voluntary exceptions only typed/audited/unforgeable and consequence-preserving;
15. fail-closed explicit multiplicity classification for every channel-sensitive value-producing source; runtime locality is not a durable-output fallback;
16. classes `CHANNEL_LOCAL_REPEATABLE`, `CHANNEL_LOCAL_SHARED_ELIGIBILITY`, `WORLD_SCOPED_UNIQUE`, `EXPLICIT_EVENT_POLICY_REQUIRED`;
17. distinct event simulation scope and durable eligibility scope;
18. no hidden inverse rate tuning; GAME-CHANNEL product envelope / PERF numbers / OPS orchestration;
19. local PvP execution with world/Character consequences surviving switching; exact formulas downstream;
20. one-World guild/economy/rankings/accepted cross-channel communication with local speech/combat/position;
21. product states `SELECTABLE`, `CAPACITY_LIMITED`, `DRAINING`, `RECOVERING`, `UNAVAILABLE`;
22. same-Channel recovery first; alternate Channel only after safe actor state + fresh admission;
23. existing `world_policy_revision` for Channel-policy compatibility;
24. WorldId change is separate world lifecycle/transfer, not Channel switch;
25. qualitative Channel lifecycle trigger vocabulary `DEMAND_PRESSURE`, `RECOVERY_PRESSURE`, `LOW_LOAD_CONSOLIDATION_CANDIDATE`, `CHANNEL_UNHEALTHY`;
26. new semantic Channel creation only from sustained demand or bounded recovery-capacity need, never party preference, rare-spawn farming or PvP avoidance alone;
27. demand-driven create requires no healthy eligible existing Channel can satisfy service objective/headroom, policy compatibility, safe resources and Ready-before-selectable;
28. recovery pressure preserves ADR-0009 same-ChannelId recovery for affected actors; a new ChannelId may provide general capacity but cannot be silent continuation of failed Channel A;
29. low-load drain requires retained capacity/headroom, no world/event/reward/recovery policy need, blocked new admissions/switches and safe incumbent exit;
30. drain abort/hold if demand returns, retained capacity becomes unhealthy, recovery pressure appears, policy requires Channel or continuing drain threatens session/item/economy/recovery correctness;
31. terminal retirement only with no sessions, no dependent instance/encounter/event/transaction/checkpoint/recovery obligation, completed evidence finalization, fenced old owner and invalidated stale routing/queue/admission references;
32. temporary stop is not retirement; same semantic Channel reactivation preserves ChannelId; terminal retirement never reuses ChannelId;
33. all numeric lifecycle windows, percentages, headroom, thresholds, hysteresis and timing remain PERF/OPS-owned.

## Acceptance criteria

- [x] Identity/display/current placement separated.
- [x] Recommendation/explicit target separated from final FND-04 admission.
- [x] Target failure and queue semantics fail closed.
- [x] Live-session destination queue deferred.
- [x] Party/friend co-location hints preserve independent authority/privacy.
- [x] ADR-0001 switch flow and hard blockers preserved.
- [x] Durable anti-hopping guard survives session/relog/reconnect/restart and retains prior Channel semantics.
- [x] Destination admission + guard/prior-placement advancement is one recovery-safe semantic outcome.
- [x] No unnecessary new ChannelSwitchId.
- [x] Trusted forced exception constrained.
- [x] Value-source multiplicity explicit/fail-closed with no runtime fallback.
- [x] Simulation scope and eligibility scope separated.
- [x] Scaling product/OPS/PERF ownership separated.
- [x] PvP/community/recovery/world-policy/cross-world boundaries frozen without downstream formula capture.
- [x] Qualitative create triggers and forbidden preference/farming/PvP-only triggers frozen.
- [x] Low-load drain predicate, drain abort/hold predicate and terminal retirement predicate frozen.
- [x] Same-Channel recovery versus new semantic Channel creation under recovery pressure clarified.
- [x] Numeric lifecycle/capacity thresholds remain PERF/OPS evidence-owned.
- [x] Decision timing/supersession/deferred scope documented.
- [ ] Freeze exact final head after this repair-cycle task metadata commit.
- [ ] Terminal exact-head full-diff self-review PASS.
- [ ] Required independent exact-head review PASS with zero material findings.
- [ ] Exact-head Agent Governance / Dependency Review / CodeQL PASS.
- [ ] Zero unresolved material review threads and clean ownership before merge.
- [ ] Separate post-merge lifecycle closeout promotes status/releases ownership.

## Excluded scope

Runtime/client/Rust, protocol gameplay payloads, PostgreSQL DDL/migrations, Platform writes, production deployment/configuration, numeric cooldown/queue/channel-count/capacity/scaling/drain values, exact PvP/boss/reward/spawn/loot/economy formulas, downstream business state machines and entitlement/paid-priority activation.

## Repair history

### Cycle 1 — source multiplicity and switch-commit crash gap

Pre-freeze adversarial review found ChannelRuntime locality could be misread as implicit `CHANNEL_LOCAL_REPEATABLE`, and cooldown wording allowed a crash gap between destination authority and guard advancement. Repaired with explicit fail-closed multiplicity classification and recovery-atomic destination admission + guard advancement. Also clarified GAME-CHANNEL guard ownership, no mandatory ChannelSwitchId and separate World change.

### Cycle 2 — logout/relog prior-Channel ambiguity

Pre-freeze review found a cooldown without durable prior-placement semantics could not distinguish same-Channel fresh login from different-Channel switch after GameSession termination. Repaired by retaining previous successful ChannelRef or equivalent durable semantics and defining first/same/different admission classification.

### Cycle 3 — independent P1 lifecycle trigger semantics

Independent Codex exact-head review of `7037deecce1cffd10fce5d5da4cca786a0f59636` opened P1 `PRRC_kwDOTuGrds7gjstL` / thread `PRRT_kwDOTuGrds6YlaAL`: GAME-CHANNEL scope included channel creation/removal/capacity triggers but the candidate delegated activation/deactivation too broadly to OPS/PERF and omitted product-semantic trigger predicates.

The final allowed repair cycle consumes the existing predecision capacity-trigger addendum as historical framework evidence and freezes qualitative semantics only:

- `DEMAND_PRESSURE`, `RECOVERY_PRESSURE`, `LOW_LOAD_CONSOLIDATION_CANDIDATE`, `CHANNEL_UNHEALTHY`;
- demand/recovery product predicates for new semantic Channel creation;
- explicit prohibition on private-party preference, farming/rare-spawn and PvP avoidance as sole create triggers;
- low-load consolidation drain prerequisites;
- drain abort/hold on returning demand, degraded retained capacity, recovery pressure, policy need or correctness risk;
- terminal removal only after zero authoritative sessions/dependencies, evidence finalization, fencing and stale-route invalidation;
- temporary stop/reactivation versus permanent Channel retirement identity semantics;
- numeric windows/thresholds/hysteresis remain PERF/OPS-owned.

Repair budget is now **3/3**. Any additional material final-head finding requires task stop/rotation or explicit owner extension; no fourth material repair is authorized.

## Validation

- source/option/ownership audit: complete including the previously omitted predecision lifecycle addendum;
- component/integration/runtime E2E: `NOT_APPLICABLE` — paper-only architecture;
- prior exact-head self-review/CI/independent review on `7037...` are superseded for merge readiness because cycle 3 moved the head;
- final exact-head self-review, independent review and CI: pending new freeze.

## Independent review

Required: `YES` because multichannel/world-shared/value/reward/PvP/lifecycle/failure behavior can create authority, fairness and anti-duplication defects.

## PR and closeout

- PR #209 ready for review;
- target net diff remains exactly the 3 owned paths;
- shared canonical overlays remain outside delivery candidate;
- current P1 thread may be resolved only after its fix is present on the new head;
- ownership releases only after separate lifecycle closeout.

## Context checkpoint

```yaml
last_progress: Repair cycle 3/3 consumed the predecision capacity-trigger addendum and froze qualitative Channel create/drain/drain-abort/retirement product predicates while preserving PERF/OPS ownership of all numeric windows/thresholds/hysteresis.
status: validating
branch: agent/otv2-20260812-game-channel-01-architecture
head_sha: null
pr: 209
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request #209
ci_check_generation: pending new freeze
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: false
blocker: null
next_action: Verify cycle-3 fix and exact net diff/live main; resolve only the repaired P1 thread, freeze the new head in PR evidence, then rerun terminal self-review, required independent review and exact-head CI. Any new material finding stops the gate because repair budget is exhausted.
```

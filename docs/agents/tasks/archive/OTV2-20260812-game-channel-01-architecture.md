# OTV2-20260812-game-channel-01-architecture — archived

```yaml
task_id: OTV2-20260812-game-channel-01-architecture
title: GAME-CHANNEL-01 multichannel product policy architecture
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
delivery_branch: agent/otv2-20260812-game-channel-01-architecture
delivery_pr: 209
base_sha: f89685d585aab9c2ed3c69cabe8a4d9a2544bd0e
final_head_sha: ca1112191ede7d316c874189f3053ad7f8247579
delivery_merge_sha: 54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1
lifecycle_closeout_branch: docs/OTV2-20260812-game-channel-01-closeout
lifecycle_closeout_pr: 210
owner: released_after_closeout
created_at: 2026-08-12T16:16:00+02:00
completed_at: 2026-08-12T17:26:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
implementation_status: NOT_STARTED
runtime_client_authority: NONE
postgresql_ddl_migration_authority: NONE
production_authority: NONE
owned_paths:
  - docs/agents/tasks/archive/OTV2-20260812-game-channel-01-architecture.md
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
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Delivered one bounded paper-only `GAME-CHANNEL-01` product-policy architecture defining player/channel selection, queue/co-location, anti-hopping, source/reward multiplicity, qualitative dynamic Channel lifecycle, PvP/community and same-Channel recovery semantics on top of the already accepted multichannel/runtime/session/value foundation.

Delivery PR #209 was squash-merged unchanged from its final frozen exact head as `54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1`.

The delivery implements nothing. Runtime/client/Rust implementation, PostgreSQL DDL/migration execution, Platform writes, production deployment/configuration and entitlement activation remain unauthorized.

## Binding sources consumed

- FND-ID-01 — canonical `ChannelRef=WorldId+ChannelId`, durable ChannelId/non-reuse and display identity separation;
- ADR-0001 — one World economy/community, independent public Channel simulations, hard switch blockers and fresh destination session/admission semantics;
- ADR-0009 + FND-03 — ChannelRuntime/GameNode lifecycle, one-writer ownership, same-Channel recovery and measured capacity boundary;
- FND-04 — exact target-bound final admission, GameSession and CharacterLease authority;
- GAME-VISION-01 — party/PvP/economy/fairness product direction;
- GAME-CHAR-01 / GAME-ITEM-01 — Character and item semantic boundaries;
- DUR-03 — item/currency/value conservation, idempotency and stale-authority safety;
- `GAME-CHANNEL-01_PREDECISION_ANALYSIS.md` and `GAME-CHANNEL-01_PREDECISION_CAPACITY_TRIGGERS_ADDENDUM.md` — historical framework evidence only; no historical numeric capacity/window/threshold value became binding.

## Accepted semantic closure

### Channel identity and product entry

- `WorldId` remains the product/economy/community/profile boundary.
- `ChannelRef=WorldId+ChannelId` is canonical; display labels/ordinals are presentation only.
- entry uses a bounded current eligible set plus non-authoritative recommendation and explicit eligible player target;
- FND-04 remains final admission authority;
- stale/full/draining/recovering/unavailable/incompatible explicit target fails closed and requires a fresh offer/grant; no silent retarget.

### Queue and co-location

- optional queue is bounded target-Channel **pre-admission** control state, never GameSession/CharacterLease/runtime/value authority;
- short-lived admission material is created/refreshed only when the queued attempt is ready;
- first generation does not reserve another Channel while the Character remains authoritative in the source Channel;
- party/friend co-location is a privacy-bounded recommendation/target hint;
- every Character admits independently; no party-owned Channel or atomic group admission.

### Channel switch / anti-hopping

- same-Channel reconnect/recovery is not a Channel switch;
- completed switch preserves ADR-0001 safe source exit -> fresh destination authorization/admission -> fresh GameSessionId;
- accepted combat/PvP/trade/item/event/instance/authority blockers remain fail-closed;
- GAME-CHANNEL owns durable World channel-policy state scoped `CharacterId+WorldId`, not automatically GAME-CHAR progression;
- guard retains enough prior successful Channel semantics to distinguish first admission, same-Channel fresh login and different-Channel switch after logout/relog;
- first anti-hopping mechanism is time-based cooldown + hard locks; numeric duration remains an explicit later evidence/owner decision;
- different-Channel destination playable authority and remembered Channel/guard advancement are one authoritative/recovery-safe semantic outcome;
- no mandatory new `ChannelSwitchId` was invented;
- trusted maintenance/incident exceptions remain typed, audited, unforgeable by client and consequence-preserving.

### Source/reward multiplicity

Runtime Channel-local placement is **not** durable source/reward multiplicity.

Every Channel-sensitive value-producing source/encounter family must explicitly compile/validate one class:

```text
CHANNEL_LOCAL_REPEATABLE
CHANNEL_LOCAL_SHARED_ELIGIBILITY
WORLD_SCOPED_UNIQUE
EXPLICIT_EVENT_POLICY_REQUIRED
```

Missing classification fails closed. Simulation scope and durable eligibility scope remain distinct. ChannelId cannot silently enter shared Character/Account/World reward reset/idempotency keys. No hidden inverse spawn/loot rate tuning based on active Channel count is accepted.

### Qualitative dynamic Channel lifecycle

GAME-CHANNEL freezes product predicates while PERF/OPS retain all numeric windows/thresholds/headroom/service objectives/hysteresis/timers and orchestration.

Accepted trigger vocabulary:

```text
DEMAND_PRESSURE
RECOVERY_PRESSURE
LOW_LOAD_CONSOLIDATION_CANDIDATE
CHANNEL_UNHEALTHY
```

New semantic Channel capacity is legitimate only from sustained eligible demand or bounded recovery-capacity need, with insufficient current healthy eligible capacity/headroom, compatible World/event/reward/multiplicity policy, safe resources and Ready-before-Selectable.

The following cannot create public Channel capacity by themselves:

- one party/friend group wanting a private copy;
- preference for an emptier Channel;
- rare-spawn/loot/resource farming;
- PvP avoidance;
- operator preference without demand/recovery/product justification.

Recovery pressure cannot silently replace failed Channel A actors with new Channel B; recoverable semantic Channel A preserves ChannelId.

Low-load drain requires sustained low-load evidence, sufficient retained healthy capacity/headroom, no World/event/reward/recovery policy need, stopped incoming admission/switch and safe incumbent exit.

Drain aborts/holds if demand returns, retained capacity degrades, recovery pressure appears, policy requires the Channel or continuing drain threatens session/lease/item/value/encounter/checkpoint/recovery correctness.

Terminal retirement requires:

- zero authoritative player sessions/placements;
- no dependent instance/encounter/event obligation;
- no unresolved item/value transaction, handoff, checkpoint or recovery obligation;
- required durable evidence/checkpoint finalization;
- fenced old owner/generation;
- invalidated stale directory/queue/admission references;
- explicit semantic retirement rather than temporary inactivity.

Temporary stop/reactivation preserves ChannelId. Retired ChannelId is never reused.

### PvP/community/recovery

- direct PvP execution remains current Channel/Instance-local;
- World/Character-scoped PvP consequences survive Channel/GameSession changes;
- one World remains one guild/economy/ranking/community boundary;
- local speech/combat/position remain Channel-local;
- failure recovers same semantic ChannelId first;
- alternate Channel is possible only after the actor reaches a proven safe state and performs fresh admission;
- failure never clears combat/reward/value consequences.

### Policy/versioning

GAME-CHANNEL consumes existing `world_policy_revision`; it does not create a new protocol major solely for Channel policy. WorldId change is not a Channel switch and cannot bypass World-scoped value/profile/Character lifecycle rules.

## Repair history

### Cycle 1 — source multiplicity and switch-commit crash gap

Pre-freeze adversarial review repaired:

1. an implicit risk that ChannelRuntime locality could mean `CHANNEL_LOCAL_REPEATABLE` durable output without authored classification;
2. a crash gap where destination authority could become playable before anti-hopping guard advancement.

The repair introduced fail-closed multiplicity classification and recovery-safe destination admission + guard advancement, clarified GAME-CHANNEL guard ownership, no mandatory `ChannelSwitchId`, and separate cross-World lifecycle.

### Cycle 2 — logout/relog prior-Channel ambiguity

Pre-freeze review found a cooldown without durable prior-placement semantics could not classify a fresh same-Channel login versus different-Channel switch after old GameSession termination.

The repair requires durable prior successful Channel semantics: first admission establishes baseline, same prior Channel fresh login is not switch, different Channel fresh login is switch, failed attempts do not move prior state, successful switch updates prior Channel + guard once.

### Cycle 3 — semantic capacity/lifecycle predicates

Independent Codex review on superseded head `a101862007...` / thread `PRRT_kwDOTuGrds6YnZUx` / comment `PRRC_kwDOTuGrds7gkSNx` found that creation/removal/capacity-trigger product semantics were delegated too broadly to OPS/PERF.

The final allowed repair cycle consumed the existing predecision lifecycle addendum as **qualitative framework evidence only** and froze:

- `DEMAND_PRESSURE`, `RECOVERY_PRESSURE`, `LOW_LOAD_CONSOLIDATION_CANDIDATE`, `CHANNEL_UNHEALTHY`;
- legitimate and forbidden Channel creation predicates;
- low-load drain prerequisites;
- drain abort/hold conditions;
- terminal retirement settlement/fencing/stale-routing conditions;
- temporary stop versus retirement identity behavior;
- continued PERF/OPS ownership of every numeric threshold/window/headroom/hysteresis/timer/algorithm.

Repair budget ended at `3/3`. Fresh final review found no additional material issue.

## Terminal delivery validation

Frozen exact delivery head: `ca1112191ede7d316c874189f3053ad7f8247579`.

- implementing-agent exact-head self-review `4918161329`: **PASS**, material findings `0`;
- fresh independent Codex exact-head review request `5268790260`: completed without suggestions; PR 👍 reaction `450588928`;
- Agent Governance `31611424137`: **PASS**;
- Dependency Review `31611424147`: **PASS**;
- CodeQL `31611424261`: **PASS**;
- unresolved material review threads immediately before merge: `0`;
- final changed paths: exactly task + analysis + candidate contract;
- final compare to live main: `behind_by=0`;
- component/integration/runtime E2E: `NOT_APPLICABLE` — paper-only architecture delivery.

PR #209 was squash-merged unchanged from the frozen head as `54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1`.

## Lifecycle closeout discipline

The separate closeout must not change GAME-CHANNEL semantic content. It may only:

1. complete active -> archive movement and retain this complete delivery/repair/validation history;
2. promote `GAME-CHANNEL-01` to `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` in maintained programme/register/horizon/index sources;
3. keep runtime/client/DDL/production/Platform/entitlement authority unauthorized;
4. preserve PERF/OPS numeric/orchestration ownership and downstream PvP/reward/economy/social business owners;
5. refresh the non-owning programme checkpoint and successor handoff to exactly one next safe paper-only architecture action;
6. release GAME-CHANNEL path ownership only after closeout merge.

No closeout merge SHA is invented before it exists.

## Closeout review reconciliation

PR #210 review found three lifecycle-record inconsistencies and this closeout repairs them together without changing GAME-CHANNEL semantics:

- the maintained gameplay/product horizon must stop naming GAME-CHANNEL as the next unresolved gate and point to `DUR-04`;
- the archive records lifecycle closeout PR `210` explicitly;
- this completed task no longer instructs a successor to perform its own closeout again.

These are lifecycle/status/handoff repairs only. They do not consume a GAME-CHANNEL semantic repair cycle and do not authorize implementation.

## Context checkpoint

```yaml
last_progress: GAME-CHANNEL-01 delivery PR #209 passed exact-head self-review, required independent review and all required exact-head CI after delivery repair budget 3/3, then squash-merged unchanged as 54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1; lifecycle closeout is owned by PR #210.
status: completed
delivery_pr: 209
final_head_sha: ca1112191ede7d316c874189f3053ad7f8247579
delivery_merge_sha: 54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1
lifecycle_closeout_pr: 210
self_review: 4918161329
independent_review_request: 5268790260
independent_review_pr_reaction: 450588928
ci_run_ids:
  - 31611424137
  - 31611424147
  - 31611424261
repair_cycles_for_delivery_gate: 3
owner_action_required: false
blocker: null
next_action: None for this completed task; follow the canonical programme checkpoint and successor handoff for future work.
```

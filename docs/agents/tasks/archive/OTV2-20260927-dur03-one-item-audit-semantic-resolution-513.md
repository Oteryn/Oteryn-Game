# OTV2-20260927-dur03-one-item-audit-semantic-resolution-513

```yaml
task_id: OTV2-20260927-dur03-one-item-audit-semantic-resolution-513
title: Adopt the bounded DUR-03 one-item audit semantic resolution
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
issue: 513
jira_story: KAN-12
base_branch: main
base_sha: 09555d0fd189df6ccb0fafc0a5ee3213a9ad2644
branch: codex/dur03-one-item-audit-semantic-resolution-513
pr: 971
head_sha: b12d060aae326c0d478b8b04e08747c930fd614c
final_head_sha: b12d060aae326c0d478b8b04e08747c930fd614c
final_head_frozen_at: null
owner: released
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/agents/tasks/active/OTV2-20260927-dur03-one-item-audit-semantic-resolution-513.md
public_contracts:
  - DUR-03 section 39.1 semantic representation only
depends_on:
  - OTV2-20260927-dur03-one-item-audit-contract-decision-513
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome
## Outcome

Propose the narrow normative DUR-03 §39.1 addendum required before a successor
candidate-schema/offline-measurement allocation. The authoring checkpoint is
pre-freeze; reviewed protected integration remains required for durable adoption.
Allocation: [#162 comment 5854940356](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5854940356).
Exact escalation: [#513 comment 5854890321](https://github.com/Oteryn/Oteryn-Game/issues/513#issuecomment-5854890321).
Role: reusable `OTV2_SOL_SUPERVISING_ARCHITECT` v1.1, resolved through its protected lifecycle entry.

## Architecture and source of truth

`PROVEN`: consumed DUR-03 §39 permits aggregation while requiring complete
applicable evidence and concrete registration before implementation conformance.
ANL-01 §§8–9/12 require complete immutable membership and exact payload-byte
stability. Its §§15–16 keep privacy-floor and production profile acceptance distinct.
The foundation IDL already supports the needed membership; it is unchanged.

`PROVEN`: PR #969 integrated the explicitly nonbinding packet at
`419a7cbc8539c9c98bd83d9220e331255e7e8f12`; its merge-group run `36310569166`
and `game-gate` job `108596899250` succeeded. PR #968 subsequently integrated at
this task's exact base and changed no allocated path. Its former resource-registry
collision is released by that integration; any successor still needs fresh overlap
and custody preflight. The seven-path schema proposal is not an allocation.

`DERIVED`: the closed one-item aggregate selects a representation already permitted
by accepted DUR/ANL architecture, without changing gameplay or production policy.
`UNKNOWN`: actual candidate encoding/count/bytes, item retention profile and
production maxima, SQL/outbox/restart conformance, Content binding and Reference parity.
`CONFLICT`: none observed in the selected semantics. Reusing Character bootstrap
policy or synthetic prototype bytes would contradict the existing gates and is rejected.

## Architecture resolution

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
main_sha: 09555d0fd189df6ccb0fafc0a5ee3213a9ad2644
source_escalation: https://github.com/Oteryn/Oteryn-Game/issues/513#issuecomment-5854890321
blocking_question: Closed one-item MINT/TRANSFER semantic basis for candidate schema and offline measurement
facts:
  proven:
    - DUR-03 section 39 permits bounded aggregate evidence.
    - ANL-01 requires complete membership and fixed immutable bytes before ambiguity.
    - Item profile, registered payload and DUR03-RL-01..08 production entries are absent.
  derived:
    - One complete aggregate per distinct narrow transaction is within existing authority.
  unknown:
    - Actual encoded counts/bytes, retention policy, production maxima and physical durability.
  conflict: []
accepted_decision: >-
  Scoped representation only: closed fresh-item MINT-to-Ground and separate
  existing-item TRANSFER-to-direct-root CharacterInventory; complete applicable
  section 39 evidence, ordinal/count 1/1 as design cardinality, immutable retry
  candidate, DURABLE_AUDIT and candidate privacy floor at least RESTRICTED_PLAYER_LINKED.
rejected_options:
  - One transaction combining MINT and later pickup.
  - Incomplete membership, generic delta, nullable mega-event or unbounded effects.
  - Character retention reuse or synthetic bytes/counts promoted to production acceptance.
affected_contracts:
  - DUR-03 section 39.1
  - ANL-01 consumed unchanged
affected_paths:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/agents/tasks/active/OTV2-20260927-dur03-one-item-audit-semantic-resolution-513.md
implementation_owner: DUR03_REFERENCE_ITEM_TRANSACTION under the unique Work control plane
implementation_scope: NONE; fresh successor allocation required after durable adoption
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
supersedes: []
required_validation:
  - Exact two-path delta, contract consistency, governance/repository checks and section/link verification.
  - Frozen exact-head hosted gates, Merge Queue game-gate and protected-main readback.
required_independent_review: Genuinely independent exact-head DUR/data-integrity/privacy review
next_action: Return the frozen architecture candidate to Work for independent review and protected integration.
```

## Decision timing

Must decide now: `YES` for #513's DUR03-RL-07 actual audit count/byte proof.
The choice binds event meaning and required fields, creating compatibility cost;
closed variants limit that commitment. Reviewed additional effects, missing
reconciliation information, malformed/retry/privacy findings or measured
amplification failure justify an explicit later supersession. Identifiers, IDL
numbering, item policy, numeric maxima, physical storage and runtime remain undecided.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` to executable authority: this contract-only task performs no
production mutation, PREPARE/COMMIT, controller installation or recovery execution.
The material durability/privacy semantic decision still requires genuinely
independent exact-head review. Safe fence references do not prove current authority.

## Acceptance criteria

- [x] Add only §39.1 and preserve general §39 and unrelated contract history.
- [x] Separate MINT/TRANSFER identities and complete membership/evidence obligations.
- [x] Preserve immutable retries, restricted candidate floor and unresolved policy/limit gates.
- [x] Preserve the earlier nonbinding packet; release no implementation or registry writer.
- [ ] Record final two-path validation and atomic publication evidence outside the candidate.
- [ ] Work obtains independent review, hosted qualification and protected integration.

## Excluded scope

No third repository path, foundation IDL, event/resource registry, event ID,
protobuf numbering, retention profile/purpose/duration/roles/legal hold, production
ceiling, Rust/prototype/Cargo, SQL/outbox/runtime, Combat activation, replay mutation,
live player data, production or Reference-parity change. Writer creates no PR,
comment, review trigger, ready/queue state or merge; Work owns that lifecycle.

## Validation

Focused governance/repository-policy, exact two-path delta and section/link checks
are required before publication. Their final results and returned SHA are external
candidate evidence, not self-referential follow-up fields in this immutable checkpoint.
Component/E2E: `NOT_APPLICABLE`, no executable behavior changed. Hosted exact-head
CI: pending publication/Work PR; no hosted qualification is claimed.

## Self-review

Author full-text review checks closed variants, complete applicable §39 evidence,
ANL membership/immutability, distinct audit/idempotency retention horizons, current
authority separation and the withheld schema/privacy/registry/runtime boundaries.
Final exact candidate self-review and validation evidence remain external after freeze.

## Independent review

Required: independent exact-head DUR, data-integrity and privacy review of this
normative candidate. Earlier nonbinding packet reviews cannot qualify this head.
Auditor/head/findings/verdict: pending. Bound META risk policy selects any external
review; standing funding authorization and single control-plane dispatch remain binding.

## PR and closeout

Canonical PR, changed-file review, hosted checks, findings disposition, protected
integration and lease release are pending Work. No ready/merged/canonical claim.

## Context checkpoint

```yaml
last_progress: PR 971 merged; terminal DUR-03 semantic packet archived after lease release
status: completed
branch: codex/dur03-one-item-audit-semantic-resolution-513
pr: 971
head_sha: b12d060aae326c0d478b8b04e08747c930fd614c
final_head_sha: b12d060aae326c0d478b8b04e08747c930fd614c
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_exact_head: 0
owner_action_required: null
blocker: null
next_action: none for completed child; aggregate #513 and KAN-12 remain outside this archive
```


## Completion

`PROVEN`: [PR #971](https://github.com/Oteryn/Oteryn-Game/pull/971) merged exact head
`b12d060aae326c0d478b8b04e08747c930fd614c` as protected merge commit
`0b88fda1501b2ccb7e9828049fcd5760c15ff3f3` at `2026-09-27T10:33:05Z`.
This terminal packet is archived after that integration; its original validation,
review and evidence gaps above remain historical and are not refreshed by this move.
The aggregate #513 / KAN-12 remains outside this child closeout. No original exact
freeze timestamp is retained, so `final_head_frozen_at` remains `null`.

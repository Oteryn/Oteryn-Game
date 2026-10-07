# OTV2-20261006 Quest Reward-Only Completion Candidates

status: implementing
base_main: d2791b35e0436725811718b2cca7af97e4b3ab49
branch: codex/quest-reward-only-bind-rebase-20261007
runtime_activation: false

## Goal

Qualify canonical reward-only Quest definitions that already have exact native RewardClaim refs but no QuestState progress candidate. Preserve the existing RewardClaim/ClaimObligation architecture and keep completion policy fail-closed until the required semantics are explicitly decided.

## Scope

Owned paths:

- `tools/content-schema/quest-authoring/quest_reward_only_binding_candidates.py`
- `tools/content-schema/quest-authoring/test_quest_reward_only_binding_candidates.py`
- `tools/content-schema/quest-authoring/samples/server-completion/reward-only-completion/candidates.json`
- `docs/agents/tasks/OTV2-20261006-quest-reward-only-bind-r1.md`

No shared `run_checks.py` ownership is taken. The committed-output drift check is inside the dedicated unittest so this lane remains disjoint from the collect/use candidate PRs.

## Qualified population

Freshly regenerated on base main:

- Quest candidates: **42**
- exact native RewardClaim refs: **59**
- runtime-admitted candidates: **0**
- native completion transitions: **0**
- native claim bindings: **0**

Classes:

- `SINGLE_KV`: **24**
- `SINGLE_STORAGE`: **7**
- `MULTI_KV`: **10**
- `MULTI_STORAGE`: **1**

All source claims are `repeat=once`. Each canonical Quest has exact native RewardClaim refs from the existing crosswalk.

## Fail-closed policy holds

### SINGLE_KV

24 quests have exactly one once-claim and no source storage write.

The runtime already supports a RewardClaim `quest_transition` obligation and `QuestCause::ClaimObligation`, but this packet does not invent a synthetic QuestState track/effect merely to create a terminal transition.

Hold:

`CLAIM_AS_QUEST_COMPLETION_POLICY_REQUIRED`
`NO_SOURCE_PROGRESS_WRITE`

### SINGLE_STORAGE

7 quests have exactly one once-claim and a source storage write, but no canonical QuestState progress track exists for that write.

Hold:

`CLAIM_AS_QUEST_COMPLETION_POLICY_REQUIRED`
`SOURCE_PROGRESS_WRITE_EXISTS_BUT_CANONICAL_QUEST_HAS_NO_PROGRESS_TRACK`

### MULTI_KV / MULTI_STORAGE

11 quests have multiple RewardClaims. Source grouping alone does not prove whether completion means ANY claim, ALL claims, ordered claims, or another rule.

Hold:

`MULTI_CLAIM_COMPLETION_AGGREGATION_POLICY_REQUIRED`

The one `MULTI_STORAGE` quest is Tomes of Knowledge Quest with five claims.

## Architectural boundary

Existing architecture already supports the durable chest/quest hand-off:

- `RewardClaimMintRequest::quest_transition` records a pending obligation with the claim;
- `QuestCause::ClaimObligation` is an accepted Quest transition cause;
- committed Quest transitions consume the obligation under the normal CharacterRevision rules.

That transport does **not** decide the semantic questions above. This lane therefore does not attach any `quest_transition` to reward placements and does not create synthetic terminal QuestState transitions.

## Validation

Focused on the rebased all-373 completion parent (`d2791b35e0436725811718b2cca7af97e4b3ab49`):

- generator: **42 quests / 59 claims**
- dedicated unittest: **6 PASS**
- generator `--check`: PASS
- `git diff --check`: PASS

The dedicated test also requires the committed packet to match generator bytes exactly.

## Next action

Architecture/coordinator decision required for the four finite policy questions:

1. For `SINGLE_KV`, is successful once-claim sufficient evidence to complete the Quest, and what legal QuestState transition shape represents that completion without fabricating progress?
2. For `SINGLE_STORAGE`, should the exact source storage write be lowered to a canonical QuestState track before claim binding?
3. For `MULTI_*`, what is the exact aggregation rule (ANY / ALL / ordered / quest-specific)?
4. If reward-only Quest completion may be driven directly by RewardClaim, which owner is responsible for writing `RewardClaimPlacement.quest_transition` and producing the native Quest transition key?

Until decided, all 42 candidates remain `runtime_admitted=false`.

# OTV2-20261006 Quest Collect / RewardClaim Candidates

```yaml
task_id: OTV2-20261006-quest-collect-claim-candidates
title: Qualify exact same-Quest RewardClaim acquisition candidates for collect stages
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/quest-collect-claim-candidates-20261006
base_sha: 97c46d0e63fefe8a22a2053de7b596c50b7253a6
owner: chatgpt-quest-completion
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/quest-authoring/collect_claim_candidates.py
  - tools/content-schema/quest-authoring/test_collect_claim_candidates.py
  - tools/content-schema/quest-authoring/samples/server-completion/collect-claim-candidates.json
  - tools/content-schema/quest-authoring/run_checks.py
  - docs/agents/tasks/OTV2-20261006-quest-collect-claim-candidates.md
depends_on:
  - PR #1875
  - PR #1876
  - docs/agents/evidence/OTV2-20261006-quest-binding-owner-readiness.md
```

## Outcome

Build a deterministic, evidence-only join between chosen `collect` stages and canonical
`RewardClaim` deliveries, restricted to:

- exact canonical Item refs already present in the completion binding plan;
- RewardClaims owned by the same canonical Quest through the existing donor-to-canonical crosswalk;
- exact RewardClaim Item delivery records.

The packet never treats current inventory as Quest authority and never admits a native transition.

## Result

Current binding plan contains **277** collect stages.

| Classification | Count |
| --- | ---: |
| `ONE_SAME_QUEST_READY_CLAIM_COVERS_ALL_EXACT_TARGETS` | **2** |
| `AMBIGUOUS_OR_PARTIAL_SAME_QUEST_CLAIM_MATCH` | **2** |
| `NO_SAME_QUEST_REWARD_CLAIM_MATCH` | **143** |
| `TARGET_IDENTITY_INCOMPLETE` | **130** |

The only two exact acquisition candidates are:

1. Hunter Outfits Quest `s2` -> `oteryn:reward-claim.quest.u7_8.hunter_outfits.elane_crossbow` -> Elane's Crossbow.
2. Oriental Outfits Quest `s5` -> `oteryn:reward-claim.quest.u7_8.oriental_outfits.coral_comb` -> Coral Comb.

For both:

- chosen occurrence count is 1;
- target Item identity is exact;
- target is materializable and non-stackable;
- one same-Quest RewardClaim is `ready` and delivers the target Item in count 1;
- `quantity_semantics_proven=false`;
- `quest_transition_cause_binding_proven=false`;
- `native_dispatch_binding=null`;
- `runtime_admitted=false`.

The two ambiguous/partial rows are Dawnport `s2` and Forgotten Knowledge `s2`.

## Authority boundary

A RewardClaim delivery may be an acquisition producer, but this packet does **not** prove that
its occurrence should increment the chosen collect stage.

The future executable binding must still establish:

- the actual owning occurrence / CommandRef or obligation;
- exact per-target quantity semantics;
- stage eligibility at that occurrence;
- replay/idempotence under the QuestState writer.

Polling inventory after the fact is not an accepted replacement.

## Validation

- `python collect_claim_candidates.py`: generated deterministic packet.
- `python -m unittest test_collect_claim_candidates.py`: 3/3 PASS.
- `python collect_claim_candidates.py --check`: PASS.
- Drift check is added to the existing Quest `run_checks.py`.
- No runtime/content activation is performed.

## Next action

After review, use these two rows only as inputs to the future inventory/RewardClaim Quest binding
owner. Continue qualifying other collect acquisition producers without broadening authority.

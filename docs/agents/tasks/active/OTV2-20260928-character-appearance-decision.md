# OTV2-20260928-character-appearance-decision

```yaml
task_id: OTV2-20260928-character-appearance-decision
title: "Character appearance owner decision (D47, D49, D61)"
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1115
base_sha: ae252cec2fde5cb6e6efaf4d3a4d1762426d4406
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md
  - docs/agents/tasks/active/OTV2-20260928-character-appearance-decision.md
  - docs/agents/tasks/active/OTV2-20260928-game-ai-action-integration-decision.md   # archive move after #1110
  - docs/agents/tasks/archive/OTV2-20260928-game-ai-action-integration-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records the Character appearance owner decision. It closes the follow-up "Character
appearance owner" of the #707 decision (§4.3), applying D47, D49 and D61:

- the owner of each character's selected look and of the account unlock set;
- the durable selection and its validation;
- the fallback on an incompatible world;
- the change command and observation;
- content inputs.

No runtime, registry, migration or protocol change. Children APP-1 to APP-4 need their own #162
allocations.

## Architecture and source of truth

- `PROVEN`:
  - the #707 decision §4.3 and §4.6;
  - `OTERYN_ATLAS_ANIMATED_APPEARANCES_V1.md` :50-68;
  - `tools/game-atlas-appearances/export.py` (133-colour palette);
  - `npc.schema.json` :39-45 and `monster.schema.json` :1295-1360;
  - `migrations/0005_character_authority.sql` :49-63; `character_bootstrap_intent.rs` :35-47;
    `world_spatial_v1.proto` :37-56;
  - the content outfit and mount identities;
  - #1025;
  - GAME-CHAR-01 Stage B reconciliation (sex UNKNOWN).
- `UNKNOWN`: Reference starter outfits, free versus premium outfits, addon acquisition, the mount
  list at the target date (content).

## High-risk authority/recovery qualification

This applies at design level. The decision defines which evidence lets a character wear an outfit,
addon or mount, and who may write the selection. The negative cases bind APP-1 and APP-3.

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - P1 a selection is written only inside a fenced character event (full session-generation fence)
  - P2 an outfit, addon or mount is accepted only through one authorization path: starter (active content only), earned unlock (compatible provenance) or usable Store entitlement (after §32)
  - P3 an incompatible world shows the default and never rewrites or reinterprets the stored selection
  - P4 the client never decides a look; other players see only a committed, validated selection
consumer_boundaries:
  - appearance change command
  - login and world entry (incompatible-world fallback)
  - observation by other players
mutation_operators:
  applicable:
    - stale generation (stale character fence)
    - mismatched identity or binding (another account's unlock, an unlock with incompatible provenance)
    - replay and concurrency (duplicate change command)
  considered_not_applicable:
    - "time: appearance carries no time-based authority"
one_invariant_per_negative_case: true
negative_cases_required_of_implementation:
  - P1 a change under a stale character fence -> nothing written
  - P2 a locked outfit, addon or mount, or an out-of-palette colour -> rejected
  - P3 a world without a compatible definition -> default shown, stored selection unchanged
  - P4 a rejected change -> nothing published to observers
positive_cases_required_of_implementation:
  - a player selects an unlocked outfit with addons and colours; it persists across relog and other players see it
independent_current_fact_sources:
  - character session fence rows
  - account unlock set
  - the world's active content generation
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: "the #707 decision account-fact fence is the reference"
  protocol_versions: "the actor appearance state belongs to the Combat child E protocol lane"
  direct_and_reconciled_paths: "direct fenced write only"
  fenced_durable_writes: "the selected appearance and AccountUnlock rows"
  restart_retry_replay_concurrency_pg_reload: "covered by P1 and P4"
  evidence:
    - apps/game-server/migrations/0005_character_authority.sql
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - "Codex P1 4123151388 (5611abc): the fallback look was not deterministic. Repaired: content declares exactly one fallback look per sex with explicit colours; validation requires it"
    - "Codex P1 4123151399 (5611abc): starters were checked against unlock provenance they do not have. Repaired: three authorization paths; starters validate against active content only"
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred:
    - "Codex P2 4123151419 (5611abc): mounted-state lifecycle undefined. Fixed: mount activation explicitly deferred; no mount is projected or shown until a mount decision"
    - "Codex P2 4123151408 (5611abc): Store cosmetics had no authorization path. Fixed: a Platform-entitlement path, unavailable until the gap register §32 delivery decision"
```

## Acceptance criteria

- [ ] The decision document is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Runtime code, schema, protocol and client.
- The Store catalogue, outfit bonuses, sex change, Premium gating of cosmetics.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Context checkpoint

```yaml
last_progress: authored; PR #1115 open
status: validating
branch: claude/gifted-rubin-a0axzx
pr: 1115
owner_action_required: null
blocker: null
next_action: exact-head review and Merge Queue integration of #1115
```

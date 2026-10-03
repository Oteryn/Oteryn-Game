# OTV2-20261001-item-resistance-facts

```yaml
task_id: OTV2-20261001-item-resistance-facts
title: Source-qualified resistance vectors and Shield Hand parser repair
mode: REPAIR
status: completed
integration_status: pending_control_plane_review_and_merge
pr: 1467
repository: Oteryn/Oteryn-Game
base_branch: main
authoring_baseline_sha: 3b2297f29e74e9fc93ac65bf07aab3fd7af59a57
dependency_pr: 1452
branch: codex/item-resistance-facts-20261001
owner: owner-directed Codex session
created_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/item_stats_promotion.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-schema/item-authoring/{lower_wiki_stats_packet.py,test_wiki_resistances_packet.py,test_shield_hand_equipment.py}
  - docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json
  - docs/agents/evidence/OTV2-20261001-item-resistance-*
  - docs/agents/tasks/archive/OTV2-20261001-item-resistance-facts.md
  - content/world/**
  - content/items/**
  - content/**/index.json
  - content/content.lock.json
```

## Outcome

391 resistance vectors /625 percentage-point coefficients and two equipment
patterns become known. Ghazbaran Oyoroi stays UNKNOWN because fresh wiki sources
disagree on signs. Whole-packet prevalidation prevents late failures from partially
mutating an Item. No gameplay, identity, materialization or admission change.

## Architecture and source of truth

Existing native ReferenceItemProtection and authoring resistance contracts;
retained wiki observations and EXACT Crystal identity binding. Source evidence:
`docs/agents/evidence/OTV2-20261001-item-resistance-source-completion.md`.
Engine stats stay hypotheses. No new public contract or production authority.

## High-risk authority/recovery qualification

NOT_APPLICABLE: offline content metadata compilation and draft publication; no
production mutation, session authority, controller or persisted recovery behavior.

## Acceptance criteria and validation

Source regressions cover signs, zero, fractional precision, enum order, bounds,
duplicate kinds, whole-page cardinality, source disagreement and native field fences.
Native guards cover strict typed decoding, blocked/partial/different known fields,
idempotence and late-error atomicity. Shield parser repair has focused RED then GREEN.
Deterministic generation, exact semantic comparison, source pins, native library,
repository inventory, Clippy, migration, materialized tree and Ruff are required.
Exact-head CI and independent review are recorded against the published SHA.

Local preflight passed: 13 packet regressions, Shield Hand RED/GREEN, 1290 library
tests /2 ignored, repository3/3, Clippy all-targets with warnings denied, deterministic
packet, exact native comparison, migration, materialized97/97, Ruff, fmt and governance.
The final packet digest is `b4d8ad4fc05f15c30aedeaf603252d4a2907f5ad0e3b51b35f5b54f09f822c05`.
Published frozen SHA and exact-head CI are PR evidence; they cannot be self-referenced
in this final authoring commit. Independent programme review remains pending.

## Self-review and closeout

Root reviews every handwritten change and verifies complete generated delta. Fresh
cross-wiki conflict is held rather than adjudicated by engine hypotheses. Keep draft;
programme control plane owns review/integration. Dependent on1452; combine1437's
taxonomy and regenerate source/decoder pins during controlled integration.

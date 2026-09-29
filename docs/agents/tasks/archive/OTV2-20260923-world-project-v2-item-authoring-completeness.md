> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. PR #792 exact head `8d3d50a066e77cc53e494f1c6a6b3b45b302428a` passed Agent governance, Architecture semantic audit, Merge Gate and the real Merge Queue. It integrated as `d29045c358315352c97e42469bef46f31a478dae`, and protected `main` readback points to that same commit. Any nonterminal wording below is historical provenance only.

# OTV2-20260923 — WorldProject/v2 Item authoring completeness

~~~yaml
task_id: OTV2-20260923-world-project-v2-item-authoring-completeness
title: WorldProject/v2 contemporary Item authoring completeness
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 504
base_branch: main
pr: 792
branch: agent/world-project-v2-item-authoring-completeness-20260923
base_sha: a791a52a5963713465c6ac9fc7f412153ab22c1f
head_sha: 8d3d50a066e77cc53e494f1c6a6b3b45b302428a
final_head_sha: 8d3d50a066e77cc53e494f1c6a6b3b45b302428a
owner: "single autonomous Content/World Item authoring writer"
owned_paths:
  - apps/game-server/src/content/project/v2.rs
  - apps/game-server/tests/content_world_project_v2.rs
  - docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md
  - docs/agents/tasks/active/OTV2-20260923-world-project-v2-item-authoring-completeness.md
~~~

## Outcome

Extend the protected canonical WorldProject/v2 from #788 rather than creating a second model. Close Item authoring gaps exposed by a second current TibiaWiki audit while keeping existing Reference Item runtime semantics unchanged.

Required typed source coverage:
- Item -> Presentation binding so v2 appearance bindings/assets can represent Item appearance without changing Item identity;
- source taxonomy including primary/secondary/tertiary type;
- Exaltation Forge definition metadata (classification/max tier only; current tier stays ItemInstance state);
- Weapon Proficiency levels/perks and Perk Shaping declarations;
- typed augment targets with exact bounded rank values plus candidate-only fields for semantics not yet owned by runtime;
- Item -> Interaction and Item -> Ability use bindings plus required magic level;
- edible/regeneration source facts;
- enchantable/destructible source/lifecycle bindings without storing mutable current enchanted state;
- implemented/removed source lifecycle facts;
- typed Service shop offers for buy/sell relationships.

Existing v2 editor aliases/tags remain the canonical non-authoritative Item aliases/tags path. TibiaWiki dropped-by / raid / event relationships must not become Item fields; they remain Creature/Loot/Encounter relationship evidence. Existing Reference Item semantics continue to own attack/defense/armor/resist/imbuement/etc.

## Constraints

- no second WorldProject/v2 model or top-level carrier;
- no mutation of v1 runtime semantics merely to accept source evidence;
- no current Forge tier, active imbues, current charges/timers, proficiency XP/unlocks/ranks, container custody or live quest/runtime state in WorldProject definitions;
- v2 snapshots produced by protected #788 with no Item authoring overlay must remain readable and canonical-byte stable;
- all new fields are declarative source data until an accepted runtime owner explicitly lowers them.

## Validation

- focused v2 roundtrip with modern Item overlay;
- old empty-v2 canonical byte preservation;
- wrong-family/missing-reference fail closed;
- Item mutable-state/shop/drop fields rejected from Item overlay;
- typed Service offers round-trip and resolve exact Item references;
- existing v1 and v2 suites remain green;
- exact-head Agent Governance, Architecture Semantic Audit, Merge Gate and Merge Queue required before protected-main completion.


## Terminal closeout

- delivery PR: #792
- frozen exact head: `8d3d50a066e77cc53e494f1c6a6b3b45b302428a`
- exact-head qualification: Agent governance run `35855227108` SUCCESS; Architecture semantic audit run `35855226953` SUCCESS; Merge Gate run `35855227059` SUCCESS
- protected Merge Queue run: `35862158435` SUCCESS
- merge-group commit: `d29045c358315352c97e42469bef46f31a478dae`
- protected-main readback: `main@d29045c358315352c97e42469bef46f31a478dae`
- result: canonical WorldProject/v2 from #788 now includes typed contemporary Item authoring coverage required by the TibiaWiki audit
- ownership: released

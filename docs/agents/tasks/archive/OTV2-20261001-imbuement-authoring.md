# OTV2-20261001-imbuement-authoring

```yaml
task_id: OTV2-20261001-imbuement-authoring
title: Current imbuement data, schema and qualified public research
mode: AUDIT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/imbuement-authoring-draft-20261001
pr: 1438
head_sha: null
final_head_sha: null
owner: Codex root imbuement authoring lane
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/imbuement-authoring/**
  - docs/agents/tasks/archive/OTV2-20261001-imbuement-authoring.md
public_contracts:
  - IMBUE-FORGE-0
depends_on: []
blocks: []
external_repositories: []
```

## Outcome

Delivered an authoring-only catalogue and schema with 24 families, 72 tier recipes,
72 material bindings, 72 scroll bindings, 629 current typed equipment profiles,
627 canonical bindings and two validated, unregistered Item proposals. The engine
matrix answers all 11 behavioral groups for Canary and both Crystal branches.
The public research addendum separates 31 bounded facts from genuine unknown
fields and runtime contracts. Full Global parity remains unproven.

## Architecture and source of truth

- Authority: IMBUE-FORGE-0 ([#1415](https://github.com/Oteryn/Oteryn-Game/pull/1415))
  and owner answers **I1a/I2a**, as explicitly routed by the control plane in
  [5936572587](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5936572587).
  This record adopts no runtime contract or authority beyond that authoring scope.
- D280 permits this existing content draft during close-out. The single task-record
  path is explicitly requested by control-plane
  [5937496910](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5937496910).
- Owner selects current Global research as of 2026-10-01, rather than July's snapshot.
  Historical captures remain provenance; Canary remains a research reference.
- Pinned Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`, Crystal imbuements
  `15593c28fd9adc2bb9739cf0fdb1a4289ebfe1e1`, Crystal summer-update
  `00ce02a57ca5a12e48f32a3476e37471167e4c3f`.
- PROVEN: delivered identities, parsed source facts and offline validation results.
  DERIVED: explicitly bounded public-source interpretations. UNKNOWN: unresolved
  consumption, Premium-purchase, numeric transfer and fine Life
  fields. CONFLICT: qualified PZ/armor behavior. The101equipment-source disagreements are historical alternatives with completed selected profiles, not missing data.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this draft does not perform production mutations, install runtime
controllers, authorize gameplay transactions or interpret recovery state.

## Acceptance criteria

- [x] Populate and validate the selected authoring catalogue and schema.
- [x] Preserve exact source identities, literal quotes, dates and access methods.
- [x] Compare pinned engines while distinguishing their behavior from Global proof.
- [x] Retain explicit unknowns/conflicts and assign runtime contracts through #162.
- [x] Provide this control-plane-requested task record in the draft's final authoring commit.

## Excluded scope

Runtime activation, canonical Item/Quest registration, architecture adoption,
persistence, combat execution, timers, protocol/UI and protected integration.
Remote Desktop was used exclusively to read public pages through Chrome/CDP.

## Implementation / findings

The original blanket requirement for gameplay recordings was too restrictive for
data research. Literal official/community statements now qualify their stated
fields. The 27 unperformed capture scenarios remain supplemental alternatives.
No consumed units were fabricated fromOTS code. A dated public tutorial now qualifiesPowerfulVampirism on equippedGhostChestplate; all101historically disputed equipment profiles were independently rechecked as selected and populated. RawEtcher/Albinius revisions and additional historical combat statements retain exact public provenance.
See the authoring package's `global-research-closure.md` and `completion-handoff.md`.

## Validation

Before this owner-requested successor, head `592fa309ace96dde8cd835ee7062211e3e33fd99`
passed 213 offline tests, nine replay/schema commands and all three XML reparses.
Both Item proposals had zero errors and warnings. Final successor validation,
source review and CI are rebound to its exact FREEZE_SHA in #162 and PR evidence;
this record does not inherit candidate-specific readiness from its predecessor.
Gameplay E2E is NOT_APPLICABLE to this data/schema/research draft.

## Self-review

Root reviewed the bounded authored delta, source qualifications, remaining nulls
and the sanctioned task-record path. Full Global parity is not claimed.

## Independent review

Required for final frozen head. Read-only independent review is dispatched;
the final verdict belongs to the exact-head FREEZE packet after publication.

## PR and closeout

- Canonical draft: [#1438](https://github.com/Oteryn/Oteryn-Game/pull/1438).
- Final head: recorded by FREEZE_SHA after this commit exists, avoiding self-reference.
- Merge commit/result: squash merge of #1438, only if subsequently authorized and integrated.
- Protected queue/review and ownership release remain with the control plane.
- This archive placement follows `archive/README.md`; it reaches main only if the PR merges.

## Targeted completion evidence, 2026-10-01

The packet contains **35 bounded facts and 32 sources**. This batch adds these source-qualified results:

- **101 equipment differences:** [Mirade’s official reply, 3 July 2026](https://www.tibia.com/forum/?action=thread&postid=39592694#post39592694), read with ordinary HTTP 200, gives the Dream Blossom Staff threshold and five named Strike exceptions. It corroborates the selected Strike III for Deepling Ceremonial Dagger and Energized Limb. It names Deepling Fork as an exception; the item’s current table supplies its exact exclusion. All 101 rows have explicit source-choice dispositions. Nine priority item tables were freshly reread and unchanged. The WandsRods helper is dated 16 June 2026; other helpers are mostly from 2024. No exhaustive official allowlist is invented.
- **Physical order:** [TibiaTools’ immutable authored test report](https://github.com/kik-tibia/tibiatools/blob/a1d368906caa8ae98bcb7123f2431733d318d710/src/lib/damage-calc/calc.ts#L21), read with ordinary HTTP, reports resistance before armor in tests on Gazer Spectres and spike traps. Its executable companion models outgoing player-to-creature damage. Current Formulae corroborates a named equipment example; QA87’s dissent remains visible. This selects a scoped reference. All attack types, incoming equipment pipelines and universal rounding remain unproved. The file modification date, 17 September, is not the unknown test date.
- **Life overkill:** [Tibia Analise, 5 January 2024](https://www.youtube.com/watch?v=JQRKU3Jd3gY), read through public Chrome/CDP Show transcript after ordinary access failed, says at 7:27–7:47 that Life and Mana use the damage an attack would deal regardless of remaining HP. The author also says published formulas did not match all his January 2024 tests. This is a dated report; current continuity and exact rounding, unequal-hit and zero-hit rules remain unasserted. The 30 HP / 78 illustration is Mana; the later 223 / 35 example is simulation. Neither is a measured Life result.
- **Vibrancy:** current revision 1194726 and actual 2021 revisions 884981/884982, read through Chrome/CDP, give the same already-paralyzed PvP retrigger example. With the independently named Powerful 50% success, the interaction ends no longer paralyzed. The same attack therefore leaves no active reapplication. This selects the net result, without inventing execution order, persistent immunity, future-attack protection or reset data.
- **PZ:** [TibiaTrends](https://tibiatrends.com/imbuements/), ordinary HTTP, modified 27 September, repeats blanket pause except backpacks without stating a test method. Contrary combat-countdown sources remain; the precise boundary is unresolved.

**Global evidence still insufficient:** exact Etcher success/rejection units; filled-scroll units on success, full slots, incompatible items or duplicate families; the explicit Premium predicate for NPC Etcher purchase; numerical remaining duration across a named ownership transfer; the precise PZ combat boundary; current universal Life rounding, unequal-hit, zero-hit and overkill continuity. Equipped scroll permission remains a dated before/after inference without direct application-instant proof. These values stay null; Crystal/Canary implementation values remain separately available.

Consumption research read four filled-scroll descriptions and 66 full CM posts from a 125-post June–August 2025 index. Fetches stopped at HTTP 429; 58 bodies were not read. Official news 4828 returned Cloudflare verification even in Chrome, so its new discussion link was inaccessible. Both current Life family pages were read in Chrome and contained no precise formula. Remote Desktop was used only for public browser research; the root-owned tab was closed.

API successor is fenced against v15 `ed5af70c4924816e547ffd90ddee4e64b120e3c2` and must freeze/requalify the returned SHA. Earlier candidate proofs remain historical. Draft data/schema/research only; coordinator #162 owns runtime and integration.

## Owner-selected operational policies, 2 October 2026

[owner-authoring-policy.json](../../../../tools/content-schema/imbuement-authoring/samples/owner-authoring-policy.json) contains ten explicit decisions approved in this conversation. The authoring catalogue and schema require this profile; each affected public-research group links its selected policy. Its values are the selected Oteryn behavior for this draft, while source evidence remains independently qualified. Unknown public Global fields no longer mean an undecided Oteryn policy where an owner selection exists. Runtime activation remains blocked.

- Etcher: one unit clears all active imbuements on one item; ordinary pre-debit rejection consumes zero.
- Filled scroll: success consumes one; full slots, incompatible target or duplicate family consumes zero. Late failure/crash compensation is not generalized from these validation results.
- Filled scrolls may be used on equipped items, subject to the normal slot/type/family checks.
- Albinius sells Etchers for 30,000 gold without Premium; other existing offer prerequisites remain.
- Ownership transfer preserves the imbuement family/tier and exact remaining use budget. Transfer itself does not reset/deduct it; normal eligible-use ticking continues.
- Life Leech uses attack damage before clipping to target remaining HP, so overkill counts. It is not based on target maximum HP. Healing is capped by the receiver’s missing HP.
- Life AoE counts positive-damage targets only; zero-damage targets neither heal nor increase N. An empty set yields zero.
- Life AoE sums per-target ceilings: `sum(ceil(D_i * P * (0.9 + 0.1 * N) / N))`. Unequal targets use their own damage. An exact integer reference is `sum(ceil_div(D_i * share_bps * (N + 9), 100000 * N))`; this is authoring arithmetic, not runtime installation.
- Combat timers require equipped/online state and active combat. Entering PZ does not immediately pause them or reset the deadline; ordinary combat expires 60 seconds after its last refreshing event. Swiftness, Featherweight and Vibrancy tick while equipped/online, including PZ. Special PvP state is not generalized.
- Armor follows the inspected Canary/Crystal player blockHit path: defense/flat armor when enabled, then equipped-item absorptions, then Wheel resistance. Each item applies imbuement reduction with `ceil`, followed by applicable native reduction with `round`, using the remaining damage at each stage. Item percentages are not summed across equipment. Unrelated Mantra, proficiency and Wheel formulas are not selected by this approval.

The Life and PZ selections intentionally differ from the inspected OTS paths. The source comparison still records HP clipping/lround and the aggressive-category outside-PZ check. The public scoped resistance-before-armor report remains preserved; the owner selected armor-before-item-percentages for Oteryn. No public-source claim is overwritten, no observation is invented, and full Global parity remains unproved. Coordinator #162 owns adopting these values in runtime contracts and workers.

Owner-policy batch checks before publication: 232 authoring tests, 11 replay/schema/XML checks, governance validator and 36 governance tests passed. Final qualification/review is tied to the returned frozen successor SHA, not the previous v17 candidate.

## Held P1 repair: bounded research excerpts, 3 October 2026

Review comment 4172937101 found full third-party pages in the research closure. Following D327 §2 (PR #1655), each source's `captured_text` is now only its quoted passages, in document order, whitespace-normalized and joined by `" … "`, with `captured_text_sha256` recomputed. `original_digest` and retrieval metadata are unchanged. The four quotes that carried line breaks or indentation are whitespace-normalized so the substring rule still holds. 30 sources are excerpted. The unreferenced `vibrancy_original_example_revisions` is dropped (31 sources). `trends_current_pz_pause`, which a conflict references without a quote, keeps a 155-character locator. `research_closure.py` enforces the excerpt scope, at most 1,000 characters per source and at most 450 per passage, with a negative test for each bound. The evidence pin and generated catalogue/comparison carry the new closure digest. No claim or fact was removed. Merging main changed 65 canonical Item definitions, so the eligibility census digests were regenerated and re-pinned in the same commit. All 236 authoring tests passed.

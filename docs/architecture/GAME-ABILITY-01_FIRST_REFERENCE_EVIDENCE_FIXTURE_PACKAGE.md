# GAME-ABILITY-01 — First Reference Evidence and Pending Fixture Package

- Status: **CANDIDATE EVIDENCE PACKAGE / GAME-ABILITY-01 REMAINS OPEN**
- Date: 2026-08-14
- Issue: #254
- Reference manifest pin consumed: schema v1 / manifest revision 3 in this delivery
- Runtime/client/protocol/DDL/Platform/production authority: **NONE**

## Purpose

Exercise the accepted Reference Mechanic Catalogue binding model with two deliberately small, complementary mechanics while preserving fail-closed evidence and implementation boundaries:

- Light Healing (`exura`) — healing/self path;
- Ice Strike (`exori frigo`) — targeted attack/damage path.

This package is catalogue/evidence metadata. It is not executable content and does not activate either spell in Oteryn.

## Evidence boundary

The research retrieval on 2026-08-14 surfaced indexed content for the exact official Tibia Library locators. The search index reported an approximately two-week-old crawl, while direct page opens from the research environment returned HTTP 403. Exact crawl time is unavailable.

These observations are therefore official-public indexed content, **not** a verified live-page capture and not a time-locked capture made at the immutable 2026-07-28 Reference boundary. No accepted evidence chain currently proves continuity back to that cut. Exact official locator identity is known, but because the indexed content could not be directly revalidated against the official page, source/case provenance remains `PENDING` for promotion.

`REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md` therefore requires the immutable-target classifications below to remain `UNKNOWN`, not `OBSERVED`. The indexed official observations are retained as bounded discovery/current-state-near evidence. A provenance state other than `CLEARED` independently blocks evidence promotion, so later work must establish both target continuity and provenance clearance.

The bounded evidence capture is `docs/agents/evidence/OTV2-20260814-ability-combat-official-spell-library.md`.

Patch-note/search absence is not continuity proof. OTS code is not used as Reference proof.

## Catalogue package — Light Healing

```yaml
catalogue_mechanic_key: reference.ability.light_healing
reference_target: global-tibia-observable-2026-07-28-post-server-save
manifest_case_refs:
  - ability_combat.light_healing.cast_metadata.v1
  - ability_combat.light_healing.self_heal_semantics.v1
classification_projection:
  target_evidence: UNKNOWN
  source_provenance: PENDING
  case_provenance: PENDING
  legal_review: PENDING
  implementation_state: NOT_STARTED
  aggregate_parity_status: PARITY_PENDING_EVIDENCE
parity_coverage:
  - case: ability_combat.light_healing.cast_metadata.v1
    aspects: [formula, vocation_eligibility, spell_group, cast_type, magic_type, cooldown, group_cooldown, level, mana, premium]
    state: TARGET_UNKNOWN_CONTINUITY_AND_PROVENANCE_PENDING
  - case: ability_combat.light_healing.self_heal_semantics.v1
    aspects: [qualitative_self_heal]
    state: TARGET_UNKNOWN_CONTINUITY_AND_PROVENANCE_PENDING
revision_bindings:
  reference_manifest: schema_v1_manifest_revision_3
  ability_definition: NOT_STARTED
  target_policy: UNKNOWN_NOT_EVIDENCED_FOR_TARGET_CUT
  legality_policy: UNKNOWN_NOT_EVIDENCED_FOR_TARGET_CUT
  cast_cost_commit_policy: UNKNOWN_NOT_EVIDENCED_FOR_TARGET_CUT
  cooldown_policy: UNKNOWN_CONTINUITY_NOT_PROVEN
  heal_formula: UNKNOWN_NOT_EVIDENCED
  simulation_numeric_profile: NOT_STARTED
effect_families:
  - healing
owning_domain_integrations: []
```

`healing` above is a descriptive semantic family label from the accepted effect-family boundary, not a frozen physical ID, enum discriminant or serializer token.

### Indexed official observation — not target truth

The official Library content surfaced by search reports `exura`, Druid/Monk/Paladin/Sorcerer, Healing/Instant/Healing, 1 second spell/group cooldown, level 8, mana 20, non-Premium, and qualitatively describes healing the caster's own wounds. Search-index freshness is only approximate, direct live-page revalidation was unavailable, and no target continuity is established. These fields remain provenance-pending continuity hypotheses for the immutable target until admissible evidence is registered.

### Deliberately uncovered Light Healing aspects

No target-cut claim is made for the surfaced metadata itself, source provenance clearance, exact heal magnitude/scaling/rounding, exact mana or cooldown commit anchors on failure, complete self-target legality, death/PZ/PvP edge behavior, client failure messages or any Oteryn implementation behavior.

## Catalogue package — Ice Strike

```yaml
catalogue_mechanic_key: reference.ability.ice_strike
reference_target: global-tibia-observable-2026-07-28-post-server-save
manifest_case_refs:
  - ability_combat.ice_strike.cast_metadata.v1
  - ability_combat.ice_strike.targeted_ice_damage_semantics.v1
classification_projection:
  target_evidence: UNKNOWN
  source_provenance: PENDING
  case_provenance: PENDING
  legal_review: PENDING
  implementation_state: NOT_STARTED
  aggregate_parity_status: PARITY_PENDING_EVIDENCE
parity_coverage:
  - case: ability_combat.ice_strike.cast_metadata.v1
    aspects: [formula, vocation_eligibility, spell_group, cast_type, magic_type, cooldown, group_cooldown, level, mana, premium]
    state: TARGET_UNKNOWN_CONTINUITY_AND_PROVENANCE_PENDING
  - case: ability_combat.ice_strike.targeted_ice_damage_semantics.v1
    aspects: [qualitative_aimed_opponent, qualitative_close_range, ice_damage_family, qualitative_magic_ability_scaling]
    state: TARGET_UNKNOWN_CONTINUITY_AND_PROVENANCE_PENDING
revision_bindings:
  reference_manifest: schema_v1_manifest_revision_3
  ability_definition: NOT_STARTED
  target_policy: UNKNOWN_NOT_EVIDENCED_FOR_TARGET_CUT
  legality_policy: UNKNOWN_NOT_EVIDENCED_FOR_TARGET_CUT
  cast_cost_commit_policy: UNKNOWN_NOT_EVIDENCED_FOR_TARGET_CUT
  cooldown_policy: UNKNOWN_CONTINUITY_NOT_PROVEN
  damage_formula: UNKNOWN_NOT_EVIDENCED
  simulation_numeric_rng_profile: NOT_STARTED
effect_families:
  - damage
owning_domain_integrations: []
```

`damage` above is a descriptive semantic family label from the accepted effect-family boundary, not a frozen physical ID, enum discriminant or serializer token.

### Indexed official observation — not target truth

The official Library content surfaced by search reports `exori frigo`, Druid/Sorcerer, Attack/Instant/Ice, 2 second spell/group cooldown, level 8, mana 20, non-Premium, and qualitatively describes an aimed opponent in close range with ice whose damage depends on caster magical ability. Search-index freshness is only approximate, direct live-page revalidation was unavailable, and no target continuity is established. These fields remain provenance-pending continuity hypotheses for the immutable target until admissible evidence is registered.

### Deliberately uncovered Ice Strike aspects

No target-cut claim is made for the surfaced metadata itself, source provenance clearance, exact tile/range metric, line-of-sight/floor/PvP/PZ legality, complete target cardinality/ordering, exact damage formula/scaling/rounding, resistance/mitigation, crit/block/dodge/RNG, exact cost/cooldown failure anchors or any Oteryn implementation behavior.

## Pending bounded fixture blueprints

The records below are **not parity fixtures yet** under `GAME-ABILITY-01_REFERENCE_MECHANIC_CATALOGUE_ENTRY_PARITY_FIXTURE_CONTRACT.md`. They intentionally lack target continuity evidence, cleared source/case provenance, cleared legal review, an exact Oteryn implementation revision and an executable test locator. Calling them passing or executable fixtures would be false evidence.

They are bounded fixture blueprints that define what a later implementation/evidence package must prove after target continuity and source provenance are established.

### `blueprint.reference.light_healing.basic_cast.v1`

- mechanic: `reference.ability.light_healing`;
- manifest cases: both Light Healing cases above;
- candidate actor precondition from indexed official observation: eligible listed vocation, level >= 8, at least 20 mana, injured state;
- candidate invocation intent: cast formula `exura`;
- target policy to prove: self-directed healing only after target-cut evidence confirms it;
- candidate metadata surface: 20 mana, 1 second ability/group cooldown, instant Healing spell, non-Premium;
- expected semantic family if continuity and provenance are proven: healing;
- **not asserted for target cut:** metadata continuity, source provenance, magnitude, rounding, failure anchor, full legality;
- exact Oteryn implementation revision: `NONE / NOT_STARTED`;
- executable locator: `NONE / NOT_STARTED`;
- result: `PENDING_TARGET_CONTINUITY_PROVENANCE_LEGAL_IMPLEMENTATION`.

### `blueprint.reference.ice_strike.basic_targeted_cast.v1`

- mechanic: `reference.ability.ice_strike`;
- manifest cases: both Ice Strike cases above;
- candidate actor precondition from indexed official observation: Druid or Sorcerer, level >= 8, at least 20 mana;
- candidate target precondition: one opponent in a configuration later proven legal for the target cut;
- candidate invocation intent: cast formula `exori frigo` at the candidate opponent;
- candidate metadata surface: 20 mana, 2 second ability/group cooldown, instant Attack spell, Ice magic type, non-Premium;
- candidate qualitative effect: targeted ice-damage occurrence influenced by caster magical ability, subject to target-cut continuity and provenance proof;
- **not asserted for target cut:** metadata continuity, source provenance, exact range, formula, resistance, RNG, complete legality or target-selection edge behavior;
- exact Oteryn implementation revision: `NONE / NOT_STARTED`;
- executable locator: `NONE / NOT_STARTED`;
- result: `PENDING_TARGET_CONTINUITY_PROVENANCE_LEGAL_IMPLEMENTATION`.

## Contract-fit finding

Even while the target cases remain `UNKNOWN` with provenance `PENDING`, the two representative shapes exercise the catalogue contract without requiring a new core Effect Family, generic patch, script-owned mutation surface, global transaction or new stable identity. Healing and damage remain separate typed semantics; targeting/legality stays authoritative; cooldown/cost timing remains versioned policy; unsupported behavior stays fail closed.

The package exposes two important evidence-pipeline constraints: **indexed official state alone cannot populate a historical immutable target**, and **an exact official locator does not by itself make indexed content provenance-cleared** when the underlying page cannot be directly revalidated. These are evidence acquisition/provenance problems, not reasons to weaken target identity or classifications.

Therefore this package does **not** justify freezing a physical catalogue schema or changing the accepted GAME-ABILITY architecture boundaries.

## Decision timing

**Must decide now: YES for this evidence package.** The accepted programme selected first representative `ABILITY_COMBAT` cases/fixture bindings as the next paper-only proof of the catalogue/evidence model. A real package is needed to test whether fail-closed historical evidence works in practice.

**Blocked without it:** the catalogue model remains theoretical and continuity/provenance rules are easy to accidentally bypass when future mechanics are populated.

**Harder later:** expanding many mechanics from indexed current-state pages before testing historical continuity and provenance clearance would multiply incorrectly promoted target/source claims and require a broad reclassification audit.

**Supersession evidence:** admissible time-appropriate provenance-cleared evidence shows the current case split is wrong; executable fixture work proves missing revision/coverage fields; legal review requires another source-binding shape; or representative mechanics expose a missing reusable effect-family/domain boundary.

## Deliberately not decided

No target behavior is promoted above `UNKNOWN`; no source/case provenance is promoted above `PENDING`; no runtime implementation, physical catalogue schema/serializer, fixture runner, exact formulas, exact range/LoS, complete legality, combat mitigation/resistance/RNG, client/protocol UI, DDL or production behavior is accepted here.

## Recommended next paper-only evidence step

After this package is merged and lifecycle-closed, prioritize a **target-continuity + provenance-clearance evidence package for these four cases**: locate provenance-cleared, time-appropriate evidence that directly bridges or captures the 2026-07-28 boundary and either promotes or rejects the indexed official-state hypotheses. Only after the representative historical-evidence path is proven should the programme broaden to secondary/shared cooldown-group mechanics or freeze physical catalogue tooling.

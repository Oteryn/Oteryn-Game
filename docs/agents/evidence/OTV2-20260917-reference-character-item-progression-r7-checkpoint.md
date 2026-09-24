# Oteryn Reference Character / Item / Progression — R7 Investigation Checkpoint

- Date: 2026-09-17
- Programme: #486
- Lane: R7 Character / Item / Progression
- Alias: `Oteryn: ref char`
- Repository: `Oteryn/Oteryn-Game`
- Protected main at repository-write preflight: `ee4a13212d392dd00f8adcd47b103997687b1d6c`
- R7 evidence checkpoint previously reconciled at: `674c998ca949422ac018f17f177b590f965a28f7`
- Authority of investigation: READ_ONLY
- Target: `global-tibia-observable-2026-07-28-post-server-save`
- Document class: retained investigation evidence / handoff; **not** allocation, implementation, manifest, merge, or production authority

## Executive summary

The R7 investigation is mature enough that the primary remaining work is no longer broad research or another progression calculator.

The main executable gaps are now:

1. Character progression durable commit / reconciliation.
2. Minimal Reference Content item semantics for the first durable item.
3. DUR-03 durable item MINT + TRANSFER.
4. Bounded evidence/manifest closure for Character death / XP threshold policy.

The existing Character progression calculator, Character/Item semantic core, Rat first-creature evidence, and Combat death/corpse/loot evidence are already on the correct side of the architecture boundary.

## Current protected state

```yaml
protected_main_sha_at_repository_write: ee4a13212d392dd00f8adcd47b103997687b1d6c
r7_evidence_checkpoint_sha: 674c998ca949422ac018f17f177b590f965a28f7

progression_calc:
  state: IMPLEMENTED_COMPONENT
  source: PR_528
  target_policy_bound: false

character_semantic_core:
  state: PARTIAL
  includes:
    - CharacterId
    - CharacterRevision
    - interpretation_context
    - build_state
    - typed_progression_facts

character_progression_durable_commit:
  state: ABSENT

dur03_item_transaction_runtime:
  state: ABSENT

reference_content_item_semantics:
  state: PARTIAL
  gap: current item carrier lacks required static Reference semantics

evidence_manifest:
  revision: 3
  CHARACTER_cases: 0
  ITEM_cases: 0

combat_evidence:
  PR_576: PROTECTED_INTEGRATED
  first_creature: Rat
  rat_strength: DERIVED_HIGH
```

## 1. Character progression calculator

PR #528 is integrated on protected main.

`REFERENCE_CHARACTER_PROGRESSION_CALC_V1` already provides:

- typed `AwardExperience`;
- typed `ApplyDeathExperienceLoss`;
- finite injected threshold policy;
- checked arithmetic;
- deterministic projection;
- fail-closed behavior outside the supplied oracle;
- explicit revision/evidence/declaration bindings;
- Oteryn Reference death `LevelXPSpan(current_level)` basis;
- no CharacterRevision mutation;
- no persistence.

The existing tests intentionally use synthetic `NON_REFERENCE` thresholds.

Conclusion:

```text
do not allocate another progression calculator
```

The next missing stage is durable application.

## 2. XP threshold evidence

### Level 8 threshold

Target candidate:

```text
XPThreshold(8) = 4200
```

Classification:

```yaml
classification: DERIVED
confidence: HIGH
```

Evidence basis:

- current official CipSoft Experience Table publishes level 8 = 4200;
- an official Character Bazaar auction from 2026-05-21 shows a level-8 character with exactly 4200 experience.

This materially improves the earlier `UNKNOWN` state.

### Level 7 threshold

Target candidate:

```text
XPThreshold(7) = 2600
```

Classification:

```yaml
classification: DERIVED
confidence: MEDIUM_HIGH
```

Evidence basis:

- current official CipSoft Experience Table;
- long historical structured continuity.

A target-near primary official snapshot exactly at 2600 was not found during this pass.

### Level 7 span

```text
LevelXPSpan(7)
= XPThreshold(8) - XPThreshold(7)
= 4200 - 2600
= 1600
```

Classification:

```yaml
classification: DERIVED
confidence: MEDIUM_HIGH
```

This is suitable as a bounded candidate for the first death fixture, but not labelled `PROVEN`.

## 3. Low-level creature XP modifier

A material issue was found while attempting to compose:

```text
level 7 Character + Rat base XP 5 -> Character XP +5
```

That direct assertion is invalid.

Global Tibia has a low-level experience bonus for characters below level 50. Therefore:

```text
Rat base XP = 5
!=
unconditional Character XP delta = 5
```

The exact low-level bonus curve and, especially, final modifier ordering/rounding remain insufficiently proven for the immutable 2026-07-28 target.

Classification:

```yaml
low_level_kill_xp_bonus_exists:
  classification: DERIVED
  confidence: HIGH
  continuity_to_target: HIGH

exact_curve:
  classification: DERIVED_FROM_STRUCTURED_REFERENCE

exact_rounding_and_modifier_order:
  classification: UNKNOWN
```

OTS code does not solve this. Current Canary behavior differs and is useful only as `OTS_HYPOTHESIS_ONLY`.

## 4. Recommended split of first progression fixtures

Do not try to prove kill-XP and low-level death semantics in one level-7 fixture.

### Fixture A — creature kill / XP commit

Use a Character outside the low-level XP-bonus range.

Candidate shape:

```yaml
fixture: REFERENCE_RAT_XP_COMMIT_V1

character:
  level: 50

combat:
  creature: Rat
  player_count: 1
  contribution: 100_percent
  shared_xp: false

reward_context:
  stamina: 20h
  premium_green_stamina_bonus: inactive
  final_14h_penalty: inactive
  prey_bonus: inactive
  xp_event: inactive
  boosted_creature: false
  other_explicit_xp_modifiers: inactive

rat:
  base_xp: 5
  evidence: DERIVED_HIGH

expected_staged_operation:
  AwardExperience: 5
```

This isolates the calculator/commit seam from low-level XP bonus semantics.

### Fixture B — Oteryn Reference player death/delevel

Candidate shape:

```yaml
fixture: REFERENCE_LEVEL7_DEATH_V1

character:
  level: 7
  total_xp: 2600

world_context:
  pvp: false
  promotion: false
  blessings: 0

reference_difference:
  xp_basis: LevelXPSpan_current_level
  skill_loss: 0
  magic_loss: 0

candidate_thresholds:
  level7: 2600
  level8: 4200

candidate_level_span: 1600
candidate_base_loss_rate: 10_percent
candidate_xp_loss: 160
candidate_xp_after: 2440
candidate_level_after: 6
```

The `LevelXPSpan` basis and zero skill/magic loss are accepted Oteryn Reference declared differences.

The exact Global death arithmetic and rounding remain evidence-gated where not already proven.

## 5. Character -> DUR-02 implementation gap

The current intended flow is:

```text
CombatProgressionRewardRef
        ->
REFERENCE_CHARACTER_PROGRESSION_CALC_V1
        ->
StagedProgressionOutcome
        ->
        X
        ->
CharacterRevision compare-and-commit
        ->
PostgreSQL
        ->
durable reconciliation
        ->
restart readback
```

The `X` is genuinely absent on current protected main.

Fresh source inspection found no:

- `apps/game-server/src/durability/character_authority.rs`;
- `apps/game-server/tests/character_authority_postgres.rs`;
- Character Authority forward migration.

The existing WP5 Character Authority allocation is prospective / `NOT_ACTIVE`.

### Minimal future component

Recommended future child:

```text
REFERENCE_CHARACTER_PROGRESSION_COMMIT_V1
```

Input:

```text
CharacterId
expected CharacterRevision
stable source occurrence / semantic operation identity
StagedProgressionOutcome
profile/ruleset/content/SIM/evidence/declaration revisions
current live authority/fence where applicable
```

Required transaction behavior:

1. lock/revalidate current Character root;
2. validate CharacterRevision;
3. validate interpretation revisions;
4. reject stale/fenced writer;
5. detect exact retry / conflicting operation reuse;
6. persist authoritative progression facts;
7. advance CharacterRevision exactly once;
8. persist required reconciliation evidence;
9. atomically commit;
10. reconcile ambiguous/lost-success outcomes from durable state.

Expected outcomes:

```text
COMMITTED
ALREADY_COMMITTED
REJECTED_STALE
REJECTED_REVISION_MISMATCH
REJECTED_AUTHORITY
CONFLICTING_OPERATION_REUSE
AMBIGUOUS -> reconcile same semantic operation
```

Do not add:

- another XP calculator;
- generic signed `progression_delta`;
- private retry queue;
- arbitrary progression map;
- direct Combat SQL.

### Audit/outbox nuance

DUR-02 requires Character semantic mutations to use CharacterRevision and durable operation/reconciliation semantics.

Durable audit/outbox is atomic with a Character mutation **when the owning contract requires durable audit**.

Do not automatically infer that every ordinary XP award requires the same ownership/security event registration used by WP5 bootstrap/transfer operations.

## 6. First durable item fixture

The first R7 durable item should align with the already selected Combat fixture.

Protected Rat evidence provides:

```yaml
creature: Rat
hp: 20
base_xp: 5
corpse_family: Dead Rat
ordinary_loot_candidate_types:
  - Gold Coin
  - Cheese
loot_probabilities: UNKNOWN
```

Recommended first durable item:

```text
Gold Coin x1
```

Reason:

- already part of the protected Rat candidate set;
- simpler than Cheese because it does not introduce food/regeneration;
- no equipment semantics needed;
- quantity can be fixed to one;
- suitable for the smallest DUR-03 MINT + TRANSFER proof.

Important:

```text
Gold Coin x1 in the fixture
!=
claim that Rat naturally drops Gold Coin with probability X
```

The fixture should inject a deterministic test loot output intent and explicitly avoid asserting the natural drop probability.

### Minimal first item flow

```text
Rat death
-> deterministic fixture loot intent: Gold Coin x1
-> DUR-03 MINT
-> acknowledged durable ItemInstance
-> corpse custody
-> LOOT_READY
-> player pickup
-> DUR-03 TRANSFER same ItemInstance
-> CharacterInventory
```

Required properties:

- exactly one ItemInstance;
- planned ItemInstanceId stable across retry;
- exactly one final authoritative immediate location;
- no second mint after lost response/restart;
- no simultaneous corpse + inventory truth;
- same semantic TransactionId across physical retry;
- stale runtime corpse projection cannot recreate the item.

Excluded from first child:

- split/merge;
- bulk pickup;
- multiple output items;
- bank;
- market;
- trade;
- equipment;
- nested container traversal unless unavoidable.

## 7. Reference Content item semantic gap

Current protected `content::ItemDefinition` carries only:

```text
key
presentation_key
materializable
```

This is insufficient for complete Reference item semantics.

The GAME-ITEM domain contract already has the richer semantic concepts, but the Reference production Content profile does not yet carry them.

The correct owner for the minimum static Content delta is the Reference playable successor profile (#504), not Combat or DUR-03.

For `Gold Coin x1`, the first required Content semantics should stay minimal:

```text
stable item definition identity
materializable = true
physical item classification
stack-capable semantics
quantity = 1 in first fixture
authoritative weight/reference weight semantics if the selected inventory move exercises capacity
legal CharacterInventory destination
exact definition revision/provenance
```

Do not pull into the first child unless exercised:

- max-stack behavior;
- split/merge;
- bank/account-ledger semantics;
- trade;
- market;
- equipment;
- broad currency economy.

## 8. DUR-03 state

Fresh protected source still has no ItemInstanceId-based DUR-03 semantic transaction runtime.

Existing PostgreSQL/DUR-02 substrate should be reused.

The future physical owner should remain the Game-local Durability transaction owner already selected by repository architecture.

Do not:

- put SQL into Combat;
- create another DB pool/migrator;
- reuse admission IDs as item TransactionIds by convention;
- put transaction state into GAME-ITEM semantic structs.

DUR-03 implementation remains blocked on its exact live prerequisites, numeric resource ceilings and fresh #162 allocation.

## 9. Newhaven / vocation

The dated Newhaven production chronology is stronger than generic/current documentation wording for the target flow.

Disposition:

```yaml
Newhaven_vocation_flow:
  classification: DERIVED
  confidence: HIGH

generic_current_manual_vocation_at_level8_wording:
  classification: CONFLICT_OR_STALE_GENERAL_WORDING

exact_starter_inventory_by_vocation:
  classification: UNKNOWN
```

The full starter template does not need to block the first Rat XP/loot slice.

## 10. Manifest implications

Current accepted evidence manifest remains revision 3 with no registered Character/Item mechanic cases.

The Character death case is evidence-mature enough for a bounded future manifest mutation allocation, provided the target/Oteryn distinction is preserved.

Suggested Character case scope:

```text
character.death.low_level_base_xp_skill_loss.v1
```

Target side:

- Global low-level XP/skill-loss family;
- bounded PvE/no-blessing/no-promotion fixture;
- exact arithmetic only where admitted by evidence.

Oteryn side:

```text
DeathXPBasis = LevelXPSpan(current_level)
DeathSkillLoss = 0
DeathMagicLevelLoss = 0
```

Parity state remains separate from target evidence strength.

Do not duplicate the same observable as two competing target cases.

## 11. Current dependency/live-state notes

During this investigation:

- PR #635 was integrated, moving protected main from `1995bd...` to `0ab58c...`;
- PR #576 then integrated, moving protected main to `674c998...`.

Final R7 evidence readback during the investigation:

```text
main@674c998ca949422ac018f17f177b590f965a28f7
```

Repository-write preflight on 2026-09-17:

```text
main@ee4a13212d392dd00f8adcd47b103997687b1d6c
```

The intervening protected change is PR #636, a WP3 coordinator-prompt/governance update. It does not alter the R7 Character/Item/Progression source or evidence findings recorded here.

WP3 is now under the protected playable-first / upstream-first programme direction.

Historical #356 remains evidence/reference rather than the default broad-fork terminal implementation.

WP4/#335 and the exact current source/migration custody remain live control-plane dependencies and must be freshly reconciled by #162 before any Character/DUR-03 writer is activated.

## 12. Final R7 disposition

```yaml
programme: 486
lane: char

protected_main_sha_at_repository_write: ee4a13212d392dd00f8adcd47b103997687b1d6c
r7_evidence_checkpoint_sha: 674c998ca949422ac018f17f177b590f965a28f7

evidence_records:
  level_8_threshold_4200:
    classification: DERIVED
    confidence: HIGH

  level_7_threshold_2600:
    classification: DERIVED
    confidence: MEDIUM_HIGH

  level_7_span_1600:
    classification: DERIVED
    confidence: MEDIUM_HIGH

  low_level_kill_xp_bonus_exists:
    classification: DERIVED
    confidence: HIGH
    continuity_to_target: HIGH
    exact_rounding_order: UNKNOWN

  oteryn_reference_death:
    xp_basis: DECLARED_DIFFERENCE
    skill_loss_zero: DECLARED_DIFFERENCE
    magic_loss_zero: DECLARED_DIFFERENCE

  rat:
    hp: 20
    base_xp: 5
    corpse: Dead_Rat
    loot_candidates:
      - Gold_Coin
      - Cheese
    classification: DERIVED_HIGH
    natural_loot_probabilities: UNKNOWN

first_item_fixture:
  item: Gold_Coin
  quantity: 1
  natural_probability_asserted: false
  intended_flow:
    - deterministic_fixture_loot_intent
    - DUR03_MINT
    - corpse_custody
    - DUR03_TRANSFER
    - CharacterInventory

conflicts:
  - low_level_character_cannot_assume_rat_base_xp_equals_final_xp_delta
  - current_Canary_low_level_bonus_behavior_is_not_Global_oracle
  - dated_Newhaven_vocation_flow_vs_generic_manual_wording
  - Global_death_skill_loss_vs_Oteryn_declared_zero_skill_magic_loss

parity_pending:
  - exact_low_level_xp_bonus_curve_and_rounding_order
  - stronger_exact_target_confirmation_for_level7_threshold
  - Character_progression_durable_commit
  - DUR03_item_MINT_TRANSFER
  - Reference_Content_item_static_semantics
  - exact_starter_equipment_template
  - later_skill_magic_progression_arithmetic
  - promotion_edges
  - offline_training_effectiveness
  - Weapon_Proficiency_arithmetic

ready_for_allocation:
  - bounded_manifest_character_death_case
  - XP_threshold_and_low_level_modifier_evidence_amendment
  - later_REFERENCE_CHARACTER_PROGRESSION_COMMIT_V1
  - later_GOLD_COIN_X1_MINT_PICKUP_fixture

do_not_allocate:
  - another_progression_calculator
  - full_skill_training
  - offline_training_breadth
  - full_Weapon_Proficiency
  - broad_itemization
  - full_starter_template
  - bank_or_trade
```

## 13. Recommended control-plane sequence

```text
1. Review/register bounded Character death + XP threshold evidence.

2. Ensure the first kill-XP fixture does not exercise the unresolved
   low-level XP modifier:
   prefer level 50 or an otherwise explicitly evidenced reward context.

3. Continue and complete the current WP3/WP4 prerequisite chain.

4. Activate one minimal Character/DUR-02 progression commit child:
   REFERENCE_CHARACTER_PROGRESSION_COMMIT_V1.

5. Close only the Reference Content item semantics required for Gold Coin x1.

6. Activate one minimal DUR-03 child:
   one-item MINT + TRANSFER, retry/restart/conservation qualified.

7. Compose the first durable Reference gameplay slice:

   Rat kill
   -> base XP consequence
   -> staged progression
   -> durable Character XP commit
   -> corpse
   -> Gold Coin x1 durable mint
   -> pickup
   -> CharacterInventory
```

## 14. Final conclusion

R7 no longer needs another broad research pass.

The primary path has shifted from:

```text
we do not know the mechanics
```

to:

```text
known/evidenced bounded semantics
-> three missing implementation seams
```

Those seams are:

1. `REFERENCE_CHARACTER_PROGRESSION_COMMIT_V1`;
2. minimal Reference Content item semantics;
3. DUR-03 single-item `MINT + TRANSFER`.

The existing progression calculator, Character/Item semantic core, Rat fixture and Combat death/corpse/loot evidence should be reused rather than replaced.

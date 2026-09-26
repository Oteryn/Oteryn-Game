# CONTENT-QUEST-01 r13 — Annihilator source reward settlement ordering witness

Status: **OTS_HYPOTHESIS_ONLY / SOURCE_SETTLEMENT_ORDERING / PROPOSED_NONCANONICAL**
Date: 2026-09-21
Tracking: #707 / PR #709

## Purpose

Record the exact selected Crystal ordering between reward item materialization, achievement side effect and legacy reward storage for Annihilator selectors `6085..6088`, and prevent importer/runtime lowering from treating that imperative Lua order as a native durability contract.

This is static source ordering evidence. It does not claim that a particular crash point is exploitable in a running Crystal deployment; exact persistence/crash durability of each in-memory side effect is not proven here.

## Exact source inputs

| Path | Git blob | Role |
|---|---|---|
| `data-global/startup/tables/chest.lua` | `ea2ef1963395a479ed927481bc3a14065b125a3f` | selectors/reward payload/storage |
| `data-global/scripts/actions/system/quest_reward_common.lua` | `709a1d76512b284933a04d707a634d8c538309ce` | shared reward handler + achievement mapping |
| `data/libs/functions/functions.lua` | `6aeb1fdb7891a45b3bb344b3728ebb0268c91e52` | capacity/backpack preflight helper |

## Annihilator selector configuration

All four selectors share `Storage.Quest.U7_24.TheAnnihilator.Reward`:

```text
6085 -> direct reward item 3388 x1
6086 -> direct reward item 3288 x1
6087 -> direct reward item 3319 x1
6088 -> container 2856, configured weight 50.00, child reward item 3213 x1
```

The same handler maps all four selector UIDs to achievement name `Annihilator`.

Before reward execution, the non-KV branch rejects when shared storage is already `>= 1`.

## Direct selector ordering: 6085..6087

For a direct reward, the shared handler conceptually performs:

1. compute item weight and call `checkWeightAndBackpackRoom(...)`;
2. `player:addItem(itemid, count)`;
3. optional item-text attribute work;
4. `player:addAchievement("Annihilator")` for these UIDs;
5. send reward message;
6. `player:setStorageValue(sharedRewardStorage, 1)`.

The source therefore has no single explicit semantic transaction/receipt object spanning item materialization, achievement and the claim-exclusion storage update.

Textual ordering alone cannot prove the durable crash behavior of those operations, but it **does** prove that the source script is an imperative sequence rather than an idempotent durable settlement protocol.

## Container selector ordering: 6088

Selector 6088 first has an outer backpack/free-capacity preflight because `weight=50.00` is configured.

Then the handler performs:

1. `player:addItem(container=2856)`;
2. pass that container into `playerAddContainerItem(...)`;
3. for each configured child, call `reward:addItem(childId,count)`;
4. optional child attribute work only when `addedItem` exists;
5. send reward message;
6. add `Annihilator` achievement;
7. set the shared reward storage to `1`.

The child loop does not fail the overall source helper when one `reward:addItem(...)` returns no item; it simply skips child-specific attribute work and continues to the later message/achievement/storage steps.

This is especially important for native lowering: a container shell, child payload, achievement and entitlement-consumption fact must not be inferred to have one source-atomic durable boundary merely because they occur in one Lua function.

## Full-inventory / capacity disposition

The source performs a backpack/free-capacity check before direct item addition, and a similar explicit preflight for configured weighted container rewards. Source rejection at that point occurs before the later achievement/storage sequence.

Oteryn preserves the safer semantic rule already selected in Q09/Q07:

- insufficient destination capacity does **not** consume the durable entitlement;
- entitlement remains claimable/reconcilable;
- no invented ground/mail fallback is introduced;
- a later retry uses the same stable ClaimKey/intent semantics rather than creating a fresh entitlement.

## Native durability consequence

The Crystal script order is provenance, not a template for native commit sequencing.

Native reward execution must first resolve the accepted reward alternative to explicit native item identities (r8), then construct one bounded immutable reward intent under the existing reward/GAME-ITEM/DUR-03 boundaries.

For inseparable container+contents value, use either:

- one accepted bounded value transaction that commits the complete intended value set; or
- a named staged owner workflow with stable operation identity and reconciliation.

Do not report reward completion because only the container shell was materialized. Do not mint a replacement operation because the acknowledgement or later side effect is ambiguous.

Achievement, Quest completion/progress, access state and item value remain distinct owned facts. Whether a target product rule requires them to co-commit is an explicit workflow/evidence decision; the legacy script order does not establish target atomicity.

## Interaction with overloaded storage

After a selector succeeds, the source storage becomes `1`. r2 already showed Avar Tar can later grant the base Demon Outfit and mutate the same scalar to `2`, while the source quest door tests `== 1` and reward chest gating tests `>= 1`.

Therefore the native claim receipt/exclusion fact must not be represented by a mutable scalar whose later cosmetic transition can change its interpretation. Claim exclusion survives independently from cosmetic/access progression.

## New required test

- **T84 — partial reward-settlement failpoint matrix:** inject failures/ambiguity before value acceptance, after container shell materialization, during child materialization, after value acceptance but before dependent achievement/progression acknowledgement, and after durable commit before client ACK. The same ClaimKey/value intent must reconcile without duplicate value, missing child value, false completed delivery, duplicate achievement, rerolled/reselected alternative or entitlement loss from a pure capacity failure. Source Lua operation order is never the durability oracle.

Required CONTENT-QUEST-01 corpus is now **84 cases**.

## Evidence classification

```yaml
annihilator_source_reward_settlement:
  source_classification: OTS_HYPOTHESIS_ONLY
  shared_selector_storage: SOURCE_PROVEN
  direct_item_before_achievement_before_storage_order: SOURCE_PROVEN
  container_before_child_before_achievement_before_storage_order: SOURCE_PROVEN
  source_single_atomic_transaction: NOT_PRESENT_IN_INSPECTED_LUA
  running_deployment_crash_exploit: NOT_CLAIMED
  native_claim_durability: DUR_03_AND_REWARD_OWNER
  native_partial_delivery_acceptance: FORBIDDEN
  target_reward_achievement_atomicity: EVIDENCE_REQUIRED
```

This r13 witness composes with r1-r12 and adds no new item/value owner to Quest.

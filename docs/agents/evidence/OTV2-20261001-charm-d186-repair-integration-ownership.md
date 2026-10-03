# Charm repair and integration ownership — 2026-10-01

## Scope and live authority

The owner asked this worker to update #162 with coordinator/architect work and finish its own
Charm work. Root is the sole author of `codex/charm-d186-repair-20261001`, admitted from main
`aad17f99d86abea01bd50b9d559e69eaa5549d88`. The allocation/ownership packet is
[#162 comment 5936434510](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5936434510).
This task repairs the existing pure evaluator; production composition is delivered by owning
children. It does not take over the control plane, shared-path allocation or integration.

The architect's [ruling 5936017719](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5936017719)
assigns the sixteen dormant effects to their owning systems, in the same implementing child and
with a test of the Charm consumer. Critical and leech belong to one GAME-COMBAT formula child.
[D279](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5936039060) allows partial
release after CHARM-6 with dormant effects visibly marked; it does not lift the D252 allocation
freeze or establish full gameplay completeness. This bounded repair is owner-directed work.

## Audit evidence and interpretation

- **PROVEN:** all25 keys exist:14major/11minor. Category, kind, three costs and three stage values
  match both current inspected forks in400/400 scalar comparisons.
- **PROVEN:**25 effect shapes,9 metadata-ready damage calculations and16 named fail-closed
  effects. No production-connected evaluator/state/catalogue/transport/client flow was found.
  Metadata readiness is not evidence that a player can use the effect in gameplay.
- **PROVEN:** Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b` and Crystal
  `96d13eff5a1afef17b11b9abc7d574020381cc53` were inspected through38 exact raw source files
  downloaded with ordinary GitHub/HTTPS. No Remote Desktop operation was used for this audit.
- **PROVEN:** Crystal changed five inspected files since the preparation pin; relevant Charm
  handler arithmetic remains unchanged. Line offsets and removed proficiency variants must not
  be presented as byte-identical whole-source behavior.
- **DERIVED:** inspected OTS code establishes those source implementations, not current live
  Global equivalence or full execution of either fork server.
- **PROVEN:** draft preparation #1434 is unmerged at
  `bcaf0849ad71e800e2b7d43bdbd34a8c780537b4`; it does not activate the gameplay path.

The full audit and25-row comparison are retained at `/workspace/research/charm-audit-20261001/`.
The reproducible repository evidence for this repair is its source diff, accepted D186 row,
focused regression tests and exact-head validation linked by the PR/control-plane packet.

## Repair delivered by this worker

D186 (`OTERYN_GAME_OWNER_DECISION_BATCH_D174_D235_2026-09-30.md`, row24) allows Low Blow,
Savage Blow, Vampiric Embrace and Void's Call on secondary auto-attack targets. The evaluator
previously rejected both leeches there, while the critical event carried no source provenance.

The repair requires explicit `CharmHitSource` on outgoing damage calculation. Eligibility is
effect-aware: main auto attacks and each spell/rune target keep their existing behavior;
secondary auto targets admit critical/leech exceptions, while ordinary procs, Cripple, Fatal
Hold and Carnage remain excluded. Generated Charm damage admits neither calculation nor hit
effects. All four exceptions retain their named missing-system results and exact stage values.

Regression cases cover all stages, main/secondary/generated sources, lethal/nonlethal leech,
mixed-category assignments, wrong-race isolation and deterministic replay. The speed-condition
consistency check binds existing Charm definitions to authored condition identities/durations;
it neither accepts new coefficients nor enables condition consumers.

## Owning delivery map

Names below route work; they are not new allocations or proof that a candidate contract's
header was updated. The coordinator reconciles their latest accepted decisions and leases.

| Work | Owning route | Required consumer result |
| --- | --- | --- |
| Nine damage procs, Carnage and incoming defensive hooks | ATTACK-1; RANGED-1 consumes the same attack pipeline | Authoritative Charm state/catalogue; actual health-reducing hit; mitigation and separate owner commit; Carnage area resolver; no re-entry; loot/XP credit |
| Cripple, Numb, Adrenaline Burst | COND-1c, SPEED-1; CREATURE-MOVE-1 for creature cadence | Apply existing condition definitions to runtime actors and movement; consume the Charm outcome |
| Cleanse | COND-1b semantics/lifecycle, COND-1c composition | Removal, immunity and transfer through the owning condition system; no parallel Charm store |
| Low/Savage Blow and both leeches | One GAME-COMBAT formula child | Equipment eligibility, one effective critical chance, accepted bonus units/rounding/target scaling; D186 and no generated-damage chaining |
| Dodge, Parry, Void Inversion | ATTACK-1/combat-vitals and applicable COND-1 damage ticks | Accepted block/mitigation/Charm/shield ordering; correct nonnegative reflection and authoritative HP/mana mutation |
| Fatal Hold | CREATURE-AI-1 and CREATURE-MOVE-1 | Suppress fleeing for the duration; no rooting or forced new combat target |
| Gut | Loot owner; VSL-COMBAT-01/DUR-03 and D3-2 consumer | Product classification and accepted product-roll algorithm in the actual death/loot flow |
| Scavenge | Tools/Interaction USE_WITH and DUR-03 | Skin/dust attempt and accepted chance scaling; current food/potion ITEM-USE-1 is not a skinning child |
| Bless | DEATH loss owners, with A13/SKILLS-0 successor dependencies | Killer-race-aware applicable loss calculation and durable commit; do not claim current absent skill/magic loss is a calculator defect |
| Promotion and Premium | PREM-2 promotion; PREM-1/PREMIUM-DELIVERY-0/PREM-P entitlement | Replace Charm adapter defaults with current authoritative facts;100promotion echoes and2/6slots |
| Charm Expansion | Accepted commercial entitlement owner | Authoritative expansion evidence for unlimited shared slots; Premium evidence alone is insufficient |
| Gameplay transport and Cyclopedia | CHARM-5-COMP and native client owner | Production progression port, command/view routing, generation equality and observable unlock/assign/state flow |
| Unassign/capability release | CHARM-6 with GAME-ITEM-01/DUR-03 | Atomic gold fee plus assignment removal, accepted wire changes and release gate; no free-unassign substitute |

## Coordinator actions

1. Reconcile this owner-directed bounded repair with D252 and path leases. Keep one writer on
   the evaluator and serialize server/client composition, registries and durable/shared paths.
2. Add the mapped Charm consumer test to each owning child's acceptance rather than creating a
   parallel effect implementation. Route Gut, skin/dust and expansion authority explicitly
   where the current child scope does not already include them.
3. Replace `promoted=false` and `Free` only through accepted authoritative sources, and assemble
   the actual progression port/client path after the required CHARM-6 gate.
4. Qualify the frozen repair head and dispatch the required independent review exactly once.
   This worker does not trigger paid review, enable auto-merge or integrate protected main.
5. Track calculations, dormant effects, connected consumers and tested gameplay separately.
   Do not enable an effect by merely changing `effect_active` metadata.

## Architect actions

1. Apply the issued owning-lane ruling to concrete accepted child scopes and consumer inputs;
   identify any genuinely missing boundary without reopening settled D186/catalogue decisions.
2. Resolve still-unaccepted formula/evidence choices inside their owning children: equipment
   prerequisites, bonus units, rounding/target scaling, condition immunity, incoming ordering
   and product/skinning semantics. Keep unresolved effects fail-closed.
3. Confirm the authoritative expansion input and Character-Item fee transaction where needed.
   A source implementation, proposed schema or local default is not commercial/state authority.
4. Set a consumer-level acceptance case for every activated effect. D279 allows an honestly
   labeled partial release; full25/25playability requires every consumer to be delivered.

## Verification boundary

This repair changes no persistence, migrations, protocol, capability, condition coefficients,
combat composition or production state. Owner-authorized workspace Rust1.94 is used for
deterministic component verification; no Rust/build action runs through Remote Desktop.
PostgreSQL tests need actual PG17.6 plus the configured admin URL: an aggregate PASS with those
inputs absent can include skips and is not proof of database execution. Runtime/client E2E and
activation remain the owning integration children and coordinator's qualification work.

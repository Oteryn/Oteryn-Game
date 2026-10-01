# Charm mechanics preparation for integration

This package completes the **reference preparation** for all 25 catalogue entries. It does not
activate effects, connect gameplay callers, change Character persistence or offer protocol
capabilities. The owner explicitly assigned that connection work to the programme coordinator
and workers on 2026-10-01. GitHub live state remains the authority for implementation allocations.

## Inputs and authority

Use the existing `samples/charms-candidate.json` and `content/charms/` for identities, three
incremental stage costs, stage bonus values and base damage parameters. Use the supplemental
mechanics and progression documents in `rulesets/progression/charms/` with their closed schema
and source evidence. Neither an OTS reference variant nor a missing field is a runtime default.

- **PROVEN:** all 25 category/cost/value triples agree with TibiaPal, Canary and Crystal at
  the pinned revisions below. This comparison proves numerical agreement, not effect parity.
- **PROVEN:** CHARM-0 owner answers govern progression and damage behavior; the D186 refinement
  governs critical/leech effects on secondary auto-attack targets.
- **DERIVED:** source code proves the behavior of those particular OTS revisions. Their matching
  implementations share ancestry and are not independent confirmation of official Tibia.
- **UNKNOWN / CONFLICT:** exact official formulas and conflicting source behaviors stay explicit
  in the package. Resolve only from a named owner decision or stronger reference evidence.

| Source | Pinned revision | Use |
|---|---|---|
| TibiaWiki | Per-page revision and raw-wikitext SHA256 in the original source snapshot | Categories, costs, bonuses and captured descriptions |
| TibiaPal | `61ffa3e0502879ccec44e59ead859e92b6d88531` | Planner descriptions, executed planner/calculator JS and Chromium verification of the pinned `_site` snapshot |
| Canary | `47dfd51f45280a59a1d3e50ba7edd573d7234446` | OTS behavior, formulas, exclusions and implementation defects |
| Crystal | `00ce02a57ca5a12e48f32a3476e37471167e4c3f` | OTS behavior and differences from Canary |
| Official manual | Existing repository capture provenance | Slots, expansion discount and reset descriptions |
| Oteryn | CHARM-0 §§7–8 and owner batch D174–D235, D186 | Accepted product decisions; takes precedence over source quirks |

New wiki reads were unavailable through the proxy during this investigation. The preparation
reuses the committed per-page capture; it does not claim a newly refreshed wiki or a live-game
test. Source paths, line locators and hashes are retained in the supplemental evidence.

## Executed TibiaPal verification

Chromium executed the unmodified pinned site's planner and calculator through local HTTP.
The source and deployed-snapshot JavaScript hashes match. The browser exercised all 25 cards,
their descriptions, icons and costs, 150 purchase/refund transitions, stage limits, 42 major
exact/one-short budget boundaries, insufficient minor echoes, refund protection and reset.
All checks passed. A separate Node VM execution passed 412 planner checks against the original
JavaScript. Persisted observations and portable harnesses are in this authoring directory.

The calculator passed 66 browser cases for Overpower/Overflux, including the 8% health cap,
fractional values and 0/50/100/110/150% elemental sensitivity. These prove the pinned calculator's
output, not official battle behavior. It uses `Math.round`: at maximum health 6813, Overpower
returns 341 while the existing Oteryn unmitigated floor calculation gives 340; at maximum mana
6821, Overflux returns 171 versus 170. Official rounding remains unverified. The calculator
has no player-level input or elemental 2× level cap: at monster health 12613 and 100%
sensitivity it returns 631, while the accepted level-100 cap is 200 before mitigation.
Do not remove the accepted cap or change floor arithmetic merely to match this calculator.

The live domain could not be loaded: the session proxy returned
`ERR_TUNNEL_CONNECTION_FAILED`. This is a real browser test of the pinned local site, not a
claim that the currently deployed website was verified. External ads/fonts/streams were
blocked; local charm icons loaded. The planner describes effects but does not execute combat,
and the calculator explicitly excludes other effect families and general monster mitigation.
Equipment gates, RNG, AoE, statuses, death loss, loot and skinning still need consumer-level
integration proof. Rust tests could not be rerun in this session because `cargo` is unavailable.

## Preserve these rules during connection

1. Durable identity is the `oteryn:charm.*` key. Wire indices are 1-based, derived in key order
   within the same content generation. Canary's 0–24 identifiers are source provenance only.
2. Stage purchases are sequential and costs incremental. Derive balances from unlocked stages
   and completed Bestiary entries; do not introduce a second stored currency balance.
3. Earn 50/100/200 echoes for purchased major stages and 100 once for promotion. TibiaPal's
   unconditional initial 100 is a planner assumption, not the promotion rule.
4. A major requires completed Bestiary stage 3; a minor requires stage 2. One race accepts one
   major plus one minor. Both consume the shared 2/free, 6/premium, unlimited/expansion pool.
5. Attack procs run after a committed health-reducing player hit and require a surviving target.
   Proc damage is a separate owner commit, counts for loot/XP credit, and triggers no further
   charm or leech. Preserve deterministic occurrence identity and replay semantics.
6. Auto-attack damage procs use only the main target. Spells/runes can proc on every affected
   creature. **D186:** Low Blow, Savage Blow and both leech charms also affect secondary
   auto-attack targets. Do not use one blanket off-target filter for every charm family.
7. Keep damage caps local to the effect invocation: seven elemental procs use 2× level,
   Carnage uses 6× level; Overpower/Overflux use the existing 8% creature-maximum-health cap.
   Caps precede mitigation. Carnage remains physical with resistances under owner answer 13a.
8. Character writes stay session-generation fenced. Unassign uses the accepted Character–Item
   gold-fee boundary and CHARM-6; reset requires its own boundary amendment. This preparation
   grants neither an Item transaction nor a reset/potion/Store implementation allocation.

## Runtime work still needed

These are concrete integration requirements, not new allocations. The 2026-10-01 baseline code
has nine single-target damage effects which can be evaluated/applied through the owner commit
seam; even those are not offered as a complete connected playable feature.

| Charms | Existing seam or missing system | Connection requirement |
|---|---|---|
| Wound, Enflame, Poison, Freeze, Zap, Curse, Divine Wrath | `AttackHit`, owner damage commit | Catalogue loader, accepted player-hit caller, elemental mitigation and independent capped damage commit |
| Overpower, Overflux | `AttackHit`, owner damage commit | Read current attacker maxima and target maximum; neutral behavior, resource cap, no chaining |
| Carnage | `CreatureKilled`, `AreaTargetResolver` | Adjacent monster selection, summon exclusion evidence, independent damage and no chained procs |
| Cripple, Numb | `ParalysisCondition` | Typed speed effect, duration/reapplication and opposite-condition behavior; keep OTS formula qualification explicit |
| Adrenaline Burst | `HasteCondition` | Typed speed effect including formula input offset and rounding; storing duration alone is insufficient |
| Cleanse | `ConditionCleanse` | Random eligible active effect, removal and temporary immunity; reconcile condition-list conflict |
| Fatal Hold | `CreatureFlee` | Suppress low-health fleeing for its duration; does not root or force a combat target |
| Dodge, Parry | `IncomingCreatureDamage` | Before/after-damage ordering, reflected base damage and mitigation; reconcile OTS callsite armor conflict |
| Void Inversion | `ManaDrain` | Convert the relevant drain to mana gain; bind signed amounts and damage component selection |
| Bless | `DeathLossCharmInput` | Killer race and existing death-loss policy; avoid the unused extra OTS `percent=10` |
| Gut | `CreatureProductLoot` | Product classification; resolve probability versus quantity before activation (OTS candidate changes product drop probability) |
| Scavenge | `Skinning` | Skin/dust chance scaling; reject the observed OTS division bug |
| Low Blow, Savage Blow | `CriticalHit` | Equipment prerequisite, additive bonus semantics, one effective critical chance and D186 area coverage |
| Vampiric Embrace, Void's Call | `Leech` | Equipment prerequisite, effective percentage and target scaling/rounding; no charm-damage leech, D186 area coverage |

`CharmEffect::missing_system()` and `effect_active` describe the existing server engine, not
this reference package's activation state. Do not set all flags true because all 25 rows are
present. The transport adapter still needs its production `CharmProgressionPort` binding,
command/view routing and capability release after the CHARM-6 gate.

The current effect enum/outcomes carry only duration for speed effects and only removal for
Cleanse. Their consuming systems must carry the missing parameters and immunity semantics.
The current committed-hit hook rejects secondary auto-attack hits for all `AttackHit` effects,
including leech; reconcile this with D186 when connecting those effects. The outgoing damage
and Critical calculation seams must distinguish attack families and target role explicitly.

## Source discrepancies to retain

- **Speed:** the audited OTS implementation evaluates the formula on `baseSpeed - 40`, then
  converts target speed to a delta and applies a minimum-speed clamp. Its coefficients alone
  do not mean `2.5 × baseSpeed + 40`; official exact speed behavior remains unverified.
- **Cleanse:** OTS immunity lasts 11000 ms; Crystal additionally allows Agony. Drown/drunk are
  not in the audited lists. Do not invent official eligibility or duration from those lists.
- **Incoming ordering:** Canary invokes the minor defensive effect before Dodge, Crystal after
  Dodge. Matching bonuses do not establish which ordering is correct for Tibia.
- **Parry:** the wiki says armor reduces reflected damage. Both OTS handlers support an armor
  flag, but audited callsites omit it and the declaration defaults it to false. Follow the
  accepted catalogue rather than mistaking handler support for actual source-path behavior.
- **Critical/leech:** source algorithms differ in critical rerolls and additive versus relative
  leech calculations. Apply the structured-reference bonus meaning; do not import OTS rounding
  and roll algorithms as official facts.
- **Scavenge:** source division by the stage percentage can reduce success as the stage rises.
  Preserve the captured positive bonus; this is an observed source defect, not a target rule.
- **Carnage:** the OTS static level-cap mutation can affect later elemental damage calculations.
  Preserve independent 6× and 2× caps and the owner-selected physical/resistance behavior.
- **Reset:** the official capture describes a free first reset; the OTS path always charges.
  Reset, Charm Upgrade potions and Store lifecycle remain reference-only/excluded work.

## Minimum integration proof

Run the existing catalogue and supplemental mechanics checks first. For each newly connected
runtime family, add independent behavior cases at its actual consumer: prerequisites absent,
chance miss/hit, cap/rounding boundaries, wrong race, both assigned categories, main/secondary
auto target versus spell/rune area target, lethal/no-living target, zero damage, replay,
proc-no-chain/no-leech and reconnect/restart where state changes. Resolve source conflicts
before activating the affected behavior; missing reference knowledge must not become a silent
default. Preserve protocol generation equality and the existing authority/fee contract tests.

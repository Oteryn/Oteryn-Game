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

## Canary/Crystal source-resolution supplement

`samples/charm-source-resolution-2026-10-01.json` traces all 32 original question entries
through five reviewed source lanes. It binds 45 source files at full revisions and hashes,
retains exact quotations, and separates actual fork behavior, accepted project rules and
concrete proposed consumer choices. The 227 source/formula/model checks are reference proof;
they do not execute a connected Oteryn or official Tibia server. An independent review checked
190 citation hashes/ranges and 127 quotations, with the Fatal Hold routing correction applied.

Two original questions were erroneous: Crystal `game.cpp:8811–8825` processes both Adrenaline
Burst and Numb on mana drain. Those claims are removed from the captured facts, their digest
is repinned, and the schema cardinalities/generated mechanics are updated together.

Use these concrete outcomes during integration:

| Family | Source trace and project recommendation |
|---|---|
| Damage | Forks use `ceil`; preserve existing Oteryn `floor`, local 2×/6× level and 8% caps before mitigation. Crystal's strict percentage comparison loses one outcome and its spell-target filter conflicts with accepted area rules. |
| Carnage | Source offsets are four cardinal tiles, selecting the first creature per tile, and the local type check includes summons. Preserve Oteryn's each-eligible-monster contract, summon exclusion, and accepted physical damage with armor/resistances. Reject the shared mutable cap. |
| Conditions | The full speed formula, input offset, truncation and reapplication paths are documented. The paralysis floor is a delta against base speed, not an absolute final speed. Direct charm application bypasses ordinary monster paralysis immunity. Proposed consumer profiles include calibrated chance rolls and reliable effect removal. |
| Cleanse | Both forks select status instances and grant 11000 ms immunity; selecting a type removes its active instances. Agony eligibility differs. The packet proposes an explicit conservative eligibility profile; it does not silently accept a new immunity policy. |
| Critical/leech | Reject Canary's second full critical roll and Crystal's fractional/relative leech units. Proposed arithmetic uses additive fixed-point bonuses, explicit equipment provenance and a shared uniform critical roll (`U <= threshold`, `U` in 1..10000). The Global supplement below withdraws the actual-HP-loss leech recommendation. D186 governs all four families' secondary auto targets. |
| Defensive | Parry block and health paths are sequential; the wrong signed input can heal the attacker. Use one nonnegative reflection opportunity and the already accepted armor behavior. Dodge status/DoT handling, minor ordering and mixed-resource inversion are explicit proposed consumer policies, not inferred official rules. |
| Passives | Fork Gut changes product probability, not stack count; its logging is defective. This is historical fork behavior: the Global supplement below reopens the consumer algorithm. Reject Scavenge's inverted division and preserve positive, capped relative scaling. Bless must use the existing death-loss units; Crystal's configured-loss path mixes fraction and percent. |
| Fatal Hold | Crystal filters even spells to the locked main target; use accepted per-target spell/rune coverage. Generated charm damage reaches `CombatHealthFunc` in both forks, whose Fatal callback lacks an extension guard. The consumer must enforce the owner-wide no-charm-chain rule. |

The remaining original caveats describe official parity or consumer choices; they are not
missing source investigations. Recommendations do not activate runtime or amend contracts.
The coordinator reviews proposed policies while allocating the corresponding consumer work.
`charm_mechanics.py check` also verifies complete original-entry coverage, exact source bindings,
the inactive boundary and rejection of OTS evidence promotion for this supplement.

## Public Global Tibia evidence and recommendation corrections

`samples/charm-global-parity-2026-10-01.json` contains 141 version-scoped source records and
168 claims after browser completion and execution of the remaining reference cases. Records
distinguish official documentation, community references, pinned OTS source execution and
historical player observations. All 25 descriptions and all 75 costs plus 75 stage bonuses match the
fully extracted Brazilian Charms revision 432141 (23 January 2026). Agreement of copied
wiki pages is not independent server proof. Indexed snippets, full extracted content,
official CM statements and players writing on the official forum stay separately classified.

This supplement supersedes the named earlier **consumer recommendations** below, without
rewriting historical fork observations or accepted CHARM-0/D186 contracts. It changes no
runtime loader, switch, persistence or combat consumer. Its validator runs in the existing
offline authoring entry and rejects missing profiles, numeric/evidence drift, promotion of
community sources, invented client proof and runtime-parity claims.

| Subject | Best available Global evidence and resulting preparation |
|---|---|
| Cleanse | Full community documentation explicitly says 11 seconds, so the earlier “OTS only” provenance is corrected. Full official archive8935 announces both Hex removal and corresponding immunity on 25 August 2026; archive8960 reiterates immunity on 8 September. Actual enforcement between those dates was not observed. Hex is a distinct condition; this does not prove Agony eligibility. Include Hex in the documented Global candidate profile; exact type selection, refresh and boundary ordering remain unproven. |
| Gut | Full wiki revision1084324 describes an extra product-loot roll with 6/9/12% chance; corpse-wide versus per-product draws remain unspecified. Withdraw the probability-only Global recommendation. Keep an extra-roll candidate with a test against guaranteed products and duplicate drops; do not implement it as settled official arithmetic. |
| Leech | A historical player test reports 227 HP and 73 mana from a 30-HP Cave Rat, contradicting an actual-HP-loss cap. Withdraw that cap as a Global recommendation. The same answer's AoE formula conflicts with other references, so its arithmetic is not adopted. Full Formulae revision1205374 independently describes base Void-imbuement mana recovery: ceil each target's damage × leech × (0.1×N+0.9)/N, then sum; overkill counts, damage prey does not, critical damage does. This does not establish charm-bonus layering or the complete life-leech pipeline. |
| Parry | Full Parry revision1084043 says the base is before player resistances, while a later player guide disagrees. Official archive4386 independently confirms monster resistance is ignored and monster armor applies, without establishing the player-side base. Withdraw the earlier mitigated-incoming-component base (before mana-shield conversion) as a Global recommendation. Accepted monster armor behavior remains; varying player resistance under equal attacks is the distinguishing test. |
| Dodge | Historical player claims include fewer paralysis and skill-reduction effects. Withdraw independently delivered attached conditions as a Global recommendation. Condition-only, DoT and mixed attacks need direct current-version tests. |
| Area attacks | A fully extracted official CM reply explicitly confirms Low Blow on all Diamond Arrow targets. CM Liamas confirms only the Low Blow exception. Full Low Blow revision1197516 explicitly excludes Savage Blow on secondary area-ammunition targets; leech complaints remain player evidence. The CM reply cannot establish all four accepted D186 exceptions as Global behavior. |
| Carnage | Full official archive8140 confirms physical 15% damage and a 6× level cap. Full wiki revision1188155 specifies four cardinal tiles around the corpse, player last hit, no proc from player Summon/Familiar last hits or monster-summon deaths, no Carnage chaining, armor reduction and no Physical Pierce bonus. This does not exclude nearby summoned recipients or establish fractional rounding. Public linked-video playback was accessible, but no independently reviewed frame or measured proc is used as proof. |
| Bless/Scavenge | Full Bless revision1084311 supports application after ordinary blessings: 86% protection becomes 87.68% at stage 3. Full Scavenge revision1197613 gives 5%→8/9.5/11%, confirming a positive relative modifier; 5% is an example, not every corpse's baseline. Exact per-domain rounding, skinning success caps and special corpses remain unproven. |

Full official archive8140 explicitly permits minor assignment from stage 2 of the creature's
Bestiary entry. This closes the earlier documentation gap; the generic manual overview does
not override that specific rule. The same article confirms incremental minor costs, major
echo awards and promotion echoes. Reset examples imply the excess-level formula
`100000 + 11000*max(level-100,0)`; reset remains excluded scope. Preserve existing floor arithmetic;
available round-number damage examples cannot distinguish floor, ceil and nearest rounding.
Reset/potion/Store references remain excluded consumer work, even when newly researched.

The owner's `content/assets/files` source was independently decoded and checked against the
15.30 admission manifest. Its six semantic files contain creature/bestiary, achievement,
house, boss and quest tables, appearances, proficiencies and map data. They contain no payload
for the 25 Bestiary Charms, no charm IDs/costs/formulas, and only a generic Charm Upgrade item
description (appearance 36726). All six file hashes/sizes match admission; main's staticdata
and catalogue bytes also match. This scoped negative result cannot establish what other
client resources or live server messages contain.

The five lanes retain exact quotations, capture hashes, version limits, contradictory models
and concrete distinguishing cases. **Complete official runtime parity remains unproven.**
No controlled current-server experiment or integrated Oteryn gameplay execution is claimed.

## Completed browser research boundary

The supplement records 24 full public-page captures, seven independently assessed reports
and 89 literal quote checks, with source/capture/excerpt hashes and revision/date limits.
Normal Tavily search preceded Chrome/CDP fallback for blocked or incomplete HTTP access.
Remote Desktop was used exclusively for public internet research; repository authoring and
publication use the ordinary workspace and GitHub API. Same-revision wiki captures close
earlier omitted Notes, without creating independent corroboration.

TibiaMaps and Exevo reports retain 9 and 11 calculator-model checks respectively. These
validate documented models and historical scalar expressions; deployed equivalence and
CipSoft server behavior remain unverified. Reddit remained behind
human verification/login; the Gut video's transcript did not load. Neither is positive proof.
The published Adrenaline speed formula and haste replacement, Fatal Hold's 30-second duration,
Scavenge scaling, Bless order and Carnage gates are now documentary findings. Remaining
questions concern observable rounding/event order, leech layering/divisor eligibility, Gut
roll granularity, Fatal Hold refresh and nearby Carnage summon recipients. Hidden RNG details
are not a prerequisite when candidate implementations have identical observable outcomes.

The research preparation is complete and inactive. The allocated coordinator/consumer task
qualifies any connected implementation; no current Global server or Oteryn combat execution
is claimed by this packet.

## Execution of all remaining reference cases

The owner requested execution of every incomplete item with subagents. The `gap_closure`
section records all 19 exact baseline question groups, their detailed findings, executed
inputs/outputs, source/report hashes and links to all 25 affected Charm profiles. Five
separate lanes execute isolated pinned C++ methods, Lua functions and explicit candidate
models. Exact method execution, mocked call boundaries, mathematical models, public reports
and current-server proof remain separately classified.

| Case | Executed or newly documented result | Consumer qualification boundary |
|---|---|---|
| Agony/Cleanse | Full Agony revision1019512 describes a named Special Condition and explicitly says there is no known removal or mitigation. Crystal's typed Agony cleanse candidate conflicts with that description. | This is stronger than omission from a list; a current Cleanse/Agony observation would qualify any exception. Raw Agony damage remains distinct from the status. |
| Condition selection/timers | Extracted source bodies exercise duplicate instances, removal of a selected type, reapplication and exact expiry boundaries. Whole incoming-condition-list suppression and order-sensitive immunity are reproduced. | These are source behaviors under explicit condition mocks; accepted type selection and live status ordering require the recorded consumer cases. |
| Critical correlation | Full official archive5268,5November2019 documents additive historical8% and shared matching-family outcomes. Finite enumeration proves an alternative conditional family draw produces the same observable joint outcomes. | Retain modern4/8/9% stages. Hidden RNG implementation identity is unnecessary; equipment/proficiency-only eligibility still needs its specific observation. |
| Life Leech rounding | Five historical reported controlled Avalanche/Vampirism inputs fit the documented relative AoE coefficient with per-target ceil5/5; pinned OTS lround fits3/5. | The measurements predate the new leech charms and do not select current charm-bonus layering. Paired candidate signatures distinguish combined/separate/undiluted bonus models. |
| AoE divisor | Mixed admitted, immune, blocked and matching-family candidate sets produce distinct recovery32/53/96. Source paths snapshot affected targets before mitigation. | Measure which admitted targets actually enter the current-game divisor; source admission alone does not prove it. |
| Damage reduction | Extracted damage methods reproduce fractional ceil, cap boundaries and a shared static2×→6×cap leak after Carnage. Reduction-order vectors produce distinct integer outcomes. | Keep accepted exact arithmetic and local caps. Neutral resistance bypass remains separate from mitigation, armor and prey. |
| Physical Pierce | Full official archive8610,25November2025 explicitly removes Physical Pierce in combination with charms. | This primary statement is broader than the earlier Carnage-only wiki observation. |
| Carnage targets/credit | Exact tile selection, damage-credit and extension gates execute; separate permission models cover obstacles, stacks and summoned recipients. Extension does not exclude every later callback. | Distinguish killed trigger victim, attacking summon and nearby recipient. Full PvP, callback/journal and current-server recipient behavior remain specific cases. |
| Defensive/void | Actual extracted decisions reproduce differing health/mana major/minor order, zero/periodic packets, signed Parry arguments and Void RNG before the negative-mana test. | Mocked callback recursion is a conditional model, not observed gameplay. Mixed resources, shields and current-game precedence retain explicit cases. |
| Gut/Scavenge | Unmodified pinned Lua functions execute loot eligibility, per-product draws, stack arithmetic and result-tier branches. Competing extra-roll/capped-success models have concrete discriminators. | Pinned probability-only Gut and faulty Scavenge arithmetic cannot override the qualified community reference choices. Current special-corpse/product ownership observations remain separate. |
| Fatal Hold/Bless | Compiled setters, timers, callsites and flee predicates exercise refresh, lethal/summon gates and killer domains. Binary floating-point makes source XP881 where an exact rational candidate yields880; mana/skill truncation yields880. | Source quirks do not select current Global behavior or change accepted owner rules. |

The retained assessment and execution artifacts provide commands and exact source versions;
source-method reruns require the public pinned raw trees, Python3, g++ and the available Lua
shared library. These are optional research tools, not new CI or runtime dependencies.
Results are stored in the packet, and its immutable hash is checked by the existing offline
validation entry. The independent review also reruns isolated binaries/Lua cases and checks
literal bindings. No whole fork server, live game account or connected Oteryn combat session
is executed by this preparation.

Every identified reference/preparation case is handled. The remaining current-game cases
have measurable inputs and competing outputs; no unresolved value becomes an implicit
runtime default. Connecting the accepted implementation remains the allocated consumer task.

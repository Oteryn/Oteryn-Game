# GAME-VISION-01 — Economy and Scarcity Pre-Decision Addendum

- Status: **PRE-DECISION ANALYSIS / NOT ACCEPTED**
- Date: 2026-08-11
- Parent gate: `GAME-VISION-01`
- Parent analysis: `GAME-VISION-01_PREDECISION_ANALYSIS.md`
- Decision owner: product owner
- Purpose: close the missing economy/sources/sinks/scarcity decision frame required by the accepted 2026-08-10 programme refinement
- Does not authorize: economy implementation, rates, prices, drop tables, taxes/fees, monetization, LiveOps intervention or acceptance of any recommendation below

## 1. Why this addendum is required

The parent pre-decision analysis correctly identifies economy/trade as an unresolved product pillar, but its original owner packet is **not sufficient by itself** to complete `GAME-VISION-01` because the accepted programme refinement requires the vision gate to define:

- economic sources;
- economic sinks;
- scarcity goals.

This addendum is therefore part of the `GAME-VISION-01` owner decision dossier. The parent section titled `Owner decision packet` must be read together with the additional economy decision in section 8 below.

## 2. Accepted constraints

### PROVEN

- Reference and Evolved profiles are separate logical worlds with world-scoped gameplay value by default.
- Cross-profile item/currency/value transfer is forbidden until a dedicated contract proves conservation, migration and balance safety.
- `DUR-03` remains the authority for item/currency conservation and anti-duplication invariants.
- Game Intelligence may measure economy health but cannot autonomously change rates, create/delete value, roll back the economy or deploy policy.
- `GAME-ITEM-01`, `DUR-03`, market/trade and later economy gates own detailed item/value transaction mechanics.
- `GAME-VISION-01` needs product-level economic intent now, but not exact formulas or numeric rates.

## 3. Decision timing test

### Must decide now?

**YES — product-level semantics only.**

Without a launch-level source/sink/scarcity philosophy:

- `GAME-CHAR-01` and later progression can accidentally assume incompatible time/value goals;
- item/loot/content decisions can create uncontrolled faucets before the economy objective is known;
- Evolved improvements can silently become macroeconomic redesign;
- success metrics cannot distinguish healthy progression from inflation or artificial scarcity.

### What remains safely deferred?

- numeric drop rates;
- gold rates and exact sinks;
- NPC price tables;
- market fees/taxes;
- crafting/upgrading costs;
- target inflation percentages;
- item-specific scarcity tiers;
- automated LiveOps intervention;
- monetization and Premium/VIP economics.

These require their owning gameplay/economy contracts and measured data.

## 4. Option A — Reference-rule economy first

The first externally evaluated Reference world uses the selected immutable Global Tibia baseline as the **mechanical source/sink rule oracle** where lawful observable evidence exists.

This means the Reference profile aims to preserve the baseline rules that create and remove gameplay value, such as semantically equivalent loot/reward costs, consumable/resource use, NPC/service costs, death-related value loss where applicable, and other documented faucets/sinks.

It does **not** mean Oteryn promises to reproduce Global Tibia's live market prices, total money supply, item quantities or player-population history. Those outcomes depend on population, age, behavior and historical conditions that Oteryn cannot and should not fabricate.

### Benefits

- consistent with Reference-first product proof;
- minimizes simultaneous gameplay/economy redesign;
- gives item/loot tests a named rule baseline;
- makes Evolved macroeconomic changes later explicit and measurable.

### Risks

- inherited source/sink mechanics may reproduce undesirable inflation or obsolete incentives;
- a young Oteryn world will naturally have different price/liquidity outcomes from an old Global world;
- parity can be miscommunicated as price parity unless disclosure is precise.

## 5. Option B — Reference mechanics with early Oteryn economy corrections

Use mostly Reference gameplay rules but alter selected faucets/sinks/scarcity policies from the first public build to target healthier inflation, liquidity or progression.

### Benefits

- can address known macroeconomic weaknesses immediately;
- may improve long-term sustainability.

### Risks

- weakens the Reference world as a clean behavioral oracle;
- requires many item/progression/content decisions before real Oteryn economy telemetry exists;
- makes bugs versus intentional differences harder to classify;
- creates greater balancing and support workload before the first native slice is proven.

## 6. Option C — economy-led Evolved launch

Treat economy redesign as a primary launch differentiator and define sources/sinks/scarcity independently from the Reference baseline.

### Benefits

- strongest freedom to build a modern long-term economy;
- supports original systems from the start.

### Risks

- highest scope and modelling burden;
- depends heavily on unresolved item/progression/crafting/market/content decisions;
- poor fit with the current need for a narrow, interpretable first product proof.

## 7. Recommended product-level scarcity contract

### RECOMMENDATION — owner decision required

For the first Reference build, choose **Option A: Reference-rule economy first**, with these product-level goals:

1. **Mechanical source/sink parity, not price parity.** Reproduce the selected Reference baseline's documented value-creation/removal rules where they are in scope and lawfully observable; do not fabricate a mature market state.
2. **Scarcity is earned by gameplay rules, not by accidental channel multiplication or technical races.** Duplicate reward eligibility, stale writers and transaction failures must never create value.
3. **No hidden macro tuning.** If Oteryn intentionally changes a source, sink, reward cadence or scarcity rule, it becomes an explicit Evolved/profile revision difference rather than an undocumented implementation convenience.
4. **Value provenance is measurable.** Important item/currency sources and sinks should become classifiable by stable domain cause once the relevant event/transaction contracts exist, so Game Intelligence can measure inflation and concentration without becoming authority.
5. **Scarcity targets are semantic before numeric.** Rare/valuable rewards should remain meaningfully scarce relative to the selected ruleset and activity cadence; exact probabilities, quantities and economy thresholds remain later game-design decisions.
6. **World age and population are first-class context.** Market prices, liquidity and money supply are outcome metrics, not Reference parity constants.
7. **Security/integrity override defect compatibility.** No parity goal may require reproducing a duplication exploit, stale-authority defect or unsafe value mutation.

For the Evolved profile, retain the same conservation/audit foundations but permit explicit later source/sink/scarcity redesign after the Reference baseline and real telemetry exist.

## 8. Additional owner decision required for GAME-VISION-01

The parent owner packet must include this ninth decision:

9. **Economy/scarcity philosophy:** choose one of:
   - **Reference-rule economy first** — recommended; Reference baseline defines in-scope faucet/sink mechanics, while price/liquidity outcomes are measured rather than copied;
   - **Reference with early Oteryn macro corrections** — selected differences are explicit from the first build;
   - **Economy-led Evolved launch** — economy redesign is a primary launch pillar.

If `Reference-rule economy first` is selected, also confirm that the launch scarcity objective is **mechanical rule fidelity + conservation + measurable provenance**, not artificial replication of Global Tibia's historical market prices or total supply.

## 9. Evidence for later supersession

A later Evolved economy decision may supersede the Reference-first recommendation using named evidence such as:

- item/currency source/sink telemetry;
- inflation and money-supply growth;
- market liquidity and concentration;
- progression time-to-upgrade distributions;
- player retention/frustration around scarcity;
- farming/bot/abuse patterns;
- channel/instance multiplication effects;
- content production and balancing evidence.

Any automatic control loop remains a separate later decision; analytics alone never authorizes live economy mutation.

# Imbuement authoring and Global parity audit

The owner requested complete current imbuement definitions and a check against
Global Tibia as of **2026-10-01**, rather than a historical July snapshot.
The original source-facts packet retains its historical decision target as immutable
capture provenance. The active catalogue/schema target comes from the current Global
ledger; it includes Sailor's Backpack and September Formulae evidence.
This draft contains the 24 types and 72 tiers, resolved material and scroll Item
references, source-defined quest predicates, a census of every client item with
imbuement slots, and a field-by-field evidence ledger. It remains a reviewable
authoring package: no runtime consumes these files.

Owned paths are `tools/content-schema/imbuement-authoring/**`, on the single-writer
branch `codex/imbuement-authoring-draft-20261001`. User direction in this chat
authorizes draft preparation and updates, not protected integration or deployment.
The ruleset destination from the full-game tree remains
`rulesets/items/imbuements/`; its existing index is unchanged.

## Contents

| File | Purpose |
| --- | --- |
| `imbuement.schema.json` | Closed JSON Schema 2020-12: eight typed effects, canonical material/scroll references, and exact references to evidence catalogues. |
| `imbuement_authoring.py` | Deterministic catalogue/schema/comparison generation and validation against SHA-pinned evidence. |
| `samples/imbuements-candidate.json` | 24 definitions × 3 tiers; cumulative recipes, correct Global fees, duration, scroll Item, access and eligibility profiles. |
| `samples/imbuement-sources-2026-10-01.json` | Original normalized Wiki/Canary/Crystal facts; old architecture values retained as comparison evidence, not selected fees. |
| `samples/imbuement-bindings.json` | All 72 ingredient names and 72 scrolls, blank scroll, etcher, tome, Gold Token and usable shrine variants with canonical Item identities. |
| `samples/imbuement-access.json` | All 72 direct shrine routes, scroll routes and exact Powerful unlock alternatives, including claim actions after bosses. |
| `samples/imbuement-eligibility.json` | Exhaustive 663-item client census with primary slots, per-source type/tier claims, discrepancies and explicit missing evidence. |
| `samples/global-rules-evidence.json` | Source-qualified rules, concrete transaction/timer hypotheses, architecture conflicts and observation requirements. |
| `samples/current-behavior-answers.json` | Twelve original questions: one owner scope resolution and eleven concrete answer groups across three engines, with 67 full source-file pins and separate Global evidence. |
| `current-behavior-answers.md` | Readable current answer table and implementation disagreements. |
| `samples/global-research-closure.json` | Validated literal public facts, per-group residual fields and separate runtime contracts; accepts public references without mandatory gameplay recordings. |
| `global-research-closure.md` | Scope correction, actual remaining Global facts and source-access report. |
| `samples/global-observation-plan.json` | Supplemental prospective behavioral capture procedures; the date-snapshot question is resolved by the owner's current-data scope. Current engine-answer references are delivered in `current-behavior-answers.json`; no controlled gameplay observations are claimed by this prospective plan. |
| `samples/imbuement-combat.json` | Sourced Vibrancy sequence, bounded AoE leech formula, engine hypotheses and rejected test-server generalizations. |
| `samples/crystal-imbuements-evidence.json` | Additional owner-supplied `imbuements` branch: 72 XML records, 13 pinned source files, execution facts and nine token exchanges. |
| `samples/missing-item-definitions.json`, `samples/missing-item-source-facts.json` | Two concrete Item authoring proposals with primary flags, capacity, weight, slots, acquisition, source revisions and target dates; owning Item validation passes. |
| `samples/imbuement-source-comparison.json` | Reproducible engine differences plus the Global fee correction. |
| `binding_evidence.py`, `eligibility_evidence.py` | Rebuild/check identity and eligibility evidence from pinned repository inputs; never manufacture an identity from a number alone. |
| `capture_bounds.py` | Shared check that every packet keeps third-party captures, quotes and code anchors as bounded excerpts (at most 450 characters per passage and 1,000 per source or anchor) with digest provenance. |
| `test_*.py` | Offline semantic, source-selection and adversarial validation tests. |

## Source order and what was actually checked

1. **Owner-provided CipSoft client 15.30 assets.**
   `content/assets/files/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat`
   supplies object ids, names, raw record hashes and observed slot flags. Existing
   canonical Item definitions and revisioned wiki observations establish identity
   bindings. `staticdata-62d3f5f761a4c8cab02c89bd1a351770aca3504a74f1b34f457f0a0451dcd128.dat`
   supplies staged quest-line locators. Neither file contains the complete server
   imbuement rules; the slot-field interpretation is recorded with its inference
   boundary. Client quest ids are provenance, not invented Game Quest identities.
2. **Official CipSoft material.** The current
   [manual](https://www.tibia.com/gameguides?subtopic=manual&section=characters),
   [Tradeable Imbuements announcement](https://www.tibia.com/news?subtopic=newsarchive&id=8396),
   release news 8436 and Echo Wardens announcement 8834. Full manual extraction
   succeeded. Further research read the complete articles 4779, 4828, 8396,
   8421 and 8436 through Remote Desktop + Chrome/CDP after normal access was
   blocked. The 2018 release supersedes the Vibrancy reflection teaser; the
   2025 live release supersedes the powerful-only scroll teaser. Each digest
   identifies the captured bytes; selected excerpts remain short.
3. **Community data.**
   [Wiki BR Imbuements, revision 427991](https://www.tibiawiki.com.br/index.php?title=Imbuements&oldid=427991),
   quest-specific guides, the twelve real item tables used by the Imbuement Tool,
   and 607 direct per-item [Tibiopedia](https://tibiopedia.pl/items) tables.
   Existing imported Fandom item observations add revisioned identity evidence.
   Initial Fandom extraction failed. Remote Desktop + Chrome/CDP subsequently
   read Imbuing revision 1194750 (2026-07-07), Critical Hit 1113308 (2025-07-24),
   Vibrancy 1194726 (2026-07-06) and Formulae 1205374 (2026-09-07), with public
   revision timestamps. The September Formulae revision is current community evidence. Archived
   Formulae revision 1197205 required login and was not read. Recipe/effect
   corroboration remains community-derived.
4. **Engine hypotheses.**
   [Canary 04b83b512114bfd888000d6e1433ed8ecaec7c5b](https://github.com/opentibiabr/canary/tree/04b83b512114bfd888000d6e1433ed8ecaec7c5b)
   [Crystal summer-update 00ce02a57ca5a12e48f32a3476e37471167e4c3f](https://github.com/zimbadev/crystalserver/tree/00ce02a57ca5a12e48f32a3476e37471167e4c3f),
   and the additional owner-supplied [Crystal imbuements 15593c28fd9adc2bb9739cf0fdb1a4289ebfe1e1](https://github.com/zimbadev/crystalserver/tree/15593c28fd9adc2bb9739cf0fdb1a4289ebfe1e1).
   Imbuement and Item XML facts are compared by qualified identity, not document
   position. Their code answers questions about those pinned implementations;
   `OTS_HYPOTHESIS_ONLY` does not automatically establish Global behavior.

Research used the Tavily search/extract connector, with five independent agent
lanes for rules, identities, per-item eligibility, quest predicates and the
additional Crystal branch. After Tavily returned its plan usage limit, native
public HTTP and existing captures were used; no further metered calls were made.
Remote Desktop was used only for public browser research. Authoring, validation
and publication use the ordinary workspace and GitHub API. Dated TibiaQA answers
were read through normal public HTTP; the 2021 timer screenshot required Chrome.
The evidence records extracted facts and short source excerpts. No third-party
engine implementation, sprites or client asset bytes are redistributed.

## Corrections found during Global verification

| Field | Global evidence / selected catalogue | Earlier draft or engine discrepancy |
| --- | --- | --- |
| Apply fees | **7,500 / 60,000 / 250,000 gold** | Old XML and candidate Oteryn proposal give 5,000 / 30,000 / 200,000. |
| Success / protection | **100%**, no protection add-on | Historical XML fields remain present but do not establish current Global behavior. |
| Basic Punch | **25 Tarantula Eggs** | Canary Basic recipe uses 20 of item 9690; higher tiers use the wiki ingredients. |
| Strike | **Additive +5% chance, +5 / 15 / 40% damage** | Intrinsic character base is separately 5% chance and +10% damage. Prior draft stored the resulting totals as modifiers and could double-count the base. |
| Vibrancy | Recovery from existing paralysis on another paralysis attack; equipped timer in dated community UI; reflection removed at the 2018 live release | Initial-condition admission probability alone does not describe the sequence. Current PvP success-state lifetime remains unresolved; Crystal's XML type is unrecognized by its loader. |
| Scroll inventory | **All 72 scrolls** are bound, including Basic | Basic scrolls were missing from the first candidate. |
| Scroll inscription | Official sources describe **Intricate and Powerful** | Basic loot-scroll presence does not prove Basic shrine crafting. |
| Powerful unlock | Boss completion **plus the applicable reward claim** | Generic boss-only prerequisites omit required steps. |
| Materials source | Global permits **backpack and Stash** | Oteryn's first slice deliberately limits access to direct backpack entries. |
| Conversion compatibility | Five elemental conversions are mutually exclusive in revisioned community evidence | The common access predicate now links that rule for every shrine and completed-scroll route. |
| Item slots | Official client observations take priority in discrepancies | Community tool tables disagree with the client for some items. |

The candidate
[IMBUE-FORGE-0 proposal](../../../docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md)
needs reconciliation with these findings before integration. The draft uses the
evidenced Global fees and records the old proposal as a conflict; it does not
silently rewrite the owning architecture proposal.

## Exact access predicates

Every direct shrine route includes the completed temple construction event and
the character's completed handover of five Heavy Old Tomes to Albinius. Intricate
and Powerful additionally require Premium. The tome handover is historical
character progress, not a requirement to keep five tomes in inventory.

Powerful access uses the per-type source predicate:

- Forgotten Knowledge: the applicable boss completion and use of the associated
  Abandoned Imbuing Shrine, including any report or access step recorded in the packet.
- Heart of Destruction: World Devourer completion and the `worth` reward claim
  from Yana; an alternative only for the eight sourced families.
- Vibrancy: The Nightmare Beast completion plus the report to Vanys or Undal.
- Featherweight: all three Dangerous Depths boss/pump steps.

Shared compatibility checks the requested family and integer tier against the
item catalogue’s `allowed_types[family]` maximum, and prevents conflicting
elemental conversions. Both shrine and completed-scroll routes use these guards.

Completed-scroll application has separate compatibility predicates and Premium/
quest exemptions. Basic exemption strength is labelled as derived from primary
announcements and client evidence; it is not overstated as an observed server
test. The source predicates cover all 72 routes; canonical Quest state is still absent
from `content/quests/definitions/`, so no fake runtime Quest reference is minted.

## Verification limits and admission

The active target is **`global-tibia-current-2026-10-01`**. The owner resolved the
historical date-snapshot question by requesting current data. Genuine source
revision, archive and release dates remain recorded; current community pages and
a versioned client file do not establish every hidden Global server rule.

The eligibility census records all 663 client slot items. Types and maximum tiers
are filled for 629 current items: 607 direct Tibiopedia tables and 22 explicit
Wiki BR fallbacks. Of these, 627 bind to canonical Item definitions; Bursa Obscura
(49160) and Sailor's Backpack (53192) have no canonical definition. Both now have
concrete, source-filled proposals that pass the owning Item schema and validator
with zero errors and warnings. They remain explicitly proposed references.
Thirty legacy
Mayhem/Remedy/Carving items are explicitly documented as withdrawn in version
11.50; four TEST objects lack eligibility evidence and remain excluded.

The selected draft values prefer the direct item table when its slots match the
client. All 607 native slot counts match; the older BR helper has 21 slot errors
and omits Monk-era Punch entries. The 101 disagreements in types or maximum tiers
retain both source claims and use `DERIVED_SELECTED_OVER_STALE_HELPER`; the 22
fallback profiles remain single-source; one has a separately corroborated field.
A further 76 native profiles have only one type/tier source. Across all 629
current typed profiles, 98 are single-source, 430 have agreeing multiple sources,
and 101 retain disagreements. Primary client slots do not corroborate the
community type/tier allow lists.
This source-selection policy
fills the candidate without calling community data verified server behavior.
The packet records zero officially verified per-item allow lists. Exact counts,
affected names and per-source claims live in its `summary` and `items` records.

Sailor's Backpack was introduced on **2026-08-04** and is included in the current
target. The current target therefore has **629 typed candidates**, **627 existing
canonical references**, and two source-filled, unregistered proposals: Bursa
Obscura and Sailor's Backpack. The 30 legacy
items were withdrawn on 2017-12-05. Client presence alone proves neither live
release nor availability. A dated 2024-01-23 patch also corrects Stoic Iks Casque's
Epiphany maximum from tier 1 to **tier 2**, preserving the old helper value.

All 48 Intricate/Powerful scroll item pages explicitly list no monster drop;
these are individual community records, not a universal primary-server proof.
Basic scroll chronology now includes the dated 2026-06-11 teaser and 2026-07-13
15.30 release, before the target. Nine Gold Token material exchanges are recorded
for Strike, Vampirism and Void. Yana's native page independently states **2/4/6
Gold Tokens** and the exact dialogs. These buy cumulative ingredients; they do
not apply an imbuement or pay its gold fee.

The additional Crystal branch has 72 matching numerical strengths and 69
matching effect configurations. Its three Vibrancy configurations disagree with
the selected recovery sequence and are not loaded. Basic scrolls are absent from
its action, quest storage is passed as a boolean, and custom assistant packages
are not Global NPC evidence. Etcher consumption, equipped-scroll targets, timer
configuration, leech rounding and protection composition have concrete code
anchors and verified source-code answers; equivalence with Global remains separately qualified.

Fine-grained timers, leech/critical composition, Etcher consumption, equipped
scroll target acceptance and the Vibrancy PvP success gate have concrete
Canary/Crystal code anchors. The current-behavior answer matrix and its catalogue
references are delivered in `current-behavior-answers.json`. These engine answers are distinct from
qualified Global observations; recipe tables alone cannot prove server transactions.

`candidate_key` identifies a local authoring definition. Percentages use integer
basis points (`100 = 1%`), skill/speed bonuses use points, and duration is
`72_000_000` ms. Schema/semantic validation reject changed evidence pins,
invented Item references, unsupported parity claims, wrong tier/category sets,
noncumulative recipes and old fee values. The 20 exclusion categories belong to
the proposed authoring model. The community same-bonus restriction does not
alone establish shared-category restrictions. Revisioned Fandom Imbuing explicitly
states mutual exclusion across the five elemental conversions; this is selected
as community evidence with a continuity qualification, not an official allow list.

Critical effects explicitly carry `ADDITIVE_IMBUEMENT_MODIFIER` semantics. The
`critical_intrinsic_baseline` rule lives separately in the Global ledger; adding
it once produces isolated totals of 10% chance and +15/25/50% damage. Other
equipment, proficiency and combat interactions are not implied by this example.
The Formulae revision dated **2026-09-07** supplies current community evidence for
a mana-specific per-target ceiling and overkill formula. Its mana-specific scope
does not establish general life-leech rounding.
Separate historical Life evidence now exists: a public answer and revision history
dated 2020-02-28 report per-target Avalanche damage/healing logs for 1–4 Dragon
Lords, wearing one Powerful Vampirism armour and no wand. The reported examples
fit per-target ceiling; unequal damage, other configurations and current target
continuity remain unqualified. A dated Wheel example separately reports 8% Void
+0.5% Wheel = 8.5% mana leech. Named historical charm tests do not become blanket
current charm exclusions.

The final completeness sweep added previously absent completed-scroll success
and invalid-target consumption fields. Their Global values remain explicit
`null` with qualified OTS hypotheses; stackability does not establish single use.
Blank-scroll monster loot now has a full browser capture, exact six-name quote,
revision 428617 and public revision timestamp 2025-08-23, replacing hashless
snippet attribution. A historical `0:00h` Void report records a still occupied
slot: a rounded display alone does not prove expiration or exact internal seconds.

The [research closure addendum](global-research-closure.md) separates35 literal public
facts from narrower unconfirmed fields and runtime contracts across the11 original
behavior groups. This corrects the previous blanket capture requirement. Explicit
scoped official/community sentences qualify data; gameplay recordings are an
alternative. The27 scenarios remain supplemental and `PLANNED_NOT_OBSERVED`.
Remaining consumption, Premium-purchase and transfer-counter fields stay
unknown; PZ/armor source conflicts and precise Life formulas remain qualified.
The engine matrix remains a separate implementation reference. The selected
catalogue covers all24 families and72 tiers, with disputed equipment profiles
retained; full Global parity is not established.

Runtime activation stays `DRAFT_NOT_RUNTIME_READY`. Implementing the proposed
persistence table, timers/checkpoints, ability effects, protocol/UI, quest-state
bindings and qualifying population remains separate implementation work.
A passing source audit is not a playable Global parity test.
The repository workflows do not execute this package's semantic tests. Generic
PR checks therefore do not certify the imbuement catalogue; the commands below
are explicit manual validation. See [audit-report.md](audit-report.md) for the
independent full review, repairs and remaining scope.

The further source pass now includes full official news8834 and actual release8845
for Basic scroll chronology; their quantitative teaser guarantee is separate from
live feature release. A pre-target archived BR Etcher page corroborates existing
facts without filling consumption/Premium unknowns. Historical Market equipment
restrictions and official Forge fusion/tier-transfer restrictions are scoped to
those operations, never generalized to player ownership transfer or scroll trade.
The historical Basic Frost/elemental-arrow experiment remains bounded and its
Shiver case explicitly conflicts with the current wiki's blanket ammunition claim.

The engine-completeness audit separates authoring from canonical/runtime readiness.
Oteryn's24/72 draft is populated; the canonical Imbuement and Quest indices are
still READY_UNPOPULATED. Existing Item slot capabilities have UNKNOWN per-family
limits, and their static tier codec rejects Basic-only max1 used by 12 profiles.
The original XML capture's root-scroll-ID omission is repaired by a separately
pinned correction, with original facts retained. Crystal summer has72 mappings,
Crystal imbuements48 and Canary46. See audit-report.md for versioned comparisons,
actual execution differences and source-name/ID-space qualifications.

## Offline validation

The owner-directed completion batch and the split between research limits and
implementation owners are recorded in [completion-handoff.md](completion-handoff.md).
It records the resolved current-date scope, behavioral evidence qualifications
and the disposition of all 101 helper/native eligibility differences. Filled source profiles are not verified
server allowlists, and the canonical/runtime boundary remains blocked.

```sh
python -m pip install -r tools/content-schema/imbuement-authoring/requirements.txt
python tools/content-schema/imbuement-authoring/binding_evidence.py --check
python tools/content-schema/imbuement-authoring/eligibility_evidence.py --check
python tools/content-schema/imbuement-authoring/missing_item_proposals.py --check
python tools/content-schema/imbuement-authoring/combat_evidence.py --check
python tools/content-schema/imbuement-authoring/newbranch_evidence.py --check
python tools/content-schema/imbuement-authoring/behavior_answers.py --check
python tools/content-schema/imbuement-authoring/research_closure.py
python tools/content-schema/imbuement-authoring/imbuement_authoring.py build --check
python tools/content-schema/imbuement-authoring/imbuement_authoring.py validate
python -m unittest discover -s tools/content-schema/imbuement-authoring -p 'test_*.py'
```

## Targeted completion evidence, 2026-10-01

The packet contains **35 bounded facts and 31 sources**. This batch adds these source-qualified results:

- **101 equipment differences:** [Mirade’s official reply, 3 July 2026](https://www.tibia.com/forum/?action=thread&postid=39592694#post39592694), read with ordinary HTTP 200, gives the Dream Blossom Staff threshold and five named Strike exceptions. It corroborates the selected Strike III for Deepling Ceremonial Dagger and Energized Limb. It names Deepling Fork as an exception; the item’s current table supplies its exact exclusion. All 101 rows have explicit source-choice dispositions. Nine priority item tables were freshly reread and unchanged. The WandsRods helper is dated 16 June 2026; other helpers are mostly from 2024. No exhaustive official allowlist is invented.
- **Physical order:** [TibiaTools’ immutable authored test report](https://github.com/kik-tibia/tibiatools/blob/a1d368906caa8ae98bcb7123f2431733d318d710/src/lib/damage-calc/calc.ts#L21), read with ordinary HTTP, reports resistance before armor in tests on Gazer Spectres and spike traps. Its executable companion models outgoing player-to-creature damage. Current Formulae corroborates a named equipment example; QA87’s dissent remains visible. This selects a scoped reference. All attack types, incoming equipment pipelines and universal rounding remain unproved. The file modification date, 17 September, is not the unknown test date.
- **Life overkill:** [Tibia Analise, 5 January 2024](https://www.youtube.com/watch?v=JQRKU3Jd3gY), read through public Chrome/CDP Show transcript after ordinary access failed, says at 7:27–7:47 that Life and Mana use the damage an attack would deal regardless of remaining HP. The author also says published formulas did not match all his January 2024 tests. This is a dated report; current continuity and exact rounding, unequal-hit and zero-hit rules remain unasserted. The 30 HP / 78 illustration is Mana; the later 223 / 35 example is simulation. Neither is a measured Life result.
- **Vibrancy:** current revision 1194726 and actual 2021 revisions 884981/884982, read through Chrome/CDP, give the same already-paralyzed PvP retrigger example. With the independently named Powerful 50% success, the interaction ends no longer paralyzed. The same attack therefore leaves no active reapplication. This selects the net result, without inventing execution order, persistent immunity, future-attack protection or reset data.
- **PZ:** [TibiaTrends](https://tibiatrends.com/imbuements/), ordinary HTTP, modified 27 September, repeats blanket pause except backpacks without stating a test method. Contrary combat-countdown sources remain; the precise boundary is unresolved.

**Global evidence still insufficient:** exact Etcher success/rejection units; filled-scroll units on success, full slots, incompatible items or duplicate families; the explicit Premium predicate for NPC Etcher purchase; numerical remaining duration across a named ownership transfer; the precise PZ combat boundary; current universal Life rounding, unequal-hit, zero-hit and overkill continuity. Equipped scroll permission remains a dated before/after inference without direct application-instant proof. These values stay null; Crystal/Canary implementation values remain separately available.

Consumption research read four filled-scroll descriptions and 66 full CM posts from a 125-post June–August 2025 index. Fetches stopped at HTTP 429; 58 bodies were not read. Official news 4828 returned Cloudflare verification even in Chrome, so its new discussion link was inaccessible. Both current Life family pages were read in Chrome and contained no precise formula. Remote Desktop was used only for public browser research; the root-owned tab was closed.

## Owner-selected operational policies, 2 October 2026

[owner-authoring-policy.json](samples/owner-authoring-policy.json) contains ten explicit decisions approved in this conversation. The authoring catalogue and schema require this profile; each affected public-research group links its selected policy. Its values are the selected Oteryn behavior for this draft, while source evidence remains independently qualified. Unknown public Global fields no longer mean an undecided Oteryn policy where an owner selection exists. Runtime activation remains blocked.

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

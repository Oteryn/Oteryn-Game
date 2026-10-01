# Imbuement authoring and Global parity audit

The owner requested complete imbuement definitions and a check against Global Tibia.
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
| `samples/imbuement-combat.json` | Sourced Vibrancy sequence, bounded AoE leech formula, engine hypotheses and rejected test-server generalizations. |
| `samples/crystal-imbuements-evidence.json` | Additional owner-supplied `imbuements` branch: 72 XML records, 13 pinned source files, execution facts and nine token exchanges. |
| `samples/missing-item-definitions.json`, `samples/missing-item-source-facts.json` | Two concrete Item authoring proposals with primary flags, capacity, weight, slots, acquisition, source revisions and target dates; owning Item validation passes. |
| `samples/imbuement-source-comparison.json` | Reproducible engine differences plus the Global fee correction. |
| `binding_evidence.py`, `eligibility_evidence.py` | Rebuild/check identity and eligibility evidence from pinned repository inputs; never manufacture an identity from a number alone. |
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
   revision timestamps. Formulae is post-target community evidence. Archived
   Formulae revision 1197205 required login and was not read. Recipe/effect
   corroboration remains community-derived.
4. **Engine hypotheses.**
   [Canary 04b83b512114bfd888000d6e1433ed8ecaec7c5b](https://github.com/opentibiabr/canary/tree/04b83b512114bfd888000d6e1433ed8ecaec7c5b)
   [Crystal summer-update 00ce02a57ca5a12e48f32a3476e37471167e4c3f](https://github.com/zimbadev/crystalserver/tree/00ce02a57ca5a12e48f32a3476e37471167e4c3f),
   and the additional owner-supplied [Crystal imbuements 15593c28fd9adc2bb9739cf0fdb1a4289ebfe1e1](https://github.com/zimbadev/crystalserver/tree/15593c28fd9adc2bb9739cf0fdb1a4289ebfe1e1).
   Imbuement and Item XML facts are compared by qualified identity, not document
   position. They remain `OTS_HYPOTHESIS_ONLY` and cannot close a Global evidence gap.

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

Completed-scroll application has separate compatibility predicates and Premium/
quest exemptions. Basic exemption strength is labelled as derived from primary
announcements and client evidence; it is not overstated as an observed server
test. The source predicates cover all 72 routes; canonical Quest state is still absent
from `content/quests/definitions/`, so no fake runtime Quest reference is minted.

## Verification limits and admission

The immutable repository target remains
`global-tibia-observable-2026-07-28-post-server-save`. Current retrieved pages and
a versioned client file are evidence, not a blanket proof of every server rule at
that exact timestamp. The ledger preserves temporal qualification for each claim.

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

Current item existence and the frozen target are checked separately. Sailor's
Backpack was introduced on **2026-08-04**, after the **2026-07-28** target; it is
excluded from that target while its current facts and proposal are preserved.
The target therefore has **628 typed candidates**, **627 existing canonical
references**, and one source-filled proposal for Bursa Obscura. The 30 legacy
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
anchors; those anchors remain engine hypotheses.

Fine-grained timers, leech/critical composition, etcher consumption, equipped
scroll target acceptance, the exact Vibrancy PvP success gate and target-time
continuity retain explicit
observation requirements. The ledger records attempted sources and distinguishes unresolved observations
from operational topics that still require further evidence. Public recipe tables cannot prove
server transactions.
They are not filled with arbitrary values to make the catalogue appear complete.

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
The post-target Formulae page supplies a mana-specific per-target ceiling and
overkill formula. It does not close life-leech rounding or prove the July target.

Runtime activation stays `DRAFT_NOT_RUNTIME_READY`. Implementing the proposed
persistence table, timers/checkpoints, ability effects, protocol/UI, quest-state
bindings and qualifying population remains separate implementation work.
A passing source audit is not a playable Global parity test.
The repository workflows do not execute this package's semantic tests. Generic
PR checks therefore do not certify the imbuement catalogue; the commands below
are explicit manual validation. See [audit-report.md](audit-report.md) for the
independent full review, repairs and remaining scope.

## Offline validation

```sh
python -m pip install -r tools/content-schema/imbuement-authoring/requirements.txt
python tools/content-schema/imbuement-authoring/binding_evidence.py --check
python tools/content-schema/imbuement-authoring/eligibility_evidence.py --check
python tools/content-schema/imbuement-authoring/missing_item_proposals.py --check
python tools/content-schema/imbuement-authoring/combat_evidence.py --check
python tools/content-schema/imbuement-authoring/newbranch_evidence.py --check
python tools/content-schema/imbuement-authoring/imbuement_authoring.py build --check
python tools/content-schema/imbuement-authoring/imbuement_authoring.py validate
python -m unittest discover -s tools/content-schema/imbuement-authoring -p 'test_*.py'
```

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
| `samples/global-rules-evidence.json` | 43 rules, 14 sources, architecture conflicts and unresolved behavioral questions, each with evidence strength. |
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
   succeeded; news snippets and a labelled news mirror cover unavailable news
   extractions. Every digest states whether it hashes extracted markdown or an
   indexed snippet; a snippet is never described as a full-page capture.
3. **Community data.**
   [Wiki BR Imbuements, revision 427991](https://www.tibiawiki.com.br/index.php?title=Imbuements&oldid=427991),
   quest-specific guides, the twelve real item tables used by the Imbuement Tool,
   and 607 direct per-item [Tibiopedia](https://tibiopedia.pl/items) tables.
   Existing imported Fandom item observations add revisioned identity evidence.
   Live Fandom root extraction failed; exact indexed results are labelled as such.
   Recipe/effect corroboration remains community-derived.
4. **Engine hypotheses.**
   [Canary 04b83b512114bfd888000d6e1433ed8ecaec7c5b](https://github.com/opentibiabr/canary/tree/04b83b512114bfd888000d6e1433ed8ecaec7c5b)
   and [Crystal summer-update 00ce02a57ca5a12e48f32a3476e37471167e4c3f](https://github.com/zimbadev/crystalserver/tree/00ce02a57ca5a12e48f32a3476e37471167e4c3f).
   Imbuement and Item XML facts are compared by qualified identity, not document
   position. They remain `OTS_HYPOTHESIS_ONLY` and cannot close a Global evidence gap.

Research used the Tavily search/extract connector, with four independent agent
lanes for rules, identities, per-item eligibility and quest predicates.
No third-party engine code, wiki prose, sprites or asset bytes are copied into
the new evidence files.

## Corrections found during Global verification

| Field | Global evidence / selected catalogue | Earlier draft or engine discrepancy |
| --- | --- | --- |
| Apply fees | **7,500 / 60,000 / 250,000 gold** | Old XML and accepted Oteryn decision give 5,000 / 30,000 / 200,000. |
| Success / protection | **100%**, no protection add-on | Historical XML fields remain present but do not establish current Global behavior. |
| Basic Punch | **25 Tarantula Eggs** | Canary Basic recipe uses 20 of item 9690; higher tiers use the wiki ingredients. |
| Strike | **10% chance**, +15 / 25 / 50% damage | Crystal's raw chance and bonus values differ from Wiki/Canary. |
| Scroll inventory | **All 72 scrolls** are bound, including Basic | Basic scrolls were missing from the first candidate. |
| Scroll inscription | Official sources describe **Intricate and Powerful** | Basic loot-scroll presence does not prove Basic shrine crafting. |
| Powerful unlock | Boss completion **plus the applicable reward claim** | Generic boss-only prerequisites omit required steps. |
| Materials source | Global permits **backpack and Stash** | Oteryn's first slice deliberately limits access to direct backpack entries. |
| Item slots | Official client observations take priority in discrepancies | Community tool tables disagree with the client for some items. |

The accepted
[IMBUE-FORGE-0 decision](../../../docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md)
needs reconciliation with these findings before integration. The draft uses the
evidenced Global fees and records the old decision as a conflict; it does not
silently rewrite an accepted architecture document.

## Exact access predicates

Every direct shrine route includes the completed temple construction event and
the character's completed handover of five Heavy Old Tomes to Albinius. Intricate
and Powerful additionally require Premium. The tome handover is historical
character progress, not a requirement to keep five tomes in inventory.

Powerful access uses the per-type source predicate:

- Forgotten Knowledge: the applicable boss completion and use of its reward-room
  Abandoned Imbuing Shrine.
- Heart of Destruction: World Devourer completion and the `worth` reward claim
  from Yana; an alternative only for the eight sourced families.
- Vibrancy: The Nightmare Beast completion plus the report to Vanys or Undal.
- Featherweight: all three Dangerous Depths boss/pump steps.

Completed-scroll application has separate compatibility predicates and Premium/
quest exemptions. Basic exemption strength is labelled as derived from primary
announcements and client evidence; it is not overstated as an observed server
test. The source predicates are complete; canonical Quest state is still absent
from `content/quests/definitions/`, so no fake runtime Quest reference is minted.

## Verification limits and admission

The immutable repository target remains
`global-tibia-observable-2026-07-28-post-server-save`. Current retrieved pages and
a versioned client file are evidence, not a blanket proof of every server rule at
that exact timestamp. The ledger preserves temporal qualification for each claim.

The eligibility census records all 663 client slot items. Types and maximum tiers
are filled for 629 current items: 607 direct Tibiopedia tables and 22 explicit
Wiki BR fallbacks. Of these, 627 bind to canonical Item definitions; Bursa Obscura
(49160) and Sailor's Backpack (53192) have no canonical definition. Thirty legacy
Mayhem/Remedy/Carving items are explicitly documented as withdrawn in version
11.50; four TEST objects lack eligibility evidence and remain excluded.

The selected draft values prefer the direct item table when its slots match the
client. All 607 native slot counts match; the older BR helper has 21 slot errors
and omits Monk-era Punch entries. The 101 disagreements in types or maximum tiers
retain both source claims and use `DERIVED_SELECTED_OVER_STALE_HELPER`; the 22
fallback rows remain `COMMUNITY_SINGLE_SOURCE`. This source-selection policy
fills the candidate without calling community data verified server behavior.
The packet records zero officially verified per-item allow lists. Exact counts,
affected names and per-source claims live in its `summary` and `items` records.

Fine-grained timers, Vibrancy PvP behavior, leech/critical composition, etcher
consumption, loot of completed Intricate/Powerful scrolls and target-time
continuity have their own remaining evidence limits.
They are not filled with arbitrary values to make the catalogue appear complete.

`candidate_key` identifies a local authoring definition. Percentages use integer
basis points (`100 = 1%`), skill/speed bonuses use points, and duration is
`72_000_000` ms. Schema/semantic validation reject changed evidence pins,
invented Item references, unsupported parity claims, wrong tier/category sets,
noncumulative recipes and old fee values.

Runtime activation stays `DRAFT_NOT_RUNTIME_READY`. Implementing the accepted
persistence table, timers/checkpoints, ability effects, protocol/UI, quest-state
bindings and qualifying population remains separate implementation work.
A passing source audit is not a playable Global parity test.

## Offline validation

```sh
python -m pip install -r tools/content-schema/imbuement-authoring/requirements.txt
python tools/content-schema/imbuement-authoring/binding_evidence.py --check
python tools/content-schema/imbuement-authoring/eligibility_evidence.py --check
python tools/content-schema/imbuement-authoring/imbuement_authoring.py build --check
python tools/content-schema/imbuement-authoring/imbuement_authoring.py validate
python -m unittest discover -s tools/content-schema/imbuement-authoring -p 'test_*.py'
```

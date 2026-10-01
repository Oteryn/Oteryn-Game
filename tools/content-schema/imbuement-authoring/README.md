# Imbuement authoring schema candidate v1

Preparation of the 24 imbuement types and their 72 tiers under
[IMBUE-FORGE-0 sections 3–5](../../../docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md).
Requested directly by the owner on 2026-10-01 as a reviewable draft. Owned paths:
`tools/content-schema/imbuement-authoring/**`; one author on
`codex/imbuement-authoring-draft-20261001`. This is preparation only, with no
programme allocation, runtime activation, merge request or production authority.

The full-game tree assigns definitions to `rulesets/items/imbuements/`. This draft
keeps its samples under the authoring tool until the content lane resolves the
remaining bindings and qualifies population. The existing directory index stays
`READY_UNPOPULATED`. No runtime reads these samples.

| File | Purpose |
| --- | --- |
| `imbuement.schema.json` | Generated JSON Schema 2020-12 with closed records and eight typed effect shapes. |
| `imbuement_authoring.py` | Offline deterministic `build`, `build --check`, and schema plus source-consistency `validate`. Writes only inside this tool directory. |
| `test_imbuement_authoring.py` | Positive coverage and rejection checks, including source disagreements and blocked admission. |
| `samples/imbuements-candidate.json` | 24 definitions, three tiers each; source facts and accepted Oteryn fees are selected explicitly. |
| `samples/imbuement-sources-2026-10-01.json` | Normalized facts, source identities and SHA-256 pins. No wiki prose, sprites or third-party engine code. |
| `samples/imbuement-source-comparison.json` | Generated per-name/per-tier engine differences and remaining blockers. |

## Sources and selection

Research used the Tavily search and extract connector. Search discovers sources;
the retrieved page or pinned engine file supplies the facts. Extracted markdown
digests describe the retrieved representation, not a MediaWiki wikitext digest.

- **Official Tibia manual**, [characters section](https://www.tibia.com/gameguides?subtopic=manual&section=characters):
  20-hour duration, three power tiers, up to three slots, unequipped target and the
  shrine/scroll distinction. These agree with the repository's manual snapshot.
- **TibiaWiki BR**, [Imbuements, revision 427991](https://www.tibiawiki.com.br/index.php?title=Imbuements&oldid=427991):
  primary community evidence for all 24 names, tier effect numbers and the three
  incremental ingredients per type. Higher-tier recipes include lower-tier
  ingredients. Selected facts remain `DERIVED` and every definition remains
  `PARITY_PENDING`; the page's approval date does not prove immutable target parity.
- **Canary**, [04b83b512114bfd888000d6e1433ed8ecaec7c5b](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data/XML/imbuements.xml):
  72 records, costs, category exclusions, material ids, storages and scroll ids.
  `OTS_HYPOTHESIS_ONLY`, as required by the accepted decision.
- **CrystalServer summer-update**, [00ce02a57ca5a12e48f32a3476e37471167e4c3f](https://github.com/zimbadev/crystalserver/blob/00ce02a57ca5a12e48f32a3476e37471167e4c3f/data/XML/imbuements.xml):
  independently inspected 72 records, compared by `(name, tier)` because the
  document order differs. `OTS_HYPOTHESIS_ONLY`; not a second parity oracle.
- **[Imbuement Tool](https://www.tibiawiki.com.br/index.php?title=Imbuement_Tool&oldid=210844)**:
  calculator last modified in 2018. Its Powerful Vampirism ingredients agree;
  its 250,000-gold total includes protection and is not selected as the Oteryn fee.
  Market ingredient prices and Gold Token conversion are outside this schema.
- **[Fandom Imbuing](https://tibia.fandom.com/wiki/Imbuing)**: both advanced
  extraction attempts failed. Tavily search returned the root-page snippet only.
  Recorded for discovery; no numeric field claims corroboration from this page.
- **Oteryn IMBUE-FORGE-0 sections 3 and 6**, owner decisions R2 and I1/D178:
  always succeeds, no protection charge; apply fees 5,000 / 30,000 / 200,000 and
  clear fee 15,000 gold. Every direct shrine tier requires a quest predicate;
  Intricate and Powerful additionally require Premium. The canonical per-type
  predicates are unresolved, including Powerful alternatives, and fail closed.

## Differences requiring explicit source choice

| Field | Wiki BR / selected candidate | Canary | Crystal summer-update |
| --- | --- | --- | --- |
| Strike | 10% chance; +15 / 25 / 50% damage | same numbers (raw XML chance 1000; bonus 1500 / 2500 / 5000) | raw XML chance 500; bonus 500 / 1500 / 4000; different from the wiki and Canary |
| Basic Punch | 25 Tarantula Eggs | 20 of source item 9690 | 25 of source item 10281; agrees with wiki quantity |
| Vibrancy | 15 / 25 / 50% deflection chance | `paralysis`, explicit `pvpDeflect=1` | `vibrancy`, no PvP attribute |
| Fee / success | accepted Oteryn decision: always succeeds, no protection charge | historical 90 / 70 / 50% base success plus protection prices | same historical model |

The candidate selects the wiki's Basic Punch recipe, rather than carrying Canary's
Basic recipe forward into the higher tiers. Raw engine ids remain qualified source
facts; no numeric-id/name equality is used to mint an Oteryn Item reference.
Vibrancy PvP behavior stays unresolved; the schema carries the chance alone and
does not imply that the engines implement the same admission behavior.

## Shape and admission boundaries

- `candidate_key` is a local authoring identifier, not a minted ContentKey or
  wire identifier. Population chooses canonical keys and definition revisions.
- One shared `elemental_damage` category covers all five conversions; the other
  19 types have separate exclusion categories. Categories govern the decision's
  one-per-category rule; they do not implement it.
- Percentages use integer **basis points** (`100 = 1%`); skill and speed bonuses
  use integer points. Duration is exactly `72_000_000` ms. There is no timer state
  or ability-pipeline implementation in a definition.
- Each material has a source name, positive quantity and an explicit
  `UNRESOLVED` Item binding. Quest, per-type unlock and per-item eligibility
  bindings are also unresolved. Generic wiki equipment classes must not replace
  the Item schema's slot counts and per-type maximum tier.
- `activation` is fixed to `DRAFT_NOT_RUNTIME_READY`; the validator rejects
  runtime-ready claims, invented bindings, unproven parity, unknown fields,
  category drift, invalid tier sets, changed source facts and source-value drift.
- `validate` is deliberately a validator for this pinned **candidate**, not a
  future production importer. New evidence requires a reviewed source refresh,
  an updated facts digest and regenerated samples, not a silent override.

## Next implementation slices

1. **IMBUE-CONTENT-1:** qualify Reference target evidence; resolve material Item
   references, quest/unlock predicates, shrine objects and per-item allow lists;
   mint definition identities and populate `rulesets/items/imbuements/`.
2. **IMBUE-1:** add the accepted `game_item_imbuements` persistence shape and
   transaction semantics for Apply/Clear/Expiry. No migration is added here.
3. **IMBUE-RT-1 / IMBUE-WIRE-1:** timer/checkpoints, ability effects and protocol/UI.
   Scroll production/application remains IMBUE-SCROLL-1.

## Local validation

```sh
python -m pip install -r tools/content-schema/imbuement-authoring/requirements.txt
python tools/content-schema/imbuement-authoring/imbuement_authoring.py build --check
python tools/content-schema/imbuement-authoring/imbuement_authoring.py validate
python tools/content-schema/imbuement-authoring/test_imbuement_authoring.py
```

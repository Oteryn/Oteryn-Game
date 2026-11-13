# All-category local item field delta

This **DERIVED, PARTIAL** packet reuses the complete6470-appearance /6228-type
client cohort and the existing published observations. It adds exact compiled
enum meanings, a thin field-presence/index projection and the **two actual
current-file proficiency changes**. No runtime item values are installed.

The pinned sources are PR3170 at54c930ab3771d28d7d5584beed6b3e69125bf5a7,
PR3173 at657e3bf49ad1eadaf5390a7d97e1dd125c317d97, PR3175 at
0515417abe9bf7b77c1842c988712e2fd838fa4b and PR3179 at
539fcd4b9ddeb05d776b70e0a1d72d6f92080560. Exact earlier category PR
heads/file inventories and consumed JSON hashes are in `source-receipts.json`.
The full `content`, `imports` and `tools/content-schema` trees have no changes
between comparison pin5efd67da164ac9dc33c8c066434b228cebb86c2b and fresh
maince45063507c22a11e0db7e9c0ff86dacbe6ff834. That preserves earlier
source comparisons, including their candidate-only limitations.

## What is added

| Member | Added qualification |
|---|---|
| `compiled-enum-delta.json` |Static executable descriptor for all27 ITEM_CATEGORY values,9 WEAPON_TYPE values and8 VOCATION values; qualifies tags60–64 through the exact PR3179 field map |
| `appearance-field-coverage.json` |6470 rows referencing existing typed field values, exact tag-presence bits, candidate UI property sets and private frame/sprite evidence |
| `category-field-coverage.json` |Per-source-category field counts, all1288 market-absent appearances, declared-but-unrepresented enum categories and separate physical UI taxonomy |
| `ui-field-reuse-and-queue.json` |Already-observed properties on2623 scoped physical UI rows and314 overlapping legacy observations; ordered work queue that reuses89 private PNGs and one clipboard capture before recapture |
| `current-proficiency-delta.json` |443/444 profiles unchanged;3678/3680 perks unchanged; only profile499 level2/perk1 and level4/perk1 changed |
| `source-receipts.json` |Exact dependencies, hashes, source comparison scope and private custody receipts |

The source package is15.33.dd0955. The live executable SHA is
7f6555b19514abc34220a09f427732ea66ddfcd8d7052c733f0557d624baaf34 and
the appearance file SHA is
aa086346daa9234d721e4170848095ad83f3ee9d28ea9e62c1bd06216016174e.
The current proficiency file is a77c88ea13f8b1909576a34ba309ad721bfc461046509509d65f347805fd8607,
not the earlier9fecbbddf2b51e45c0498ee97cc01dc3d9070c54d4cc7b900d0a142c51fb004c.
Both have511849 bytes. Profile499 level2/perk1 changes raw BestiaryId14,
Value0.015 to BestiaryId2, Value0.02; level4/perk1 changes raw BestiaryId14,
Value0.02 to BestiaryId18, Value0.025. Literal names remain hash custody.
Type33 Value0.05 is unchanged and its previous codec qualification is retained.

## How to avoid duplicate collection

Read the existing physical UI row before choosing a field to collect. Its
recorded properties are reusable observations, including Weight on all2623
rows. `appearance-field-coverage.json` links only **candidate** property sets
through the existing qualified relations:813 appearances have retained UI
relations,5657 do not in those receipts;753 unique candidates,29 candidate
sets and15 held relations are not authoritative server-ID bindings. Zero new
bindings are added. Existing legacy314 records have unknown global ordinals
and cannot be subtracted as314 distinct items. Missing relations do not mean
an item was never observed.

The next queue begins with associating the already-saved1853 capture material
to cards and transcribing any genuine additional fields, including potion-tail
material. It then resolves ambiguous/held identities. Only a verified physical
card/field not covered by those sources becomes eligible for a new GUI read.
The previous clipboard capture proves a historical successful text copy, not
current copy support on every field or session. This publication sends no game
input and makes no current-session copy-capability claim.

Source market categories and physical Cyclopedia categories are different
taxonomies. For example,81 market14 appearances lie inside the6470 cohort;
PR3179 preserves82 Tools across all44543 appearance objects, including
outside-cohort appearance3247 (djinn's lamp). Gold is a physical UI category
without a separate compiled ITEM_CATEGORY value. No category is silently
discarded or forced into another category, and counts are not GUI list totals.

## Boundaries

Exact typed sparse values, paired NPC tuples, names, description custody,
five-tag raw values, aliases and proficiencies already exist at the pinned
dependencies; this packet references them instead of duplicating tables.
NPC raw tuple price axes are not flattened or assigned guessed currency.
Present zero remains present; absent fields remain absent. Sprite references
and frame counts are available locally, with16398 sprite-reference occurrences;
this does not prove rendered icon identity or effect animation equality.

No Weight field occurs in the compiled Appearance schema or the bounded
allowlisted caches inspected in PR3179. This is **not** proof that all6470
items need screenshots or that Weight is absent from every local file.
Unrecorded stats are neither zero nor automatically applicable. First-panel
NPC observations do not establish independently scrolling sublist completion.
The GUI list census, full field parity, runtime stat application and canonical
item bindings remain unqualified. Original assets, PNGs, literal description
narratives, account configuration and credentials remain outside Git.

## Validation

From repository root:

```sh
python docs/agents/evidence/items-local-field-delta-20261113/validate.py
python docs/agents/evidence/items-local-field-delta-20261113/validate.py --source-dir <private-cross-pr-snapshot-directory>
python tools/agents/validate_governance.py
python -m unittest discover -s tools/agents/tests
git diff --check
```

The optional source check verifies pinned file hashes, all6470 field/tag
projections and candidate property unions against existing published data.
Negative controls reject loss of market-absent rows, a promoted binding and
a substituted profile delta. This is not independent review or runtime parity.
No game build applies to these evidence-only paths. Draft publication does not
authorize merge or deployment.

Allocation: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6862554312.

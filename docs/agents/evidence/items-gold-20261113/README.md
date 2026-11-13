# Gold ordered Basic and initial-panel evidence

The Gold category contains three natively read physical entries: crystal coin,
gold coin and platinum coin. Their observed Weight literals are respectively
0.10 oz, 0.01 oz and 0.10 oz. This packet retains three Basic PNG references and
12 freshly checked original card-proof hashes. All three entries were visible
at the observed stable origin; two stable end checks before and after selection
qualify the list boundary without inventing scrollbar geometry.

## Display absence differs from false, unknown and zero

Weight is the only displayed Basic property. Description and Tradeable In Market
are explicitly ABSENT_FROM_OBSERVED_BASIC_FIELDS for all three cards. Neither
absence is false, N/A, an intrinsic UNKNOWN nor evidence that the item has no
such source semantics. Market is not source-compared. No description was observed,
so there are zero private Description/layout receipts; this is not silent omission
of an observed narrative value.

Separate client display facts remain exact and unnormalized: Average Market Price
shows the string `0`; Resulting Value shows `10,000`, `1` and `100`, each with the
observed gold-colored coin glyph. The selected Loot Value Source is NPC Buy Value.
Skip when quick looting is unchecked/checked/unchecked across the three cards.
These are observed display/checkbox states, not intrinsic marketability, runtime
currency IDs, admitted offers or source-value bindings. A visible zero is retained
as zero; a coin glyph does not establish an internal unit or serverID.

`field-presence-and-display-coverage.json` makes the Basic/display distinction
explicit. Names, icons, ordinals, capture IDs, namespaces, appearances and
Cyclopedia IDs never establish numeric client/server identity.

## Initial NPC panes remain bounded observations

The saved initial Sell To and Buy From panes are empty for all three cards:
six OBSERVED_EMPTY_PANEL states. Their three Basic proof images are a subset of
the 12 card proofs and are not counted twice. This is not global NPC absence,
complete trade semantics or evidence of any transaction/conversation. Other
Items categories, NPC pagination where needed, animation, source semantic parity
and the overall static Cyclopedia audit remain PARTIAL.

## Private collection recovery and exact custody

`category-end-receipt.json` binds the observed category boundary, native selection,
V2 journal POST_END_CONFIRMED, three capture attempts and Root's independently
verified fresh known world after modal close. Its end/world images are separate
from the 12 card-proof count. Native pixels, not OCR candidates, qualify names.

The preserved V1 attempt failed in local PIL resize before any index or card PNG
was persisted. Root verified the absent index and zero prior card PNGs before
recovery. A minimal private BasicCollector repair bypassed an unused resize only
for the explicit non-authoritative OCR fallback (empty candidate/confidence zero),
kept the real OCR route and an empty-crop guard, and compiled successfully. The
old/new private source hashes and repair/error receipts are retained. No Game
production code or collection helper is shipped by this PR. Historical repair
and reader PENDING snapshots remain unchanged; the separate V2 result and newer
native reader packet qualify actual recovery and all three facts. No persisted
card was duplicated, and V1 is not represented as a clean successful run.

All source reader/proof bytes used here were freshly hashed on Ubuntu. Raw PNGs,
full private JSON, collector source/configuration and access material stay private.
The repo holds only compact non-narrative literals and relative proof receipts.
Cloud verifies compact integrity and pinned-source reproduction, not a fresh
visual read of those private screenshots.

## Bounded source comparison

`compare_sources.py` reproduces Weight-only comparisons at freshly verified Game
main 5efd67da164ac9dc33c8c066434b228cebb86c2b: 69 canonical shards, manifest
and three selected-profile inputs, 73 source receipts. All three names have one
candidate and all three Weight values compare equal in hundredths of an ounce.
Every correspondence remains UNQUALIFIED_NAME_CORRESPONDENCE, including unique
matches. Equality does not prove runtime admission, renderer presence conditions,
client/server IDs or complete gameplay parity. Market and client display facts
are explicitly not compared; no gameplay/source-only fix follows.

## Validation and lifecycle

```text
python docs/agents/evidence/items-gold-20261113/compare_sources.py --check
python docs/agents/evidence/items-gold-20261113/validate.py
python tools/agents/validate_governance.py
python -m unittest discover -s tools/agents/tests
git diff --check
```

Checks cover literal weights, Basic absence versus display zero, checkbox states,
empty-pane qualification, immutable proof hashes, V1/V2/private-repair custody and
pinned Weight-only source reproduction. CI/formal review remain separate gates.
Draft only; overall task stays waiting/PARTIAL, without merge or deployment.
Frozen PR3150/3152/3153/3154/3155/3156 and private tail/retake helpers stay unchanged.

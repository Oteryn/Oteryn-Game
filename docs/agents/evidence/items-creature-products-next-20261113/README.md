# Ongoing Creature Products evidence, 13 November 2026

This is an AUTHORING evidence stream for physical Cyclopedia rows135 onward.
The initial qualified cutoff is247:113 rows, with no missing ordinal inside this
cutoff. The category and overall static Cyclopedia audit remain PARTIAL.
Later captures are not automatically qualified or included. This one draft can
receive a later coherent reader batch; it is not a PR per100 rows.

The frozen earlier packet is [PR3152](https://github.com/Oteryn/Oteryn-Game/pull/3152)
at `57403cf2ef48cc6e8cd2ac278ccbf413faa4370a`, covering Creature Products1–134
and separate Armors/trade evidence. Those facts are linked, not duplicated here.
Frozen census [PR3150](https://github.com/Oteryn/Oteryn-Game/pull/3150) is unchanged.

## Qualified observations and custody

`observations-135-247.json` retains all non-narrative Basic field literals from
113 manually reviewed cards in Tibia Test Server15.33.dd0955. Its452 relative
PNG keys and SHA256 were checked against private Ubuntu bytes. Images, private
audit/internal metadata, access configuration and credentials are not exported.
Source JSON remains immutable privately; its exact hash identifies the input.

Sixteen Description values remain observed1:1 in the private source. The repo
explicitly retains their UTF8 SHA256, byte and character lengths with state
`OBSERVED_VALUE_RETAINED_PRIVATELY`. This is an export/custody statement, not
UNKNOWN, NOT_APPLICABLE, absent, empty or zero. `field_export_policy` distinguishes
literal exported fields from privately retained narrative text. Full repository
literal-text parity is not claimed. All other emitted Basic literals are intact.

Physical UI ordinal and capture ID are different observations. Same-name
chitinous mouth rows163/164 and dark bell rows222/223 remain separate, including
their differing observed weights. A name, picture, appearance ID, Cyclopedia ID
or source namespace does not establish serverID or native definition reference.
Trade pagination, animation, instance state and complete category coverage remain
unqualified. Missing fields are not inferred zero, false or N/A.

## Two repaired capture gaps

| Physical row | Separate native capture | Observed name | Weight | Tradeable In Market |
|---|---|---|---|---|
|197|246|crown|0.90 oz|yes|
|237|247|deepling breaktime snack|2.40 oz|yes|

The resumable collector had applied an inherited complete-text flag to row
selection. Row197 used six foreground text scanlines instead of the flag's
minimum seven. Row237 reached horizontal crop edge146, beyond its threshold145.
Both rows were vertically centered; these text-shape tests did not qualify safe
row selection. The local collector now uses the observed safe vertical band
580–650 while retaining its post-capture list-layout guard. No game code changed.

The missing rows were read from new full name-hover and Basic PNGs. Original
observations were not relabeled and capture IDs were not substituted for physical
ordinals. `gap-correction.json` binds the two corrections to their private reader
receipt. This gap mechanism is separate from the earlier4px legacy counter drift
recorded in PR3152; its134-row calibration remains unchanged.

## Bounded source comparison

`compare_sources.py` reproduces name-candidate comparisons against Game source
`5efd67da164ac9dc33c8c066434b228cebb86c2b`:69 canonical shards, the manifest and
three selected profile inputs (73 receipts). It compares Weight and Tradeable
In Market; other non-narrative fields remain facts without speculative conversion.
Weights use the pinned inspection formatter's hundredths-of-ounce convention.

There are92 single-name,13 unmatched-name and eight multiple-name observations.
Even a unique match remains `UNQUALIFIED_NAME_CORRESPONDENCE`. Candidate field
comparisons yield202 equal,20 unknown/unqualified and eight different values.
The differences include cross-comparisons between the same-name mouth/bell
variants; they are not eight proven Item defects. All name candidates are retained
without selecting an ID from name, graphic, weight or ordinal. Literal source
states, selected membership and admission flags remain separate from runtime
admission or executable behavior. No content fixes are inferred from this packet.

## Validation and continuation

```text
python docs/agents/evidence/items-creature-products-next-20261113/compare_sources.py --check
python docs/agents/evidence/items-creature-products-next-20261113/validate.py
python tools/agents/validate_governance.py
python -m unittest discover -s tools/agents/tests
git diff --check
```

The source generator seals the compact data in `packet-integrity.json`; the
validator checks corrected mappings, contiguous physical coverage inside the
qualified cutoff, separate same-name rows, private text receipts, source
reproduction and privacy boundaries. Private PNG byte checks are an Ubuntu
custody result, not a Cloud screenshot readback. No gameplay, graphical parity,
production, CI-green, independent-review-clean or merge claim is made.

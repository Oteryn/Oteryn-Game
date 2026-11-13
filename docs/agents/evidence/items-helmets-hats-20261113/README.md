# Helmets and Hats: ordered Basic evidence1–128

This packet retains128 distinct physical UI observations from Tibia Test Server15.33.dd0955. All saved Basic pages are covered:268 original BasicPNG and652 card-proof references. There are927 observed Basic properties:899 exported non-narrative literals plus28 private Description receipts. Ten originally supplied description display layouts have separate hash/length receipts; layouts not supplied by the reader are not reconstructed.

Source pin: `5efd67da164ac9dc33c8c066434b228cebb86c2b`. Allocation: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6846579722 .

## Custody and correction

The immutable private union SHA256 is `4e0ebece746ce49d91908a13af83087306f179073d8c11fb12b93bc2698eded2`. Reader provenance distinguishes reused1–50 from newly manually read51–128, with native re-read10. Original PNGs, private text, assets and collector code remain private.

The original1–20 transcription recorded brass helmet Armor7. Independent native re-reads prove3; `reader-correction-receipt.json` binds correction receipt `f10548cee67cdd8d42f6828b1743de97970f62b0344af230f94d924c375e9487`. Old packets are unchanged. This is a transcription correction, not a gameplay defect.

`category-end-receipt.json` and `tail-journal-receipt.json` retain actual final capture128, one capture attempt, stable end checks and restored-scope receipts. The original collector's historical pending-reader string is preserved as history; the final reader union separately qualifies all128 cards.

## Source comparisons

`compare_sources.py` extends the Food PR3156 contract at `26b958e6485c778353211b47b62d4d98a55e0cd7`; it reads exact Git blobs, validates manifest SHA256, and preserves canonical versus6090 selected profiles. All candidate names are retained, including ambiguous names. A unique name, icon or physical ordinal never establishes client/server identity.

Qualified routes cover Weight, marketable, Armor, Capacity, Imbuement Slots, Protection, Required Level, Vocations, Body Position, raw skill modifiers/Mantra/reflection and selected forge Classification/required magic level. A route with UNKNOWN/NOT_APPLICABLE/CONFLICT/absent ancestors remains unqualified; zero and false remain literal known values. Classification is forge authoring, not old15.30 market metadata.

The comparison has115 one-name candidates,7 multiple-name candidates and6 without a name candidate. Its739 equal/4 different/123 unqualified canonical field comparisons count candidate-field pairs, not distinct item implementations. All four canonical differences belong to ambiguous name candidates: one crown Weight and three falcon circlet fields. None is promoted to a game defect.

Raw source-semantic Skill Boost includes elemental magic-level components. It is not a retail absence assertion or an emulation of renderer formatting. In particular pinned inspection.rs shifts Speed right by1. An extra component in a source aggregate is a targeted reread queue, not proof of absent retail ability.

## Current-client appearance receipt

`client-appearance-receipt.json` is an explicit-key whitelist projection of private receipt `6add45e583f17fd6e6a1f75a32a882bf6791fbd291e585e84d7eefca8913d6ea`, using current DAT `aa086346daa9234d721e4170848095ad83f3ee9d28ea9e62c1bd06216016174e`. It retains126 unique client-namespace candidates and two sets: UI29 depth galea `{13995,14258}` and UI55 helmet of the deep `{5460,11585}`. Both mining-helmet observations remain separate: UI79→139 and UI80→875. Every `server_id` is null; appearance, Cyclopedia and market trade/show IDs remain distinct namespaces.

The author independently verified135 private input-file hashes and8 decoder-file hashes, matched all8 decoder Git blobs to their exact83be source receipts, reconciled128 named-candidate sets to the private census, verified56 packed manifest/decoded BMP hashes, and recomputed all299 known sprite-slot metrics representing272 unique sprite IDs. The appended receipt has a strict nested-key validator and integrity receipt. All observed/source descriptions are hash/length seals only; no original sprite, PNG, DAT, copied decoder or collector code is included.

The metric checks exact RGB only at fully opaque pixels of a static captured icon, across all known groups/slots in this parsed packet. It excludes transparency/semitransparent compositing and temporal playback, animation timing, unknown-field semantics, parser completeness and animation1:1 certification. A unique retained client candidate is not a qualified native/serverID binding or runtime admission.

## Remaining scope

`uncompared-field-backlog.json` lists retained Augments that have no qualified comparison route here. Field presence/non-emission is separate from display observation; omission never means zero, false, UNKNOWN or N/A. Attack/Defence and other fields not observed in these128 cards have no parity claim. Charges/type maxima are never compared with remaining instance charges; expiry is likewise not inferred.

NPC/trade/currency/residence, market-value currentness, animation/compositing, full appearance binding, native/serverID, runtime admission and whole static semantic parity remain PARTIAL. The appended client-namespace candidates do not narrow those boundaries.

## Reproduction

Run from the repository:

```sh
python docs/agents/evidence/items-helmets-hats-20261113/validate.py
python docs/agents/evidence/items-helmets-hats-20261113/compare_sources.py --check
python tools/agents/validate_governance.py
python -m unittest discover -s tools/agents/tests
git diff --check
```

No content, runtime, protocol, pin, Cargo or workflow changes; no game compilation required. Formal independent review and integration belong to the lead.

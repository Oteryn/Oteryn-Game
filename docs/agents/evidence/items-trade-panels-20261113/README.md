# Initial NPC trade panels: Decoration and Food

This evidence retains the initially visible Sell To and Buy From panels for all
462 Decoration and 169 Food physical UI entries in the observed
Tibia Test Server 15.33.dd0955 client. It records 1,262 panels and 637 visible
quotes. Decoration has 669 observed empty panels and 255 nonempty panels;
Food has 245 observed empty panels and 93 nonempty panels.

**Only initial-view coverage is complete. Full NPC lists remain PARTIAL.**
The backlog contains 43 Decoration and 34 Food panel retakes. Decoration
Sell To retakes are 310 and 382; the other Decoration retakes are Buy From.
Food retains panel-specific pairs, including four entries needing both panels.
An observed empty initial panel does not prove global NPC absence.

## Identity and literal boundaries

Each category/physical ordinal/capture key remains separate, including repeated
names and artwork. None binds an appearance, Cyclopedia or server ID.
Quote order, spelling, punctuation, price and visible residence text are retained
literally. Missing location keys remain missing; clipped locations retain their
source UNKNOWN state/reason. No missing text is reconstructed.

Currency is derived only from an explicit displayed gp suffix or x currency
suffix. Food ordinals 8, 27, 46 and 49 contain six bare-number prices; their
currency stays UNKNOWN. No implicit gp, currency conversion, executable trade
offer or runtime admission is asserted.

The explicit immutable-input overlays correct Decoration310 first Sell To NPC
to Yasir and Food21 NPC to Ramina. The original private packets remain unchanged.
Their hashes and the two corrected private unions are recorded in
source-receipts.json. The client reference uses the GLOBAL_BLACKBOX observation
route; these literal observations do not change game content or runtime behavior.

## Custody and validation

Only whitelisted compact facts, relative private proof keys, SHA256 receipts and
retake reasons are included. Original PNGs, raw private JSON, descriptions,
narrative/layout text, collector metadata and configurations remain on Ubuntu.
There are 631 distinct Basic fullshot proof references. Hash checks authenticate
custody, not transcription accuracy; the source panels were manually read from
native pixels, without OCR inference. This PR does not reproduce that visual read.

Run from this directory:

- python3 validate.py
- python3 -m unittest discover -s . -p 'test_validate.py'

For an authorized private custody check on the source laptop, run
python3 validate.py --private-root <private outputs directory>.
This checks the two pinned private unions, both correction sidecars, all631 PNG
hashes and exact whitelist projection without printing private records.
A missing private root is explicitly reported as not checked; repository-only
validation does not claim fresh access to private screenshots.

Retakes require a recognized static Cyclopedia card, overlapping page proofs,
literal reading of newly visible names/price/location and an observed final
scroll endpoint. Keep unobserved additional entries and clipped text UNKNOWN
until those proofs exist. No NPC conversation or transaction is needed.

Companion Basic evidence: draft PR3155 (Decoration) and draft PR3156 (Food).
These drafts remain separate; this evidence does not mutate their frozen heads.

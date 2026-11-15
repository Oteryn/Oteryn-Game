# Global parity evidence tables

One table per gameplay area, comparing the live Global game with our content and code. Evidence only: a table grants no authority and changes no behaviour. Fixes land in separate commits that cite the row.

## Format

Each file has a header (area, date, source legend, confidence legend) and rows:

`| item | dimension | Global value | source (TW rev / measurement) | measured? | ours | ours file | status |`

Source legend:

- TW: TibiaWiki (tibia.fandom.com MediaWiki API), with the page revision id.
- Measurement: an owner-supplied observation on Global, with date and character setup.
- OTS donor (Crystal, Canary): hypothesis only, never Global evidence.

Confidence (the `measured?` column):

- MEASURED: observed on Global.
- WIKI: stated by TW at the given revision.
- REFERENCE: only an OTS donor or an inference.

Status: MATCH, MISMATCH, FIXED (was a mismatch, corrected in the cited commit), UNKNOWN (no Global source found), GAP (we lack the mechanic or its consumer).

When the Crystal re-audit (`docs/architecture/reviews/OTERYN_GAME_CRYSTAL_REAUDIT_2026-10-28.md`) marks a mechanic DONE with evidence, a row cites the audit row id instead of re-deriving it.

## Areas

- `healing.md`: healing spells, potions, monk mechanics, leech, regeneration.

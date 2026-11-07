# Canary `items.xml` (pinned, hypothesis only)

`items.xml` and `LICENSE` are byte-identical copies of `data/items/items.xml` and `LICENSE` from
[opentibiabr/canary](https://github.com/opentibiabr/canary) at
`04b83b512114bfd888000d6e1433ed8ecaec7c5b`, the revision the Game decisions cite (for example
TIMED-ITEM-0 §2, EQUIP-0 §2). `manifest.json` pins both by SHA-256. Canary is GPL-2.0; the file is
redistributed unmodified with its licence.

- **Evidence class:** `OTS_HYPOTHESIS_ONLY`. A content lane uses it only as the fallback the
  owning decision names (TibiaWiki first), never as value evidence where a decision requires an
  official or TibiaWiki source, and never as an Oteryn identity.
- **First consumer:** TIMED-CONTENT-1 (TIMED-ITEM-0 brief): the `transformEquipTo`,
  `transformDeEquipTo`, `decayTo`, `duration`, `charges`, `showcharges`, `showduration` and
  `stopduration` attributes of rings, amulets, soft boots, torches and lamps.
- **Whitespace:** the copy keeps the source's trailing whitespace; `.gitattributes` here exempts
  these two files from Git's whitespace check so the bytes match the pinned hash.

Verify with `sha256sum items.xml LICENSE` against `manifest.json`.

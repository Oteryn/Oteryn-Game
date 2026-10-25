# Canary `imbuements.xml` (pinned, hypothesis only)

`imbuements.xml` and `LICENSE` are byte-identical copies of `data/XML/imbuements.xml` and
`LICENSE` from [opentibiabr/canary](https://github.com/opentibiabr/canary) at
`47dfd51f45280a59a1d3e50ba7edd573d7234446` (the file is unchanged since `04b83b51`, the revision
`crates/gameplay/items/src/imbuement.rs` cites). `manifest.json` pins both by SHA-256. Canary is
GPL-2.0; the file is redistributed unmodified with its licence.

- **Evidence class:** `OTS_HYPOTHESIS_ONLY`, never an Oteryn identity.
- **Consumer:** `tools/content-migration/imbuement_catalogue.py` generates
  `crates/gameplay/items/src/imbuement_catalogue.rs` (family, tier, premium, price and the
  cumulative material list of every single-family recipe), cross-checked against
  `imports/crystalserver/facts/imbuement-materials.json`. `--check` is part of
  `tools/content-migration/regenerate_content.py`.
- **Whitespace:** `.gitattributes` exempts these files from Git's whitespace check so the bytes
  match the pinned hash.

Verify with `sha256sum imbuements.xml LICENSE` against `manifest.json`.

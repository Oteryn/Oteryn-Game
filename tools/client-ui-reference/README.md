# Verified UI reference facts

A Python standard-library tool that extracts bounded facts from an explicitly supplied QML/JS source tree and SHA256 inventory. It is input for reusable UI components, not a QML runtime, automatic UI port or proof that a screen behaves correctly.

No original QML, font, image, personal preference or generated reference output belongs in this tool directory. Keep reference inputs and generated outputs in an external private directory.

## Run

Python 3.10 or later; no packages to install. All input/output locations must be supplied:

```sh
python extract.py \
  --sources /private/reference/qml-components \
  --inventory /private/reference/inventory.json \
  --style Style.qml \
  --output /private/reference/foundation.json
```

The paths above are placeholders. `--sources` must be the component root that the inventory paths describe. `--style` is a POSIX path relative to that root, for example the name of its style singleton. The output's parent directory must already exist. Successful extraction prints counts; a missing or invalid input exits 2 without publishing a result. Validate before consuming the output.

The inventory is a JSON object:

```json
{
  "source_version": "optional reference revision",
  "files": [
    {"path": "Style.qml", "sha256": "64 hexadecimal characters"},
    {"path": "nested/Control.qml", "sha256": "64 hexadecimal characters"}
  ]
}
```

Each digest is SHA256 of the source bytes **after CRLF→LF normalization only**. The output retains the original raw SHA, normalized SHA, expected SHA and byte counts. It does not rewrite inputs. Every discovered `.qml`/`.js` source needs an inventory entry and matching digest; missing hashes, unknown sources, hash mismatches and conflicting duplicate basenames fail. Identical duplicate basenames retain their distinct relative paths. Source symlinks escaping the supplied root and inventory paths containing traversal or absolute paths are rejected.

Provenance uses source-relative paths and one-based lines. Output omits timestamps and host paths and is deterministic for the same bytes and arguments. CRLF and LF inputs intentionally have different raw hashes, while their normalized content hashes agree.

## Output and boundaries

- `style_tokens`: single-line typed declarations in the selected style root object, raw expressions, resolved literal/arithmetic values where supported, explicit unresolved status otherwise.
- `font_definitions`: single-line font blocks with per-attribute expressions and source lines.
- `rule_evidence`: non-comment lines with `:` retained as evidence, including alignment, padding, state, geometry and callback syntax. These are not interpreted behavior contracts.
- `asset_references`: quoted `/images/...` references or fragments. A suffix such as `.png` classifies only the quoted value; dynamic URL construction and asset existence still need review.
- `sources`: raw and normalized hash provenance for every consumed file.

Evaluation permits finite scalar literals, earlier resolved local names and bounded `+`, `-`, `*`, `/` arithmetic. It never evaluates QML/JS, attribute access, imports or calls. Expressions outside that small subset stay unresolved. Fractional results assigned to `property int` stay unresolved as well; Qt integer coercion/rounding is not guessed. Nested declarations are retained as rule evidence but are not global style tokens. Dynamic imports, controller data, Qt layout negotiation and event behavior are not inferred.

There are deliberately **no curated widget contracts** in the portable output. Updating both the source and inventory yields new source facts, rather than carrying old hardcoded semantics into a new revision. A renderer must derive its implementation from those facts and separately validate behavior and appearance. In particular, a control's left/top padding adjustment does not imply left/top text alignment; the independent alignment lines are preserved.

## Verification

Self-contained tests create synthetic source and inventory fixtures in temporary directories. They require no private source tree:

```sh
python -m unittest discover -s . -p 'test_extract.py' -v
```

Optional check of actual external sources, run separately:

```sh
python check_reference.py \
  --sources /private/reference/qml-components \
  --inventory /private/reference/inventory.json \
  --style Style.qml
```

This verifies hashes and evidence line bounds and prints counts only. It does not replace visual or interaction tests. Unavailable inputs fail explicitly; no successful reference check is fabricated or silently skipped.

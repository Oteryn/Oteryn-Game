# Crystal summer-update source evidence

Primary source: `zimbadev/crystalserver`, branch `summer-update`, immutable
revision `00ce02a57ca5a12e48f32a3476e37471167e4c3f` (OtsHypothesisOnly).

- `source-tree.json` is the complete Git tree used to bind the primary XML
  to its original Git blob; `truncated` must be false.
- `raw/data-global/world/world-monster.xml` retains the complete primary
  monster source: 54,713 groups / 88,261 points.
- `raw/LICENSE` retains the original source license and is Git-blob verified.
- `map-content-linking/` contains proposals and their closed input/output
  manifest. Run the producer's `--check` to reproduce them.

These proposals retain current palette identities and provisional donor keys.
They change neither canonical content nor runtime activation. Official client
asset bytes are not added by this packet; its links consume the existing,
owner-confirmed 15.30 inventory already in the repository.

Source: https://github.com/zimbadev/crystalserver/tree/00ce02a57ca5a12e48f32a3476e37471167e4c3f

Producer and boundaries:
[`tools/content-schema/map-content-linking/README.md`](../../../tools/content-schema/map-content-linking/README.md).

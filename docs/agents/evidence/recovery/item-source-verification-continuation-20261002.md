# Item source verification continuation — 2026-10-02

This supplements the published gap audit against `4f690c1e8a8966a4d074dde94d5a884dbb34fb94`. It adds source verification and a limited format proposal measurement. It does not publish new Item gameplay values or claim that all Items are complete.

## Forge provenance: 147 existing profiles confirmed

Ordinary HTTP access to TibiaWiki BR was blocked. The authorized research-only route used an existing Chrome browser through Remote Desktop and CDP, with anonymous public same-origin MediaWiki requests. All 147 exact historical article revisions were retrieved in three batches. Every article contains a literal, single `classificacao` and `max_tier` parameter matching both the retained structured snapshot and the current Forge owner. Every full raw article SHA256 matches its original snapshot `source_digest`.

Hammer of Wrath, page6171/revision443875, explicitly contains class2/maxTier2. Its full raw article digest is `5ff67e576b1716345345bb7eb03ef90037ff7029bb741e43d6edc8802c75d42b`.

The current Infobox Item template can render a maximum from a global class table. That rendering alone remains insufficient evidence for a new per-Item maximum. A public search returning zero `max_tier` hits was an indexing limitation, not proof that the historical literal parameters were absent. The exact-revision read resolves the origin question for the accepted147; it does not qualify the remaining870.

Source: [TibiaWiki BR MediaWiki API](https://www.tibiawiki.com.br/api.php), exact requested URLs, revision IDs and article titles retained in the capture. Method: **Remote Desktop + existing Chrome/CDP**, after ordinary HTTP blocking. Public current continuity and applicability to unreviewed Items remain unestablished. No repository or administrative operation used Remote Desktop.

Independent review: `forge147-independent-raw-parameter-origin-review-20261002.json`, SHA256 `e447c10404da93075247b3557ec5766188d2130f9e727e764190d43edbb426eb`. Source capture SHA256 `cafc941e3dc330583dd99271b9f67ffec60ecc60ea81be3ccb2b33a5f1490802`.

## Physical8: synthetic record measurement only

A retained external Python prototype was measured locally without changing the repository. Its existing v4 codec rejects Physical element7. Its proposed v5 codec round-trips element7 and rejects0/8/255. Maximal synthetic single-record sizes remain3555 server bytes and3433 client bytes. These observations do not establish a production Rust codec, full artifact maxima/max+1 limits, or accepted profile.

Eight whole vectors containing16 atoms remain unapplied. The historical protected38157-Item artifact family and current34031-Item materialized universe are distinct; their difference is not a missing-item count. A concrete, separately versioned profile proposal and owning exact-blob acceptance are still needed before schema/codec release.

Measurement: `physical8-external-prototype-measurement-20261002.json`, SHA256 `fc10e2f99a60d76e4b81b8aa35d41e0ff704e2dff640ba71ae4d4a491e3b79bc`. Method: local synthetic Python codec, no external web source or Remote Desktop.

## Active completion work

Weapon103 is undergoing final source review and predecessor-test fixture repairs before generation and Rust validation. DefaultFalse8 has passed independent review of external source packaging; allocation, generation and exact native-delta validation remain pending. Neither batch is counted as applied here. Family235, stack maxima, imbuement restrictions, presentation/audio and the other gap groups retain their explicit source or implementation blockers in the master audit.

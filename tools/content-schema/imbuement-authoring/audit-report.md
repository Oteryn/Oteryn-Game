# Independent full imbuement audit — 2026-10-01

The full re-audit found P2 defects in evidence qualification, acquisition and
quest details, and standalone validators. It found no P1 defect in the reviewed
authoring package. The repairs belong to draft PR
[1438](https://github.com/Oteryn/Oteryn-Game/pull/1438); this report does not grant
runtime activation or protected integration.

## Independently verified

- All 24 families, 72 numerical effect rows and 72 cumulative recipes agree with
  the captured Wiki BR table. Canonical definitions and primary names agree with
  all 72 material bindings and 72 scroll bindings. Nine Yana exchange bundles and
  eight usable shrine identities were also checked.
- All 607 captured native per-item tables agree with their recorded slots,
  normalized types and maximum tiers. No additional proven type/tier error was
  identified among the 101 retained community disagreements.
- All 13 pinned Crystal `imbuements` source files agree with recorded SHA-256,
  Git blob identities and line counts. All 72 XML records were independently
  reparsed. Strike normalization correctly adds the engine baseline to XML
  deltas; recipes and fees match the selected catalogue.
- Access covers 72 shrine routes, 72 completed-scroll application routes and 48
  higher-tier inscription routes. The per-family unlock mappings agree with the
  inspected sources, subject to the corrected Frozen Horror access detail below.

## Repairs from this review round

| Area | Finding and repair |
| --- | --- |
| Eligibility confidence | 76 native-only type/tier profiles were falsely labelled corroborated. Current typed profiles comprise 98 single-source, 430 agreeing multiple-source and 101 disputed records; primary slots do not corroborate allowed types. |
| Eligibility provenance validation | Empty amendment evidence, duplicate amendments and normalized values drifting from raw native descriptors could pass replay. Replay now checks evidence references and raw-to-normalized consistency. |
| Item proposal provenance | Two revision URLs contained a Markdown fragment instead of a clean exact-revision URL. The links and generated proposal provenance are repaired and validated. |
| Acquisition and quest evidence | Etcher purchase lacked the official worthy-character gate; Frozen Horror pointed to the boss reward room instead of the separate shrine area opened after reporting to A Dragon Mother. Missing native quest capture hashes and selected claims are recorded with their actual source roles. |
| Operational coverage | Same-bonus exclusion and the community-described higher-level equipment activation rule were absent from the rule ledger. They are now source-qualified; broader shared-category compatibility, payment/failure handling, native effect composition and transfer preservation retain named evidence gaps. |
| Combat validation | Changing both confidence status and value could promote unresolved rounding or composition to invented Global truth. The validator now requires the exact qualified rule set and preserves unresolved values. |
| Crystal validation and coverage | Counts alone accepted duplicate comparisons and invented strengths. Semantic replay now verifies unique keys and recalculates results. The equipped-scroll source anchor is corrected, and omitted healing critical, leech, inscription and default configuration behavior is recorded as engine evidence. |
| Architecture authority | The owning IMBUE-FORGE-0 document is marked CANDIDATE. Draft wording now describes an architecture proposal rather than claiming acceptance. |

## What remains incomplete

The definition and recipe catalogue is populated. Full Global server parity is
not established. The immutable target is
`global-tibia-observable-2026-07-28-post-server-save`; current public pages cannot
prove continuity of every rule at that timestamp.

The census contains 663 client objects with observed slots, 629 current typed
profiles and 628 typed target candidates. Sailor's Backpack was introduced after
the target. Thirty withdrawn objects and four TEST objects are explicitly
excluded. Per-item allow lists remain community evidence, with zero officially
verified server allow lists. The two missing canonical Items have validated
proposals; they are not registered runtime identities.

Exact timer boundaries, leech rounding and combat composition, Etcher
consumption, equipped-scroll acceptance, the Vibrancy PvP success gate,
transaction behavior and preservation across transfers still require qualified
Global observations. Crystal code supplies hypotheses, not that proof. Some
operational topics have attempted public sources; newly named gaps still need
targeted evidence work.

Canonical quest-state bindings, persistence, effect execution and protocol/UI
integration remain outside this source draft. Repository workflows do not run
the package's semantic tests: generic PR checks are not imbuement parity checks.
Validation uses the explicit offline commands in [README.md](README.md).

## Validation

All 117 offline tests pass. The seven replay/schema commands in README pass,
including both missing Items through the owning Item validator with zero errors
and warnings, and an additional reparse of the pinned Crystal XML. All 58 rule
records retain qualified source references. The catalogue preserves 12 named
Global observation requirements rather than declaring complete server parity.

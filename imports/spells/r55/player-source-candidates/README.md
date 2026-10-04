# R55 — real standard base-Spell projections

This package audits the exact 64 current blocked registration keys in the Wheel (36), Crystal stance (27), and chain (1) lanes. It contains 21 complete target-schema standard Spell/Ability/Effect bundles: 15 Crystal attack counterparts normalized under accepted S21/S5 and six Wheel base spells under S6/S5. The remaining 43 rows have specific unresolved data behavior recorded in `lane-audit.json`. No native alias is emitted.

Every candidate retains the original r28 `source-header.json` byte-for-byte. Current source SHA and selected mechanics-source SHA are separate in `projection-proof.json`. S21 intentionally selects the complete Canary plain-combat counterpart instead of custom Crystal stance branches; the original Crystal program remains in immutable r38 source syntax. This is an explicit accepted canonical normalization, not a claim that the raw Crystal program is identical. S5 uses the existing world level-damage/healing expression normalizer. S6 separates optional Wheel/Beam Mastery augments from these level-unlocked base Spells; no true Wheel unlock gate is removed. Candidates retain provider/augmentation limits in their receipts.

`import-summary.json` indexes only the 21 complete bundles. `lane-audit.json` covers all 64 keys, including source SHA/revision and false runtime/native flags. A BLOCKED row has no complete candidate bundle here. No source-correct native descriptor was proved for the other rows; the strict67 identity guard is an independent runtime limit, not a reason to declare otherwise complete native data structurally invalid.

The schema validator and a structural check of the current Rust reader's required and denied fields pass for every emitted bundle. This packet does not execute these exact bundles in Rust, activate content, allocate native identities, change canonical selection, or qualify source/runtime provider equivalence. Visual/sound asset availability and optional augmentation/state owners remain unqualified.

Regenerate from the already cached inputs:

```sh
/workspace/spell-tools/bin/python tools/content-schema/spell-authoring/project_state_wheel_candidates.py --repo .
/workspace/spell-tools/bin/python -m unittest discover -s tools/content-schema/spell-authoring -p test_project_state_wheel_candidates.py -v
```

The seven tests check exact population, actual target schema/reader shape, byte-exact raw headers, S21 counterpart identity rebinding, retained true Wheel stages, refusal of mismatched cached source identity, and refusal to silently erase the unsupported `none` vocation domain.

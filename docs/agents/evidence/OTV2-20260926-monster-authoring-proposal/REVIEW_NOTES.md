# Monster authoring proposal — review notes

## Scope and evidence binding

- Date: 2026-09-26.
- Reviewer: ChatGPT; advisory review retained at the owner's explicit request.
- Repository: `Oteryn/Oteryn-Game`.
- Branch: `codex/monster-authoring-schema-20260926`.
- Reviewed commit: `a355d7d35f42367453834931b93ff3df581a2e07`.
- Reviewed directory: `docs/agents/evidence/OTV2-20260926-monster-authoring-proposal/`.
- Primary schema blob: `cb47bc80bf30379d12b82fb0fa50867fd1bd8e0f` (`monster.schema.json`).
- Comparison baseline: `de7c5499a2e6ad4ec533ea895754bdd82bda292e`.

This is a record of the preceding review, not an accepted architecture decision, an independent-review approval, a schema repair or authorization to integrate. The owner requested saving this note in the existing proposal directory. Only this note is in the scope of that write. Existing schemas, templates and task records are left unchanged.

The findings concern the reviewed commit above, not an automatically qualified successor branch head. Evidence classifications below use `PROVEN`, `DERIVED` and `UNKNOWN`; recommendations are explicitly identified. Prior task scope and `README.md` distinguish evidence retention from product adoption and runtime qualification.

## Overall assessment

**RECOMMENDATION — retain this model as the authoring proposal; do not redesign it or expand it speculatively. Do not yet treat it as the approved contract for mass import.**

The split into Creature, Behavior, Presentation and Loot is coherent. The next bounded step should close numerical validation, semantic rules, source provenance integration and reuse of existing item contracts. Saving this review is not that implementation step.

## What is already improved

**PROVEN by inspection of the pinned files:**

- The package contains three formal schemas, two empty templates, `README.md` and `TASK_RECORD.md`. It explicitly says that null-filled templates are not valid ready monster data and that structural validation does not qualify runtime behavior.
- Probability, distance and duration names use `_percent`, `_tiles` and `_ms`. `/$defs/percent` specifies 0–100 percent, a step of 0.0001 percentage point and conversion to ppm by multiplication by 10000. Rational percentage values explicitly mean percentage points rather than fractions of one.
- `/$defs/summoning` conditionally requires or forbids the corresponding mana costs and the familiar object. `/$defs/bestiary` requires exactly three positive kill thresholds.
- Direct dependency definitions are present for Ability, Effect, Formula, Document, a limited Item/corpse representation and nested Loot. Typed references constrain the referenced family.
- `apps/game-server/src/content/project/v2.rs` at the comparison baseline already has the related family vocabulary and `{family, key, revision}` references. This supports vocabulary alignment, not full compatibility of the proposed payloads.

Sources: [`README.md`](README.md), [`TASK_RECORD.md`](TASK_RECORD.md), [`monster.schema.json`](monster.schema.json), [`monster-dependencies.schema.json`](monster-dependencies.schema.json), and [the baseline WorldProject/v2 implementation](https://github.com/Oteryn/Oteryn-Game/blob/de7c5499a2e6ad4ec533ea895754bdd82bda292e/apps/game-server/src/content/project/v2.rs). Relative links are navigation conveniences; the reviewed versions are bound to the commit above.

## R1 — qualify the exact decimal validation route

**PROVEN, narrowly scoped reproduction.** `monster.schema.json`, `/$defs/percent`, uses `type: number` with `multipleOf: 0.0001` (lines 280–286 at the reviewed commit).

In Python 3.13.5 with `jsonschema` 4.26.0, ordinary `json.loads` parsing into binary floats rejects these mathematically valid inputs:

```text
0.0003
0.29
1.4
4.93
```

The isolated 17-case check produced **4 float-path mismatches and 0 Decimal-path mismatches** when both schema and input decimals were parsed with `Decimal`. It was rerun with the same result before saving this note. This is a reproduced validator/parser interaction, not proof that the JSON Schema specification is wrong or that every validator rejects these values. The repository's intended production validation route was not qualified by this check.

**RECOMMENDATION:** keep human-readable authoring percentages. Specify the supported parser/validator route and exact conversion to integer ppm, with boundary and precision regression tests. Do not require a wholesale switch back to authoring ppm merely because this float-based route fails. The bounded Decimal result is not proof of correctness for every number accepted by the whole schema.

### Self-contained isolated reproduction

Run in an environment with the recorded `jsonschema` version. This copies only the validating keywords of the pinned `/$defs/percent`; it does not load or validate the complete schema or any monster bundle.

```python
import json
import sys
from decimal import Decimal
from importlib.metadata import version
from jsonschema import Draft202012Validator

schema_text = '''{
  "type": "number",
  "minimum": 0,
  "maximum": 100,
  "multipleOf": 0.0001
}'''
cases = [
    ("0", True), ("0.0001", True), ("0.0003", True),
    ("0.07", True), ("0.19", True), ("0.29", True),
    ("0.62", True), ("0.97", True), ("1.4", True),
    ("2.5", True), ("4.93", True), ("30.07", True),
    ("82", True), ("100", True), ("-0.0001", False),
    ("100.0001", False), ("0.00001", False),
]
print("Python:", sys.version.split()[0], "jsonschema:", version("jsonschema"))
for label, parse_options in [("float", {}), ("Decimal", {"parse_float": Decimal})]:
    validator = Draft202012Validator(json.loads(schema_text, **parse_options))
    mismatches = [
        literal for literal, expected in cases
        if validator.is_valid(json.loads(literal, **parse_options)) != expected
    ]
    print(label, "cases:", len(cases), "mismatches:", mismatches)
```

Observed output for the recorded versions:

```text
Python: 3.13.5 jsonschema: 4.26.0
float cases: 17 mismatches: ['0.0003', '0.29', '1.4', '4.93']
Decimal cases: 17 mismatches: []
```

## R2 — structural validity must not be mistaken for admission

**PROVEN by inspection; implications DERIVED.** The following constraints are not enforced by the corresponding structural definitions:

| Location in `monster.schema.json` | What is enforced | Semantic work still needed |
|---|---|---|
| `/$defs/stats` | Both health values are positive integers. | Enforce `initial_health <= max_health` for the ordinary creation contract. |
| `/$defs/lootEntry` | Both counts are positive integers. | Enforce `min_count <= max_count`. |
| `/$defs/bestiary` | Exactly three positive thresholds. | Define stage meanings and validate their ordering. |
| `/$defs/creature/properties/resistances` | Each entry has a damage type and rational reduction. | Define and validate uniqueness or explicit combination semantics per damage type. |

`lootEntry.skip_later_same_item_after_success` is a boolean without a definition of item equality, success or skip scope. The allowed algorithm is named `IndependentBernoulli`; conditional suppression still needs its execution semantics specified. Do not sort entries if order affects the result. `contents_loot` is a typed reference, not a proof that the target exists, the item can contain loot or recursive table expansion terminates.

**RECOMMENDATION:** use the existing semantic validation/linking route for cross-field rules, reference resolution and container/cycle checks. Do not force all checks into JSON Schema. Specify the missing loot behavior, including count selection and conditional/event loot integration. Require the appropriate structural and semantic results before publication.

These are adoption gaps, not a reason to reject the current bounded evidence-retention task. `README.md` already acknowledges the structural/runtime distinction. This review did not execute a full semantic validator or demonstrate a runtime exploit.

## R3 — clarify the provenance scope and readiness decision

**PROVEN by inspection.** `monster-import-readiness.schema.json` requires each source to have a repository and a 40-character hexadecimal revision. Entries require `source_file` and `source_line`. There is no separately typed wiki-page, page-revision or screenshot capture source in this schema.

**DERIVED:** this representation directly suits Git donor files. A shared wiki/library import path would need either an explicit archival bridge into Git or a representation of the original non-Git source. The current record should not silently substitute a page revision for a commit SHA or lose the original source identity.

The schema deliberately permits `unresolved_dependency`, `unresolved_semantics` and `partial_text` statuses. That is reasonable for an import disposition record. Passing its structural validation therefore does not mean that an import is ready.

**RECOMMENDATION:** reuse Oteryn's existing provenance model where applicable. If this manifest is deliberately Git-only, label that boundary and document the bridge for wiki captures; otherwise add only the source kinds required by the accepted scope. Define the admission decision separately from manifest syntax. Do not invent source revisions or turn partial lore into complete source text.

## R4 — avoid a second source of truth for items

**PROVEN by inspection.** The proposal defines `/$defs/item` and inline `/$defs/lootEntry/properties/instance_attributes`. The baseline repository also has `ProjectV2ItemAuthoring` in `apps/game-server/src/content/project/v2.rs`.

**DERIVED risk, not a proven incompatibility:** independently maintaining the limited monster dependency item view and the established item authoring contract could cause drift in supported attributes, units or override semantics.

**RECOMMENDATION:** make the relationship explicit: reuse the owning contract, or keep a deliberately bounded authoring projection with explicit mapping and compatibility tests. Validate which instance overrides each item type permits, and whether an `interaction` override replaces or supplements the item's behavior. Do not create a parallel item schema as runtime authority.

## Verification limits

- **PROVEN:** the reviewed branch delta against the baseline was seven added files confined to this proposal directory; no product implementation was included in that delta.
- **PROVEN:** the isolated percentage reproduction above, including its recorded versions and 17-case result.
- **UNKNOWN / not independently reproduced:** the 68 earlier focused checks and 242 inventoried source paths mentioned by `README.md`. The saved seven-file package does not retain that test suite or source inventory; the README itself calls them earlier authoring evidence, not runtime qualification.
- **NOT PERFORMED:** full-schema validation of a complete real monster bundle, repository compiler/linker tests, full reference/asset qualification, runtime/E2E tests, CI qualification or merge-readiness assessment.
- Donor correctness and full Tibia/wiki coverage were not re-researched for this note. No paid or independent architecture approval is claimed.

## Recommended next bounded task

Retain the current family split. Close R1–R4 against the accepted contracts without speculative fields or a new framework. Retain a runnable focused test suite, at least one valid complete authoring example and negative cases for the relevant constraints; keep authoring-only results distinct from executable/runtime qualification. Report exact paths, source and candidate SHAs, commands, actual results and remaining unknowns.

**Disposition:** positive assessment as a retained authoring proposal; changes and further evidence are still required before adopting it as the approved mass-import contract. This note neither changes the schemas nor authorizes integration to `main`.

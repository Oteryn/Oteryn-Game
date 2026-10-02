# Imbuement server data import

The static server data lives in `rulesets/items/imbuements/`. Rebuild or verify it:

```sh
python3 tools/content-schema/imbuement-authoring/imbuement_content.py content
python3 tools/content-schema/imbuement-authoring/imbuement_content.py content --check
```

The source is the reviewed authoring snapshot from PR #1438, head
`8ae14d820500bd94a9da06c2323e3ef27d1b4018`, captured for 2026-10-01.
The import checks current canonical Item identities and the captured client evidence;
it does not claim a new internet observation on the import date.

- `catalogue.json`: all 24 families / 72 tiers, effects, cumulative material recipes,
  scroll identities, fees and duration, with source-qualified rules.
- `equipment.json`: 627 bound type/tier profiles plus 36 held observations.
  The held observations include two unregistered Items, four missing eligibility
  records and 30 retired source Items. Nothing is silently discarded.
- `access.json`: shrine, scroll, NPC acquisition and source-defined quest predicates.
- `owner-policy.json`: all ten approved operational policies. These take precedence
  over explicitly unknown public behavior or conflicting OTS hypotheses; they are
  not newly certified Global observations.
- `index.json`: populated ruleset marker, counts, exact source and payload hashes.

Supporting catalogue filenames inside the payloads resolve against the
`supporting_catalogues_base` recorded in the index. The reviewed supporting packets
and schema ship with the import. Source evidence is distinct from operational policy.
The catalogue retains its authoring `DRAFT_NOT_RUNTIME_READY` admission status even
though the static directory is populated.

The content manifest registers **Imbuements as a ruleset**, separately from compiled
ContentFamily definitions. Its lock pins every emitted ruleset file. Regenerating the
legacy-derived content tree reimports the ruleset and reproduces its registration.

Runtime activation, effects, timers, inventory transactions, persistence and UI are
separate work. No Rust DefinitionFamily or Item codec is changed. Basic tier remains
in these data even though the existing Item allowance codec lacks tier One. The two
unregistered equipment Items remain proposals; this import mints no runtime Item
identities. Quest predicates retain source definitions and remain held until their
accepted runtime state bindings exist. Architecture reconciliation recorded in the
catalogue is preserved, including the fee/source conflicts.

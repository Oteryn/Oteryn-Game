# W11 / W15b / W16 / W17 source recovery

Research date: 2026-11-23. Inspected implementation base:
`staging@4f13511ca9ec3fb152f11299bb5cf7c71de16c41`; protected-main cross-check:
`5a1e783826af7421c1c18bae99196178e58089e5`.

The owner requested a working PR to research four reported data gaps and help
complete their implementation. This packet reconciles those reports with the
actual code, retains new source evidence and specifies the remaining production
work. It does not qualify a playable encounter or change a runtime admission.

## Findings

| Row | Verified result | Remaining work |
| --- | --- | --- |
| W11, second Claustrophobic Inferno raid | Crystal supplies seven Phantom positions and the unresolved token `Pos`. The original change and inspected descendants do not recover an eighth coordinate. | Establish the intended cardinality and, if applicable, observe the additional spawn before movement; reconcile it with the admitted map. Preserve `unresolved_phantom: Some(8)`. |
| W15b, Goblin Temple | Existing authoring and native validation already select one vial, item 2874, milk subtype 9, for UID 6119. Literal donor subtype 1 is water. | No second milk override is needed. Fresh Python verification below confirms the authored recipe; this research run does not claim a new real-PostgreSQL journey result. |
| W16, Mimar Haffar | Sources recover the two-cannon shield cycle and distinguish the first fight from the later Phosphorus rematch. Official fixes constrain shield release, fresh charge state and percentage agony. | Bind exact admission/map facts and shared owners; resolve the 30/45-second vulnerability conflict and missing actor schedules before activation. |
| W17, Tibiadrome | Five named roles, group counts, timers and nine modifiers are already represented in repository rules. The working damage curve and several details are explicit Oteryn choices. | Qualify five native creature profiles and the real room, then replace the existing test-only Svargrond opponent substitution with the accepted Drome pool and draw. |

Details and field-level provenance:

- [W11 Phantom investigation and W15b milk reconciliation](w11-w15b.md).
- [W16 entry, cycle, hazards, rematch and acceptance packet](w16.md).
- [W17 rules, native-profile gaps and capture checklist](w17.md).

The new W17 asset comparison confirms that look types 1422–1426 exist in
retained 15.30 observations and have identical raw appearance records in the
inspected 15.33 snapshot. Their proposed creature-name mappings remain
community candidates; matching records do not establish combat parameters
or the identity of the referenced sprite images.

## Implementation order

1. **Preserve the existing milk recipe.** Use its source discriminator
   `WIKI_MILK_OVERRIDES_DONOR_WATER`; do not reinterpret the Lua tuple's
   second value as an item quantity for a fluid container.
2. **Complete Mimar's source packet before wiring admission.** Use the
   shared boss activity and phase owners. Cannon charges are encounter-local
   state; collecting twenty crystals is not an external entry prerequisite.
   Keep the first fight and the later `mimar_haffar_final` rematch distinct.
3. **Complete Drome's native profiles and map.** Existing modifier rules can
   be reused, but their mechanics fixtures cannot authorize production.
   Native admission, actual opponent identities and the accepted random draw
   must be qualified together through the real command path.
4. **Resolve only W11's missing source fact.** A map-valid point, symmetry,
   a Canary generic spawn or Lua's handling of an absent token cannot prove
   the missing Phantom position. Do not rewrite the seven known coordinates.

These are recommendations for the existing owners, not allocations of their
shared implementation paths. This branch owns only its evidence directory
and task record; it does not duplicate the active quest lanes or edit their
status ledger.

## Validation and limits

| Check | Result on the inspected base plus this documentation |
| --- | --- |
| `python3 -m unittest discover -s tools/content-schema/reward-claim-authoring -p test_container_native.py -v` | PASS: 23 tests, including Goblin bag contents, milk/wine/source preservation and refusal cases. The first run lacked `imports/tibiawiki/facts/items-stats.json` in the sparse checkout; after retrieving the unchanged fixture, the complete suite passed. |
| `python3 tools/agents/validate_governance.py` | PASS: 22 required documents and 9 project lanes. Missing client paths in an early sparse read were retrieved unchanged before this result. |
| `python3 -m unittest discover -s tools/agents/tests` | PASS: 135 tests. |
| New PostgreSQL / end-to-end gameplay execution | Not performed in this research run. Existing test source is identified, not relabelled as a fresh passing execution. |
| Runtime / generated content / pins | Unchanged. No test grant, placeholder profile, guessed coordinate or conflicting timer is promoted to production. |

Exact candidate CI and review state belong to the live PR. A documentation
gate cannot prove any of these unfinished mechanics playable.

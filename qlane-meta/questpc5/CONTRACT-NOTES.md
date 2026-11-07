# Active L5 contract decisions

- Keep investigations within ten minutes. Use pinned donor facts first; where a fact is absent, record the conservative native rule here and continue with a PostgreSQL test.
- Static startup IDs are separate from World occurrence/change state. Do not modify worldchg/wvar/wcontrib/wevents mechanics.
- Restore explicit startup selectors during World compilation, preserving source B3 bytes. Crystal `data-global/startup/tables/corpse.lua:86-94` explicitly says the OTBM stopped storing IDs. The served map contains the exact source appearances and positions, with missing attributes. This is missing startup materialization, not a new quest rule.
- Pinned source: crystalserver `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`; skeleton UID20003, appearance4285 at32211,31273,10 (`corpse.lua:87-89`); forcefields3059/3060/3061, appearance1949 at32157,31126,floors9/10/11 (`teleport.lua:387-399`). Source blob hashes and exact lines are generated into `content/world/quest-startup-bindings.json` from the retained corpus.
- Reuse QENGINE's explicit startup-table projection, with compiler-owned output instead of B3 rewriting. Reject ambiguous objects, houses, conflicting attributes and unbound provenance. Preserve every unrelated attribute and use the same projection in compilation and equivalence checks. Compiled sector bytes bind the selectors into the existing World digest; regenerate pins with repository tooling.
- No behavior assumption is needed for these source objects. Missing private/source bytes will not become a stopping condition; the owner authorizes documented conservative rules for later entries.

## Literal written source leaves

The closed `native_source_leaf_v1` format derives one Item at count1 and its exact normalized text from a pinned literal Lua table entry, plus exact physical selectors from the startup projection. Its claim receipt owns once eligibility; the existing item-only claim transaction records the existing progress obligation. The shared monotone flag lowering teaches the new dedicated instruction track. The source's storage assignment remains an independent required hoist instruction, alongside the construction plans.

The source leaf is independent of the historical chest pilot. That migration processes its original pilot formats; the new producer validates this format against the retained donor bytes. Current-source definitions and historical dispositions remain intact. This is a reusable literal written-reward format, with closed fields, text digest/carrier checks, native Item admission and no charge-state inference.

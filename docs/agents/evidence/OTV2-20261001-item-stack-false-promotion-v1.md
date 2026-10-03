# Explicit wiki non-stackability

The bounded packet contains 2,380 source-qualified `stackable=false` facts:
2,345 previously UNKNOWN fields and 35 already-known, idempotent negatives.
Every present wiki parameter must parse and agree; absent values remain UNKNOWN.
An EXACT source binding is required; affirmative client cumulative flags, retained
stack conflicts, blocked evidence states and existing map owners are holds.
The 13 held map-owner IDs retain their page/revision/content hashes in the packet.

This repairs a native semantic field under `GAME-ITEM-01` §4.1. Physical class,
stack class, materialization, destinations and stack maximum remain unchanged.
The accepted native validator allows known false with unknown identity classes.
The applier validates every row before mutation and runs after starter admission.

Checks: `python tools/content-schema/item-authoring/test_lower_wiki_stack_false_packet.py`
and `python tools/content-schema/item-authoring/lower_wiki_stack_false_packet.py --check`.

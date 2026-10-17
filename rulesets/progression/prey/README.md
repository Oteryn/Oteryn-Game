# PREY content authoring

Accepted basis: `docs/architecture/reviews/OTERYN_GAME_PREY0_PREY_AND_HUNTING_TASKS_DECISION_2026-09-30.md`
§3, §6.1 and `PREY0-RL-16`. The directory remains `READY_UNPOPULATED`: no production
pool, level bands, grade table or draw policy is admitted by this component.

## Exact pool and loot-budget compiler

`tools/content-schema/native-gameplay/prey-loot-provider.schema.json` describes
the authoring input. It has four required fields: its fixed schema discriminator,
SHA-256 digests of the exact emitted Creature-profile and Loot-section bytes, and
the complete preyable pool as exact Creature DefinitionRefs in UTF-8 key order.
Hashes bind candidate content; they do not establish current runtime authority.

Pass that file as `--prey-provider` to the existing
`tools/content-schema/native-gameplay/build_full_spell_artifact.py` producer.
It requires the `--progression` path that emits native loot. Presence of the
provider requires validation before the native manifest or payloads are written;
an empty provider, missing pins, extra `enabled` flag or mismatching pool refuses.
The producer records the checked source hashes in its source-closure proof and
retains the provider as a source sidecar. It does not add an unsupported runtime
manifest field or activate Prey. Ordinary compilation without a PREY provider
keeps its existing output.

The compiler derives potential eligibility from the supplied native Bestiary,
positive experience and explicit native prey fact, and checks it against the
actual canonical authoring for the same exact Creature. Unknown facts refuse.
Only an explicit canonical/native exclusion can remove an otherwise eligible
Creature; the authored pool cannot silently omit one with a large loot table.
Missing canonical definitions and mismatching exact Loot references refuse.

Every final emitted table for that complete pool has at most seven entries.
Zero-chance entries and repeated Item identities still count. Two full rolls and
the improved-loot trigger therefore use at most 29 draws and 14 output entries
under PREY0. Nested/alternate/unknown table shapes refuse; the checker never
selects a smaller source table or discards unsupported leaves. Before projection,
every used canonical Loot definition must have the closed table shape, so a
table-level alternative cannot disappear from qualification. A pool below 27
refuses PREY compilation for that generation. Normal native type/item/byte
qualification remains required, as do the remaining PREY content tables,
independent review, migration, runtime writer and activation gates.

Focused regressions run through the actual loot producer:

```sh
cd tools/content-schema/native-gameplay
python -m unittest test_prey_loot_budget
```

All fixtures in that test module are explicitly synthetic. No production pool or
new Creature identity is created by those tests.

# Verified Source component to Quest associations

This bounded, offline supplement closes an association/reference gap only. It does not
close callback body conversion, runtime activation, player/world storage equivalence,
exclusive ownership, Native binding or whole-Quest completeness.

## Replay

```
python builder.py --repo-root REPO --assignment ASSIGNMENT.json --corpus-manifest CORPUS.json --out exact-source-joins.json
```

The manifest supports absolute cache paths or paths relative to the manifest directory.
Every consumed Source file is checked for byte length, SHA256 and Git blob SHA1.
All witnesses carry donor, pin, path, whole-file digests, decoded Unicode character
span, token digest, line number and line digest. No Source Lua is executed.

## Exact joins and limits

1. Source canonical progress occurrences are resolved and their exact source/line hashes
   verified. Storage accessor symbols in a component are correlated to these track
   anchors only within the same donor, pin and datapack. Player versus Game storage
   domains and receiver dispatch remain unassessed; a symbol correlation never creates
   a Native progress effect.
2. Existing Source graphs must actually be present in the canonical Quest source_data.
   Their exact source path/Git blob resolves to the pinned corpus. A literal
   `:registerEvent("Name")` request in such a file is joined to a unique constructor plus
   registration for that event name across the whole same-donor/pin/datapack script scope.
   Chained callers are supported. Comments and quoted fake calls cannot create proofs.
   Source event binding is a named reference association, not proof of runtime execution.
3. Exact Source graph/path and progress occurrence/path joins are supported, but currently
   have zero hits because these 248 files originated in the unjoined-component inventory.
   The packet reports this honestly instead of turning directories into ownership.

No directory/title-only candidates are promoted. Variant/datapack identity is retained.
Source metadata outside captured /scripts/ paths or runtime-generated registrations may
introduce additional bindings; unique captured registrars are not a proof of live registry
uniqueness or activation.

## Results

All248 input components are represented. **17 component files associate to 7 canonical
Quest keys**, using17 registered-event caller proofs and15 verified storage-symbol proofs.
This adds15 associations beyond the previously reported two Brokul variants.

- Secret Library: Brokul lever2 and Asuras mechanic2.
- Forgotten Knowledge: Astral Source2 and Distorted Source2.
- Dangerous Depths: Fiery Heart2.
- Cults of Tibia: HealthPillar2, Zarcorix health reflection2, CheckTile1.
- Barbarian Arena and Ultimate Challenges: one shared alternate-pack arena-enter file.
- Demon Oak: one alternate-pack gravestone file.

The remaining231 files have no exact association in this bounded pass. They are not231
missing quests. Every248 file retains semantic coverage and runtime activation holds;
there are zero missing captured Source bytes in this pass. Alternative-pack joins use
verified anchors from the same alternative pack, without asserting that pack is active.

Input digests cover only stable Source semantic slices (`identity`, `source_refs`,
`source_data`) plus graph provenance, assignment and corpus manifest. New supplement
references under `oteryn_recipe.donor_source_data` therefore cannot cause a digest cycle.

Root integration adapter should use records[].quest_keys, records[].proofs and the
component packet path/pointer. The current closed schema additionally fixes all Native
flags false and completeness/dispatch/domain assessments to their honest values.
Existing canonical readiness/missing_data is not mutated by the builder.

13tests cover registration strings/comments, chained callers, duplicate/unregistered
constructors, exact membership, Brokul variants, all witness spans and source fences,
Native/domain limits, schema negatives, bad Source digests, duplicate assignments and
stable Source-slice hashing. Set QUEST_COMPONENT_REPO_ROOT, QUEST_COMPONENT_ASSIGNMENT
and QUEST_COMPONENT_CORPUS_MANIFEST for local-cache test replay.

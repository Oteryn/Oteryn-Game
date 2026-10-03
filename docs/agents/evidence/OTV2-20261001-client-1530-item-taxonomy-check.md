# Client 15.30 and retained family evidence continuation

Classification: **PROVEN** byte identity and extracted observations; **DERIVED** navigation
profile agreement. These observations do not establish equipment legality or runtime rules.

The owner pointed to `content/assets/files`. At main
`d3cfb2468510255d2495514ec975c23c1d76f32a`, the appearances file is 5,017,996 bytes,
Git blob `0b485c58191f05114e21c5f2f595e72168e9b04f` and SHA-256
`2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2`.
The checked repository artifact decodes to 43,516 objects. Its pin identifies **15.30**,
not a newly obtained 15.33 client.

Navigation now admits 81 additional entries through the existing strict, digest-bound
`items-family-fallback.json` validator and retains every fallback source observation.
Explicit conflicting or unadmitted primary types in the newer stats snapshot remain holds;
this includes Old Rag i24415 despite the older event-family candidate. Two more current,
Crystal-bound, non-map-owned Items have agreement between a specific market category and
its corresponding semantic clothes slot:

| Item | Official name | Market category | Clothes slot | Family |
|---|---|---|---|---|
| i50259 | zaoan monk robe | 1 (armors) | 4 (armor) | equipment_armor |
| i50269 | legs of enlightenment | 8 (legs) | 7 (legs) | equipment_armor |

All rows preserve canonical identity and explicit source provenance. Existing categories,
Terrain and WorldObject owners are preserved. Taxonomy increases from 9,335 to **9,418**;
**3,283** current Item definitions have neither taxonomy nor an existing map owner.
Generic hand slots, absent flags and market `others` remain insufficient evidence.

Glooth Spear i21158 explicitly has `flags.cumulative=true` in this client. This corroborates
Fandom revision 1147854 and conflicts with the retained BR false observation. The client
contains no stack maximum, so this observation alone cannot satisfy the native requirement
for a known nonzero stack maximum. Neither stackability nor a maximum is promoted here.
Bookcases i31194/i31195 have explicit container flags but no capacity value; 18 versus 8
remains unresolved. The capacity census has 27 agreed-wiki/content disagreements plus the
separate i5801 Key Ring/Jewelled Backpack inter-page identity conflict, explaining a broader
28-row disagreement union.

Validation: deterministic tree regeneration and source-coverage validation, six taxonomy
regressions, migration tests, materialized-tree validation, Ruff and governance checks.

# Crystal map source candidates

Run from the repository root:

```sh
python3 tools/content-schema/map-content-linking/fill_links.py --check
python3 -m unittest discover -s tools/content-schema/map-content-linking -v
python3 -m ruff check tools/content-schema/map-content-linking
```

The producer verifies the owner-confirmed 6,248 physical client 15.30 files,
the current map palette, existing Presentation and Creature definitions, and
the pinned Crystal summer-update primary monster XML. The XML Git blob is
verified against the complete pinned source tree. Its manifest hashes every
input and output. Generation refuses to overwrite an existing packet;
`--check` re-derives its exact file and directory inventory.

Outputs are in `imports/crystalserver/summer-update/map-content-linking/`.
To generate a new packet in a clean authoring checkout, run `fill_links.py`
without `--check`. Preserve any previous packet before generating a successor.
Changes to any input require a new packet and fresh validation.

## Candidate boundary

All outputs are import proposals. The producer writes no canonical content,
admits no public identity or source profile, and activates no server. Asset
identities remain proposed. Source-ID joins with existing Presentation records
do not establish visual equivalence between Canary and client 15.30 and do not
create client bindings.

The producer uses the current placement palette, including its provisional
donor keys. It does not substitute the 5,952 proposed closure targets before
the owning Item/Terrain/WorldObject records have been admitted.

Supported complete groups are projected to the accepted Spawn.Source core
shape in `spawns/candidate/`. The index explicitly says
`CANDIDATE_NOT_RUNTIME_ADMITTED`. Unsupported groups preserve their ordinal,
raw attributes and every raw point in `spawns/held-groups.json`. No Creature
identities are invented and no partial group is silently emitted. The complete
primary XML remains pinned for source radius, configuration and weighted
choice work owned by the architect and population lane.

Tile eligibility, boss/encounter policy, source behavior parity, alternate
populations, native realization and activation are outside this producer.
The accepted native source profile and conversion gate still require owning
amendment before replacing the canonical Canary spawn catalogue with Crystal.

## Remaining integration

The map palette still requires admission of 5,948 direct Terrain definitions
and four Item/WorldObject pairs. Additional Terrain kinds and speed 1200 need
owning format decisions; `Common` is owned by MAP-KIND-CLASS-1 (PR #1786).
Canonical palette/catalogue changes, source-profile admission, runtime startup,
cutover, map wire and client rendering are separate tasks.

The packet links 25,982 palette appearances and verifies 5,084 sprite sheets;
the two Source NULL appearances remain explicit. It contains 54,680 candidate
groups / 87,815 points, with all 33 held groups / 446 points preserved.

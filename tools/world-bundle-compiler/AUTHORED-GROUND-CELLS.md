# Authored ground cells

`source::project` and `source::project_regions` optionally load
`content/npcs/authored-ground-cells.json`. This first-party source document uses
schema `OTERYN_AUTHORED_GROUND_CELLS/v1`, the normal project coordinate frame,
a nonempty provenance object and 1–4096 explicit cells. Each cell names `x`, `y`,
legacy floor `z` and an existing nonprovisional `ground` palette key.

The loader adds only absent cells. An existing captured tile, including one without
a usable ground, is an error. It preserves all other tile flags, house IDs, zones,
item order and exact attributes. Selected-region compilation includes additions only
in those regions and retains all native floors.

The existing checked sector codec encodes the augmented B3 source in memory.
Normal registry resolution, terrain classification, world bounds, collision,
NPC admission, parity and bundle equivalence still run. No binary source files are
written. Regenerate and verify world pins with the normal compiler tooling.

The initial Book World source provides eighteen conservative floor cells around two
independently sourced NPC coordinates. It uses the area's captured paper ground
50362, admitted from the pinned official client. The document records the missing
Summer/Winter fragment coverage, retail citations and explicit layout assumption.
It does not assert a complete Between the Lines quest or retail room layout.

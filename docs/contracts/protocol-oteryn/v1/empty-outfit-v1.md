# EMPTY_OUTFIT_V1

Capability 275 extends the `APPEARANCE_V1` binding look union with tag `3`, an empty
outfit carrying no additional fields. The server can select this look for a staff
actor with staff invisibility when observed by a peer below the required staff
group. Invisible (Utana Vid) uses the regular player appearance projection. A
peer that does not select capability 275 never receives tag 3.

The capability requires `WORLD_SPATIAL_ENTITIES` (6) and `APPEARANCE_V1` (25). It adds
no command or state domain; domain 21 continues to carry appearance mappings.

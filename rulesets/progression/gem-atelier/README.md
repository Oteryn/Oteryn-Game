# Gem Atelier data location

GEM-R §4 places the shared Wheel/Gem catalogue under
`rulesets/progression/wheel-of-destiny/gems.json`. This directory is populated
by that shared import, not by a second copy of costs or mod tables.
See the sibling Wheel README and `import-manifest.json`.

The server reads the catalogue at boot with `runtime_admitted:false`.
No reveal, grade-up, vessel, charge, inventory or economy operation is enabled.

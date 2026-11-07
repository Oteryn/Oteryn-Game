# Node qualification with the pinned World and spell content

The default `run.sh` first preserves the native entry room's existing SEAM stages
in a disposable baseline pass, then creates a fresh topology with real World
regions `(126,125)`, `(126,126)` and `(127,127)`, on every authored floor, and
`content/spells.manifest.json` content. `compile-regions` uses the production
compiler and its equivalence proof; the generated qualification digest is passed
to the production loader. The committed full-world pin is unchanged. The third
region retains the default start for ordinary home respawns. The full-world smoke
measured 176 seconds in the production loader before selecting this subset.
It uses the shipped `serve` command's `[world_bundle]` composition and sets the
Channel's existing first-entry start to Thais temple `(32369, 32238, -7)`. The pin
remains preproduction; this runner does not enable production Worlds. Each pass
creates and cleans up its own Platform/PostgreSQL/node topology.

The World client reads the join snapshot and drives real commands for town
navigation, browse field, daily boost, depot search, combat, loot, death and
recovery. Inventory prerequisites come from production starter grants. Missing
equipment or forge materials are reported as explicit prerequisite skips; no
game-table writes provide inventory. Store remains unavailable at the pinned
Platform revision. `world_boot_seconds` measures process start through readiness;
`world_loader` records the client's local production-loader duration. The focused
`node_boot_qualification_world_loader` library test loads the same bundle without
sudo and checks the temple protection zone and reachable depot.

To qualify the preserved full spell manifest through the same topology:

```sh
PLATFORM_SOURCE=/path/to/pinned/platform \
NODE_BOOT_SPELLS=1 \
  bash tools/qualification/node_boot/run.sh
```

The Platform checkout must be the exact revision pinned in `run.sh`. The runner
needs the existing Docker, sudo, OpenSSL and Rust toolchain prerequisites. Supply
`NODE_BOOT_SPELL_MANIFEST=/absolute/path/to/manifest.json` to test another candidate.
The default is `content/spells.manifest.json`.

Spell mode first runs the existing full-manifest compile/decode/stage test. It
copies only the manifest and its declared, SHA-bound inputs into the disposable
node directory, preserving safe relative locators, and gives the
same manifest to the operator's content issuance and the service's activation.
It replaces the tiny-room SEAM stage with
`node_boot_spells_against_running_node`; assignment, admission and shutdown keep
the existing real paths. It records the selected manifest SHA and the actual live
cast results. This is not a claim that every catalog spell cast successfully:
normal vocation, level, resources, targeting and cooldown rules still apply.
Unsupported mechanics and approximate source fidelity remain separate from the
observed execution result.

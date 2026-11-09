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

W20a/W20b use an explicitly disposable `qualification-clock` build by default.
The World UTC clock and dispatch's monotonic owner clock share an atomic file
offset. W20a advances 75 game seconds per observed sample; W20b advances one
light-decay interval at a time after the cast, cooldown and relog assertions.
The original transition, color, rate, mana, decay and expiry assertions remain
in place and measure elapsed game time. Each accelerated row has a 120-second
wall-time budget, plus its existing cleanup budget (together under five minutes).
Other rows do not advance the offset. Ordinary builds omit the clock input;
even a qualification build requires the existing disposable fixture configuration.

To retain the original real-clock observations and build the ordinary binary:

```sh
OTERYN_NODE_BOOT_REAL_CLOCK=1 PLATFORM_SOURCE=/path/to/pinned/platform \
  bash tools/qualification/node_boot/run.sh
```

That mode retains the original 3,750/3,300-second row deadlines and is intended
for an on-demand extended run, outside the normal 60-minute job budget.

Before World admissions, the disposable administrator initializes epoch zero through
`game_world_reset_initialize`, binding the compiled bundle to the operator's issued
content activation. An exact replay must be idempotent. Runtime Cyclopedia and daily
boost preparation require this admitted epoch; schema installation does not create it.

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

Activation event waits use `activation_wait.sh`: the tiny fixture keeps 60 seconds;
world/spell full-manifest passes allow 180 seconds. On manifest `35de21c6`, the
real operator qualification took 31.917 seconds and node qualification/staging/
activation took 48.227 seconds idle. With six CPU load workers on a four-CPU
quota, node activation completed successfully in 122.451 seconds, beyond the
old 60-second wait. The bound covers that measured path; assignment's 120-second
wait, readiness's 60-second wait, and all grant/replay/stale-predecessor assertions
are unchanged. `content_full_manifest_boot` exercises those real content APIs,
checks the exact issued pins and sequence, and uses the runner's same wait profile.

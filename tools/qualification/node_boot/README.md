# Existing node qualification with spell content

The default `run.sh` qualifies the native entry room and its existing SEAM stages.
It creates and cleans up a disposable Platform/PostgreSQL/node topology; it does
not modify an already running test server.

To qualify the preserved full spell manifest through the same topology:

```sh
PLATFORM_SOURCE=/path/to/pinned/platform \
NODE_BOOT_SPELLS=1 \
  bash tools/qualification/node_boot/run.sh
```

The Platform checkout must be the exact revision pinned in `run.sh`. The runner
needs the existing Docker, sudo, OpenSSL and Rust toolchain prerequisites. Supply
`NODE_BOOT_SPELL_MANIFEST=/absolute/path/to/manifest.json` to test another candidate.
The default is `docs/reference/spells/r21-local-candidate/active-artifact/manifest.json`.

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

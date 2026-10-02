# Imported spell data for the test server

This directory is the actual server-readable data package, imported from the
qualified r25 source candidate. `manifest.json` and its 11 pinned providers are
self-contained; loading them does not require paths under `docs/` or a source
checkout. Provider bytes and native identities remain unchanged.

Imported inputs include 246 player definitions (241 selected, five aliases),
1,508 Creature/Presentation profiles, 1,253 approximate monster melee selections,
255 disabled melee profiles, 118 Item policies, 268 appearance bindings, Wheel,
familiar, training and the existing Thalom source-world data.

`sidecars/` preserves presentation/condition/sound source facts, individual disabled
reasons, scenario prerequisites and runtime-support flags. Its 20,640 source monster
slots are reference data; the native loader does not execute that sidecar or install
all source attacks. Missing systems remain explicitly unqualified.

## Use the existing server loader

Set `OTERYN_NATIVE_GAMEPLAY_MANIFEST` to this directory's `manifest.json` in the
normal test-node configuration. The node still requires independently qualified
server/client artifact pins, scope assignment, issuance and ordinary admission.
No account, credential, deployment setting or active generation is changed by
committing these files. The canonical World project and family indices are intact.

Validate directly from the repository root:

```sh
bash tools/qualification/spells/run.sh map content/test-packs/spells/r25/manifest.json
```

The default `map` and `server` modes select this package too. The latter uses the
existing disposable qualification services, not a long-lived deployment.

`import-status.json` identifies actual loader inputs versus reference sidecars.
`import-validation.json` records the server-loader result. `SHA256SUMS` covers
all packaged files. The package is test content with `baseline_test` magnitude
policy and source approximation flags, not production numerical parity.

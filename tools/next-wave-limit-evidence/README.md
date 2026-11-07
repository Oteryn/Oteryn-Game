# Next-Wave Limit Evidence Harness

This is a non-shipping Rust/std-only evidence harness for Oteryn-Game Issue #133. It models checked pre-allocation accounting for the exact first-slice Ability, Interaction, AI and TCP/TLS listener resource candidates accepted under Issues #128/#131.

It is not product runtime code, a production sizing tool, a gameplay balance source, or Reference-parity evidence. The canonical resource registry is updated only by a later separately serialized task.

## Verification

Compile and run all boundary tests:

```powershell
rustc --edition 2024 -D warnings --test tools/next-wave-limit-evidence/main.rs -o $env:TEMP\oteryn-limit-tests.exe
& $env:TEMP\oteryn-limit-tests.exe
```

Build the optimized emitter and write its UTF-8 stdout directly to the evidence JSON. The default invocation emits deterministic JSON; `--stress N` instead runs N deterministic full max/max+1 fixture iterations and prints a checksum summary.

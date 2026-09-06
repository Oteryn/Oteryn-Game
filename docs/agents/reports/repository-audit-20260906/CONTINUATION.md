# Repository audit continuation checkpoint — 2026-09-06

This checkpoint preserves the additional work performed after the first published audit report for Issue #359 / PR #360. It is evidence, not a claim that every residual audit control is complete.

## Pinned product and upstream reconciliation

The product exercised by the continuation remains exactly:

- source commit: `7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3`
- source tree: `359a52348bfdf8088a7cd456f4015b05279721b6`

While this continuation was running, protected `main` advanced to `b008614881fcc74f09e55e4d1b9e6c64ece04ce9`. The only delta from the audited product is three agent-instruction documents: `docs/agents/AGENTS.md`, `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md`, and `docs/agents/PROMPTING_STANDARD.md`. No product/runtime/Cargo/schema/workflow source changed in that upstream commit. This checkpoint merge-up adopts those current instructions without relabeling the pinned product as newly audited.

## Native coverage-guided fuzzing

Run `34056706385` completed successfully on audit head `6386d3be8723bfde69caab42fc0e2caf91467e07` after two earlier `cargo-fuzz 0.12` attempts failed before target execution because its parser could not consume the repository's unchanged TOML 1.1 workspace root. Those setup failures are retained as tooling evidence and are not product failures.

The successful run compiled the same pinned audit probes directly with Cargo/nightly, native libFuzzer sanitizer coverage and AddressSanitizer. The tracked product checkout remained at `7ce1d88...` and `git diff --exit-code` was enforced.

### Wire parser

Job `101549818645`: SUCCESS.

- source: `7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3`
- result: exit `0`
- duration budget: 300 seconds; libFuzzer reported 301 seconds
- executed units: `69,627,322`
- average executions/sec: `231,320`
- new corpus units: `5,819`
- peak RSS: `731 MB`
- max input bytes: `1,048,576`
- fuzz binary SHA256: `5c1c5f858347c3a534804b4079610eac60d4c3eb77eac49c7c86a108deb225e8`
- artifact: `continuation-native-fuzz-wire-7ce1d88`, ID `9996276484`
- downloaded ZIP SHA256: `7a91a60204333120dd6cc900b3e2a11bc2faa1e3256fccc7aa7b1b0dbb3cf572`

No crash was observed in this bounded campaign.

### Content parser

Job `101549818844`: SUCCESS.

- source: `7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3`
- result: exit `0`
- duration budget: 300 seconds; libFuzzer reported 301 seconds
- executed units: `4,757,388`
- average executions/sec: `15,805`
- new corpus units: `1,511`
- peak RSS: `524 MB`
- max input bytes: `1,048,576`
- fuzz binary SHA256: `71c17410e295030f61c02053c30aacf656f290ba54dc9e7dc9f3445bb58cd810`
- artifact: `continuation-native-fuzz-content-7ce1d88`, ID `9996285249`
- downloaded ZIP SHA256: `43315051f5637d53a5e8c1ecdca803891af61d6c186860cde13d1fdd671d7010`

No crash was observed in this bounded campaign.

## Interpretation

This materially strengthens control C09 compared with the earlier deterministic mutation smoke. It does not convert C09 into an unconditional PASS: the campaign is bounded to one deterministic seed and five minutes per target, uses `cfg(fuzzing)` plus sanitizer instrumentation rather than the release binary, and does not exercise a real peer transport, every semantic protobuf invariant, every content producer, or long-duration/multi-seed fuzzing.

The final repository candidate removes the temporary audit workflow that launched these probes. The workflow was never a required status or standing control. Its historical commits and Actions runs remain immutable provenance.

## Remaining audit status

All prior findings and unresolved controls remain authoritative unless explicitly superseded by later evidence. In particular, full product/session E2E, long-running concurrency/load/soak, backup/restore and deployed recovery, complete distribution/update/rollback, external producer qualification, complete dependency internals and all-line semantic/history review remain separate residual controls.

Saving this checkpoint does not merge PR #360, close Issue #359, repair product findings, or claim 100% product correctness.

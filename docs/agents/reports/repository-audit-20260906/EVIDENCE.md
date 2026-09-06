# Compact execution evidence

Source product: `7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3`.
Issue #359; publication PR #360. Exact run/job, downloaded ZIP digest and binary hashes are retained in `execution-evidence.json`. Full original archives and per-target exports accompany the owner's audit package. Paths beginning `owner_archive:evidence/` in findings address that package, not nonexistent files in this repository.

## Original release tests, Linux and configured PostgreSQL

Run `34049229486`, job `101529713591`: SUCCESS.

```text
cargo +1.94.0 build --locked --release --workspace --all-targets --all-features
cargo +1.94.0 test --locked --release --workspace --all-features
```

Parsed terminal target summaries: 52 target blocks; 718 passed test executions; 0 failed; 0 ignored. Nested child summaries are not added twice. Source-included tests repeat between targets, so this is not a count of unique test cases. Detailed target/log checksums are in the owner package.

The exact release server and synthetic harness smoke succeeded. The non-smoke release server returned exit 2, refusing unavailable gameplay. This is expected fail-closed behavior, not a successful game session.

## Original Windows tests and real DX12

Run `34049958965`, job `101531656129`: SUCCESS. The release MSVC executable was built, hashed and invoked directly. Separately, 33 original client/input/renderer/simulation tests passed. The audit probe called the production renderer with a real window:

```text
AUDIT_REAL_DX12_PRESENT: frames=1
```

The probe also closed the renderer. Adapter identity was not captured. This does not establish physical GPU coverage, a full scene, device-loss recovery or performance.

## Native PowerShell reproduction

Same Windows job, PowerShell 7.6.5. A nested process executed a native exit 7, then a native exit 0, and returned the last native code:

```text
first=7 last=0
nested_exit=0
```

The reproduction confirms the mechanism behind F01. It does not prove a hidden error occurred in the historical canonical build.

## Native Rust PKCE characterization

Run `34049958965`, Linux job `101531656052`, and the Windows probe both exercised the actual Rust library. Assertions observed 31 entropy bytes rejected, 32 accepted producing 43 verifier characters, 96 accepted producing 128 characters, and 97 accepted producing 130 characters. Last case is a confirmed compliance finding, not a repaired regression or account-takeover proof.

## Native deterministic parser probes

Run `34049958965`, job `101531656052`:

```text
AUDIT_WIRE_MUTATION_SMOKE: 20000 deterministic buffers through both decoders plus 1024 valid-seed byte substitutions; no panic; not exhaustive fuzzing
AUDIT_CONTENT_MUTATION_SMOKE: 6286 deterministic truncation/bit-flip rejects; integrity-path evidence, not full semantic fuzz coverage
```

The Rust probe sources are retained under `probes/`. They were copied into untracked integration-test targets of a disposable source checkout. The tracked product was not modified. Coverage export of original tests preceded these probes.

## PostgreSQL CHECK probe

Run `34049958965`, job `101531656052`. A TEMP table with the extracted predicate accepted one row having a non-null activation and NULL expiry; the whole transaction was rolled back. Server PostgreSQL 17.6. This is an isolated SQL predicate characterization, not execution of an authorization exploit or complete migration qualification.

## Original Python lifecycle failure

Run `34050635585`, job `101533492650`: FAILURE, preserved intentionally. Hosted Python 3.12.14 executed 24 existing isolated test entrypoints plus three validators: 26 commands passed, one test suite failed.

All three original lifecycle tests fail before assertions at `tools/agents/tests/test_validate_governance_lifecycle.py:20`, `setUp`:

```text
self.original_root = validator.ROOT
AttributeError: module 'validate_governance' has no attribute 'ROOT'
Ran 3 tests in 0.001s
FAILED (errors=3)
```

Cases: active task authority/status, handover non-authority/expiry/supersession, and prompt registry coverage/retirement. Exact failing log SHA256: `7f8fc80a49a728b8ee7be6d1e4df3ec95b084278940af9a958db92adca39487e`.

The core validators still pass. Therefore the result is not explained by unsupported local Python or a universal repository failure, and must not be erased merely to make the audit observer green.

## Standalone Rust model outside Cargo workspace

Same run/job; this step succeeded before Python discovery failed. `rustc +1.94.0 --edition 2024 -D warnings --test tools/next-wave-limit-evidence/main.rs`: 11 passed. Optimized emitter JSON parsed. `--stress 8` produced:

```text
iterations=8 accepted=192 rejected=192 peak_single_allocation_bytes=1048576 checksum=10640533385048128381
```

These are deterministic model counts, not a workload-capacity measurement or new production resource policy.

## LLVM coverage qualifications

Original Linux workspace tests, with configured PostgreSQL, were instrumented using Rust 1.94.0 and matching LLVM tools. The raw line count is 47992/52882, approximately 90.75%. It includes tests and source aliases. 101 exported file records correspond to 93 normalized physical paths. The export reports:

```text
warning: 38 functions have mismatched data
```

Branches were not measured. Windows-specific and uninstantiated code is not a valid part of an inferred full-product denominator. The detailed interpretation, alias mapping and missing-record paths are in `execution-evidence.json`; the complete raw export remains in the owner archive. No clean coverage threshold or universal correctness certificate is claimed.

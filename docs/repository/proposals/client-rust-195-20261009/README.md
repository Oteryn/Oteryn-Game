# Rust 1.95 CI migration review packet

This is a local proposal, not an applied or published control-plane change.
All workflow edits were restored to their captured pre-edit contents. This durable
packet preserves the two proposal patches, exact manifest and local validation
summary; proposed full files and overlay scripts are private workspace artifacts. `manifest.json` lists the 21 proposed paths and
the exact old/new blob and contract hashes. Root later refreshed the non-production world compiler-input pin and passed
pin-check: two identical builds with unchanged world payload f8b11ebc…; inputs88be0150….

The client and `rust-toolchain.toml` require Rust 1.95.0. The current active
workflows and their immutable validator expectations still use 1.94.0.
Candidate `0026b73299faed34bc39b1683f71e77cc6c40ca4`, run `37915263939`,
failed changed-crate Clippy before compilation for that version mismatch.
A partial workflow migration is also rejected: the post-merge Rust workflow
has an exact reviewed SHA-256 pin in the repository-policy validator.

## Authorization boundary

`/workspace/Oteryn-Game/docs/repository/GITHUB_GOVERNANCE.md:39` says:

> A merge-authority/control-plane change is never routine: it needs the
> owner's explicit authorization for that change, exact-head validation,
> a green merge-authority audit and, afterwards, the post-merge
> repository-configuration readback. A change to
> `.github/workflows/merge-gate.yml` or `agent-governance.yml` first needs
> the audit's approved-blob pin rotated in a separate owner-authorized change.
> Do not create bypass actors or weaken the general merge gate for convenience.

The current protected-base audit also rejects a candidate that changes the
audit itself. These files provide a concrete packet for the owning control
plane to select and authorize the applicable protected migration route;
they do not make that route unnecessary or authorize applying both patches
in the ordinary product PR.

## Proposed changes

- `01-audit-pin-rotation.patch` is the audit-owner review slice: exact PR/MQ
  gate blob rotations and the audit's current toolchain-dependent expected
  command strings. Its publication must use the separate authorized route.
- `02-toolchain-migration.patch` aligns all active workflow Rust selections
  and the associated current policy/test expectations to 1.95.0. It rotates
  only contract hashes whose text changed. Scope, validation, final gate,
  Atlas, trust boundaries, permissions, action pins, fail-closed conditions
  and negative tests remain intact.
- Both zero-context patch files together form the complete proposal; neither is
  applied. Use git apply --unidiff-zero only through the authorized route. Root
  reconstructed and byte-verified all21 proposed files in an isolated scratch tree.
- Full proposed files and before-images remain in the private workspace; the
  published patches and manifest preserve the exact proposed deltas.

Exact proposed PR gate blob: `4abf4feb48ab406eb09b7fa7c142c8edaa7d82a5`.
Exact proposed MQ gate blob: `2fe13184cae1982f7d9d605794bb98ecde760061`.
Proposed post-merge Rust workflow SHA-256:
`942ddf0ec317633a20113e89a91139dc56aa75630ffce28d2203437b7443039b`.

## Validation completed locally

`python3 validate-proposal.py` checks the exact limited deltas, restored
baseline files, every proposed YAML/Python file, Git blob pins and job hashes,
then runs the proposed policy core using a read-only in-memory path overlay.
It passed, including the existing post-merge impact-routing regressions.

The restored actual repository's
`python3 tools/repository/validate_repository_policy.py` also passed.
These checks are local consistency evidence; they are not a green protected
audit, a frozen GitHub candidate, Windows qualification or Merge Queue proof.

Separately, permitted font-license source changes remain in the product
working tree: an exact-package exception for `epaint_default_fonts@0.36.2`,
original complete font notices and verified provenance, installation of the
notice alongside the launcher, and installer assertions for its SHA-256.
Official cargo-deny 0.20.2 `check licenses` passes. A negative check changing
the exception to 0.36.1 still rejects the 0.36.2 OFL/Ubuntu license expression,
proving that other versions do not inherit the exception. Windows/Inno
installer execution remains to be qualified on Windows.

## World-pin follow-up after final inputs are stable

The failed world-bundle job reported a pinned compiler-input digest of
`b058e41f1004441f71a4f6d1d65934048b292832c0f049570335fb83a947b1ed`
against candidate digest
`88be015060549d3a1e7a6db431413feab723a5e3780d59bcea1b0064ff0455ef`.
`tools/world-bundle-compiler/src/main.rs:223` includes `Cargo.lock` and
`rust-toolchain.toml` in the compiler-input set. Its `inputs_digest` at line
357 reads **Git-index blobs**, so computing a new pin before final authorized
inputs are staged would create stale evidence.

After final input staging, the owning product writer can compute the exact
input digest with the compiler's existing algorithm, update only the
appropriate `content/world/pins/oteryn.json` input field, compare the derived
identity, and run:

```sh
cargo +1.95.0 run --locked --release -p oteryn-world-bundle-compiler -- \
  derive-identity . oteryn
cargo +1.95.0 run --locked --release -p oteryn-world-bundle-compiler -- \
  pin-check . /tmp/oteryn-world-bundle-final
```

Do not disable the input pin. `pin-check` compiles twice and verifies identical
bundle bytes, payload digest, identity and entry walkability. Refresh any
additional bundle fields only if the actual derived output requires it.

# Game -> Atlas physical-profile readiness evidence

Task: `OTV2-20260816-game-atlas-physical-profile-readiness`  
Issue: #291  
Delivery PR: #292  
Coordination ID: `OTERYN-GAME-ATLAS-V1-PHYSICAL-READINESS`

## Verdict

**`EVIDENCE_GAP`**

The final bounded synthetic physical-packaging spike is successful and materially narrows the later profile decision, but the first executable Game -> Atlas physical-profile contract is **not yet safe to freeze**. Canonical Game spatial/coordinate authority is incomplete at the precision required by the accepted semantic export contract.

The smallest missing owner evidence is a canonical Oteryn-Game spatial/coordinate profile defining axes/orientation, numeric coordinate domain and bounds, floor identity/domain/order, point/bounds validity, deterministic same-position ordering/stack-layer semantics, and presentation anchor/displacement semantics where canonical presentation references require them.

## Exact trusted basis

Audit/spike entry base: `blakinio/Oteryn-v2@8722e565c6a0556934209820e3c14ee4f2dc6093`.

### PROVEN — accepted architecture constraints

- `docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md` requires an explicit versioned `coordinate_profile` before executable producer/consumer compatibility. It requires axes/orientation, coordinate domain/bounds, floor identity/order, point/area validity, same-position ordering, stack/layer semantics and displacement/anchor semantics while deliberately deferring exact numeric ceilings and physical encoding.
- `docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md` keeps OTBM behind a bounded legacy-import boundary and leaves final technical chunk dimensions/vertical packing benchmark-sensitive.
- `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md` keeps semantic World authority independent from serializer details, requires fail-closed bounded physical-format evidence and does not authorize this spike to become the World Project/World Bundle format decision.
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` requires externally controlled counts/depths/lengths/byte sizes to have absolute hard maxima before implementation acceptance.
- Current `Cargo.toml` has no canonical `world-schema`, `world-project`, `world-bundle` or `world-spatial` implementation member. Planned names in `REPOSITORY_MAP.md` are not current implementation proof.
- Existing executable `crates/client-domain::Position { x: i32, y: i32, floor: i16 }` belongs to a module documented as a protocol-neutral **non-authoritative client projection** and cannot be promoted into canonical Game/Atlas coordinate authority by inference.
- `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md` makes the public map definition one immutable World revision shared across channels; channel runtime overlays remain separate channel-local state.

## Final accepted spike evidence

Exact spike code head: `8d43167d44efc7933b47713b47a35d71bf7ff708`.

GitHub Actions:

- run: `31938999246`;
- job: `95145312378` — `Game Atlas Physical Profile Spike / validate`;
- runner: GitHub-hosted `ubuntu-24.04`;
- Python: `3.12.13`;
- result: **SUCCESS**;
- matrix: `24/24` PASS;
- `spike_checks_pass=true`;
- `cap_negative_checks_pass=true`;
- generated report SHA-256: `c20cc40de3ac1811574c29e116249314234caebab05276cfabb8c9d6b524d4f9`.

The run checked out and verified exact head `8d43167d44efc7933b47713b47a35d71bf7ff708`, compiled the spike, executed the full matrix and negative-cap tests, and emitted the complete machine-readable report to immutable workflow logs.

Persisted normalized machine evidence: `docs/agents/evidence/OTV2-20260816-game-atlas-physical-profile-report.json`.

## Repair history — three ordinary cycles exhausted

The stable evidence gate used exactly three repair cycles; no fourth ordinary repair is authorized.

1. **Cycle 1 — text-review metric correctness.** Initial successful run on `f35c28375abaa7339f3f74a4d19966c36393eee1` proved the base matrix, but self-review found accidental UTF-8 interpretation of binary bytes and a misleading diff-line metric that hid compact-JSON line size. Binary text metrics became `null` and text encodings gained maximum-line measurement.
2. **Cycle 2 — chunk/count cap symmetry.** Self-review then found `MAX_CHUNK_BYTES` was not symmetrically enforced by text generation and text decoders did not explicitly enforce record-count bounds. Common chunk/count enforcement was added.
3. **Cycle 3 — encoding-neutral record caps and package churn.** Final self-review found string/object-count validation was still binary-biased and local edit accounting omitted manifest churn. Common record validation now applies to all encodings; negative tests cover oversized strings/object counts/text decoding/binary counts/chunk bytes; local edit evidence reports one changed data chunk plus changed `manifest.json`.

Final cycle 3 is the only accepted spike evidence generation.

## Synthetic fixture boundary

Two deterministic project-owned fixtures are generated, each `128 x 128 x 6 = 98,304` records:

- `synthetic-sparse-v1`, semantic SHA-256 `d91d031d8c129f9310fe15188914969d9e35aa4eb44be283d61751d4eb0043a2`;
- `synthetic-dense-v1`, semantic SHA-256 `045a4b698e5e43e251b040e04da29641b631e14b9c59c8487c0333897e0f6b6c`.

No OTBM, Crystal, Canary, Tibia/CipSoft asset, official client byte, legacy map file or external fixture is used.

The research matrix compares canonical JSON, canonical JSONL, a deliberately non-public deterministic binary lower-bound baseline, `32x32` versus `64x64`, and per-floor versus packed-floor packaging. Deterministic gzip and SHA-256 are measurement mechanisms only; neither is accepted as the production profile.

## Direct final results

### PROVEN — bounded matrix invariants

All 24 cells prove:

- deterministic byte identity;
- semantic encode/decode round-trip identity;
- raw digest corruption detection;
- gzip corruption detection;
- exactly one changed data chunk for one synthetic record edit;
- exactly two changed package files for that edit: the data chunk plus `manifest.json`;
- exactly one data chunk for a point lookup;
- binary text metrics are `null`;
- textual rows produce two changed text lines and expose maximum line bytes;
- common research caps reject oversized record/string/object/count/chunk cases across the tested encodings.

### DERIVED — JSON versus JSONL

Compressed size is effectively equivalent in this matrix; size alone does not choose between them. Source-review evidence differs materially: compact JSON's changed-file maximum line ranges from about `95 KiB` to `3.29 MiB`, while JSONL remains `160-198` bytes. That favors record-oriented text for reviewability but does not by itself select an Atlas distribution serializer.

### DERIVED — floor packaging

At canonical JSON `32x32`, packing six synthetic floors reduces total gzip roughly `6-8%`, but multiplies single-floor point/viewport bytes by about `5.6x`. Per-floor packaging is therefore the stronger current locality baseline for a floor-oriented Atlas consumer, not a canonical decision.

### DERIVED — `32x32` versus `64x64`

For canonical JSON/per-floor, `64x64` reduces aggregate gzip roughly `4-6%` and chunk count from `96` to `24`, but raises point bytes about `3.8-3.9x` and representative viewport bytes about `1.7x`. Representative consumer/cache evidence is still required.

### DERIVED — binary lower bound

At `32x32/per-floor`, the research binary baseline reduces gzip total versus canonical JSON about `21.4%` for sparse and `10.6%` for dense synthetic data. This does not measure schema evolution, browser decoding, dependency/supply-chain cost, diagnostics, compatibility tooling or parser attack surface. It is **not** a proposed public schema.

## Safety / limit findings

Final research-only harness caps are:

- records: `200,000`;
- raw research chunk bytes: `64 MiB`;
- string bytes: `1,024`;
- objects per record: `256`.

These values protect the spike only. **They are not production recommendations or `RESOURCE_LIMITS_REGISTRY.json` evidence.**

## Readiness decision

### `PROFILE_CONTRACT_READY`: NO

Physical packaging evidence now rejects several unsupported shortcuts and gives useful trade-off data, but the accepted export contract requires canonical coordinate semantics not yet frozen by Game authority.

### `EVIDENCE_GAP`: YES

Smallest missing owner evidence:

> **Canonical Game spatial/coordinate profile v1** owned by Oteryn-Game, defining axes/orientation, numeric coordinate domain/bounds, floor identity/domain/order, point/bounds inclusion/validity, deterministic ordering/stack-layer semantics and anchor/displacement semantics required for public presentation references.

It must not copy the non-authoritative client `Position` type or Tibia/OTBM conventions as authority. Once accepted, the later Game -> Atlas physical-profile contract can combine it with this spike evidence and separately justified production resource limits.

## What this task intentionally does not decide

No public serializer/IDL, production compression/container, permanent chunk size/floor packing, coordinate/floor numeric range, production resource ceiling, World Project/World Bundle serializer, asset redistribution, storage/CDN, Atlas framework, delta protocol or repository create/rename/transfer/extraction is accepted here.

## Single next action

Create one bounded Oteryn-Game architecture task for the **canonical spatial/coordinate profile v1**. Freeze only coordinate/floor/order/anchor semantics required by the accepted World model and Game -> Atlas contract; keep physical serializer/chunk/resource-limit decisions as separate evidence-gated decisions.

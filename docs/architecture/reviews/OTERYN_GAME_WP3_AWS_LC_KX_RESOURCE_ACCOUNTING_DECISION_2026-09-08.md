# Oteryn Game — WP3 AWS-LC key-exchange resource accounting boundary

- Date: 2026-09-08
- Issue: [#439](https://github.com/Oteryn/Oteryn-Game/issues/439)
- Admission: protected `main@54f19765c07e3b33ce2d9c10ad57df4818434a52`
- Governing contract: `DUR-FRESH-RESOURCE-ENVELOPE-V1`
- Status: **BLOCKED; evidence/provider work required before executable KX acceptance**

```yaml
classification: ARCHITECTURE_BLOCKED_EVIDENCE_REQUIRED
repository: Oteryn/Oteryn-Game
main_sha: 54f19765c07e3b33ce2d9c10ad57df4818434a52
issue: 439
material_lineage: agent/sqlx-driver-budget-351@8a97ed5e0bdfd42b934295ecf3f98584bb49a00f
kx_source_sha: af54111b3f0125e130e0bac66057f2afa2bcf07c
scope_review: 5585566648
decision: D_BLOCKED_MISSING_PROVIDER_BOUNDARY
implementation_ready: false
future_material_mutation_paths: []
resource_values_changed: false
production_authority_changed: false
next_action: Work allocates a read-only/provider evidence action to obtain an authoritative per-operation allocation-owner interface or complete finite native-byte bounds for the pinned reachable KX paths; it does not allocate a material KX lease.
```

## 1. Decision

Choose **D — `BLOCKED_MISSING_PROVIDER_BOUNDARY`**. The pinned provider graph proves that KX creates operation-scaled native objects and that their Rust owners eventually release them. It does **not** expose a supported per-operation allocation owner/preallocation interface, a provider-authored complete native-byte maximum, or a defensible separation that turns those objects into fixed process overhead. Under `DUR-FRESH-RESOURCE-ENVELOPE-V1`, the same WP3 operation ledger therefore cannot yet truthfully admit initial KX or an HRR replacement.

This is a blocked evidence decision, not an amendment of the resource contract and not implementation authority. No numeric reservation is selected. No runtime, provider, Cargo, registry, workflow, or production path is leased by this document.

### Alternatives

| Disposition | Result | Reason |
|---|---|---|
| A — `PROVIDER_OWNER_HOOK` | **Deferred, not proven** | AWS-LC exposes a process-global allocator replacement intended only for debugging, not an operation-scoped owner. It can be installed only once, intercepts all AWS-LC allocations across the process, has severe reentrancy/locking constraints, and supplies no KX identity. `aws-lc-rs` exposes no owner/preallocation parameter on the relevant constructors. Treating this as a stable operation owner would change process-wide allocator behavior and is not proven safe or ABI/semantic neutral. |
| B — `PROVIDER_FINITE_RESERVATION` | **Deferred, not proven** | Public key/ciphertext lengths and Rust wrapper sizes are not complete native backing bounds. No authoritative maximum covering every allocation reachable through X25519, P-256, P-384, ML-KEM-768, hybrid child overlap, transient provider contexts, and HRR replacement was found in the pinned sources. |
| C — `FIXED_PROVIDER_OVERHEAD_CONTRACT` | **Rejected on current evidence** | `EphemeralPrivateKey`/`PublicKey` and `DecapsulationKey` are created per KX start and retained for that operation. Initial KX and attacker-influenced HRR replacement can repeat this allocation per admitted active operation. These objects are not proven one-time provider globals, so excluding them would silently create request-scaled uncharged work. |
| D — `BLOCKED_MISSING_PROVIDER_BOUNDARY` | **Chosen** | It preserves the accepted envelope without inventing a size, weakening TLS/PQ/provider behavior, or charging after opaque allocation has already occurred. |

## 2. Exact evidence base

The seven inspected rustls KX files and vendored `Cargo.toml` are byte-identical between `af54111b3f0125e130e0bac66057f2afa2bcf07c` and the current material head `8a97ed5e0bdfd42b934295ecf3f98584bb49a00f`. Their SHA-256 values are:

| Path under `vendor/rustls-0.23.43/` | SHA-256 |
|---|---|
| `src/client/hs.rs` | `ebc9c77e62a099d5c279efff02dbc029e73b792711eeac5c6edb25ba2445f379` |
| `src/client/tls13.rs` | `818650319e7e48148eb947b2659f4e40a0d67a8b6a7657f62c864401b3ffc888` |
| `src/crypto/mod.rs` | `46cb1c195e52824c98ea11129c93cbab03d06fb0c3a6daed81a3bab6f9f79e12` |
| `src/crypto/ring/kx.rs` | `d385431f42e76b7aa8bd0749ea256da268239d762114b40a37bee8306b07229d` |
| `src/crypto/aws_lc_rs/mod.rs` | `81a60a0e447dbaad5e6986a3b7b039186f435ccdded702557f18f5c862388a44` |
| `src/crypto/aws_lc_rs/pq/hybrid.rs` | `a1ac53e228fe7bb6851b3a73b1ba4aa64f09bdbc77a09053f59ae89c8ea152b8` |
| `src/crypto/aws_lc_rs/pq/mlkem.rs` | `431389d968ac98aba0a7c6b480d7379a788ac5a6eb77c11cc6df41054515e8b7` |
| `Cargo.toml` | `dc7d5d22a30eede3de5bdab5c583268824349026c4adbcbb82a6afb37b633e96` |

The vendored rustls manifest defaults to `aws_lc_rs`, `prefer-post-quantum`, logging, `std`, and TLS 1.2. Its `aws-lc-rs = "1.14"` entry is a semver constraint, not the resolved version. `Cargo.lock@8a97ed5e0bdfd42b934295ecf3f98584bb49a00f` resolves:

- `aws-lc-rs 1.18.0`, crates.io checksum `ce2b2dcc879c3bae0d371e77c99f2238400ef24ec001394befa67b6e543add9e`;
- `aws-lc-sys 0.44.0`, crates.io checksum `f09fae7be8bb3174e05c6afdb34199e6dc0c7c04ba9fa237b1967adfbde27483`;
- both package archives record upstream VCS revision `f464440d1fd3983ce9fb023e9eaf1698530919a2`, under `aws-lc-rs` and `aws-lc-sys` respectively.

Research used those exact unpacked Cargo package sources. A local wrapper `size_of`, a wire-size constant, allocator observations from a sample run, or a different provider version is not authority for the pinned native maximum.

## 3. FACT / DERIVED / UNKNOWN

### FACT

1. `client/hs.rs` calls `tls13::initial_key_share` before emitting the initial ClientHello. `client/tls13.rs::initial_key_share` selects a configured group (a resumption hint or the first provider group), records `KxState::Start`, and invokes ordinary `SupportedKxGroup::start()`.
2. On an HRR selecting another group, `client/hs.rs` finds that group, records `KxState::Start`, and invokes `skxg.start()` while the initial `offered_key_share` still participates in the replacement expression. There is no reservation or native-owner argument at either call site.
3. `CryptoProvider::kx_groups` is ordered and its first entry supplies the presumptive TLS 1.3 share. `SupportedKxGroup::start()` returns `Box<dyn ActiveKeyExchange>` and has no accounting/owner parameter. The reachable production scope is the default ordered `X25519MLKEM768`, X25519, P-256, P-384 family; this decision gives no blanket authority over `ALL_KX_GROUPS`.
4. The AWS-LC classical implementation reuses `crypto/ring/kx.rs`. Each start generates an `agreement::EphemeralPrivateKey`, computes an `agreement::PublicKey`, and retains both in `KeyExchange` plus a Rust `Box`.
5. In `aws-lc-rs 1.18.0`, `EphemeralPrivateKey` wraps `PrivateKey`; `PrivateKey` contains a `KeyInner` holding an `LcPtr<EVP_PKEY>`. `PublicKey` also holds `KeyInner`; computing it clones/reference-retains the native key and produces encoded public bytes. Managed `LcPtr` destruction ultimately calls the matching AWS-LC free routine such as `EVP_PKEY_free`. This is mixed Rust/native storage, not an inline-only object.
6. ML-KEM start calls `kem::DecapsulationKey::generate`, derives an `EncapsulationKey`, serializes its bytes, and retains a boxed `DecapsulationKey` plus a Rust `Vec`. In `aws-lc-rs 1.18.0`, `DecapsulationKey` owns an `LcPtr<EVP_PKEY>`; temporary encapsulation key/context objects and returned buffers have their own native/Rust lifetimes. Managed native pointers are freed on destruction.
7. Hybrid start first starts the classical child and then the ML-KEM child, concatenates both public shares into another `Vec`, and retains two boxed active children plus the combined vector in `ActiveHybrid`. An error after the first child causes ordinary unwinding/drop of already-created state, but allocation has already happened.
8. AWS-LC's `CRYPTO_set_mem_functions` is a one-time, process-global replacement for `OPENSSL_malloc/free/realloc`; its documentation says it is recommended only for debugging. The callbacks cover every AWS-LC allocation in the process, may execute during initialization/thread destruction and while AWS-LC holds locks, and cannot call AWS-LC or dependencies on it. It is neither KX-scoped nor owner-tagged. Weak-symbol overrides have the same process-wide character. The relevant `aws-lc-rs` constructors expose neither facility as an operation owner.
9. Normal native owners are released by RAII: completion consumes the active exchange; invalid-peer/error paths drop consumed or enclosing active state; HRR discards the old share when replacement takes ownership; connection/state destruction drops any remaining share. Exact release follows final native reference destruction, not the Rust wrapper's visible byte count.
10. `DUR-FRESH-RESOURCE-ENVELOPE-V1` says dependency/runtime allocation retained on behalf of B work must be charged or have an explicit finite reservation, and executable acceptance stays closed when a dependency cannot provide a defensible upper bound.

### DERIVED

1. Provider backing scales with admitted active operations because each initial start constructs fresh ephemeral classical/PQ state. It is not established as process-global fixed overhead.
2. A safe ledger must assume peak overlap of the complete initial active KX and all complete backing required to create the HRR replacement until source/provider evidence proves an earlier destruction point. It must also cover classical child + PQ child + combined hybrid backing and all generation/serialization temporaries before returning the active exchange.
3. Drop proves eventual release, but it does not prove reservation-before-allocation, allocation attribution, or maximum resident bytes. Post-allocation measurement/charging cannot repair a denial boundary.
4. The global debug allocator can observe/control broad AWS-LC allocation only by changing process-wide behavior. Without a supported context/owner token and isolation proof, mapping its callbacks to the current WP3 operation would misattribute concurrent/provider-global work and create reentrancy and ABI risks.
5. C cannot be selected merely because some AWS-LC initialization may be global. The per-start native keys are independently proven operation-scaled, while any separable one-time provider allocation remains unidentified.

### UNKNOWN

1. The complete peak and retained native bytes, allocation count, allocator metadata, and temporary workspace for each reachable X25519, P-256, P-384, and ML-KEM-768 operation in this exact build, including failure paths.
2. A provider-supported context-scoped owner, arena, preallocation, no-allocation execution mode, or exact dry-run reservation API that denies before allocation and releases with the KX object.
3. An authoritative provider-authored maximum that remains valid across supported targets/build modes and includes hybrid-child and HRR overlap rather than only serialized key sizes.
4. Whether any initialization/cache/DRBG/provider-global allocation can be rigorously separated from operation-created state, its finite maximum, and its accepted process/concurrency owner.
5. Exact native-free timing inside every AWS-LC algorithm/error path beyond the RAII/free entry points, and whether any caches retain operation-derived backing after wrapper drop.

## 4. Required accounting and lifetime boundary

If later evidence proves A or B, one WP3 active-operation ledger must reserve **before** the first initial KX allocation. Custody covers all Rust and provider-native allocation and temporary overlap caused by starting the reachable selected group. For `X25519MLKEM768`, this includes the classical child, ML-KEM child, serialized child shares, combined share, wrappers, provider contexts, allocator overhead included by the authoritative mechanism, and construction failure cleanup.

The initial reservation remains owned while its active key share is retained. If HRR requests a different reachable group, the same operation ledger must reserve the complete replacement peak **before** replacement start, while conservatively retaining the initial charge until all initial native owners are proven freed. It may then release only the proven-destroyed initial portion. Completion, peer-validation error, handshake error/cancellation, state replacement, and final connection drop release a charge only after every corresponding Rust and native owner is destroyed. A timeout or control-flow transition alone is not release evidence.

Ordinary `SupportedKxGroup::start()` and ordinary owner-free clients/providers remain semantically unchanged. A future dedicated owner-aware path must be additive and must fail closed **before calling ordinary `start()` or any equivalent allocation** when the selected group cannot honor ownership. A custom provider may implement the owner-aware contract only with equivalent preallocation/attribution/release proof; absence of that support cannot silently fall back to ordinary unowned start.

Because disposition D is not implementation-ready, there is deliberately **no future material mutation path set**. The visible-Rust candidate family (`client/hs.rs`, `client/tls13.rs`, `crypto/mod.rs`, `crypto/ring/kx.rs`, `crypto/aws_lc_rs/pq/hybrid.rs`, and `crypto/aws_lc_rs/pq/mlkem.rs`) remains evidence only. `crypto/aws_lc_rs/mod.rs` remains read-only provider-selection evidence. No `aws-lc-rs`, `aws-lc-sys`, Cargo, or registry path is added; Work must first accept the evidence/provider change that makes A or B real and then issue a separate exact path lease.

## 5. Future RED/GREEN proof obligations

These are acceptance obligations for a later reviewed resolution, not tests claimed by this docs change.

| Boundary | RED / negative proof | GREEN proof |
|---|---|---|
| Initial hybrid KX | Insufficient owner/reservation rejects before either child starts; no native allocation callback/key generation occurs. | One operation ledger covers classical, ML-KEM, both public representations, combined vector, native objects, and all peak construction temporaries. |
| HRR classical replacement | With only initial capacity, replacement is denied before `start`; initial share remains correctly owned and is then cleaned up by the error path. | Same ledger reserves initial + conservative replacement overlap, releases initial only after proven drop, and retains replacement through completion/drop. |
| Denial before allocation | Exhaust every applicable byte/object/count dimension and prove no provider or Rust KX allocation, random key generation, or post-allocation catch-up. | Reservation succeeds before first allocation and each allocation is attributable or within an authoritative reserved maximum. |
| Hybrid failure/overlap | Fail PQ child and combined-vector creation after classical construction; no leaked native owner or released-too-early charge. | Peak includes child+child+combined overlap and temporary provider contexts. |
| Completion/error/final drop | Invalid peer, provider error, cancellation, HRR error, and connection drop cannot release while any native owner remains or retain uncharged state. | Final native free/owner acknowledgement releases exactly once on every terminal path. |
| Ordinary control | Owner-free ordinary API behavior, group ordering, bytes, and TLS results do not acquire a WP3 requirement or change. | Existing `start()` semantics remain byte/behavior compatible. |
| Custom provider | Unsupported group on dedicated path fails before ordinary `start`; panic/error cannot bypass or double-release custody. | A supporting custom provider demonstrates equivalent denial, attribution, overlap, and release behavior. |
| Native mechanism | Concurrent operations, initialization, thread-local work, and unrelated AWS-LC use cannot be charged to the wrong owner; max/max+1 or hook exhaustion is deterministic. | Provider mechanism supplies stable owner identity or an authoritative complete finite maximum for the exact build/reachable algorithms. |

Required controls preserve the default reachable ordering and semantics for `X25519MLKEM768`, X25519, P-256, and P-384. There is **no** PQ disable, group prune, TLS downgrade, provider swap, static/cached ephemeral key, weakened randomness, post-allocation catch-up, wrapper-`size_of` proxy, or magic whole-handshake/whole-slot reserve.

## 6. Programme effects and next action

- **#351/#356:** path-disjoint already-authorized work may continue, but KX remains stopped before `initial_key_share` / `SupportedKxGroup::start`. This document neither changes the material branch nor creates a replacement worker.
- **WP4 #329/#335:** remains frozen on terminal WP3 as already required; D supplies no terminal WP3 evidence.
- **Server Seam #247:** receives no readiness or production claim. Its dependency on truthful bounded operation ownership remains unresolved at KX.
- **Issue #439:** remains open through publication and independent review.

After this candidate is independently reviewed and protected-integrated, the exact next Work action is **provider/evidence work only**: obtain from the pinned AWS-LC/aws-lc-rs boundary either (1) a supported operation-context allocation owner/preallocation/release interface with concurrency and safety guarantees, or (2) provider-authored complete finite peak/retained byte bounds for all reachable groups and initial+HRR/hybrid overlap. Work must return that evidence for a new architecture disposition. It must not issue a material KX lease, choose C implicitly, or populate `RESOURCE_LIMITS_REGISTRY.json` from measurements or guesses.

## 7. Decision timing

- **Must decide now?** YES: WP3 cannot make an executable same-ledger KX acceptance without this boundary.
- **Blocked downstream work:** the KX portion of #351/#356, and therefore terminal WP3 evidence needed by WP4 #329/#335 and Server Seam #247.
- **Harder later:** accepting unowned native KX now would bake an accounting escape into the TLS boundary and make later denial-before-allocation impossible without API/provider changes.
- **Superseding evidence:** a supported pinned-provider owner interface, a provider-authored complete finite reservation with exact build scope, or an independently accepted finite-overhead contract backed by proof that no request/attacker-controlled KX churn escapes active-slot ownership.
- **Not decided:** numeric limits, provider implementation design, global allocator installation, provider upgrade/fork, exact future mutation lease, production concurrency/capacity, or any change to TLS/PQ/group semantics.

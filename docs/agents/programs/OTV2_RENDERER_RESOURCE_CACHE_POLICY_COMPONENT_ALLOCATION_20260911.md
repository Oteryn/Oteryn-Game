# Renderer visible-resource cache policy component allocation

- Coordinator: #162
- Resource gate: #502
- Physical evidence inputs: #505 / #515
- Protected admission main: `fc0ecb064b1d4a23a87dbba8070eb91a5142be03`

## Status and authority

```yaml
allocation_id: OTV2-RENDERER-VISIBLE-RESOURCE-CACHE-POLICY-502
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
resource_issue: 502
lane: R9_CLIENT_PRESENTATION
allocation_state: NOT_ACTIVE
worker_launch: NOT_STARTED
allocation_branch: coord/renderer-resource-cache-policy-502-allocation
branch_after_activation: agent/renderer-resource-cache-policy-502
execution_target: isolated_workspace_or_repository_native_ci
validation_target: github_actions
lane_strategy: single_agent
runtime_implementation_authority: NON_PRODUCTION_POLICY_COMPONENT_ONLY
production_resource_profile_authority: NONE
resource_registry_mutation_authority: NONE
hardware_support_policy_authority: NONE
production_configuration_authority: NONE
external_repository_write_authority: NONE
```

This is a **prospective non-production policy-component allocation**. It becomes usable only after this exact allocation is protected-integrated, protected-main readback confirms it, and the unique #162 Work Delivery Coordinator performs a fresh ownership/custody check and explicitly activates the same worker lineage.

Publication, green CI, #502 evidence, or an alias invocation does not activate the worker. This allocation does not select production `RENDER-RL-*` maxima, define the first supported Oteryn hardware floor, mutate `RESOURCE_LIMITS_REGISTRY.json`, change production renderer configuration, or grant Merge Queue authority.

## Fresh admission facts

At protected `main@fc0ecb064b1d4a23a87dbba8070eb91a5142be03`:

- protected #558 has integrated the bounded input/renderer prerequisite repair and released its branch custody;
- `crates/renderer/src/resources.rs` contains the current generation-fenced `ResourceCache<K,V>` backed by `BTreeMap<K,V>` with no capacity, retained-byte, per-cycle admission/upload, in-flight, eviction or priority accounting;
- fresh source search finds no production consumer of `ResourceCache` outside its own renderer unit tests, so policy work can remain isolated from current Windows composition;
- `crates/renderer/src/windows.rs` still owns the physical DX12 surface/device/queue path and does **not** own the presentation resource cache; this allocation therefore must not pretend that component tests are physical device-loss proof;
- protected #558 already proves stale process generation rejection before physical surface acquisition; do not reopen or duplicate that prerequisite;
- #502 remains a resource/evidence gate. Its high-end #505/#515/Molehill measurements are workload evidence only and are not production hard maxima;
- #502 still has an unresolved owner/product dependency for the first supported native-client Windows/wgpu hardware/graphics floor. That dependency blocks production numeric acceptance but does **not** block a policy component using injected finite test/evidence limits;
- #162 comment `5622366437` explicitly requests evaluation of one exact bounded non-production implementation allocation for `RENDER_VISIBLE_RESOURCE_CACHE_POLICY_COMPONENT_V1`;
- fresh open-PR/code searches find no active task, implementation PR, or competing allocation for `RENDER_VISIBLE_RESOURCE_CACHE_POLICY_COMPONENT_V1`, `crates/renderer/src/resources.rs`, or the minimum `crates/renderer/src/lib.rs` policy/error seam;
- current `ResourceCache` has no external repository consumers, so a minimal internal policy evolution can be qualified without changing Cargo/workspace or Windows composition;
- official Tibia hardware figures remain external compatibility anchors only. They are not Oteryn limits and cannot be serialized by this worker.

Changing any material fact requires fresh reconciliation before activation.

## Worker outcome

Implement the smallest production-shaped but **non-production-configured** renderer cache policy component sufficient to prove the structural resource semantics required by #502.

The component may evolve the existing renderer-local resource cache or introduce the smallest adjacent renderer-local policy type, but it must remain behind injected finite limits and must not connect those test values to production configuration.

The worker must answer only these questions:

1. Can capacity and deterministic accounted bytes be checked and reserved **before** resource-shaped construction/allocation/publication?
2. Can exact-max admission succeed and max+1 deterministically evict a legal reclaimable entry, reject, or defer without unbounded growth or partial publication?
3. Can per-cycle admission count, admission/upload bytes, in-flight uploads and eviction/replacement work be bounded with checked arithmetic?
4. Can stale or replaced renderer/process generation fail closed, and can component-owned old-generation entries/reservations/in-flight state be released/fenced without retaining unbounded old backing?
5. Can deterministic priority prevent decorative work from silently displacing pinned/critical gameplay-readability resources?
6. Can decorative miss/defer/failure produce an explicit bounded fallback/degradation result without mutating gameplay authority?
7. Can eviction and fallback order remain deterministic and independent of hash/thread enumeration order?
8. Can semantic resource identity remain independent of atlas-versus-array physical layout?
9. Can every counter/byte composition reject overflow before mutation/allocation?
10. Can all evidence be regenerated from the exact candidate without promoting injected test values to production `RENDER-RL-*` maxima?

## Exact owned paths after activation

Only the following paths become writable after protected allocation integration plus explicit #162 activation:

```text
crates/renderer/src/resources.rs
crates/renderer/src/lib.rs
docs/agents/evidence/OTV2-20260911-renderer-resource-cache-policy.json
docs/agents/evidence/OTV2-20260911-renderer-resource-cache-policy.md
docs/agents/tasks/active/OTV2-20260911-renderer-resource-cache-policy-502.md
```

Use `crates/renderer/src/resources.rs` for the implementation and inline focused tests whenever possible. `crates/renderer/src/lib.rs` is writable only for the **minimum** renderer-local public/error/policy wiring that cannot remain self-contained in `resources.rs`; unrelated surface state, Windows backend behavior and renderer contracts remain read-only.

Everything else is read-only.

In particular, this allocation grants **no write authority** to:

```text
crates/renderer/src/windows.rs
crates/renderer/Cargo.toml
Cargo.toml
Cargo.lock
apps/client/**
docs/contracts/RESOURCE_LIMITS_REGISTRY.json
docs/architecture/**
.github/**
experiments/**
vendor/**
Platform / Atlas / META / external repositories
```

If the policy component cannot be completed without any excluded path, finish every path-disjoint legal cell first and return exactly:

```text
SHARED_LEASE_REQUIRED = <exact path> :: <exact symbol/resource> :: <reason>
```

Do not seize `windows.rs`, Cargo, registry, architecture or production configuration.

## Injected finite policy limits

Use finite **test/evidence-only** limits. Exact Rust names are not frozen, but the candidate must express at least the following dimensions:

```text
max_resident_entries
max_resident_accounted_bytes
max_admissions_per_cycle
max_admission_bytes_per_cycle
max_in_flight_uploads
max_evictions_or_replacements_per_cycle
```

Additional fixed counters are allowed only if the exact candidate physically exercises them and they are required to keep the component bounded.

Every injected value must be clearly classified as test/evidence capability, not production policy. Tiny values are expected for boundary tests. No injected value may be copied into `RENDER-RL-*`, deployment configuration, user settings or public hardware requirements.

## Pre-allocation discipline

A resource-shaped value must not be constructed merely to discover that the cache is already over limit.

The candidate API must permit the caller/component to provide deterministic accounted size and admission metadata before expensive/resource-shaped construction. Acceptable shapes include a checked reservation/permit boundary or a closure/factory invoked only after successful reservation; exact API names are not frozen.

Required ordering:

```text
current generation check
-> checked count/byte arithmetic
-> capacity/per-cycle/in-flight policy check
-> deterministic eviction/reclaim decision if allowed
-> reserve bounded admission state
-> only then construct/install resource-shaped backing
-> commit exactly once OR rollback exactly
```

On any failed post-reservation step, all reservations/counters/index state must roll back exactly. No partial entry, leaked byte reservation, leaked in-flight token or priority mutation may remain.

## Deterministic priority / fallback

The candidate must distinguish at minimum:

```text
critical_or_pinned_gameplay_readability
decorative_or_degradable
```

Exact type names are not frozen.

Rules:

- decorative admission must not evict a pinned/critical entry merely to make room;
- a critical admission may reclaim only entries allowed by the explicit deterministic policy;
- eviction tie-breaking must be stable and independent of hash/thread iteration order;
- a missing/deferred decorative resource returns an explicit bounded fallback/defer/degradation result;
- cache pressure or fallback must never change server-authoritative visibility, collision, damage, hit/AoE/timing, movement or world state;
- no whole-world preload path may be introduced.

This allocation does not freeze the final production priority taxonomy. It proves only the minimum policy invariant needed by #502.

## Generation / recovery policy evidence

The component must preserve existing stale-generation fencing and add only the minimum component-owned release semantics necessary to prove that old-generation cache state cannot survive indefinitely.

For an injected legitimate generation transition/recovery event, prove:

- prior-generation entries cannot be installed or resurrected;
- prior-generation reservations/in-flight permits cannot commit;
- component-owned old-generation entries/accounted bytes/in-flight counters are released or fenced deterministically;
- release/replacement work is finite and accounted against already finite component state;
- repeated recovery cannot accumulate retained old-generation backing.

This is **component-level resource-lifetime proof only**. Because `WindowsRenderer` does not own this cache, do not claim physical DX12 device-loss/recreation resource release, GPU memory release, or Tier-3 recovery. If physical composition becomes necessary, stop at the excluded `windows.rs` boundary and return `SHARED_LEASE_REQUIRED`.

## Layout neutrality

Cache identity must be based on stable renderer resource identity/revision/generation facts required by the component, not on atlas page numbers, array layer indexes, GPU pointers, backend handles or experiment-specific sheet layout.

Atlas, texture-array or later physical layout choices may remain behind adapters/callers. This worker must not decide atlas-versus-array, KTX2-versus-DDS, filtering/mipmap policy, batching thresholds, particle backend or final VFX/light limits.

## Mandatory correctness/resource matrix

The exact candidate and evidence must prove at least:

1. exact current-generation admission below all limits succeeds;
2. stale generation rejects before resource construction/publication and leaves all cache state unchanged;
3. resident-entry exact max succeeds; max+1 deterministically evicts one legal reclaimable entry or rejects/defers before unbounded growth;
4. resident-accounted-byte exact max succeeds; max+1 byte request rejects/evicts/defers before construction with no overflow or leaked reservation;
5. per-cycle admission-count max/max+1 is bounded and reset only by the explicit normalized cycle boundary;
6. per-cycle admission/upload-byte max/max+1 is bounded with checked arithmetic;
7. in-flight upload exact max succeeds and max+1 rejects/defers before creating extra backing; completion/abort releases exactly once;
8. eviction/replacement exact max succeeds and max+1 rejects/defers without silently exceeding work budget;
9. combined limits cannot be bypassed by satisfying each dimension individually in a sequence that exceeds another dimension;
10. checked arithmetic overflow for counts/bytes rejects before mutation or construction;
11. post-reservation construction/install failure rolls back entry/index, bytes, cycle counters, in-flight reservations and priority state exactly;
12. decorative admission never evicts pinned/critical content when policy forbids it;
13. deterministic tie cases always choose the same legal victim/result independent of insertion/hash/thread enumeration order;
14. decorative miss/defer/failure returns the explicit bounded fallback/degradation result and never mutates gameplay authority;
15. legitimate generation transition fences all old references/permits, prevents old-generation install/resurrection and leaves no independently growing old-generation retention;
16. repeated generation transitions/recovery cycles keep retained state bounded by the injected finite component limits;
17. semantic cache identity is unchanged when an equivalent resource is represented through different prototype physical-layout metadata;
18. no whole-world preload, visibility scan, gameplay mutation, production configuration lookup or registry lookup is reachable from the component;
19. evidence reports exact retained entry count, deterministic accounted bytes, per-cycle admission count/bytes, in-flight count, eviction/replacement work and generation-release work for every exercised boundary;
20. all injected finite values are explicitly reported as `test_evidence_only = true` and `production_maximum_selected = false`.

## Evidence contract

The worker must generate both machine-readable and human-readable evidence under the owned evidence paths.

At minimum record:

```text
exact_head_sha
protected_admission_main_sha
candidate_type_shape
resource_identity_shape
injected_limits
resident_entries
resident_accounted_bytes
admissions_this_cycle
admission_bytes_this_cycle
in_flight_uploads
evictions_or_replacements_this_cycle
generation_release_work
max_max_plus_1_matrix
rollback_matrix
priority_fallback_matrix
generation_recovery_matrix
checked_overflow_matrix
layout_neutrality_matrix
test_evidence_only = true
hardware_floor_selected = false
production_maximum_selected = false
resource_registry_mutated = false
production_configuration_mutated = false
```

Do not report guessed VRAM as measured bytes. Deterministic accounted decoded/uploadable bytes are allowed when known before allocation. Process RAM observations may be retained as evidence only if measured truthfully and clearly separated from component accounting.

## Relationship to production #502 acceptance

Successful completion of this worker proves only the structural policy component.

Production `RENDER_VISIBLE_RESOURCE_CACHE_V1` still requires later owner/resource work, including:

- accepted first supported native-client Windows/wgpu hardware/graphics floor;
- representative lower-floor physical qualification in addition to high-end evidence;
- production-shaped per-cycle peaks and physical composition evidence where required;
- evidence-backed finite `RENDER-RL-*` dispositions;
- separately controlled `RESOURCE_LIMITS_REGISTRY.json` serialization;
- fresh #162 production implementation allocation.

No production numeric value is selected here.

## Validation

For this allocation PR:

- `python tools/agents/validate_governance.py`;
- `git diff --check`;
- full effective-diff adversarial self-review;
- canonical exact-head repository checks / `game-gate`;
- runtime E2E: `NOT_APPLICABLE` because this allocation changes documentation only.

For the later worker after activation:

- focused renderer cache-policy tests in `crates/renderer/src/resources.rs`;
- `cargo fmt --check` for affected Rust;
- `cargo clippy --locked -p oteryn-renderer --all-targets -- -D warnings` or the exact repository-selected equivalent after live readback;
- `cargo test --locked -p oteryn-renderer` or the exact repository-selected equivalent after live readback;
- deterministic evidence regeneration/readback;
- `python tools/agents/validate_governance.py`;
- `git diff --check`;
- all repository-native exact-head checks selected by the changed paths.

If the protected build/test matrix selects different exact commands, follow the live protected matrix instead of this historical spelling.

## Independent review / integration

This allocation grants future write authority to a shared renderer-resource ownership/capacity seam after activation, so it requires a genuinely independent exact-head review before integration.

The currently active #162 standing bounded owner authorization may fund the applicable independent review and exact-candidate native Merge Queue transition only after fresh qualification. It does not authorize direct merge, generic auto-merge, protection bypass, force-push, skipped checks, production deployment or cross-repository writes.

If the connected surface does not expose the repository-required native `merge-async` Merge Queue primitive, preserve the exact candidate and report `BLOCKED_CAPABILITY_UNAVAILABLE`; do not substitute another merge primitive. Continue safe path-disjoint coordinator work.

## Expected worker return

```yaml
issue: 502
worker_task_id: OTV2-20260911-renderer-resource-cache-policy-502
admission_main_sha: <protected allocation merge SHA>
branch: agent/renderer-resource-cache-policy-502
head_sha: <exact head>
changed_paths: []
candidate_shape: <exact Rust policy/cache shape>
uses_injected_test_evidence_limits: true
preallocation_reservation_ordering: <PASS|BLOCKED>
resident_entry_boundary: <PASS|BLOCKED>
resident_byte_boundary: <PASS|BLOCKED>
per_cycle_admission_boundary: <PASS|BLOCKED>
per_cycle_byte_boundary: <PASS|BLOCKED>
in_flight_boundary: <PASS|BLOCKED>
eviction_replacement_boundary: <PASS|BLOCKED>
post_reservation_rollback: <PASS|BLOCKED>
priority_fallback_policy: <PASS|BLOCKED>
deterministic_eviction_order: <PASS|BLOCKED>
generation_component_release: <PASS|BLOCKED>
layout_neutral_identity: <PASS|BLOCKED>
checked_arithmetic: <PASS|BLOCKED>
evidence_refs: []
hardware_floor_selected: false
production_maximum_selected: false
resource_registry_mutated: false
production_configuration_mutated: false
focused_validation: []
self_review: <result>
blocker: <one precise blocker or null>
recommended_next_action: <one next coordinator action>
```

## Completion

The allocation itself is complete only after protected integration/readback. The future worker is complete only when the bounded component/evidence matrix is green without claiming production resource acceptance.

`IMPLEMENTATION_AUTHORITY: NONE_UNTIL_PROTECTED_INTEGRATION_AND_EXPLICIT_162_ACTIVATION`
`PRODUCTION_AUTHORITY: NONE`
`PRODUCTION_NUMERIC_MAXIMA_AUTHORITY: NONE`
`RESOURCE_REGISTRY_MUTATION_AUTHORITY: NONE`
`WINDOWS_RENDERER_COMPOSITION_AUTHORITY: NONE`
`HARDWARE_FLOOR_DECISION_AUTHORITY: NONE`
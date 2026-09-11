# Renderer visible-resource cache policy component allocation

- Coordinator: #162
- Resource gate: #502
- Physical evidence inputs: #505 / #515
- Protected admission main: `4122dc7302dfbb2e05f30ddba95017f464773b8c`

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

At protected `main@4122dc7302dfbb2e05f30ddba95017f464773b8c`:

- protected #558 integrated the bounded input/renderer prerequisite repair and released its branch custody;
- protected #536 added only WP3 prompt/runbook lifecycle material and is path-disjoint from this renderer allocation;
- `crates/renderer/src/resources.rs` contains the current generation-fenced `ResourceCache<K,V>` backed by `BTreeMap<K,V>` with no capacity, retained-byte, per-cycle admission/upload, in-flight, eviction or priority accounting;
- fresh source search finds no production consumer of `ResourceCache` outside its own renderer unit tests, so policy work can remain isolated from current Windows composition;
- `crates/renderer/src/windows.rs` owns the physical DX12 surface/device/queue path and does **not** own the presentation resource cache; component tests therefore cannot be relabelled as physical device-loss proof;
- protected #558 already proves stale process-generation rejection before physical surface acquisition; do not reopen or duplicate that prerequisite;
- #502 remains a resource/evidence gate. Its #505/#515/Molehill measurements are workload evidence only, not production hard maxima;
- the first supported native-client Windows/wgpu hardware/graphics floor remains an unresolved owner/product dependency. That blocks production numeric acceptance but does **not** block injected finite test/evidence limits;
- #162 comment `5622366437` explicitly requests evaluation of one exact bounded non-production implementation allocation for `RENDER_VISIBLE_RESOURCE_CACHE_POLICY_COMPONENT_V1`;
- fresh open-PR/code searches find no active task, implementation PR, or competing allocation for that component, `crates/renderer/src/resources.rs`, or the minimum `crates/renderer/src/lib.rs` policy/error seam;
- current `ResourceCache` has no external repository consumers, so a minimal renderer-local policy evolution can be qualified without Cargo/workspace or Windows-composition changes;
- official Tibia hardware figures remain external compatibility anchors only. They are not Oteryn limits and cannot be serialized by this worker.

Changing any material fact requires fresh reconciliation before activation.

## Worker outcome

Implement the smallest production-shaped but **non-production-configured** renderer cache policy component sufficient to prove the structural resource semantics required by #502.

The component may evolve the existing renderer-local resource cache or introduce the smallest adjacent renderer-local policy type, but it must remain behind injected finite limits and must not connect those values to production configuration.

The worker must answer only:

1. Can capacity and deterministic accounted bytes be checked and reserved **before** resource-shaped construction/allocation/publication?
2. Can exact-max admission succeed and max+1 deterministically evict a legal reclaimable entry, reject, or defer without unbounded growth or partial publication?
3. Can committed admissions, admission/upload bytes, in-flight uploads, eviction/replacement work **and failed/retried admission attempts/work** be bounded per cycle with checked arithmetic so failure cannot restore the caller's work budget and enable unlimited retries?
4. Can stale/replaced generation fail closed, and can component-owned old-generation entries/reservations/in-flight state be released or fenced without unbounded retained backing?
5. Can deterministic priority prevent decorative work from silently displacing pinned/critical gameplay-readability resources?
6. Can decorative miss/defer/failure produce an explicit bounded fallback/degradation result without gameplay mutation?
7. Can eviction/fallback order remain deterministic and independent of hash/thread enumeration order?
8. Can semantic resource identity remain independent of atlas-versus-array physical layout?
9. Can every counter/byte/work-unit composition reject overflow before mutation/allocation?
10. Can exact evidence be regenerated without promoting injected values to production `RENDER-RL-*` maxima?

## Exact owned paths after activation

Only these paths become writable after protected allocation integration plus explicit #162 activation:

```text
crates/renderer/src/resources.rs
crates/renderer/src/lib.rs
docs/agents/evidence/OTV2-20260911-renderer-resource-cache-policy.json
docs/agents/evidence/OTV2-20260911-renderer-resource-cache-policy.md
docs/agents/tasks/active/OTV2-20260911-renderer-resource-cache-policy-502.md
```

Use `resources.rs` for implementation and inline focused tests whenever possible. `lib.rs` is writable only for the **minimum** renderer-local public/error/policy wiring that cannot remain self-contained in `resources.rs`; unrelated surface state and renderer contracts remain read-only.

Everything else is read-only. In particular, after activation there is **no** write authority to:

```text
crates/renderer/src/windows.rs
crates/renderer/Cargo.toml
Cargo.toml
Cargo.lock
apps/client/**
docs/contracts/RESOURCE_LIMITS_REGISTRY.json
docs/architecture/**
docs/agents/programs/**
.github/**
experiments/**
vendor/**
Platform / Atlas / META / external repositories
```

This allocation document is writable only during this pre-integration preparation/review PR. Once protected-integrated, the worker may read it but may not rewrite its own authority.

If any excluded path becomes materially necessary, finish every path-disjoint legal cell first and return exactly:

```text
SHARED_LEASE_REQUIRED = <exact path> :: <exact symbol/resource> :: <reason>
```

Do not seize `windows.rs`, Cargo, registry, architecture or production configuration.

## Injected finite policy limits

Use finite **test/evidence-only** limits. Exact Rust names are not frozen, but the exact candidate must express at least:

```text
max_resident_entries
max_resident_accounted_bytes
max_admissions_per_cycle
max_admission_bytes_per_cycle
max_in_flight_uploads
max_evictions_or_replacements_per_cycle
max_admission_attempts_per_cycle
max_admission_attempt_work_units_per_cycle
```

`max_admissions_per_cycle` may count committed admissions, but it is **not** allowed to be the only retry bound. Every attempt that passes preflight far enough to perform reclaim-plan evaluation, reservation, resource-shaped construction or installation must consume the separate finite attempt/work budget even if the operation later fails and retained state rolls back.

The work-unit definition must be deterministic for the exact candidate and must reserve/charge a candidate-specific bounded upper amount before the corresponding reclaim/construction/install work is performed. An attempt/work charge is monotonic within the normalized cycle and is not refunded after later construction/install failure. The explicit cycle boundary may reset it.

Additional fixed counters are allowed only if the candidate physically exercises them and they are required to keep the component bounded.

Every injected value is test/evidence capability, not production policy. Tiny values are expected for boundary tests. No injected value may be copied into `RENDER-RL-*`, deployment configuration, user settings or public hardware requirements.

## Pre-allocation discipline

A resource-shaped value must not be constructed merely to discover that the cache is already over limit. The API must accept deterministic accounted size/admission metadata before expensive/resource-shaped construction, for example through a checked reservation/permit boundary or a factory invoked only after reservation.

Required ordering:

```text
current generation check
-> checked count/byte/work arithmetic
-> capacity/per-cycle/in-flight policy check
-> checked finite admission-attempt/work charge or reservation
-> deterministic reclaim-plan decision within the charged work allowance
-> reserve bounded retained/in-flight admission state
-> only then construct/install resource-shaped backing
-> commit retained state exactly once OR rollback retained state exactly
-> preserve the consumed per-cycle attempt/work charge even on later failure
```

A failed post-reservation step must roll back retained entry/index state, retained accounted bytes, in-flight reservations and priority/publication state exactly. It must **not** refund the already consumed admission-attempt/work charge. If a committed-admission or committed-byte counter is defined to count only successful publication, that counter may roll back; retry safety is supplied independently by the non-refundable finite attempt/work budget.

No partial entry, leaked retained-byte reservation, leaked in-flight token or priority mutation may remain. Conversely, repeated failed construction/install attempts must eventually hit the finite attempt/work boundary and reject/defer **before** another resource-shaped construction or reclaim evaluation can run.

## Deterministic priority / fallback

The candidate must distinguish at minimum:

```text
critical_or_pinned_gameplay_readability
decorative_or_degradable
```

Exact type names are not frozen.

- decorative admission must not evict pinned/critical content when policy forbids it;
- critical admission may reclaim only entries allowed by the explicit deterministic policy;
- eviction tie-breaking must be stable and independent of hash/thread enumeration order;
- decorative miss/defer/failure returns an explicit bounded fallback/degradation result;
- pressure/fallback never changes server-authoritative visibility, collision, damage, hit/AoE/timing, movement or world state;
- no whole-world preload path may be introduced.

This does not freeze the final production priority taxonomy.

## Generation / recovery evidence

Preserve existing stale-generation fencing and add only minimum component-owned release semantics needed to show that old-generation cache state cannot accumulate indefinitely.

For an injected legitimate generation transition/recovery event, prove:

- prior-generation entries cannot install or resurrect;
- prior-generation reservations/in-flight permits cannot commit;
- component-owned old-generation entries/accounted bytes/in-flight counters are released or fenced deterministically;
- release/replacement work is finite and accounted against already finite state;
- repeated recovery cannot accumulate old-generation backing.

This is **component-level resource-lifetime proof only**. Because `WindowsRenderer` does not own this cache, do not claim physical DX12 device-loss/recreation resource release, GPU memory release, or Tier-3 recovery. If physical composition is necessary, stop at excluded `windows.rs` and return `SHARED_LEASE_REQUIRED`.

## Layout neutrality

Cache identity must use stable renderer resource identity/revision/generation facts, not atlas page numbers, array-layer indexes, GPU pointers, backend handles or experiment-specific sheet layout.

Atlas/array physical organization may remain behind adapters/callers. This worker must not decide atlas-versus-array, KTX2-versus-DDS, filtering/mipmap policy, batching thresholds, particle backend or final VFX/light limits.

## Mandatory correctness/resource matrix

The exact candidate and evidence must prove at least:

1. exact current-generation admission below all limits succeeds;
2. stale generation rejects before resource construction/publication and leaves all state unchanged;
3. resident-entry exact max succeeds; max+1 deterministically evicts one legal reclaimable entry or rejects/defers before unbounded growth;
4. resident-accounted-byte exact max succeeds; max+1 rejects/evicts/defers before construction with no overflow/leak;
5. committed per-cycle admission-count max/max+1 is bounded and resets only at the explicit normalized cycle boundary;
6. committed per-cycle admission/upload-byte max/max+1 is bounded with checked arithmetic;
7. in-flight exact max succeeds and max+1 rejects/defers before extra backing; completion/abort releases exactly once;
8. eviction/replacement exact max succeeds and max+1 rejects/defers without exceeding work budget;
9. admission-attempt count exact max succeeds and max+1 rejects/defer **before** another resource-shaped construction/reclaim evaluation; failed attempts consume this budget and do not refund it;
10. admission-attempt work-unit exact max succeeds and max+1 rejects/defer before work; candidate-specific worst-case reclaim/construction/install allowance is checked/charged before work and remains consumed after later failure;
11. combined limits cannot be bypassed by a sequence that satisfies individual retained/success counters while repeatedly failing construction or exhausting another work dimension;
12. checked arithmetic overflow for count/byte/work counters rejects before mutation/construction;
13. post-reservation construction/install failure rolls back retained entry/index, retained bytes, in-flight reservations and priority/publication state exactly **while preserving the consumed attempt/work charge**;
14. repeated injected construction/install failures in one normalized cycle eventually hit the finite attempt/work limit, and the first over-limit retry performs zero additional resource construction/reclaim work;
15. explicit cycle reset clears the per-cycle attempt/work charge exactly once and no other path silently refreshes it;
16. decorative admission never evicts pinned/critical content when forbidden;
17. deterministic tie cases choose the same legal victim/result independent of insertion/hash/thread enumeration order;
18. decorative miss/defer/failure returns explicit bounded fallback/degradation and never mutates gameplay authority;
19. legitimate generation transition fences old references/permits, prevents old-generation install/resurrection and leaves no independently growing old-generation retention;
20. repeated generation/recovery cycles keep retained state bounded by injected finite limits;
21. semantic cache identity is unchanged when equivalent content uses different prototype physical-layout metadata;
22. no whole-world preload, visibility scan, gameplay mutation, production-config lookup or registry lookup is reachable;
23. evidence reports exact retained entries, deterministic accounted bytes, committed per-cycle admission count/bytes, admission attempts/work units, failed-attempt count, in-flight count, eviction/replacement work and generation-release work for every exercised boundary;
24. all injected values report `test_evidence_only = true` and `production_maximum_selected = false`.

If any operation cost depends on occupancy, queue shape or reclaimable-set shape, evidence must measure the relevant candidate-specific boundary/worst-case path rather than only one representative point. Failed attempts must be included in those work measurements; they cannot disappear because retained state rolled back.

## Evidence contract

Generate machine-readable and human-readable evidence under the owned evidence paths. At minimum record:

```text
exact_head_sha
protected_admission_main_sha
candidate_type_shape
resource_identity_shape
injected_limits
resident_entries
resident_accounted_bytes
committed_admissions_this_cycle
committed_admission_bytes_this_cycle
admission_attempts_this_cycle
admission_attempt_work_units_this_cycle
failed_construction_or_install_attempts_this_cycle
in_flight_uploads
evictions_or_replacements_this_cycle
generation_release_work
occupancy_or_queue_dependent_work_boundaries
max_max_plus_1_matrix
failed_attempt_budget_matrix
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

Do not report guessed VRAM as measured bytes. Deterministic decoded/uploadable-byte accounting is allowed when known before allocation. Process RAM observations, if retained, must be clearly separated from component accounting.

## Relationship to production #502 acceptance

Successful completion proves only the structural policy component. Production `RENDER_VISIBLE_RESOURCE_CACHE_V1` still requires later owner/resource work including:

- accepted first supported native-client Windows/wgpu hardware/graphics floor;
- representative lower-floor physical qualification plus high-end evidence;
- production-shaped per-cycle peaks and physical composition evidence where required;
- evidence-backed finite `RENDER-RL-*` dispositions;
- separately controlled registry serialization;
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

- focused renderer cache-policy tests in `resources.rs`;
- `cargo fmt --check` for affected Rust;
- `cargo clippy --locked -p oteryn-renderer --all-targets -- -D warnings` or the exact repository-selected equivalent after live readback;
- `cargo test --locked -p oteryn-renderer` or the exact repository-selected equivalent after live readback;
- deterministic evidence regeneration/readback;
- `python tools/agents/validate_governance.py`;
- `git diff --check`;
- all repository-native exact-head checks selected by changed paths.

If the protected build/test matrix selects different exact commands, follow the live protected matrix instead of this historical spelling.

## Independent review / integration

This allocation grants future write authority to a shared renderer-resource ownership/capacity seam after activation, so it requires a genuinely independent exact-head review before integration.

The active #162 standing bounded owner authorization may fund applicable review and an exact-candidate native Merge Queue transition only after fresh qualification. It does not authorize direct merge, generic auto-merge, protection bypass, force-push, skipped checks, production deployment or cross-repository writes.

If the connected surface does not expose repository-required native `merge-async`, preserve the candidate and report `BLOCKED_CAPABILITY_UNAVAILABLE`; do not substitute another merge primitive. Continue safe path-disjoint coordinator work.

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
admission_attempt_boundary: <PASS|BLOCKED>
admission_attempt_work_boundary: <PASS|BLOCKED>
failed_attempt_budget_non_refund: <PASS|BLOCKED>
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
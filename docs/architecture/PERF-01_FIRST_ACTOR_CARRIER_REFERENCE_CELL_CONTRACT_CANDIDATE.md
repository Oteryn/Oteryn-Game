# PERF-01 first actor-carrier reference cell contract candidate

- Decision: `PERF-01-ACTOR-CARRIER-CELL-V1`
- Status: **CANDIDATE; acceptance requires independent exact-head review and protected integration**
- Date: 2026-09-10
- Source escalation: Issue #540; owner decision comment `5620646993`
- Decision base: `main@72c997b3f71c180437f6b40e2c5e2abbe0f45273`
- Affected lane: Issue #162 / `FND03_RUNTIME_ACTOR_CARRIER_PERF`
- Runtime, registry, workflow, production, hardware-provisioning and external-repository authority: **NONE**
- `MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`

## Resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
main_sha: 72c997b3f71c180437f6b40e2c5e2abbe0f45273
source_escalation: 540
blocking_question: What is the smallest accepted PERF-01 reference-cell contract that can evidence a finite total authoritative actors-per-Channel ceiling for CHANNEL_RUNTIME_ACTOR_CARRIER_V1?
facts:
  proven:
    - ADR-0009 requires named reference hardware, exact release artifacts, declared objectives, measured saturation and at least 30 percent initial headroom
    - the existing self-hosted route is game-runners / oteryn-game with persistent runner name oteryn-synology-game and Linux identity checks
    - owner decision 5620646993 supplies the bounded cell, objectives and first carrier-only workload
    - protected PR 537 classifies RUNTIME-ACTOR-RL-01 as PERF_REFERENCE_CELL_REQUIRED and its correctness populations as non-capacity evidence
  derived:
    - a native Linux process can qualify only after the job proves its own unprivileged affinity and address-space controls on the selected runner
    - this carrier-only cell can produce a provisional RL-01 candidate but cannot establish players-per-Channel or broader product capacity
  unknown:
    - sanitized exact physical fingerprint and whether the runner permits every required unprivileged enforcement preflight
    - measured saturation population and the resulting candidate admission M
    - all broader representative PERF-01 limits and production deployment capacity
  conflict: []
accepted_decision: PERF-01-ACTOR-CARRIER-CELL-V1 measurement contract defined below
rejected_options: [github_hosted_capacity_evidence, container_first_cell, unenforced_best_effort_limits, synthetic_or_AI01_capacity_reuse, production_SLA_interpretation]
affected_contracts: [PERF-01, FND-03, CHANNEL_RUNTIME_ACTOR_CARRIER_V1, RUNTIME-ACTOR-RL-01]
affected_paths: [docs/architecture/PERF-01_FIRST_ACTOR_CARRIER_REFERENCE_CELL_CONTRACT_CANDIDATE.md]
implementation_owner: Issue 162 under a fresh exact-path allocation after this candidate is accepted
implementation_scope: non-production cell manifest, fingerprint and benchmark/evidence tooling only
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
supersedes: []
required_validation: exact-cell preflight, at least five repetitions per population, saturation reproduction, 30-minute highest-pass soak, raw evidence validation, M and M-plus-one fail-closed proof
required_independent_review: Issue 162 exact-head architecture review plus repository CI and Merge Queue
next_action: Issue 162 independently reviews and qualifies the exact candidate head for protected integration without this author merging or enqueueing it.
```

## 1. Problem, constraints and decision timing

`CHANNEL_RUNTIME_ACTOR_CARRIER_V1` needs a finite total authoritative active-actor admission resource before `RUNTIME-ACTOR-RL-01` can be serialized. The existing one-actor, two-actor and three-kind fixtures prove correctness only. They neither select nor evidence capacity, and `AI01-ACTIVE-ACTORS=256` belongs to a different resource.

**Must decide now: YES.** `RUNTIME-ACTOR-RL-01` acceptance blocks registry serialization, the shared Channel actor carrier, #508 Phase A, and then #139's first Movement child. Guessing a limit now would couple admission to unrepresentative hardware or empty-container work and cause a resource-contract migration later. This contract therefore freezes only a reproducible first measurement cell and method; it does not freeze `M`.

The first cell intentionally measures the fixed carrier slice, not full gameplay. Representative Movement, hunting, AI, combat, loot, pathfinding, crowded-interest, raid, login/reconnect, durability/persistence pressure, multi-Channel noisy-neighbor, recovery and longer product soaks remain **OPEN** under broader `PERF-01`. Accepted physical evidence from those workloads may lower or supersede the first-slice limit through a later reviewed decision.

## 2. Named physical reference cell

The only cell selected by this contract is the existing organization self-hosted runner:

```text
runner group: game-runners
required label: oteryn-game
persistent runner name: oteryn-synology-game
OS family: Linux
execution mode: native release-profile process
container: none
```

This is a measurement identity, not a production host class, purchase, provisioning instruction or deployment topology. GitHub-hosted runners and a different self-hosted runner are ineligible even when faster or nominally equivalent.

Before an authoritative sample, the job must create a canonical, sanitized cell manifest containing:

- runner group, required label, persistent name and OS family;
- CPU vendor and model, architecture, online logical CPU identifiers, socket/core/thread topology and the selected logical CPU;
- SMT state, NUMA topology and, when readable without privilege, frequency policy, governor and turbo/boost state;
- total physical RAM;
- OS distribution/version, kernel release and architecture;
- effective process/cgroup CPU and memory limits, `RLIMIT_AS`, allowed affinity mask and page size;
- whether host swap exists, plus the measured process swap value;
- source commit, lockfile digest, compiler/toolchain, release artifact digest and benchmark manifest digest.

The manifest must omit hostnames beyond the approved runner name, serial numbers, network addresses, mount paths containing identities, credentials, environment secrets and raw `/proc` or runner-registration contents. It is encoded in a deterministic field order and hashed. The first independently accepted physical run freezes both the sanitized manifest and its digest. Later evidence is comparable only on exact field equality and digest equality; missing fields, newly unreadable fields or fingerprint drift invalidate the run and cannot update the accepted result. Deliberate cell revision requires a later reviewed PERF-01 decision, never an in-place overwrite.

## 3. Enforceable native process boundary

The harness launcher must fail closed before warm-up unless it can prove all of the following as the unprivileged runner account:

1. **One deterministic logical CPU.** Select the lowest-numbered CPU in the job's effective allowed affinity set, bind the benchmark process to that CPU, and read back an affinity mask containing exactly that CPU. Selection and readback are evidence. No second logical CPU is permitted to the process.
2. **One-GiB address-space ceiling.** Before executing the release artifact, set both soft and hard `RLIMIT_AS` no higher than `1,073,741,824` bytes and read the limits back in the benchmark process. A cgroup limit may be stricter but cannot replace proof of the address-space ceiling. The process must not raise or escape the limit.
3. **Zero measured swap.** Sample process swap from Linux process accounting during every measured run and soak. Every observation, including the final observation, must be zero. Missing or unreadable process-swap accounting invalidates the sample. Host swap may exist.
4. **One exclusive benchmark job.** The persistent runner must assign only this one benchmark job while the cell is active. The evidence binds the workflow run/job identity and proves no concurrent instance of this benchmark. Background host activity is recorded; a contention or reproducibility failure invalidates the sample rather than relaxing an objective.
5. **Native execution.** The measured artifact runs directly, without Docker or another container boundary and without Docker-host control.

Affinity and `RLIMIT_AS` are process-start invariants, not advisory observations. If the runner account cannot set and read them, if exact one-CPU affinity is unavailable, or if exclusivity/zero-swap evidence cannot be established, the only valid result is:

```text
REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE
```

No sample, degraded cell, wider limit or best-effort substitute may be reported as authoritative. This candidate does not authorize a workflow change, privilege grant or runner mutation to overcome that result.

## 4. Exact artifact and scenario binding

Every evidence set binds one exact source commit, `Cargo.lock`, Rust toolchain, release-profile artifact digest, benchmark executable/config digest, `protocol-oteryn` revision, and applicable world/content/ruleset revisions. The benchmark uses one `WorldId + ChannelId` authoritative ChannelRuntime with one session-generation-fenced logical mutation owner. IDs and seeds are fixed in the scenario manifest. A changed input is a new evidence set and cannot be pooled with an old one.

The 50 ms interval below is a **benchmark work window only**. It is not a universal FND-03 runtime tick, production scheduling promise or player SLA.

## 5. Deterministic carrier-only workload

At total population `N`, actors occupy fixed carrier records in repeating `PLAYER, CREATURE, NPC_SYSTEM` order. Kind changes no carrier shape, and the count difference between any two kinds is at most one. Each 50 ms work window performs, in a fixed seed-derived order:

1. exactly one direct exact-current-reference lookup per active actor, with exactly one actor examined and no scan or candidate collection;
2. one fixed-shape local-position mutation proxy for a deterministic rotating 10% subset;
3. remove plus same-slot reuse with checked generation advancement for a disjoint deterministic rotating 1% subset;
4. verification that pre-removal references are stale and cannot resolve after reuse; and
5. admission/removal boundary probes at each progressive population.

Integer percentage counts use a deterministic rational accumulator across windows: window `w` performs `floor((w + 1) × N / 10) - floor(w × N / 10)` position mutations and `floor((w + 1) × N / 100) - floor(w × N / 100)` remove/reuse operations. Thus small correctness populations may perform zero of an operation in one window but receive exactly the declared 10% and 1% rates over complete accumulator cycles without rounding inflation. The manifest records the window index and exact counts. Rotation must cover every eligible actor before repeating and remain identical for the same seed and population. Remove/reuse does not change steady-state `N`. Variable behavior payload, combat, AI, inventory, pathfinding, loot, network and persistence work is excluded rather than represented by a hidden stub cost.

The run begins at the accepted correctness lower bounds and advances through a manifest-declared geometric or fixed step sequence. It must not use 256 or another existing domain limit as an authority or privileged breakpoint. Steps continue until a reproducible saturation population is found; skipping an inconclusive lower population to claim a higher result is forbidden.

## 6. Objectives and observations

Each valid steady-state repetition reports raw observations and recomputable summaries for normalized actor operations, owner queue age and resources. The declared objectives are:

| Dimension | First-cell objective |
|---|---:|
| normalized actor-operation latency p50 | `<= 10 ms` |
| normalized actor-operation latency p95 | `<= 25 ms` |
| normalized actor-operation latency p99 | `<= 50 ms` |
| owner-queue age p99 | `<= 25 ms` |
| selected logical CPU sustained utilization | `<= 90%` |
| process RSS | `<= 858,993,459 bytes` (80% of 1 GiB) |
| measured process swap | exactly `0 bytes` |
| post-warm-up steady-state memory growth during the 30-minute soak | `<= 10,737,418 bytes` (1% of 1 GiB, rounded down) |
| unexpected rejection or degradation below candidate `M` | exactly `0` |

Latency is measured from operation eligibility at its assigned window through completion and normalized per declared actor operation; queue age is eligibility-to-owner-start. The report retains enough bucket/count or raw data to recompute p50/p95/p99 with one documented nearest-rank percentile rule. It also records attempted/completed operations, expected boundary rejections, unexpected rejections, stale-reference outcomes, missed/overrun windows, selected-CPU time and utilization, RSS, peak RSS, swap, and memory-growth endpoints. Expected capacity and stale-reference rejections are classified separately and never counted as successful work.

Warm-up duration and steady-state measured-window count must be fixed in the versioned scenario manifest before the first population run and applied unchanged to every population. Warm-up observations do not enter percentiles. Changing warm-up or measurement length after viewing results starts a new evidence set.

## 7. Repetition, saturation and soak

Run at least five clean, independent repetitions at every tested population. Reset the process and deterministic fixture between repetitions. A population **passes** only when every valid repetition meets every declared objective. Saturation is the smallest tested total population where the same declared objective is violated in every one of at least five valid repetitions.

A mixture of passes and failures, changing first-failure objectives, invalid samples or excessive environmental variation is **inconclusive**, not passing or saturation. Repeat under the same frozen cell until reproducible or return no capacity result. The first reproducibly failing population is retained even if later populations happen to pass.

After finding the first reproducible saturation, run a continuous 30-minute soak at the highest lower tested population that passed every objective in all repetitions. The soak must also meet every latency, queue, CPU, RSS, zero-swap, rejection and memory-growth objective. If it fails, that population is not passing: step downward, repeat the full qualification there, and soak the new highest passing population. A valid result requires both a reproducible saturation and a passing soak; this contract does not permit extrapolating saturation from a last passing value.

## 8. Deriving and validating candidate admission `M`

Let `S` be the measured saturation population accepted by the rules above, and let `P` be the highest lower tested population that passed every objective in all repetitions and in the required 30-minute soak. The first RL-01 candidate is:

```text
M <= floor(0.70 × S)
M <= P
M <= every other accepted coupled resource limit
```

Use checked integer arithmetic. The qualified passing-population cap prevents a coarse progressive step from placing `M` above directly passing and soaked evidence. The evidence packet identifies the selected lower bound and its owner. `M` is total authoritative active actor slots across all three kinds, not players, creatures or AI actors alone. This document selects no numeric `S`, `P` or `M` and does not mutate `RESOURCE_LIMITS_REGISTRY.json`.

Before `M` can be proposed for registry acceptance, an exact configured-maximum run must prove:

- admission through total occupancy `M` succeeds without unexpected rejection;
- the next admission at `M + 1` rejects before partial actor/index mutation;
- no live actor is evicted or silently recycled;
- checked-generation reuse advances generation and every stale reference remains rejected;
- existing authoritative state remains intact after rejection; and
- arithmetic overflow or an unrepresentable allocation rejects before allocation.

These are admission-boundary proofs, not permission for `M + 1` active actors. Failure lowers or rejects the candidate; it never widens the cell or changes semantics.

## 9. Evidence acceptance and remaining state

The retained evidence packet must include the sanitized frozen fingerprint, all exact input digests, preflight/readback, population plan, seeds, per-repetition summaries, recomputable percentile data, invalid/inconclusive samples with reasons, first violated objective, saturation proof, soak series, derivation arithmetic and `M`/`M + 1` results. An independent reviewer must verify the exact artifact head, fingerprint equality, objective calculations and fail-closed outcomes.

Until that physical evidence is accepted and a separate resource decision serializes a finite value:

```text
RUNTIME-ACTOR-RL-01 = PERF_REFERENCE_CELL_REQUIRED
```

An accepted first-slice `M` remains provisional and can only be lowered or superseded by a reviewed limit supported by broader PERF-01 workloads. It does not establish final players per Channel, players per GameNode/world, channels per GameNode, production hardware, production admission policy, universal tick cadence, or product-wide performance readiness.

## 10. Options, trade-offs, risks and exclusions

**Selected:** the owner-named physical runner, native one-CPU/1-GiB fail-closed cell and deterministic fixed-carrier workload. This is the smallest method that measures real carrier work while keeping domain payloads out of the result.

**Rejected:** GitHub-hosted or interchangeable hardware; a container-first cell; best-effort affinity/memory; average-only or single-run evidence; empty actor storage; AI01/synthetic values as capacity; and choosing `M` in architecture. These either destroy comparability, hide enforcement failure or confuse correctness with capacity.

The cell is intentionally narrow. It reduces time to first physical evidence and avoids inventing full gameplay, but it can overstate capacity once domain work is added. The 30% headroom and explicit lower-coupled-limit rule limit that risk; keeping broader PERF-01 open prevents the first number from becoming a permanent production claim. A one-CPU cell may underrepresent later multithreaded topology, but that is preferable to silently making concurrency part of a carrier-only limit.

No runtime, registry, Cargo, workflow, production, Movement, Ability, AI, Combat, hardware provisioning, privilege, credential or external-repository mutation is authorized. Harness/fingerprint/schema work requires a fresh exact #162 allocation after acceptance. This author must not merge, enqueue or declare this candidate canonical.

## 11. Supersession and decision test closeout

- **What is blocked:** RL-01 evidence and serialization, shared actor carrier allocation, #508 Phase A and the first #139 child.
- **What becomes harder if guessed:** arbitrary admission bakes hardware and workload assumptions into a shared resource contract and creates migration or overload risk.
- **Evidence that may supersede this:** named physical results for the broader ADR-0009/PERF-01 workload set, changed accepted product objectives, or an accepted different deployment cell.
- **Deliberately undecided:** actual `M`; all production capacity/SLA/topology; runtime design; other RL rows; variable gameplay payload; and all broader PERF-01 workloads.

The bounded recommendation is therefore to accept this method only after exact-head review, then let #162 separately allocate evidence tooling. The first physical run may produce `REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE`; architecture must not conceal or repair that fact by weakening the cell.

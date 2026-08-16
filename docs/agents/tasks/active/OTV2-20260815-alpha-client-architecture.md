# OTV2-20260815-alpha-client-architecture

```yaml
task_id: OTV2-20260815-alpha-client-architecture
title: ALPHA-CLIENT-01 native client architecture
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/arch-e-alpha-client
pr: 273
base_sha: 088b46638ac014cd7928d6b0b75cee44902fe22c
head_sha: 7855d55abf964f527d46aec0f921ef0f2b0a1767
final_head_sha: null
final_head_frozen_at: null
owner: DOMAIN ARCHITECTURE DESIGN AGENT / worker E
created_at: 2026-08-15T00:19:00+02:00
updated_at: 2026-08-16
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260815-alpha-client-architecture.md
  - docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_ANALYSIS.md
  - docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md
public_contracts:
  - docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md
depends_on:
  - issue:#263
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0011-native-client-pre-protocol-migration-state.md
  - docs/architecture/ADR-0016-gameplay-transport-client-mode-runtime-readiness.md
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  - docs/architecture/ADR-0007-native-end-to-end-test-platform.md
  - docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-07.md
  - docs/architecture/CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md
  - docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md
  - docs/migration/VSL-02_RUST_CLIENT_MIGRATION_AND_CUTOVER_CONTRACT.md
blocks:
  - fresh exact-head repository validation after final task-truth checkpoint
  - genuinely independent non-Codex semantic review because cycle 4 changes privacy/security-relevant settings semantics and the Architecture Coordinator materially authored the repair
cross_repository_coordination_id: null
external_repositories: []
repair_cycles_for_current_gate: 4
repair_cycle_4_owner_override: explicit owner instruction on 2026-08-16 authorizing C/D/E/F continuation beyond the three-cycle stop
owner_review_constraint: no Codex for this continuation
```

`head_sha` records the immediately preceding fully synchronized/self-reviewed checkpoint (`7855d55a...`). The exact final task-containing head is recorded externally on PR/check/review evidence after this truth-correction commit; no self-referential follow-up commit is created merely to record its own SHA.

## Outcome

Delivered the bounded `ALPHA-CLIENT-01` architecture package and completed **owner-authorized repair cycle 4** for the stable gate. The owner explicitly overrode the repository's ordinary three-cycle stop for C, D, E and F and instructed continuation to clean review/merge **without Codex**.

This repair closes the two material findings from the final review of `e2eb37e1d099d25dd87ebc02a68c111dd8dd91ac`:

1. settings persistence now has an explicit minimum `ACCOUNT` / `OS_USER` / `INSTALLATION` / `DEVICE` semantic scope model, deterministic precedence, privacy fail-closed behavior and versioned cross-scope migration rules;
2. the candidate now defines the reusable low-level Oteryn Studio sharing boundary, dependency direction, product-specific exclusions, content export/runtime projection seam and required conformance evidence.

All earlier repaired admission, wire-oracle, audio, directory-precision, production-codec and visual-scene boundaries remain preserved. Runtime/client/Platform implementation remains `NOT_STARTED`; no executable code, DDL or production state was modified.

The repaired semantic blobs were reconciled to current `main@8722e565c6a0556934209820e3c14ee4f2dc6093` without semantic modification. Exact-head self-review on pre-checkpoint head `7855d55abf964f527d46aec0f921ef0f2b0a1767` passed with zero material findings and confirmed the final scope remained exactly the three worker-E paths. The only post-repair main ancestry addition was unrelated Atlas lifecycle closeout #289.

PR: `#273`.

## Architecture and source of truth

- `PROVEN` — issue #263 and the parallel-work allocation assign worker E and exactly the three paths listed above.
- `PROVEN` — root `AGENTS.override.md` ordinarily stops after three repair cycles for one gate.
- `PROVEN` — the owner explicitly authorized exceeding that ceiling for C/D/E/F on 2026-08-16; the historical count remains preserved rather than reset.
- `PROVEN` — ADR-0003 requires Platform Identity -> one-time Game Login Ticket -> Platform-owned Game Gateway -> selected endpoint/channel/revisions + short-lived pre-admission material -> `protocol-oteryn` -> final game-owned FND-04 admission/CharacterLease/GameSession authority.
- `PROVEN` — ADR-0011/ADR-0016 preserve current fail-closed `pre-native-protocol` behavior and runtime-unavailable gameplay transports.
- `PROVEN` — current `crates/platform-client/src/lib.rs` recursively rejects only the 12 literal keys `host`, `port`, `endpoint`, `endpoint_uri`, `protocol`, `protocol_profile`, `ticket`, `credential`, `game_session`, `admission`, `route`, `address`; complete-schema/unknown-field rejection is not currently proven.
- `PROVEN` — FND-02 requires generation/sequence/revision/snapshot reconciliation and independent wire evidence against common-mode codec defects.
- `PROVEN` — ADR-0007 Tier 1 must speak the production transport and use the same production schemas, codecs, sequencing and admission contracts as the native client.
- `PROVEN` — `ARCHITECTURE_ANALYSIS_GAP_REGISTER.md` registers renderer/scene, settings/account-device scope and reusable Studio/client low-level component boundaries as part of `ALPHA-CLIENT-01`.
- `PROVEN` — DUR-04 client-safe content/revision discipline applies to visual/audio presentation assets and the Studio-export/client-ingest runtime projection boundary.
- `UNKNOWN` — exact gameplay transport implementation APIs, scene graph/animation/lighting/particle technology, renderer/UI/audio libraries, updater/signing, installer, secure credential storage, account-profile synchronization backend, exact shared crate/package names and numeric resource maxima remain deliberately unfrozen.

## Acceptance criteria

- [x] Screen/composition/provider, runtime/session, protocol/reconciliation, renderer/UI/input, content, filesystem/config, diagnostics, update/install, Windows-first and E2E boundaries remain defined.
- [x] Current `pre-native-protocol` behavior remains fail closed and no unavailable transport is exposed as runtime-ready.
- [x] ADR-0003 ticket/Gateway/pre-admission/final-game-authority chain remains explicit with no directory shortcut.
- [x] Independent FND-02 wire evidence remains required in addition to product-path E2E.
- [x] Tier 1 explicitly exercises the supported production transport and production schemas/codecs/sequencing/admission path; test-only codecs cannot substitute for shipping-codec coverage.
- [x] Audio remains application-owned, client-safe/revision-compatible, bounded and presentation-only without a selected library/vendor.
- [x] Platform-directory current-state claim remains narrowed to the exact 12-key recursive denylist.
- [x] Visual scene/presentation ownership is explicit and remains non-authoritative/reconstructable/bounded.
- [x] Durable settings declare semantic account/OS-user/installation/device scope and deterministic precedence; privacy opt-out cannot be weakened by a less restrictive layer.
- [x] Selected audio output and other hardware-specific choices are device-scoped; portable account preferences require an accepted account-profile owner rather than invented local authority.
- [x] Oteryn Studio may share only representation-neutral/non-authoritative low-level components; product composition/session/UI/authoring boundaries remain separate and dependency direction is acyclic.
- [x] Studio authoring state reaches the client only through a revisioned client-safe export/projection boundary; authoring-only/server-only state cannot become runtime client content by shared-type convenience.
- [x] `DECISIONS_NOT_TAKEN` records deferred concrete libraries, shared-package naming/publication and settings synchronization implementation.
- [x] Foreign-domain findings remain report-only; no runtime/client/server/protocol/DDL/Platform/production or coordinator-overlay path is intentionally modified.
- [x] Changed paths remain within worker-E ownership.
- [x] Branch was reconciled to current `main@8722e565c6a0556934209820e3c14ee4f2dc6093` without semantic loss before this task-truth checkpoint.
- [x] Full-diff self-review on synchronized pre-checkpoint head `7855d55abf964f527d46aec0f921ef0f2b0a1767` was clean with zero material findings.
- [ ] Required repository checks pass on the final task-containing exact head after this checkpoint correction.
- [ ] Required genuinely independent non-Codex review on that unchanged final exact head is clean.

## Excluded scope

No executable client, Studio, scene, renderer, audio, networking, protocol, server or Platform implementation; no gameplay authority rewrite; no DDL/migration; no production activation; no credential/token representation; no concrete shared crate/package publication; no cross-repository write; no concrete account-profile backend; no concrete scene graph, camera, animation, lighting, particle/effects, audio, UI or renderer library selection; no coordinator-only global overlay edit; no lifecycle archive/ownership release before lawful merge.

## Implemented architecture repairs

### Earlier P1 — admission chain

Preserved explicit ADR-0003 fresh-entry chain through Game Login Ticket and Platform-owned Game Gateway before FND-02 transport and final game-owned FND-04 authority.

### Earlier P1 — independent wire oracle

Preserved canonical byte goldens, malformed/adversarial fixtures, properties, fuzzing, cross-version fixtures, resource ceilings and stable failure classes as independent evidence against common-mode production-codec defects.

### Earlier P2 — audio boundary

Preserved application-owned audio provider/device lifecycle, client-safe/revision-compatible assets, bounded resources, non-authoritative degradation, settings/accessibility and evidence ownership without choosing technology.

### Earlier P2 — Platform-directory precision

Preserved exact current 12-key denylist truth and future-only complete-schema/reject-unknown-field requirement.

### Earlier P1 — production codec path in Tier 1

Preserved mandatory production transport plus the same production schemas/codecs/sequencing/admission contracts. Independent wire evidence supplements rather than replaces that product code path.

### Earlier P2 — visual scene/presentation boundary

Preserved `apps/client` composition ownership of scene/presentation lifetime and the non-authoritative, reconstructable, revision-compatible, bounded scene/camera/animation/lighting/particles/effects boundary.

### Repair-cycle-4 P2 — settings account/device ownership

Repaired. The candidate now defines semantic `ACCOUNT`, `OS_USER`, `INSTALLATION` and `DEVICE` scopes, requires each durable setting to declare scope, defines deterministic default precedence for multi-scope user preferences, makes installation state non-generic, makes privacy/security restrictions dominate convenience precedence, scopes hardware selections such as audio output to `DEVICE`, and requires explicit versioned conflict/rollback rules for scope migration.

### Repair-cycle-4 P2 — Oteryn Studio sharing boundary

Repaired. The candidate permits only low-level representation-neutral/non-authoritative sharing, prohibits product composition/session/UI/authoring state from becoming shared contracts, requires acyclic dependency direction, keeps Studio authoring data behind revisioned export/client-safe projection, and names dependency/content-compatibility/negative evidence required before implementation acceptance.

## DECISIONS_NOT_TAKEN

No exact UI toolkit; scene graph/entity-presentation framework; camera algorithm; animation runtime; lighting model; particle/effects engine; shader architecture; renderer backend; audio library/vendor; exact shared client/Studio crate/package names or publication mechanism; account-profile synchronization backend; promotion/replacement of synthetic client crates; prediction/rollback algorithm; gameplay transport implementation or QUIC activation; Gateway/admission/reconnect token/API shape; protocol/TLS/protobuf library; content bundle/patch/CDN format; installer/updater/signing provider; exact Windows paths; credential vault; crash backend/retention; release-channel/version-skew policy; Linux/macOS commitment; numeric scene/effect/audio/network/cache limits; server/gameplay/persistence/balance authority.

## CROSS_DOMAIN_FINDINGS

The companion analysis retains the existing report-only findings for protocol runtime, admission/session integration, content/release tooling, security/release/SRE, QA/E2E and diagnostics/privacy. The cycle-4 repairs do not grant authority to mutate those owners. Any future account-profile synchronization or cross-repository Studio package publication requires the appropriate owning contract/owner authorization; this candidate only fixes the client-side scope/dependency boundary.

## Repair review findings

```yaml
review_finding:
  id: PR273-P1-ADMISSION-CHAIN
  repaired: true
  source: earlier owner-funded review
```

```yaml
review_finding:
  id: PR273-P1-INDEPENDENT-WIRE-ORACLE
  repaired: true
  source: earlier owner-funded review
```

```yaml
review_finding:
  id: PR273-P2-AUDIO-BOUNDARY
  repaired: true
  source: owner-funded review of d9786582e7f3a15c60a3796f2eb6189ed9d7b222
```

```yaml
review_finding:
  id: PR273-P2-PLATFORM-DIRECTORY-PRECISION
  repaired: true
  source: owner-funded review of d9786582e7f3a15c60a3796f2eb6189ed9d7b222
```

```yaml
review_finding:
  id: PR273-P1-TIER1-PRODUCTION-CODECS
  severity: P1
  repaired: true
  source: owner-funded review of b660a4e05c48bf6ac96d783573b81b3f227515ae
```

```yaml
review_finding:
  id: PR273-P2-VISUAL-SCENE-BOUNDARY
  severity: P2
  repaired: true
  source: owner-funded review of b660a4e05c48bf6ac96d783573b81b3f227515ae
```

```yaml
review_finding:
  id: PR273-P2-SETTINGS-SCOPE-PRECEDENCE
  severity: P2
  repaired: true
  source: final independent review of e2eb37e1d099d25dd87ebc02a68c111dd8dd91ac
  repair: added account/OS-user/installation/device ownership, deterministic precedence, privacy fail-closed and versioned scope migration
```

```yaml
review_finding:
  id: PR273-P2-STUDIO-SHARING-BOUNDARY
  severity: P2
  repaired: true
  source: final independent review of e2eb37e1d099d25dd87ebc02a68c111dd8dd91ac
  repair: added low-level shared-component allowlist, product-specific exclusions, dependency direction, revisioned export/runtime projection and required evidence
```

## Validation

### Focused

- verify the task-truth correction is the only semantic delta after `7855d55a...` and the analysis/candidate blobs remain unchanged;
- verify final compare remains exactly the three worker-E paths and `behind_by=0`;
- verify settings-scope rules still provide explicit ACCOUNT/OS_USER/INSTALLATION/DEVICE ownership, deterministic precedence, privacy restrictive-wins and migration requirements;
- verify Studio-sharing rules still preserve explicit shared/non-shared responsibilities, acyclic dependency direction and revisioned content projection/evidence;
- verify no runtime/client/server/protocol/DDL/Platform/production or cross-repository path changes.

### Component/integration

- `NOT_APPLICABLE` — paper-only architecture repair; no executable component changed

### E2E

- `NOT_APPLICABLE` for this delivery — the contract defines future Tier 1/Tier 2/Tier 3 obligations but does not implement them

### Exact-head CI

- current final task-containing head: recorded externally on PR #273 after this checkpoint commit
- required Agent governance / Merge authority / Merge gate: pending on that exact head

## Self-review

- required: YES
- pre-checkpoint synchronized head: `7855d55abf964f527d46aec0f921ef0f2b0a1767`
- result: PASS, zero material findings; final task-containing head requires focused truth-check/self-review after this checkpoint commit

## Independent review

- required: YES
- exact head: must be the unchanged final task-containing repair head
- allowed method: genuinely independent fresh separate agent/session, qualified human reviewer or dedicated audit mechanism that actually evaluates the architecture diff
- Codex: **NOT TO BE USED** for this continuation per explicit owner instruction
- verdict: pending

## PR and closeout

- PR #273 remains open and unmerged during repair validation.
- Historical material review threads are repaired and resolved after exact semantic repair proof.
- No archive or ownership release before merge.
- **Stable-gate repair budget:** historical ordinary ceiling was 3; current count is `repair_cycles_for_current_gate: 4` under the explicit 2026-08-16 owner override. The override permits continued bounded repair; it does not reset history or weaken any other gate.

## Context checkpoint

```yaml
last_progress: cycle-4 settings/Studio repairs were reconciled to current main and self-reviewed clean; this commit corrects the durable task checkpoint to the real remaining gates
status: validating
branch: docs/arch-e-alpha-client
head_sha: 7855d55abf964f527d46aec0f921ef0f2b0a1767
pr: 273
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: push/pull_request
ci_check_generation: final-task-truth-checkpoint
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 4
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: independent non-Codex semantic review after exact-head repository validation
next_action: VALIDATE_FINAL_TASK_HEAD_THEN_INDEPENDENT_NON_CODEX_REVIEW
```

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`
`IMPLEMENTATION_AUTHORITY: NONE`
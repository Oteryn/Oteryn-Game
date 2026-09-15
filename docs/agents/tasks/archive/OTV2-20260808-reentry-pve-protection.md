# OTV2-20260808-reentry-pve-protection — archived

```yaml
task_id: OTV2-20260808-reentry-pve-protection
title: Freeze four-second defensive PvE re-entry protection
mode: ARCHITECTURE_ONLY
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260808-reentry-pve-protection
pr: 96
base_sha: 19756eab0a66db37cb6f27ec367aaf2e4986df69
head_sha: ddf62ec48443fb4ce119eed0900662e222a99726
final_head_sha: ddf62ec48443fb4ce119eed0900662e222a99726
final_head_frozen_at: 2026-08-08T18:53:21+02:00
owner: Oteryn project owner
created_at: 2026-08-08T13:30:00+02:00
updated_at: 2026-08-08T18:55:00+02:00
execution_budget_minutes: 30
large_budget_reason: null
owned_paths:
  - docs/architecture/DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md
  - docs/architecture/DISCONNECT_CLIENT_OS_FORENSICS_OWNER_DIRECTION.md
  - docs/architecture/DISCONNECT_CLIENT_OS_FORENSICS_PRIVACY_TIMING_REFINEMENT.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/agents/tasks/active/OTV2-20260808-reentry-pve-protection.md
public_contracts:
  - docs/architecture/DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md
  - docs/architecture/DISCONNECT_CLIENT_OS_FORENSICS_OWNER_DIRECTION.md
  - docs/architecture/DISCONNECT_CLIENT_OS_FORENSICS_PRIVACY_TIMING_REFINEMENT.md
depends_on:
  - LAG_DISCONNECT_PROTECTION_OWNER_BASELINE.md
  - LAG_DISCONNECT_REENTRY_ACTION_POLICY_OWNER_BASELINE.md
  - FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md
  - DISCONNECT_FORENSIC_EVIDENCE_OWNER_BASELINE.md
  - CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md
  - docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md
blocks:
  - FND-03/FND-04 must consume the reconciled re-entry protection semantics
  - later client diagnostics and ANL contracts must consume the client/OS corroborating-evidence direction and privacy/timing refinement
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
delivery_pr: 96
delivery_exact_head: ddf62ec48443fb4ce119eed0900662e222a99726
delivery_squash_merge: 496f0b9ad5231d24325e937a3f09ba221cf5c86b
closeout_pr: 97
closeout_branch: docs/OTV2-20260808-reentry-pve-protection-closeout
completed_at: 2026-08-08T18:55:00+02:00
ownership_released: true
next_gate: FND-03
```

## Outcome

The owner-accepted disconnect/re-entry clarification is complete at the architecture level and canonical on `main`.

Canonical deliverables:

- `docs/architecture/DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md`;
- `docs/architecture/DISCONNECT_CLIENT_OS_FORENSICS_OWNER_DIRECTION.md`;
- `docs/architecture/DISCONNECT_CLIENT_OS_FORENSICS_PRIVACY_TIMING_REFINEMENT.md`;
- synchronized `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`.

The accepted package freezes exactly four seconds of defensive PvE protection after valid recovery from unexpected loss of playable control, excludes ordinary graceful logout/login from that benefit, permits movement/self-heal/health and mana/resource potions under normal legality and costs, prohibits outgoing PvE offense and healing another player during the window, and never buffers prohibited outgoing actions for execution after expiry.

The package preserves same-actor continuity, session-generation fencing, one-character-per-account, combat/PZ/logout obligations, committed effects, threat/aggro and anti-duplication invariants. It does not authorize runtime implementation.

For disconnect forensics, server-generated gameplay/liveness/runtime evidence remains authoritative. Client/OS/Launcher/Guardian evidence is optional corroborating diagnostics. Automatic client-originated incident-capsule upload remains governed by the existing global client-diagnostics opt-out, and opt-out or missing client evidence is not adverse evidence.

Launcher/Guardian is preserved only as an extension point. A separate process is not required by the foundation architecture, and a direct Guardian heartbeat remains separately gated by purpose, privacy, security, resource and measurement evidence.

## Architecture and source of truth

- `PROVEN` — delivery PR #96 squash-merged as `496f0b9ad5231d24325e937a3f09ba221cf5c86b` after exact-head CI and architecture audit.
- `PROVEN` — the delivered package is canonical on `main` and `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` routes the next foundation step to `FND-03`.
- `PROVEN` — `CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md` remains authoritative for automatic client-originated diagnostic upload; the delivered privacy/timing refinement preserves rather than supersedes it.
- `PROVEN` — `DISCONNECT_FORENSIC_EVIDENCE_OWNER_BASELINE.md` remains the authoritative server-side forensic evidence baseline.
- `DERIVED` — a concrete separate Launcher/Guardian process is not needed to start FND-03 because only the authority/timing/privacy extension boundary is required by the accepted runtime inputs.

## Acceptance criteria

### Re-entry gameplay behavior

- [x] Freeze `reentry_pve_protection = 4 seconds`.
- [x] Limit protected re-entry to valid recovery after unexpected loss of playable control; accepted graceful logout/login does not manufacture the protection window.
- [x] Keep protection eligibility server-authoritative; client/OS evidence is not required synchronously to receive protection.
- [x] Allow ordinary self-healing subject to normal legality/cost/cooldown rules.
- [x] Allow health and mana/resource potion use subject to normal item/cooldown rules.
- [x] Preserve previously accepted movement permission.
- [x] Prohibit healing another player while the protected character remains inside the four-second window.
- [x] Allow the protected character to receive legal healing from another player.
- [x] Keep non-healing support actions unresolved for the owning combat/action contract.
- [x] Prohibit all offensive actions against PvE monsters during the window.
- [x] Prohibit buffering prohibited outgoing actions for post-protection execution.
- [x] Preserve already committed pre-protection effects.
- [x] Preserve ordinary resource consumption, cooldown, exhaustion, combat/PZ/logout state, threat/aggro and authoritative actor continuity.
- [x] Resolve the conflict with older generic no-protection reconnect wording explicitly and narrowly.
- [x] Keep PvP, non-healing support, non-combat interactions, UI and exact enforcement thresholds unresolved.

### Disconnect-abuse and forensic direction

- [x] Preserve server-generated disconnect/gameplay evidence as authoritative and client/OS evidence as corroborating only.
- [x] Record separate investigative classes for graceful exit, crash, abrupt process loss, NIC/interface loss, administrative interface-state change, network-path loss, system crash/power loss, infrastructure failure and unknown incidents without claiming intent from one event.
- [x] Reject unrestricted Event Viewer/Event Log ingestion; allow only bounded, allowlisted, normalized incident evidence around the relevant episode.
- [x] Preserve native live network/interface/process observation plus bounded post-incident OS evidence as candidate implementation sources without freezing exact Windows APIs/providers/event IDs.
- [x] Preserve a client-side bounded rolling incident buffer as the preferred direction.
- [x] Preserve post-reconnect/post-boot submission of a bounded incident capsule when evidence could not be sent during the outage.
- [x] Preserve an independent client-side observer/launcher/guardian capability boundary without requiring a separate process in the first implementation.
- [x] Keep a separate guardian diagnostic heartbeat as a candidate for later benchmark/contract rather than a frozen runtime requirement.
- [x] Record that guardian alive + game process lost + network alive is materially different evidence from simultaneous game/guardian/path loss.
- [x] Record that post-boot evidence may corroborate system crash, hard reset or power interruption where available without overclaiming the exact physical cause.
- [x] Record that abrupt process disappearance without normal crash evidence is suspicious but is not automatic proof of deliberate force-close.
- [x] Record that repeated local network-interface transitions around high-risk combat are a strong investigative signal but do not alone prove intent.
- [x] Require longitudinal Game Intelligence analysis combining combat risk, HP/resources, incoming damage, hostile pressure, reconnect timing, protection use, healing/potions, escape outcome, client/guardian/OS evidence and infrastructure correlation.
- [x] Preserve detection of unusually deterministic disconnect timing as evidence potentially consistent with automation without treating one threshold or episode as proof.
- [x] Define the analytical goal as detecting abuse of disconnect protection rather than proving the exact physical/software mechanism that caused one disconnect.
- [x] Require correlation with GameNode/runtime health and other affected players so Oteryn-side or regional failures are not misclassified as player abuse.
- [x] Keep mechanical protection and retrospective abuse analysis separate: a suspicious episode may receive protection and still be investigated later.
- [x] Prohibit automatic sanctions from one client/OS event or one disconnect episode.
- [x] Preserve Game Intelligence/AI as read-only investigative support; human-reviewed enforcement requires a separate policy.
- [x] Keep kernel-driver/invasive anti-cheat outside the scope of this decision.
- [x] Preserve privacy minimization: no arbitrary files, unrestricted process inventory, full Event Log export, unrelated SSID/MAC/device data, credentials or secrets.
- [x] Bind automatic client-originated incident-capsule uploads, including client/OS/Launcher/Guardian-produced diagnostics, to the existing global diagnostics opt-out.
- [x] Preserve that diagnostics opt-out or missing client evidence is not adverse evidence and cannot weaken server-authoritative incident visibility.
- [x] Require any future independent Guardian heartbeat to receive a separate purpose/privacy/retention/resource contract and prohibit using it as a silent opt-out bypass.
- [x] Apply the architecture decision-timing test: trust/privacy/authority boundaries are required now; concrete Launcher/Guardian topology, privileges, transport and cadence are deferred.

### Coordination and governance

- [x] Synchronize `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` so the delivered clarification is mandatory input to `FND-03`/`FND-04`.
- [x] Make current-status wording transition-safe before and after delivery merge.
- [x] Preserve live/current-status precedence over stale long-lived coordination wording.
- [x] Exact-head Agent governance validation passed before delivery merge.
- [x] Independent architecture review passed with zero open material findings.
- [x] Delivery squash-merged as `496f0b9ad5231d24325e937a3f09ba221cf5c86b`.
- [x] Archive record prepared with immutable delivery evidence and ownership release for lifecycle closeout PR #97.

## Excluded scope

This task did not authorize or implement:

- Rust gameplay runtime behavior;
- `protocol-oteryn` listener/client adapter code;
- persistence/database schema changes;
- Platform changes;
- client/Windows diagnostic collection;
- a Launcher/Guardian process or Guardian heartbeat;
- production telemetry collection;
- Game Intelligence detector implementation;
- automatic enforcement or sanctions;
- production deployment or live operations;
- writes to external repositories.

## Implementation / findings

Material findings resolved before acceptance:

1. **Client diagnostics privacy boundary** — the earlier forensic direction could be read as allowing Launcher/Guardian/OS diagnostic transmission to bypass the existing global client-diagnostics opt-out. The final package explicitly prohibits such a bypass and preserves missing/opted-out evidence as non-adverse.
2. **Current-status transition safety** — the earlier current-status text would have become stale immediately after merge by retaining `PR #96 VALIDATING` and a self-referential merge-next step. The final wording is correct both while the PR is open and after it is merged.
3. **Premature Launcher/Guardian concretization** — the final package preserves an independent observer capability boundary without requiring a separate executable/process, specific privilege model, transport or heartbeat before evidence justifies those choices.

No material finding remained at delivery merge.

## Supersession boundary

The package narrowly supersedes only the older generic reconnect wording that prohibited any protection/invulnerability window, for the exact four-second PvE defensive re-entry interval defined by the delivered decision.

It does not supersede the underlying anti-reset, session-fencing, account/character concurrency, combat/PZ/logout, durable-state or anti-duplication invariants.

The privacy/timing refinement preserves rather than supersedes `CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md`, `DISCONNECT_FORENSIC_EVIDENCE_OWNER_BASELINE.md`, ADR-0006 and the architecture decision-discipline policy.

## Validation

### Focused

- changed-file review: `PASS`; exactly five declared delivery files on PR #96.
- architecture consistency review: `PASS`; privacy/timing and status-transition findings resolved before final head freeze.

### Component/integration

- result: `NOT_APPLICABLE`; documentation-only architecture delivery changed no runtime component.

### E2E

- scenario: `NOT_APPLICABLE`; no runtime/product capability was implemented by this task.
- result: `NOT_APPLICABLE`.

### Exact-head CI

- final head: `ddf62ec48443fb4ce119eed0900662e222a99726`;
- trigger source: pull request #96;
- Agent governance run `31268053954`: `PASS`;
- Dependency review run `31268053946`: `PASS`;
- CodeQL run `31268053944`: `PASS`;
- classification: exact delivery head;
- result: `PASS`.

## Independent audit

- exact head: `ddf62ec48443fb4ce119eed0900662e222a99726`;
- method/auditor: full changed-path architecture continuation audit recorded as PR review `4889231397`;
- material findings: `0` open after the three repairs above;
- unresolved review threads at delivery acceptance: `0`;
- verdict: `PASS`.

## PR and closeout

- changed-file review: `PASS` for delivery PR #96;
- unresolved delivery review threads: `0`;
- related/superseded PRs: none requiring closure;
- delivery merge policy: squash;
- delivery merge commit/result: `496f0b9ad5231d24325e937a3f09ba221cf5c86b`, `PASS`;
- lifecycle closeout PR: `97`;
- ownership release: represented by this archived record when closeout PR #97 merges to `main`.

## Cross-repository disposition

No external repository was modified by this task.

`blakinio/Oteryn-Platform`, `blakinio/Otheryn` and `blakinio/otclient` remained read-only. Any future Platform reconciliation, native client diagnostic implementation or production telemetry change requires its own separately authorized task/branch/PR and rollout contract.

## Context checkpoint

```yaml
last_progress: Delivery PR #96 passed exact-head Agent governance, Dependency review, CodeQL and architecture audit with zero material findings, then squash-merged as 496f0b9ad5231d24325e937a3f09ba221cf5c86b; lifecycle closeout PR #97 now preserves the complete task record and releases ownership on merge.
status: completed
branch: docs/OTV2-20260808-reentry-pve-protection
head_sha: ddf62ec48443fb4ce119eed0900662e222a99726
pr: 96
final_head_sha: ddf62ec48443fb4ce119eed0900662e222a99726
final_head_frozen_at: 2026-08-08T18:53:21+02:00
ci_trigger_source: pull_request
ci_check_generation: delivery-final
ci_checks_for_current_head: 3
ci_run_ids:
  - 31268053954
  - 31268053946
  - 31268053944
ci_job_ids: []
runner_assignment_state: completed
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 3
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Start the bounded architecture-only FND-03 Runtime Execution Contract after closeout PR #97 is merged and ownership release is canonical on main.
```

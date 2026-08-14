# Oteryn v2 Foundation Programme — Successor Handover

- Handover ID: `OTV2-20260812-foundation-handover`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Refreshed: 2026-08-14 13:55 +02:00
- Repository: `blakinio/Oteryn-v2`
- Trusted refresh base: `main@d04f0939f0078cb677ca3ad66f5949e9f3dadc8d`
- Reference manifest acceptance: PR #252 / merge `52ef65f67e8a0e9c6f31d4754f8a6b7322d8d6d8`
- First representative ABILITY_COMBAT evidence delivery: PR #255 / exact final head `6744f655c6438eebeab70b30aae17d33b5bd2fa7` / squash merge `d04f0939f0078cb677ca3ad66f5949e9f3dadc8d`
- Current lifecycle closeout: issue #256 / draft PR #257 / branch `docs/ability-combat-evidence-closeout`
- Terminal successor result after closeout: **`ROTATE`**
- Runtime/client/combat/AI/script implementation authority: **NOT GRANTED**
- PostgreSQL DDL/migration execution authority: **NONE**
- Platform write authority: **NONE**
- Production authority: **NONE**

## 1. Purpose

This is the durable successor handoff for Oteryn-v2 architecture continuation. Chat history is non-authoritative. A successor must read trusted-base governance and live GitHub state first, verify drift/ownership, and then execute the one `next_action` below.

The current closeout is lifecycle/status/handoff reconciliation only. It does not promote any Reference mechanic, provenance/legal state, implementation state or parity classification.

## 2. Canonical current state

```text
GAME-VISION-01        ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
GAME-CHANNEL-01       ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
GAME-CHAR-01          ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
GAME-ITEM-01          ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-01                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-02                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-03                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-04                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
ANL-01                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
SIM-DETERMINISM-01    ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
REFERENCE MANIFEST    ACCEPTED / schema v1 / revision 3
ABILITY_COMBAT CASES  4 REGISTERED / UNKNOWN / provenance PENDING / implementation NOT_STARTED / parity PENDING
PROD-ENTITLEMENTS-01  PROPOSED / PLANNED / NOT_STARTED
```

Architecture acceptance and evidence registration do **not** imply runtime/client/server implementation, compiler/loader/Studio/WIT-host implementation, PostgreSQL DDL/migrations, Platform implementation, broad content import, gameplay traffic, exact gameplay formulas or production readiness.

## 3. Accepted architecture — consume rather than rediscover

Unless explicitly superseded, preserve:

- one native Rust client/server stack and one project-owned `protocol-oteryn`;
- `protocol-canary` reference-only and excluded from target runtime/fallback/translation;
- FND-ID/FND-02/FND-03/FND-04 identity/protocol/runtime/session/admission/fencing/recovery semantics;
- DUR-01/DUR-02 durable identity/persistence transaction/migration/restore architecture;
- ANL-01 event/audit/privacy/read-only investigation boundary;
- GAME-VISION minimum product direction and immutable first Reference target after the 2026-07-28 Global Tibia server-save/maintenance boundary;
- GAME-CHAR formula-neutral authoritative progression facts with unresolved Reference arithmetic remaining fail-closed;
- GAME-ITEM typed item semantics;
- DUR-03 durable item/currency/value conservation/idempotency/anti-duplication;
- GAME-CHANNEL selection/queue/co-location/anti-hopping/multiplicity/qualitative lifecycle and one-World community/economy policy;
- DUR-04 typed semantic content graph, exact package lock, deterministic compilation, immutable bundle activation/migration, bounded loading/provenance and capability-bounded deterministic scripting;
- SIM-DETERMINISM deterministic arithmetic/RNG/order/replay/state-hash architecture;
- Reference manifest v1 owner acceptance, evidence hierarchy and fail-closed rules;
- GAME-ABILITY typed effect/targeting/cast/cooldown/composition/reference-catalogue architecture already accepted as partial baselines.

Do not restart accepted gates merely because older backlog/predecision prose reflects an earlier state.

## 4. Reference evidence registry — binding current result

Canonical sources:

- `docs/architecture/GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md`;
- `docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md`;
- `docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md`;
- `docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.schema.json`;
- `docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json`;
- `docs/architecture/GAME-ABILITY-01_FIRST_REFERENCE_EVIDENCE_FIXTURE_PACKAGE.md`;
- `docs/agents/evidence/OTV2-20260814-ability-combat-official-spell-library.md`.

Current machine state after PR #255:

```text
schema_version = 1
manifest_revision = 3
status = ACCEPTED
canonical_digest = null
reference_target = global-tibia-observable-2026-07-28-post-server-save
ABILITY_COMBAT = MECHANIC_CASES_REGISTERED
cases = 4
```

The four cases are Light Healing cast metadata, Light Healing qualitative self-heal semantics, Ice Strike cast metadata and Ice Strike qualitative targeted ice-damage semantics.

Every case remains deliberately fail closed:

- target evidence `UNKNOWN`;
- source provenance `PENDING`;
- case provenance `PENDING`;
- legal review `PENDING`;
- Oteryn implementation `NOT_STARTED`;
- exact implementation revision null;
- fixture/test links empty;
- parity `PARITY_PENDING_EVIDENCE`.

The indexed official Library content does not prove the immutable target. Exact official locators are known, but search-index freshness was only approximate, direct official page fetch from the research environment returned HTTP 403, exact crawl/live observation time is unknown and source/case provenance is not cleared. Patch-note/search absence is not continuity proof.

The human fixture records are **pending blueprints**, not executable/passing parity fixtures. `healing` and `damage` are descriptive accepted semantic families, not frozen physical serializer/enum identifiers.

## 5. PR #255 delivery evidence

- issue #254: completed;
- final exact head: `6744f655c6438eebeab70b30aae17d33b5bd2fa7`;
- final self-review comment `5292724813`: PASS, zero open material findings;
- three repaired findings: target evidence over-promotion, live-page retrieval overstatement, source/case provenance over-promotion;
- owner-authorized Codex review scope: **PR #255 only**;
- Codex result: no findings, final PR `+1` reaction `454048359`;
- Agent governance `31795833321`: PASS;
- Merge authority audit `31795833334`: PASS;
- Merge gate `31795833324`: PASS including scope, governance, Dependency Review, CodeQL actions, CodeQL python and aggregate `Merge gate / validate`;
- unresolved review threads before merge: 0;
- squash merge: `d04f0939f0078cb677ca3ad66f5949e9f3dadc8d`;
- post-merge `main`: verified exactly at that SHA.

The Codex authorization for PR #255 is consumed and does not authorize closeout PR #257 or future work.

## 6. One next paper-only action

The versioned Reference manifest and the first representative ABILITY_COMBAT evidence package are already delivered. **Do not build them again.**

After lifecycle closeout, the one selected pre-VSL paper-only programme action is:

```text
Obtain target-continuity + provenance-clearance evidence for the four registered
ABILITY_COMBAT Light Healing/Ice Strike cases.
```

A successor should create one bounded task that:

1. keeps the immutable 2026-07-28 Reference target unchanged;
2. seeks provenance-cleared, time-appropriate evidence that directly bridges or captures the target boundary;
3. updates existing case classifications only when the accepted evidence contract permits it;
4. leaves `UNKNOWN/PENDING` unchanged where evidence is insufficient;
5. never treats patch/search silence as continuity evidence;
6. does not broaden mechanic inventory or freeze physical catalogue/fixture tooling before this representative historical-evidence path is proven;
7. does not invent a new stable architecture gate ID;
8. does not claim `PARITY_CONFIRMED` without sufficient target evidence, cleared provenance/legal state, exact Oteryn implementation revision and a passing bounded fixture/test.

This next action is paper-only. It does not authorize official-client automation beyond separately authorized/legal evidence acquisition, runtime implementation, proprietary asset/code copying, DDL, Platform writes or production changes.

## 7. Implementation boundary

A future executable server/persistence/content/Channel/SIM package still requires separate explicit owner implementation authorization and its own bounded evidence. Current accepted architecture may be consumed only after that authorization.

`PROD-ENTITLEMENTS-01` remains independently unaccepted for Oteryn-v2 consumption; Premium/VIP/game-consumed entitlement activation remains unauthorized.

## 8. Successor bootstrap

Before mutation, a successor must read/follow at minimum:

1. root `AGENTS.md` and `AGENTS.override.md`;
2. `docs/agents/AGENTS.md`;
3. `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`;
4. `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md`;
5. `docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md`;
6. `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`;
7. this handoff;
8. `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`;
9. accepted Reference/GAME-ABILITY contracts and manifest files relevant to the four existing cases;
10. live `main`, open PRs, active tasks/owned paths, review threads and CI.

Live merged repository state overrides this report if state has legitimately advanced.

## 9. Context checkpoint

```yaml
status: ready
terminal_invocation_result: ROTATE
repository: blakinio/Oteryn-v2
trusted_base_sha: d04f0939f0078cb677ca3ad66f5949e9f3dadc8d
closeout_issue: 256
closeout_branch: docs/ability-combat-evidence-closeout
closeout_pr: 257
owned_paths: []
public_contracts: []
last_progress: PR #255 exact final head 6744f655c6438eebeab70b30aae17d33b5bd2fa7 passed final self-review, owner-authorized Codex no-finding review and exact-head repository gates, then squash-merged as d04f0939f0078cb677ca3ad66f5949e9f3dadc8d; issue #254 closed completed; lifecycle closeout #256 / draft PR #257 reconciles archive/status/handoff and releases task ownership.
validation_state: delivery #255 merged after exact-head self-review, no-finding Codex and green Agent governance/Merge authority/Merge gate; closeout #257 must pass its own exact-head documentation/governance validation.
e2e_state: NOT_APPLICABLE documentation-only architecture/closeout
blocker: null
owner_action_required: false
next_action: After lifecycle closeout, create one bounded paper-only target-continuity + provenance-clearance evidence task for the four registered Light Healing/Ice Strike cases; do not broaden/freeze tooling first and do not implement runtime/DDL/production behavior.
```

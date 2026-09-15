# Oteryn Game Sol-Lead + Selective-Codex Execution Design

## Status

Owner-approved execution-model direction captured for canonical review under Issue #179.

This design changes **how the remaining post-blocker programme is executed**, not the accepted gameplay/runtime architecture. It does not itself authorize any runtime mutation. A later implementation/governance plan must package the exact prompts, allocations and lifecycle transitions before Sol lane leads can write product code.

## Goal

Complete the remaining post-blocker Oteryn Game vertical-slice programme faster and with deeper reasoning by combining:

- the existing Work delivery coordinator as the GitHub/control-plane owner;
- separate ChatGPT GPT-5.6 Sol sessions at Extra High / highest available reasoning as domain lane leads;
- selective Codex assistance for concrete repository implementation/debug/test/build work;
- the existing independent Work auditor;
- the owner-designated Supervising Architect for material architecture decisions.

The design deliberately avoids using Codex as the default end-to-end executor for every lane so scarce Codex capacity is spent only where repository execution materially benefits.

## Non-goals

This design does not:

- change Game runtime architecture, protocol semantics, persistence guarantees, resource limits, product behavior or accepted ADRs/contracts;
- grant production/protected-environment/live-account/live-session/database authority;
- grant Platform, Atlas, META or any external-repository write authority;
- reactivate terminal Issues #93, #115, #116, #123 or #131;
- make AI a prerequisite for the first Movement/Combat slice unless a later accepted contract does so;
- authorize a Sol chat to write merely because its alias exists;
- authorize Codex automatically or imply that every ChatGPT session can directly spawn Codex;
- weaken exact-head CI, independent review, ownership, allocation or shared-lease gates.

## Transition snapshot

At the design admission snapshot:

- protected `main` is `cb9c5f4f53dd880c9d338dafd21b6184a4419993`;
- Ability PR #171 is merged as `2faa280b406a313d02ee1330c65651bc36e215a9`;
- Interaction PR #172 is merged as `73f82e4864aa15ece50625bda8bac7868f779ba3`;
- AI PR #178 is merged as `cb9c5f4f53dd880c9d338dafd21b6184a4419993`;
- Durability Issue #167 remains open and is the next critical implementation lane;
- Server Seam remains dependency-gated on the durable `ReconnectAttemptJournal` adapter;
- independent audit has identified at least two transition concerns that must remain visible until reconciled: missing durable independent-review proof/reconciliation for Ability #171 and stale coordinator/task/live-allocation metadata for already-merged lanes.

These facts are transition evidence only. Every future invocation must resolve fresh live GitHub state and use newer truth when it exists.

## Execution hierarchy

```text
Owner
  -> Supervising Architect
  -> Work Control-Plane Coordinator
     -> Sol Lane Leads
        -> selective Codex assistance when useful and available

Independent control:
  Work Independent Auditor
```

### Owner

The owner approves material changes to execution authority and all architecture/product decisions that existing repository authority does not already resolve.

### Supervising Architect

The Supervising Architect owns material architecture interpretation and decisions. It receives `ARCHITECTURE_ESCALATION_REQUIRED` packets and does not become a routine coding lane.

### Work Control-Plane Coordinator

The existing Work coordinator remains the authoritative programme control plane. Its primary responsibility becomes coordination rather than deep lane implementation.

It owns:

- fresh GitHub reconciliation;
- programme Issue/task state;
- exact child allocations;
- dependency/DAG readiness;
- path ownership and shared leases;
- integration ordering;
- exact-head qualification/merge coordination;
- closeout/archive/ownership release;
- truthful programme state.

It should not perform the substantive implementation of high-complexity lanes when a dedicated Sol lane lead has been allocated.

This is a role specialization, not permission for Work to widen repository or merge authority beyond current governance.

### Sol Lane Leads

A Sol lane lead is the principal reasoning owner for one bounded delivery lane. The requested profile is GPT-5.6 Sol with Extra High / highest available reasoning. If that profile is unavailable, the agent must report the limitation rather than silently claiming the requested profile.

A lead may:

- inspect live GitHub and repository state;
- reconcile its allocated lane against current contracts and accepted architecture;
- plan implementation and tests;
- write product code only after exact merged allocation authority exists;
- run non-destructive local validation;
- use selective Codex assistance under the policy below;
- repair findings inside its owned paths;
- produce one exact handoff back to the Work control plane.

A lead may not:

- take unallocated shared paths;
- consume an unmerged sibling branch as implicit truth;
- create new architecture semantics to progress;
- change registered resource limits without accepted authority;
- merge around missing required review/checks;
- mutate production/protected environments or external repositories;
- claim Codex execution occurred when no real supported handoff occurred.

### Work Independent Auditor

`Oteryn: work auditor` remains a separate read-only control. It audits both Work control-plane behavior and the evidence produced by Sol lane leads. It must remain independent of the implementation it audits.

## Selective Codex policy

Canonical policy name:

```text
CODEX_USE: SELECTIVE_IMPLEMENTATION_ASSISTANCE
```

Codex is an optional technical executor, not the programme brain.

### Preferred Codex uses

A Sol lead should consider Codex for bounded tasks where repository execution is the expensive part:

- `IMPLEMENT` — concrete code changes within exact owned paths;
- `DEBUG` — reproduce and diagnose failing tests/builds;
- `TEST` — run focused/component/workspace test commands and return exact evidence;
- `BUILD` — compiler/lint/format/dependency work;
- `REPO_EXECUTION` — branch/worktree/diff operations that materially benefit from a coding agent.

### Work that normally stays with Sol

Do not consume Codex by default for:

- architecture interpretation;
- programme coordination;
- dependency/DAG reasoning;
- prompt/governance design;
- status reconciliation;
- ordinary evidence comparison;
- broad read-only planning;
- routine review that Sol can perform directly.

### Handoff discipline

When a supported direct Codex mechanism exists, the Sol lead gives Codex a narrow packet containing:

```yaml
repository: Oteryn/Oteryn-Game
issue: <issue>
task_id: <task>
branch: <branch>
base_or_head_sha: <exact sha>
owned_paths: []
forbidden_paths: []
objective: <single bounded technical outcome>
commands_to_run: []
expected_return:
  - exact_changed_paths
  - exact_head_sha
  - commands_and_results
  - unresolved_findings_or_blocker
```

The Sol lead remains responsible for reviewing Codex output before treating it as evidence or integrating it.

If no supported direct mechanism exists, the lead must return a durable/explicit `CODEX_HANDOFF_REQUIRED` packet or continue safely without Codex. It must never pretend the Codex task was invoked.

### Token-conservation rule

Default programme posture:

- at most **one heavy Codex implementation lane** active at a time;
- a second Codex-heavy lane is allowed only when the Work control plane proves path/shared-surface independence and there is a concrete throughput benefit;
- read-only Sol preparation/review lanes do not consume the Codex concurrency budget;
- do not use Codex merely because a slot exists.

This is an execution-efficiency policy, not a product/platform quota claim.

## Concurrency model

The programme may have up to **five active Sol lane chats** when they have distinct responsibilities.

However, simultaneous repository mutation is more restricted:

- normally no more than **two or three mutating Sol lanes** at once;
- only when all primary owned paths are disjoint;
- no simultaneous writers to any serialized shared surface;
- a blocked/waiting lead should release active mutation ownership where safe rather than occupy a writer slot indefinitely.

### Serialized shared surfaces

At minimum, continue to serialize:

- root/app Cargo manifests;
- `Cargo.lock`;
- `workspace-boundaries.toml`;
- stable protocol/event/resource registries and stable numeric IDs;
- `apps/game-server/src/lib.rs` and equivalent composition roots;
- shared accepted ADRs/contracts consumed by multiple active lanes;
- workflow/protection/governance files.

A Sol worker encountering one of these paths reports `SHARED_LEASE_REQUIRED`. Only the Work control plane may allocate the exact shared turn under existing authority.

## Planned Sol lead aliases

The later prompt-package implementation should create reusable profiles for at least:

```text
Oteryn: sol durability lead
Oteryn: sol server seam lead
Oteryn: sol client qa lead
Oteryn: sol movement lead
Oteryn: sol combat lead
```

Transition-only roles may also be packaged when the completed audit proves they remain needed:

```text
Oteryn: sol ability reconciliation
Oteryn: sol lifecycle reconciler
```

An alias alone grants no mutation authority. Every lead must bind to a current merged coordinator allocation before writing.

## Wave scheduling

The scheduler optimizes for useful parallel reasoning while respecting the mostly serial critical path.

### Transition Wave T0

Run concurrently where still applicable after fresh audit reconciliation:

1. **Durability Lead** — critical-path owner; may become the primary mutating/Codex-assisted lane after exact shared Cargo lease/base reconciliation.
2. **Ability Reconciliation** — independent exact-tree post-merge review/reconciliation if still required; normally read-only evidence work.
3. **Lifecycle Reconciler** — bounded docs/governance synchronization for merged Ability/Interaction/AI and truthful Durability/coordinator state; no product runtime changes.
4. **Server Seam Lead** — read-only readiness, exact dependency/API/negative-test/Tier-1 preparation until Durability merges.
5. **Client/QA Lead** — read-only client/Tier-1/Tier-2 dependency/test preparation until Server Seam provides the real boundary.

The Work auditor may run independently of this five-lead execution set.

### Wave T1 — after Durability merge

- Server Seam Lead becomes the primary mutating lane and may selectively use Codex.
- Client/QA Lead continues preparation against the exact merged seam contract.
- Movement Lead may perform read-only #139/current-contract readiness analysis but cannot activate resource decisions prematurely.
- an independent reviewer/auditor prepares for Server Seam exact-head qualification.
- Work reconciles the new protected-main state and shared leases.

### Wave T2 — after Server Seam merge

- Client/QA Lead executes compatible native Client work under a fresh exact allocation.
- real applicable Tier 1 evidence is executed/recorded through the production server/protocol boundary.
- Movement Lead prepares/executes the exact #139 resource gate only when all current prerequisites are proven.
- Combat Lead may perform read-only preparation, not runtime implementation.

### Wave T3 — Movement

After Client/QA readiness and terminal #139 resource closure:

- Movement Lead becomes the primary gameplay implementation lane;
- Codex may be used selectively for implementation/test/debug/build;
- exact physical E2E and required review must pass before merge;
- Combat remains preparation-only until Movement protected-main readback.

### Wave T4 — Combat

After Movement and all real current prerequisites merge:

- Combat Lead becomes the primary gameplay implementation lane;
- any persistence/value/item/resource semantic gap outside accepted authority is escalated before mutation;
- high-risk independent exact-head review remains mandatory where policy requires it.

### Terminal wave

Work performs final programme reconciliation and closeout only after the exact vertical-slice result is merged and physically evidenced. The Work auditor verifies no stale tasks, ownership, leases, false QA claims or bypassed architecture escalation remain.

## State machine for lane leads

Use these exact execution states where applicable:

```text
READ_ONLY_PREPARATION
WAITING_ALLOCATION
READY_TO_IMPLEMENT
IMPLEMENTING
SHARED_LEASE_REQUIRED
CODEX_HANDOFF_REQUIRED
WAITING_EXTERNAL
WAITING_ARCHITECTURE
READY_FOR_INTEGRATION
REVIEW_RECONCILIATION_REQUIRED
COMPLETED_RELEASED
```

`UNKNOWN` or `CONFLICT` on authority, ownership, exact head, prerequisite state or shared lease must fail closed before mutation.

## Architecture escalation

A Sol lane lead uses the same durable classification as Work:

```text
ARCHITECTURE_ESCALATION_REQUIRED
```

Escalate before mutation when the lane requires a new or conflicting decision about:

- public API/wire/schema/stable identity;
- persistence/durable transaction/value ownership;
- authentication/session/reconnect/fencing trust authority;
- unaccepted hard resource limits;
- cross-repository responsibility;
- permanent Content/product/reference semantics;
- production deployment/secrets/topology;
- semantic ownership conflicts between valid lanes;
- weakening safety, tests, provenance or review requirements.

Routine compile/test failures, path-local implementation details and already-authorized merge conflicts remain lane execution work.

## Evidence and handoff

Every Sol lane lead returns one compact evidence packet to Work:

```yaml
lane:
issue:
task_id:
admission_main_sha:
integration_main_sha:
branch:
pr:
final_head_sha:
changed_paths: []
shared_lease_used: null
codex_usage:
  used: true | false
  purpose: null
  evidence: null
focused_validation: []
component_validation: []
e2e: PASS | BLOCKED | NOT_APPLICABLE | FAIL
self_review:
independent_review:
architecture_escalation: null
unresolved_findings: []
recommended_control_plane_action: integrate | reconcile | wait | escalate | closeout
```

Work independently validates the packet against GitHub before integration. Worker text is never proof by itself.

## Transition from current Work lifecycle

The execution-model migration must preserve history and avoid pretending the old Work task already used this architecture.

Before the first Sol implementation lead mutates product code under this model, the implementation plan must:

1. reconcile the completed independent Work audit and any remaining Ability review finding;
2. reconcile active task packets and `OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md` with actual merged Ability/Interaction/AI state;
3. record the exact truthful Durability blocker/readiness and current protected-main base;
4. update the Work coordinator profile or add an explicit companion control-plane profile so deep lane implementation is delegated rather than duplicated;
5. package Sol lead aliases and lifecycle entries;
6. define exact current child allocations/leases before any lead writes;
7. preserve the existing master programme dependency order unless fresh accepted authority changes it.

Do not edit historical task evidence to imply pre-merge reviews or lifecycle events that did not occur.

## Review and safety requirements

The model does not change repository risk policy.

- self-review remains mandatory;
- independent exact-head review remains mandatory when required by risk policy, owner or governing contract;
- security/session/persistence/durable-data/production-trust/value work receives the stronger required review;
- Codex output does not count as independent review when the same Codex task authored the change;
- a Sol lead that materially authored the implementation cannot review itself as an independent reviewer;
- all applicable exact-head repository gates remain required before merge.

## Success criteria

This execution architecture is successfully adopted only when:

- Work has one truthful control-plane role with no duplicate heavy implementation ownership;
- Sol lead prompts are canonical and bind writes to exact merged allocations;
- selective Codex behavior and no-fake-handoff rules are explicit;
- shared surfaces remain serialized;
- transition audit/lifecycle findings are reconciled truthfully;
- Durability -> Server Seam -> Client/QA -> Movement -> Combat sequencing remains dependency-correct;
- final programme closeout can reconstruct every lane from durable GitHub evidence.

## Supersession

This design specializes the execution model for the remaining post-blocker programme. It does not supersede accepted runtime/product architecture or the canonical implementation DAG by itself.

A later execution-model change must explicitly identify which clauses of this design it replaces and must preserve historical evidence rather than rewriting this document as though the newer model had always applied.

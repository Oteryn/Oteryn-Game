# OTERYN GAME — INDEPENDENT PROGRAMME & ARCHITECTURE AUDIT

```yaml
prompt_id: OTERYN-GAME-INDEPENDENT-PROGRAMME-AUDIT
prompt_version: "1.1"
prompt_mode: AUDIT
working_mode: READ_ONLY_INDEPENDENT_AUDIT
target_repository: Oteryn/Oteryn-Game
meta_repository: Oteryn/Oteryn
runtime_implementation_authorized: false
repository_mutation_authorized: false
additional_ai_invocation_authorized: false
short_invocation: "Oteryn: audyt"
recommended_model: GPT-5.6 Sol
recommended_effort: Pro
```

---

# 0. ROLE

Act as an independent principal-level auditor of **Oteryn Game**.

Think simultaneously as:

- software architect;
- distributed systems architect;
- senior Rust engineer;
- game-engine developer;
- MMO server developer;
- networking/protocol engineer;
- security engineer;
- persistence/database engineer;
- concurrency engineer;
- DevOps/SRE engineer;
- QA/E2E engineer;
- producer;
- game designer;
- live-service operator;
- player;
- long-term maintainer.

Your purpose is not to confirm that substantial work has been completed.

Your purpose is to determine whether the project is:

- building the correct system;
- building it in the correct order;
- preserving accepted invariants;
- producing evidence proportional to risk;
- avoiding architectural debt that will become expensive later;
- converging toward a real playable native vertical slice;
- avoiding accidental inheritance of legacy MMO/OTS design mistakes.

Prefer detecting a wrong direction now over preserving sunk-cost implementation.

Do not soften findings because significant work already exists.

Do not reward activity, code volume, number of PRs, green checks or architectural sophistication by themselves.

Evaluate **direction, correctness, evidence and delivery value**.

---

# 1. AUTHORITY AND SAFETY

This audit is **READ ONLY**.

You MAY:

- inspect repository contents;
- inspect Git history;
- inspect Issues;
- inspect Pull Requests;
- inspect review threads;
- inspect CI/check/workflow state;
- inspect relevant external repositories as read-only evidence;
- inspect generated artifacts and historical test evidence;
- run non-destructive local validation when tooling and environment permit.

You MUST NOT:

- modify tracked repository files;
- create commits;
- push branches;
- open, edit, merge or close PRs;
- edit Issues;
- alter labels, milestones or repository settings;
- modify CI configuration;
- rerun or manually dispatch remote workflows merely to obtain a result;
- mutate databases;
- modify live environments;
- deploy anything;
- access or expose secrets;
- trigger production operations;
- invoke Codex or another owner-funded AI/API reviewer unless the owner separately authorizes that exact invocation.

Do not turn audit findings into implementation during this task.

## 1.1 Local validation safety

When running local validation:

- do not modify tracked files;
- do not regenerate committed source or schemas;
- do not modify lockfiles;
- prefer locked/offline/reproducible modes where supported;
- prefer temporary or ignored build-output locations;
- do not contact production services;
- do not depend on owner secrets;
- do not perform destructive migrations;
- do not treat a command that altered repository truth as valid read-only evidence.

Where practical, verify after local validation that tracked repository state remains unchanged.

If meaningful validation cannot be executed safely in read-only mode, classify it as:

`NOT_EXECUTABLE_IN_CURRENT_ENVIRONMENT`

or:

`BLOCKED`

rather than weakening the read-only boundary.

---

# 2. REPOSITORY AUTHORITY MODEL

Resolve authority by subject.

## `Oteryn/Oteryn-Game`

Canonical authority for:

- native Rust game server;
- native Rust client;
- `protocol-oteryn`;
- gameplay/domain logic;
- world/runtime implementation;
- Game persistence behavior;
- Game-owned content tooling;
- Game-owned export contracts.

## `Oteryn/Oteryn`

META authority for:

- ecosystem topology;
- cross-repository coordination;
- ecosystem-level compatibility;
- cross-repository orchestration contracts;
- repository/release topology metadata.

META documentation must not silently override Game-owned runtime or implementation authority.

## Legacy/reference repositories

Repositories such as:

- `blakinio/Oteryn-v2`;
- `blakinio/Otheryn`;
- `blakinio/otclient`;
- Canary;
- Crystal Server;
- other OTS sources;

are historical, migration, behavioral or implementation references unless a current accepted Oteryn decision explicitly states otherwise.

Never treat legacy implementation as target architecture by default.

---

# 3. SOURCE-OF-TRUTH DISCIPLINE

Do not use one global source hierarchy for every kind of claim.

Resolve authority according to the subject being evaluated.

## 3.1 Programme intent and acceptance

Prefer:

1. live authoritative GitHub Issues and their acceptance criteria;
2. accepted milestone/programme governance;
3. current programme/status documents;
4. historical programme evidence;
5. chat or worker summaries.

## 3.2 Merged implementation truth

Prefer:

1. code and configuration present at the frozen default-branch SHA;
2. applicable repository instructions;
3. accepted contracts/ADRs governing that implementation;
4. exact-SHA CI/test evidence.

Issue text or status documents cannot make absent merged code exist.

## 3.3 Proposed implementation truth

For open PRs prefer:

1. exact PR head SHA;
2. exact PR diff/files;
3. PR review state and check state;
4. linked Issue acceptance criteria;
5. current base SHA and merge relationship.

PR-only code is proposed state, not merged project state.

## 3.4 Architecture and contracts

Prefer:

1. currently accepted ADRs/contracts owned by the authoritative repository;
2. current canonical architecture documentation;
3. merged implementation where documentation claims to describe existing behavior;
4. live Issues explicitly superseding an older decision.

## 3.5 Agent instructions

For every inspected path resolve:

1. repository-root `AGENTS.md`;
2. applicable nearer `AGENTS.md`;
3. applicable `AGENTS.override.md`;
4. `docs/agents/AGENTS.md` where relevant.

## 3.6 Conflicts

If credible authoritative sources disagree, record:

`CONFLICT`

Do not silently select whichever source supports the easier conclusion.

Do not use chat memory to override inspectable repository evidence.

---

# 4. MANDATORY AUDIT SNAPSHOT

Before assigning any verdict, freeze an audit snapshot.

Record:

```yaml
audit_snapshot:
  timestamp_utc:
  repository:
  default_branch:
  main_sha:
  meta_main_sha:
  current_milestone:
  authoritative_programme_issues:
  active_workstreams:
  open_prs:
    - pr:
      base_sha:
      head_sha:
      issue:
      purpose:
  required_checks_observed:
```

All findings must refer to this frozen snapshot.

## 4.1 Snapshot immutability

If the default branch advances during the audit:

- keep the originally frozen `main_sha`;
- do not silently mix old and new repository state;
- note material drift if observed;
- re-freeze and restart only if necessary for a reliable verdict.

If a PR head changes during the audit:

- do not transfer findings from the old head to the new head;
- either audit the new exact head;
- or state explicitly that the finding applies only to the previous SHA.

If a CI/check result belongs to another SHA, do not attribute it to the frozen SHA.

Exact-head evidence must remain exact-head evidence.

---

# 4.2 MERGED VS PROPOSED STATE

Always distinguish:

- `MERGED_STATE` — present at frozen default-branch SHA;
- `PROPOSED_STATE` — present only in an open PR/head;
- `HISTORICAL_STATE` — present only in older revisions;
- `DOCUMENTED_ONLY` — claimed by documentation but not verified in implementation;
- `UNKNOWN_STATE` — insufficient evidence.

Never count PR-only implementation as functionality already present in the project.

A PR may be evaluated for:

- architectural fitness;
- correctness;
- safety;
- readiness;
- compatibility;
- integration order;

but it must not upgrade merged programme implementation state until merged.

Likewise, documentation describing intended behavior must not be reported as implemented behavior without implementation evidence.

---

# 5. MANDATORY STARTUP

Before analysing implementation quality:

## 5.1 Resolve instructions

Read:

- repository-root `AGENTS.md`;
- applicable nearer `AGENTS.md`;
- applicable `AGENTS.override.md`;
- `docs/agents/AGENTS.md` where applicable.

Do not assume root instructions are the complete instruction chain.

---

## 5.2 Resolve current programme state

Inspect current:

- GitHub Issues;
- open PRs;
- active workstreams;
- dependencies;
- acceptance criteria;
- CI/check state;
- branch/base/head relationships.

Determine what the project is **currently trying to prove**.

Inventory all materially relevant open work, but deep-audit only workstreams relevant to:

- the current milestone;
- its prerequisites;
- current architecture invariants;
- contracts consumed by current work;
- integration conflicts.

Do not expand the audit simply because unrelated open Issues exist.

---

## 5.3 Read canonical architecture/governance

At minimum inspect applicable current versions of:

- `docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md`;
- `docs/agents/PROMPT_EVAL_STANDARD.md`;
- `docs/agents/BUILD_TEST_MATRIX.md`;
- `docs/agents/END_TO_END_FEATURE_COMPLETENESS.md`;
- `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`;
- `docs/agents/CROSS_REPO_CONTRACTS.md`;
- `docs/agents/KNOWN_RISKS.md`;
- `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`;
- `docs/agents/MULTI_AGENT_ARCHITECTURE_ORCHESTRATION.md` when active multi-agent work applies;
- relevant `docs/architecture/**`;
- relevant `docs/contracts/**`;
- relevant migration/provenance material.

Also inspect the architecture-continuation principles represented by:

`docs/agents/prompts/OTV2_ARCHITECTURE_CONTINUATION_AGENT.md`

Use its broad architecture/runtime/security/MMO/player/producer perspective as audit coverage, not as mutation authority.

If a listed document no longer exists or has clearly been superseded:

- locate the current replacement where possible;
- record a conflict only if the missing/superseded authority materially affects the current audit;
- do not fail the programme merely because an obsolete documentation filename disappeared.

---

# 5.4 AUDIT DEPTH PRIORITY

Spend audit effort in this order:

1. current authoritative milestone and its prerequisites;
2. active implementation workstreams and open PRs;
3. invariants on which those workstreams depend;
4. current cross-workstream contracts;
5. current cross-repository contracts;
6. merged implementation directly supporting the next evidence-producing milestone;
7. future architecture only where current choices materially constrain it.

Depth on current risk is more valuable than superficial coverage of every possible future MMO subsystem.

Do not perform broad speculative audits of future systems merely because they appear in this prompt.

Do not turn the checklist into a requirement that every listed subsystem must already exist.

---

# 5.5 EXTERNAL REPOSITORY SCOPE

Inspect a legacy/reference/external repository deeply only when current Oteryn evidence:

- references it;
- derives code, data, assets or behavior from it;
- claims compatibility with it;
- uses it as migration input;
- creates a concrete architectural question that cannot be resolved from canonical Oteryn repositories.

Do not crawl legacy repositories speculatively.

When external evidence is used, identify the exact repository and revision where practical.

---

# 5.6 AUDIT STOP DISCIPLINE

Do not spend effort proving low-risk facts beyond the point needed for a reliable verdict.

Once sufficient evidence exists to classify an area confidently:

- record the evidence;
- move to higher-risk unresolved areas.

Conversely, do not stop after one weak clue when a finding would materially affect:

- P0/P1 severity;
- a workstream pause;
- the current gate;
- the next architecture decision;
- programme direction.

The goal is **maximum decision quality**, not maximum repository traversal.

---

# 6. PHASE-AWARE AUDIT

This is mandatory.

Do not evaluate an early implementation milestone as though the entire final game must already exist.

Classify every relevant capability as exactly one of:

- `REQUIRED_NOW`
- `REQUIRED_BEFORE_NEXT_GATE`
- `FUTURE_REQUIRED`
- `DELIBERATELY_DEFERRED`
- `UNRESOLVED`
- `NOT_APPLICABLE`

Absence of a future layer is not automatically a defect.

Report missing implementation as a defect only when:

1. the current authoritative milestone requires it; or
2. a prerequisite invariant must already exist; or
3. current implementation is making the future layer materially harder or unsafe to add; or
4. documentation/status falsely claims it already exists or is proven.

Do not manufacture speculative requirements for runtime layers that do not yet have an owning implementation.

---

# 6.1 CURRENT-GATE RELEVANCE

For material findings distinguish:

- `CURRENT_GATE` — directly affects the current milestone/gate;
- `NEXT_GATE` — must be resolved before the next named gate;
- `FUTURE_CONSTRAINT` — not required now, but a current choice is making future work materially unsafe or prohibitively expensive;
- `FUTURE_ONLY` — valid future concern with no current blocking effect.

A future-only concern must not fail the current programme gate.

An unresolved future decision is not a blocker merely because it is architecturally interesting.

P0/P1 severity and gate impact are separate concepts.

A P1 blocks current PASS only when it is relevant to:

- `CURRENT_GATE`;
- `NEXT_GATE`; or
- a concrete `FUTURE_CONSTRAINT` already being created by current implementation.

---

# 6.2 NEGATIVE-EVIDENCE RULE

Do not claim that code, validation, ownership or a contract is absent after one failed lookup.

Before reporting material `ABSENT` or equivalent negative findings, use reasonable corroboration such as:

- expected-path inspection;
- repository-wide search;
- symbol/reference search;
- relevant Issue/PR inspection;
- relevant architecture/contract documentation;
- build/test matrix evidence where applicable.

If absence cannot be established confidently, report:

`UNKNOWN`

not:

`ABSENT`.

Absence of evidence is not evidence of absence unless the search coverage is sufficient for the claim being made.

---

# 7. AUDIT OBJECTIVE

Determine whether Oteryn Game is progressing correctly toward the accepted native target architecture and next evidence-producing milestone.

Detect:

- architectural drift;
- wrong dependency direction;
- premature coupling;
- hidden global state;
- weak ownership boundaries;
- incorrect state authority;
- protocol leakage into domain logic;
- legacy contamination;
- insufficient invariants;
- unsafe persistence assumptions;
- economy/item duplication risks;
- concurrency races;
- reconnect/session/fencing defects;
- incorrect multichannel assumptions;
- security/trust-boundary defects;
- resource exhaustion risks;
- accidental unbounded work;
- insufficient tests;
- misleading green CI;
- false E2E claims;
- stale evidence;
- cross-repository drift;
- incompatible parallel workstreams;
- unnecessary overengineering;
- irreversible decisions made too early;
- missing decisions that block the next safe proof;
- work that should be paused before more code depends on it.

Do not search for defects merely to populate the report.

A clean result is valid when supported by sufficient evidence.

---

# 8. ACCEPTED HIGH-LEVEL INVARIANTS

Verify against current repository truth before applying them, but challenge any implementation that contradicts currently accepted equivalents of these principles:

- native Rust client/server;
- target runtime protocol is `protocol-oteryn`;
- server-authoritative gameplay legality and results;
- client sends intent rather than arbitrary authoritative state;
- `WorldId` and `ChannelId` remain distinct;
- multichannel world model;
- one logical writer per authoritative simulation scope where required;
- character writes protected by session-generation/fencing semantics;
- Game does not become a second Identity/OAuth authority;
- authoritative Game admission follows accepted Platform/Gateway/Game Session contracts;
- legacy Tibia/Canary/OTS protocol architecture is not automatically target architecture.

If repository evidence shows any of these have been superseded, cite the newer accepted authority instead of assuming the older rule.

Do not promote an old principle to an invariant merely because it appeared in historical planning.

---

# 9. ARCHITECTURE AUDIT

Evaluate:

## 9.1 Boundaries

Check:

- bounded contexts;
- module boundaries;
- ownership;
- dependency direction;
- cohesion;
- public contracts;
- schema ownership;
- versioning;
- compatibility;
- migration boundaries;
- failure domains.

Ask:

- Does every authoritative state transition have exactly one owner?
- Is presentation separated from domain truth?
- Is transport separated from gameplay semantics?
- Is persistence separated from domain legality?
- Are Game/Platform/Atlas boundaries preserved?
- Are dependencies pointing toward stable ownership boundaries?
- Is any temporary implementation accidentally becoming a public architectural contract?

Flag abstractions that exist only because a legacy OTS had them.

Do not flag an abstraction merely because another design might be cleaner.

Require material impact.

---

# 10. FOUNDATION AUDIT

Assess whether foundation primitives are sufficient and correctly timed for dependent work.

Inspect where applicable:

- IDs;
- world/channel/session identities;
- time/tick abstractions;
- deterministic execution requirements;
- error taxonomy;
- capability/version concepts;
- state ownership;
- concurrency primitives;
- cancellation;
- overload behavior;
- protocol seam;
- persistence seam;
- resource-limit abstractions;
- observability hooks.

Do not require speculative foundation abstraction without a real current consumer.

Conversely, identify missing foundation semantics that downstream work is already duplicating, guessing or independently redefining.

Pay particular attention to foundation concepts that would become expensive to change after:

- protocol stabilization;
- client implementation;
- persistence adoption;
- multichannel runtime implementation;
- content schema adoption.

---

# 11. DOMAIN/GAMEPLAY AUDIT

Check that authoritative gameplay logic:

- lives on the server;
- validates untrusted client intent;
- protects game-state invariants;
- uses stable identities where required;
- has deterministic behavior where required;
- handles failure/timeout/cancellation explicitly;
- does not rely on presentation state;
- does not leak transport details through core domain logic.

Where relevant inspect:

- movement;
- combat;
- abilities;
- inventory;
- items;
- loot;
- creatures/AI;
- progression;
- PvP;
- trade;
- houses;
- quests;
- rewards;
- shared-world services.

Do not require every future gameplay system to exist now.

Audit whether the current architecture keeps those systems safely addable later.

Prefer direct domain invariants over historical Tibia behavior unless compatibility is explicitly required.

---

# 12. CONTENT AND MIGRATION AUDIT

Evaluate whether content/data is appropriately separated from engine authority.

Check:

- schema/versioning;
- deterministic loading;
- validation;
- bounds;
- malformed input handling;
- provenance;
- legal/licensing status;
- target/reference distinction;
- migration reproducibility;
- exact source revisions;
- failure behavior;
- cross-channel state leakage.

Legacy data/code/assets may be used only according to accepted provenance and licensing rules.

Do not infer redistribution rights from technical accessibility.

Do not require migration tooling before its accepted phase unless current content work already depends on it.

---

# 13. LEGACY CONTAMINATION AUDIT

Explicitly search current evidence for architectural decisions copied from:

- Tibia;
- Canary;
- Crystal Server;
- OTClient;
- other OTS implementations;

without a current Oteryn justification.

Classify legacy use as one of:

- `BEHAVIOR_REFERENCE`
- `MIGRATION_REFERENCE`
- `COMPATIBILITY_REFERENCE`
- `IMPLEMENTATION_REFERENCE`
- `ACCEPTED_TARGET_DESIGN`
- `UNJUSTIFIED_INHERITANCE`

Flag:

- historical bugs copied as behavior;
- unnecessary protocol constraints;
- accidental global state;
- synchronous assumptions;
- coupling inherited only because old clients/servers had it;
- outdated security/trust models;
- implementation complexity with no current product need.

Do not reject useful legacy behavior merely because it is old.

Require a current reason for carrying it forward.

Do not infer inheritance solely from superficial structural similarity.

---

# 14. PROTOCOL AND CLIENT/SERVER SEAM

Inspect:

- framing;
- serialization;
- version negotiation;
- sequencing;
- command IDs;
- replay handling;
- duplicate handling;
- snapshot/delta semantics;
- error vocabulary;
- limits;
- downgrade behavior;
- malformed/adversarial packet handling;
- client/server compatibility matrix.

Do not accept client/server agreement alone as proof of wire correctness.

Where the owning layer exists, look for independent protocol evidence such as:

- canonical byte-level golden fixtures;
- malformed/adversarial corpora;
- property tests;
- fuzzing;
- cross-version fixture validation;
- explicit resource ceilings.

The client and server sharing the same incorrect codec is a common-mode failure, not independent validation.

Do not require every listed validation technique simultaneously unless current risk or governance requires it.

Require evidence proportional to the protocol risk being claimed as solved.

---

# 15. CLIENT AUDIT

Where client implementation currently exists or is required, assess:

- networking separation;
- state reconciliation;
- presentation vs authority;
- renderer/UI boundaries;
- resource management;
- device-loss/recovery where relevant;
- input handling;
- prediction/interpolation if applicable;
- version compatibility;
- packaging assumptions;
- error/reconnect UX;
- telemetry/privacy.

Do not require the client to start before the accepted protocol/foundation seam is sufficiently stable.

Conversely, flag server abstractions that are becoming client-hostile or impossible to consume cleanly.

Distinguish:

- architecture needed to permit a client;
- actual client implementation;
- native-client E2E proof.

These are separate states.

---

# 16. PERSISTENCE AND ECONOMY AUDIT

For owning layers that exist or are required now, inspect:

- transaction boundaries;
- atomicity;
- idempotency;
- stable operation IDs;
- revisions;
- fencing;
- rollback;
- restart behavior;
- crash consistency;
- recovery ordering;
- concurrent mutation;
- stale writer rejection;
- duplicate suppression;
- audit/outbox consistency.

For items/currency/economy, challenge:

- duplication;
- loss;
- rollback exploits;
- race conditions;
- partial commit;
- reconnect abuse;
- cross-channel abuse;
- repeated/replayed requests.

Require explicit conservation reasoning where state has economic value.

Do not require production-scale persistence infrastructure before the phase needs it.

Do require state-transition semantics early enough that dependent gameplay does not encode unsafe assumptions.

---

# 17. CONCURRENCY / MULTICHANNEL AUDIT

Where applicable inspect:

- single-writer authority;
- session generations;
- character lease ownership;
- stale writers;
- relog;
- reconnect;
- fencing;
- channel isolation;
- cross-channel misuse;
- shared-world services;
- handoff;
- failure recovery;
- ordering;
- races;
- deadlocks;
- starvation;
- cancellation.

Explicitly search for process-global mutable state that would make multichannel operation unsafe or prohibitively expensive to retrofit.

Inspect shared-world systems for duplication/loss at channel boundaries.

Do not require premature distributed coordination where current architecture deliberately remains single-process and does not violate future ownership semantics.

---

# 18. SECURITY AUDIT

Assume the game client is untrusted.

Assess:

- authentication;
- authorization;
- session lifecycle;
- audience binding;
- replay;
- expiry;
- revocation;
- spoofing;
- malformed inputs;
- injection;
- parser safety;
- rate limiting;
- resource exhaustion;
- privilege escalation;
- path traversal;
- decompression limits;
- unsafe deserialization;
- secrets handling;
- artifact trust;
- updater trust;
- supply-chain risk;
- auditability.

Use:

`secure by design + secure by default`

Do not accept client validation as authoritative security.

Do not manufacture a security requirement for a component that has no current trust-boundary role.

Focus on actual or imminently created attack surfaces.

---

# 19. PERFORMANCE, DETERMINISM AND RESOURCE SAFETY

Check where current architecture makes the concern relevant:

- CPU amplification;
- memory amplification;
- network amplification;
- unbounded queues;
- unbounded recursion;
- unbounded scripts;
- pathfinding limits;
- packet/input limits;
- content limits;
- backpressure;
- overload behavior;
- scheduling;
- lock contention;
- allocation patterns;
- tick behavior;
- clock/time assumptions;
- deterministic replay assumptions;
- failure isolation.

Do not optimize prematurely.

But flag architectural choices that create hard future scaling, safety or determinism constraints.

Distinguish measured performance defects from speculative scaling concerns.

Do not report performance estimates as FACT without measurement or direct structural evidence.

---

# 20. OBSERVABILITY AND OPERABILITY

Assess whether failures can be diagnosed without unsafe production intervention.

Check:

- structured diagnostics;
- correlation IDs;
- relevant revisions;
- audit trails;
- events;
- metrics;
- logs;
- tracing where appropriate;
- privacy/redaction;
- reconnect/failure observability;
- persistence recovery evidence;
- operator visibility;
- actionable failure categories.

Do not require maximal telemetry.

Require enough evidence to diagnose important state transitions and failure modes relevant to the current or next gate.

---

# 21. SUPPLY CHAIN / PROVENANCE / LICENSING

Inspect where applicable:

- dependency review status;
- `cargo-deny`;
- advisories;
- banned/source policies;
- provenance of migrated code;
- provenance of datasets/content;
- asset licensing;
- third-party licenses;
- exact legacy source revisions;
- reproducibility of conversion/import.

A technically correct implementation with unclear redistribution rights remains a project risk.

Do not block the current gate for unrelated future assets/content whose redistribution is not part of the current milestone.

---

# 22. TEST AND VALIDATION AUDIT

Use the repository's current `BUILD_TEST_MATRIX`.

Distinguish:

- unit;
- property;
- fuzz;
- component;
- integration;
- synthetic harness;
- Tier 1 headless E2E;
- Tier 2 instrumented native-client E2E;
- Tier 3 production-binary smoke;
- CI status;
- manually inspected evidence.

For every material validation claim classify evidence source as:

- `EXECUTED_DURING_AUDIT`
- `INSPECTED_EXISTING_RESULT`
- `NOT_EXECUTABLE_IN_CURRENT_ENVIRONMENT`
- `BLOCKED`
- `NOT_REQUIRED`

Also classify evidence freshness where useful as:

- `EXACT_HEAD`
- `EXACT_MERGED_SHA`
- `COMPATIBLE_BUT_DIFFERENT_SHA`
- `STALE_SHA`
- `HISTORICAL_ONLY`
- `UNKNOWN`

Do not claim that you executed a test merely because a historical CI run exists.

A green build is not E2E proof.

Environment startup is not gameplay E2E.

Tier 1 does not prove native-client presentation.

Tier 2 does not prove the exact production-default binary.

Hidden retry-until-green is unacceptable.

A test result from another SHA may be informative, but it is not exact-head proof.

Do not rerun remote CI merely because historical evidence is missing unless such mutation was separately authorized.

---

# 23. FEATURE COMPLETENESS AUDIT

For every feature currently claimed as meaningful, implemented or proven, check required layers where applicable:

- producer;
- authoritative behavior;
- protocol/API;
- consumer/client/UI;
- persistence;
- failure behavior;
- security;
- limits;
- telemetry/privacy;
- migration;
- rollout;
- rollback;
- exact revision matrix;
- E2E evidence.

Use truth states such as:

- `PROVEN`
- `PARTIAL`
- `SYNTHETIC_ONLY`
- `UNKNOWN`
- `BLOCKED`
- `DEFERRED`
- `ABSENT`

Do not promote:

`IMPLEMENTED`

to:

`PROVEN`

without the required evidence.

Do not downgrade an intentionally phase-limited feature because future layers are not yet required.

---

# 24. CROSS-REPOSITORY CONTRACT AUDIT

Inspect material Game interactions with Platform/META/Atlas.

Check:

- canonical owner;
- producer;
- consumer;
- exact revision;
- API version;
- protocol version;
- content revision;
- ruleset revision;
- session binding;
- rollout order;
- rollback;
- mixed-version behavior;
- compatibility windows;
- stable error vocabulary;
- resource limits;
- fixture ownership.

Classify compatibility step where applicable as:

- `SERVER_FIRST_SAFE`
- `CLIENT_FIRST_SAFE`
- `BACKWARD_COMPATIBLE`
- `ATOMIC_REQUIRED`
- `BREAKING_MIGRATION`
- `UNVERIFIED`

Do not treat documentation as proof of implementation.

Do not deep-audit unrelated ecosystem repositories unless a current Game contract actually depends on them.

---

# 25. MULTI-WORKSTREAM AUDIT

Discover active workstreams from live Issues/PRs rather than assuming names.

For each concurrent workstream inspect:

- issue;
- objective;
- acceptance criteria;
- base SHA;
- current head SHA;
- owned paths;
- dependency direction;
- contract ownership;
- overlap with sibling work;
- assumptions about not-yet-merged work;
- drift from current main;
- integration order.

Explicitly detect:

- two workers defining the same public invariant;
- ownership overlap;
- incompatible contract assumptions;
- dependent work building against an unstable seam;
- stale branch assumptions;
- circular dependencies;
- parallel code that can no longer merge cleanly conceptually even if Git reports no textual conflict.

When current lanes correspond to foundation/domains/content/QA/client, audit those explicitly.

Do not infer workstream ownership from branch names alone when Issues or explicit programme ownership exist.

---

# 25.1 CONTRACT-OWNER RULE

For every shared public invariant used by more than one active workstream, determine:

- canonical owner;
- current merged definition;
- proposed modifications;
- consumers;
- integration order.

If two PRs independently define incompatible versions of the same public invariant, treat it as a cross-workstream conflict even if both individually pass tests.

Prefer one canonical seam over duplicated temporary contracts when dependent work is already active.

---

# 26. ARCHITECTURE DECISION TIMING

Every material architecture recommendation produced by this audit must answer:

1. **Must decide now?** `YES` / `NO`
2. What exact downstream work is blocked?
3. What becomes harder or impossible if deferred?
4. What evidence would justify superseding the decision?
5. What is deliberately not decided?

Follow this bias:

```text
freeze what blocks the next safe proof
register what matters later
measure the real system
then refine
```

Do not recommend freezing:

- technology;
- schema;
- topology;
- framework;
- broker;
- datastore;
- serialization;
- abstraction;

merely because it could someday be useful.

Avoid architecture as a substitute for product progress.

A recommendation to introduce a new abstraction must identify its current consumer or concrete downstream risk.

---

# 27. PLAYER PERSPECTIVE

For major architectural findings consider impact on:

- latency perception;
- responsiveness;
- movement feel;
- combat feel;
- reconnect;
- rollback;
- lost progress;
- fairness;
- PvP;
- economy;
- exploitability;
- stability;
- loading;
- UI;
- future game mechanics.

A technically elegant architecture that produces poor player experience is not sufficient.

Do not invent gameplay preferences unsupported by current product goals.

---

# 28. PRODUCER / DELIVERY PERSPECTIVE

Evaluate:

- time-to-next-real-proof;
- implementation cost;
- maintenance cost;
- migration cost;
- operational cost;
- irreversible coupling;
- dependency bottlenecks;
- rollout;
- rollback;
- compatibility windows;
- debugging cost;
- ability to deliver future mechanics.

Explicitly identify overengineering.

Also identify short-term shortcuts that create unacceptable architectural debt.

Prefer recommendations that reduce risk while preserving delivery momentum.

Do not recommend large rewrites when a smaller correction restores the invariant.

---

# 29. PROGRAMME TRAJECTORY

Determine whether the project is:

- converging toward a real executable vertical slice;
- accumulating disconnected components;
- blocked by an actual missing decision;
- blocked by implementation;
- blocked by validation infrastructure;
- building on an unstable seam;
- spending effort on premature future architecture.

Identify the **smallest evidence-producing next milestone** consistent with accepted safety invariants.

Do not redefine product priorities without evidence.

Do not recommend a larger milestone when a smaller one can prove the same critical architectural assumption.

---

# 30. FINDING SEVERITY

Severity measures technical/programme impact.

It is separate from:

- truth confidence;
- phase requirement;
- current gate impact.

Use:

### `P0`

Immediate correctness/security/data-loss/economy-corruption issue.

Work depending on the defect should stop.

### `P1`

Architectural flaw or invariant violation likely to cause:

- major rework;
- unsafe dependent implementation;
- invalid programme direction;
- violation of a prerequisite needed by the current or next gate.

### `P2`

Material implementation/design/test/operational issue that should be corrected soon but does not invalidate the whole current direction.

### `P3`

Localized debt, maintainability issue or bounded improvement.

### `NOTE`

Useful non-blocking observation.

Do not inflate severity merely because an issue is interesting.

Do not lower severity merely because the defective code is new.

A future-only concern may still be important, but it must not automatically become a current-gate blocker.

---

# 31. FINDING TRUTH CLASSIFICATION

Truth classification describes evidence certainty only.

Use exactly one of:

- `FACT` — directly verified;
- `INFERENCE` — derived from verified facts;
- `UNKNOWN` — insufficient evidence;
- `CONFLICT` — credible authoritative sources disagree.

Do **not** use `BLOCKER` or `RECOMMENDATION` as truth states.

A blocker is a programme/gate effect.

A recommendation is a proposed action.

Keep:

- evidence certainty;
- severity;
- gate relevance;
- corrective action;

separate.

Do not report `UNKNOWN` as `FACT`.

Do not report absence of evidence as proof that something does not exist.

---

# 31.1 EVIDENCE PRECISION

For every FACT supporting a P0/P1/P2 finding, provide the most precise available locator.

Prefer:

- repository + commit SHA + file path + line/range;
- Issue number + exact relevant acceptance criterion;
- PR number + exact head SHA + file/path;
- check/workflow name + exact run/result;
- ADR/contract path + revision;
- test/fixture name + exact revision.

When line locators are unavailable, provide the strongest exact artifact locator available.

Statements such as:

`code inspection shows...`

without an addressable locator are insufficient for a material finding when more precise evidence is available.

For INFERENCE:

- identify the verified facts from which the inference follows.

For UNKNOWN:

- state exactly what evidence is missing.

For CONFLICT:

- identify both conflicting authorities.

---

# 31.2 FINDING DEDUPLICATION

Report one root cause as one material finding.

If the same defect affects:

- architecture;
- persistence;
- security;
- concurrency;
- protocol;
- multiple workstreams;

record those impacts under the same finding rather than duplicating it.

Cross-reference finding IDs from:

- matrices;
- workstream decisions;
- programme risks;
- corrective actions.

Do not inflate programme risk by counting one root cause multiple times.

Distinct consequences may remain separate findings only when they require materially different corrections or can fail independently.

---

# 31.3 NO-FINDING DISCIPLINE

Do not state:

`no problem exists`

unless the inspected evidence is strong enough to support that scope.

Prefer:

`NO MATERIAL ISSUE FOUND IN INSPECTED SCOPE`

where exhaustive proof is impossible.

An audit is not formal verification.

Avoid universal claims unsupported by exhaustive evidence.

---

# 32. WORKSTREAM DISPOSITION

For each active implementation stream return exactly one:

- `CONTINUE`
- `CONTINUE_WITH_CONDITION`
- `PAUSE`
- `REDIRECT`
- `BLOCKED`
- `SUPERSEDED`

Definitions:

### `CONTINUE`

No material current evidence requires changing direction.

### `CONTINUE_WITH_CONDITION`

Direction is sound, but a concrete prerequisite/correction must be satisfied before a named merge/gate/dependency step.

### `PAUSE`

Continuing current implementation would materially increase risk, debt or invalid dependent work.

### `REDIRECT`

Objective remains valid but current implementation direction is materially wrong.

### `BLOCKED`

Required progress cannot continue because a real dependency/evidence/authority gap exists.

### `SUPERSEDED`

The workstream has been replaced by a newer authoritative direction.

`PAUSE` or `REDIRECT` requires concrete evidence.

Do not pause work merely because architecture could be more elegant.

---

# 33. REQUIRED OUTPUT

Produce the audit in this exact top-level structure.

Do not add unrelated appendices.

---

## 1. Executive verdict

Choose exactly one:

- `ON_TRACK`
- `ON_TRACK_WITH_CORRECTIONS`
- `AT_RISK`
- `OFF_TRACK`
- `INCONCLUSIVE`

Maximum 10 concise lines.

State the dominant reason for the verdict.

Do not use `INCONCLUSIVE` merely because future systems are unimplemented.

Use it only when material evidence necessary to judge the current programme direction cannot be obtained.

---

## 2. Audit snapshot

Report:

- audit time;
- Game repository branch/SHA;
- META repository SHA;
- authoritative current milestone;
- programme Issues inspected;
- open PRs and exact heads;
- relevant CI/check state.

Clearly distinguish the frozen snapshot from any later observed drift.

---

## 3. Current milestone

State:

- intended outcome;
- authoritative acceptance criteria;
- dependencies;
- current blockers;
- what is explicitly not required yet.

If no single authoritative milestone can be resolved, state the conflict.

---

## 4. Programme evidence matrix

Use:

| Area | Phase requirement | Implementation state | Validation state | Evidence freshness | Verdict |
|---|---|---|---|---|---|

Phase requirement must use:

- `REQUIRED_NOW`
- `REQUIRED_BEFORE_NEXT_GATE`
- `FUTURE_REQUIRED`
- `DELIBERATELY_DEFERRED`
- `UNRESOLVED`
- `NOT_APPLICABLE`

Implementation state should distinguish where applicable:

- `MERGED_STATE`
- `PROPOSED_STATE`
- `PARTIAL`
- `DOCUMENTED_ONLY`
- `ABSENT`
- `UNKNOWN_STATE`
- `DEFERRED`

Do not collapse proposed PR implementation into merged state.

---

## 5. Architecture consistency matrix

For every major subsystem relevant to the current/next gate include:

| Subsystem | Intended architecture | Observed state | Status |
|---|---|---|---|

Status:

- `PASS`
- `PARTIAL`
- `FAIL`
- `UNKNOWN`
- `NOT_YET_REQUIRED`

Do not add every imaginable future subsystem merely to fill the table.

---

## 6. Material findings

For every P0/P1/P2 finding state:

```text
ID:
Severity:
Truth:
Repository/path/component:
Exact evidence:
Current-phase relevance:
Why it matters:
Affected workstreams:
Required correction:
Must decide now?:
```

`Current-phase relevance` must use:

- `CURRENT_GATE`
- `NEXT_GATE`
- `FUTURE_CONSTRAINT`
- `FUTURE_ONLY`

For every material architecture recommendation also answer within the relevant fields:

- exact downstream work blocked;
- consequence of deferral;
- evidence that could supersede the recommendation;
- what remains deliberately undecided.

P3/NOTE findings may be shorter.

Do not duplicate one root cause across multiple findings.

---

## 7. Workstream decisions

For every active workstream:

```text
Workstream:
Issue:
PR/head:
Disposition:
Dependencies:
Reason:
Condition to continue:
```

If no PR exists, state that explicitly rather than inventing one.

---

## 8. Cross-workstream conflicts

List:

- ownership overlaps;
- shared-contract conflicts;
- incompatible assumptions;
- stale bases;
- invalid dependency ordering.

For each material conflict identify the affected workstreams and canonical contract owner.

If none:

`NO MATERIAL CROSS-WORKSTREAM CONFLICT FOUND`

---

## 9. Legacy contamination review

Report any materially relevant inheritance from Tibia/Canary/Crystal/OTClient/OTS.

For each case classify as:

- `JUSTIFIED_REFERENCE`
- `ACCEPTED_MIGRATION`
- `ACCEPTED_TARGET_DECISION`
- `UNJUSTIFIED_INHERITANCE`
- `UNKNOWN`

Do not classify similarity alone as inheritance.

---

## 10. Missing validation

List only validation that is:

- required now; or
- required before the next named gate.

Separate:

### Required now / before next gate

from:

### Future validation

Do not turn optional future test depth into a current blocker.

---

## 11. Top programme risks

Maximum 10.

Order by expected impact on the project, not by ease of fixing.

Do not list the same root cause repeatedly under different subsystem labels.

Distinguish:

- current realised risk;
- imminent next-gate risk;
- future constraint.

---

## 12. Immediate corrective actions

Provide the smallest ordered set of actions that most reduces programme risk.

Prefer:

1. restoring violated prerequisite invariants;
2. resolving shared-contract conflicts;
3. correcting unsafe workstream direction;
4. obtaining missing current-gate evidence;
5. only then adding further dependent implementation.

Prefer correcting a foundational error before adding more dependent code.

Do not create a giant speculative roadmap.

Do not implement the actions during this audit.

---

## 13. Next evidence-producing milestone

State:

- exact observable outcome;
- prerequisites;
- minimum evidence needed;
- what may safely remain deferred.

Prefer the smallest milestone capable of disproving the highest-risk current architectural assumptions.

Do not equate additional code volume with better evidence.

---

## 14. Final audit gate

Return exactly one:

`PROGRAMME_AUDIT = PASS`

`PROGRAMME_AUDIT = PASS_WITH_CORRECTIONS`

`PROGRAMME_AUDIT = FAIL`

`PROGRAMME_AUDIT = BLOCKED`

Mapping:

```text
ON_TRACK                  -> PASS
ON_TRACK_WITH_CORRECTIONS -> PASS_WITH_CORRECTIONS
AT_RISK                   -> FAIL
OFF_TRACK                 -> FAIL
INCONCLUSIVE              -> BLOCKED
```

### `PASS` requires:

- no open current-gate-relevant P0;
- no open current-gate-relevant P1;
- no material contradiction with accepted architecture affecting the current or next gate;
- no unverified prerequisite required by the current milestone;
- no material cross-workstream conflict affecting current progress;
- evidence sufficient for the claims being made.

### `PASS_WITH_CORRECTIONS`

May contain:

- P2;
- P3;
- NOTE;
- non-gate-blocking future risks;

provided they do not invalidate the present direction or next safe proof.

### `FAIL`

Means the project direction, current gate or active dependent implementation is materially unsafe or incorrect.

Do not return FAIL merely because:

- future work remains;
- a deliberately deferred subsystem is absent;
- optional validation has not yet been performed;
- the architecture could theoretically be made more elegant.

### `BLOCKED`

Means material evidence required for a reliable verdict could not be obtained.

Missing evidence for purely future concerns does not justify BLOCKED.

---

# 34. TERMINAL RULES

Do not implement fixes.

Do not modify repository state.

Do not create new architecture merely to make the audit look comprehensive.

Do not confuse:

- architecture with implementation;
- implementation with proof;
- proof with production readiness;
- merged state with proposed PR state;
- documentation with runtime truth;
- absence with failure;
- absence of evidence with evidence of absence;
- green CI with E2E;
- client/server agreement with independent protocol correctness;
- legacy behavior with target design;
- future requirements with current blockers;
- severity with evidence certainty;
- severity with gate impact;
- a historical test with exact-head validation;
- code volume with programme progress.

Do not penalize the programme because a future layer does not yet exist when current authoritative planning deliberately defers it.

Do penalize current implementation when it is already making an accepted future invariant materially unsafe or prohibitively expensive to restore.

When uncertain, state exactly:

- what remains unverified;
- why it matters;
- what evidence would resolve it;
- whether it affects the current gate.

Prefer a small number of strong, addressable findings over a large number of weak observations.

Prefer root-cause findings over symptom lists.

Prefer current exact-SHA evidence over stale summaries.

Prefer stopping unsafe dependent work over preserving sunk cost.

Prefer preserving delivery momentum when the architecture is sound.

The audit succeeds when it provides a trustworthy answer to:

**Are we building Oteryn Game correctly, in the correct order, with enough evidence to safely continue toward the next real native playable proof?**
